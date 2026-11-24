//! `item create|show|list|shape|dismiss|promote`: the work that is not a
//! task yet — an idea, a bug or a small feature, in `ITM*`.

use coding_crew_core::store::{ACTIVITY_FILE, by_number};
use coding_crew_core::{
    Actor, ActorKind, EventKind, Item, ItemError, ItemKind, ItemStatus, Note, Role, Store,
    StoreError,
};
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::project::Project;
use crate::write::{
    Call, NewEvent, append_journal, read_journal, required_text, text, title, work_file,
};

/// The creator agent, or the refusal. Every item write is the creator's.
fn creator(actor: &Actor, verb: &str) -> Result<(), Failure> {
    if actor.kind != ActorKind::Agent {
        return Err(Failure::new(
            Code::ActorNotAgent,
            format!("{verb}: only a creator agent writes items"),
        )
        .remedy("act as --role creator"));
    }
    if actor.role != Some(Role::Creator) {
        return Err(
            Failure::new(Code::RoleWrong, format!("{verb} needs the creator role"))
                .remedy("act as --role creator"),
        );
    }
    Ok(())
}

fn parse_kind(value: &str) -> Result<ItemKind, Failure> {
    Ok(match value {
        "idea" => ItemKind::Idea,
        "feature" => ItemKind::Feature,
        "bug" => ItemKind::Bug,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("kind `{other}` is not one of idea, feature, bug"),
            ));
        }
    })
}

fn parse_status(value: &str) -> Result<ItemStatus, Failure> {
    Ok(match value {
        "open" => ItemStatus::Open,
        "dismissed" => ItemStatus::Dismissed,
        "promoted" => ItemStatus::Promoted,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("status `{other}` is not one of open, dismissed, promoted"),
            ));
        }
    })
}

/// One item by id; an id no file holds is `item.not_found`.
fn load(store: &Store, id: &str) -> Result<Item, Failure> {
    store.load_item(id).map_err(|error| match error {
        StoreError::NotFound(_) => Failure::new(Code::ItemNotFound, format!("item {id} not found"))
            .remedy("talita item list"),
        other => other.into(),
    })
}

fn not_open(item: &Item) -> Failure {
    Failure::new(
        Code::ItemNotOpen,
        format!(
            "{} is {}; only an open item changes",
            item.id,
            item.status.as_str()
        ),
    )
}

fn item_failure(id: &str, error: ItemError) -> Failure {
    match error {
        ItemError::Settled(_) => Failure::new(Code::ItemNotOpen, format!("{id} is settled")),
        ItemError::UnknownKind => {
            Failure::new(Code::InputInvalid, "item shape takes feature or bug")
        }
        ItemError::NotPromotable(_) => Failure::new(
            Code::ItemIsIdea,
            "an idea carries no acceptance criteria, and a task without them can never be signed off",
        )
        .remedy(format!("talita item shape {id} --kind feature")),
    }
}

fn is_promotable(item: &Item) -> bool {
    item.status == ItemStatus::Open && item.kind.is_small_task()
}

/// A text list: each value is one flag value, `@path` and `@-` included.
fn texts(flag: &str, values: &[String]) -> Result<Vec<String>, Failure> {
    values
        .iter()
        .map(|value| text(flag, value))
        .collect::<Result<Vec<_>, _>>()
}

/// The file the item would get; `wrote` lists it relative to `coding-crew/`.
fn item_file(call: &Call, item: &Item) -> String {
    work_file(
        &call
            .project
            .store
            .root()
            .join(format!("{}-{}.json", item.id, item.slug)),
    )
}

/// `item create`: a new `ITM<nn>` in the next free id, `open`.
pub fn create(
    call: &Call,
    kind: &str,
    title_value: &str,
    body: Option<&str>,
    scope: &[String],
    tags: &[String],
) -> Result<Done, Failure> {
    creator(&call.actor, "item create")?;
    let title = title(title_value)?;
    let kind = parse_kind(kind)?;
    let body = body.map(|value| text("--body", value)).transpose()?;
    let scope = texts("--scope", scope)?;
    let tags = texts("--tag", tags)?;
    let lock = call.project.store.lock()?;
    let mut item = Item::new(lock.next_item_id()?, title);
    item.kind = kind;
    item.body = body.unwrap_or_default();
    item.scope = scope;
    item.tags = tags;
    lock.save_item(&item)?;
    Ok(Done {
        data: json!({
            "id": item.id,
            "title": item.title,
            "kind": item.kind.as_str(),
            "status": item.status.as_str(),
            "slug": item.slug,
        }),
        wrote: vec![item_file(call, &item)],
        journal: Vec::new(),
    })
}

/// `item show`: the item as stored, and whether it may be promoted.
pub fn show(project: &Project, id: &str) -> Result<Done, Failure> {
    let item = load(&project.store, id)?;
    let promotable = is_promotable(&item);
    Ok(Done::data(json!({
        "item": item,
        "derived": { "promotable": promotable },
    })))
}

/// `item list`: summaries in numeric id order, filtered by status and kind.
pub fn list(project: &Project, statuses: &[String], kind: Option<&str>) -> Result<Done, Failure> {
    let statuses = statuses
        .iter()
        .map(|value| parse_status(value))
        .collect::<Result<Vec<_>, _>>()?;
    let kind = kind.map(parse_kind).transpose()?;
    let mut items = project.store.list_items()?;
    items.sort_by(|a, b| by_number(&a.id, &b.id));
    let rows: Vec<Value> = items
        .iter()
        .filter(|item| statuses.is_empty() || statuses.contains(&item.status))
        .filter(|item| kind.is_none_or(|kind| item.kind == kind))
        .map(|item| {
            json!({
                "id": item.id,
                "title": item.title,
                "kind": item.kind.as_str(),
                "status": item.status.as_str(),
                "promoted_to": item.promoted_to,
            })
        })
        .collect();
    Ok(Done::data(Value::Array(rows)))
}

/// `item shape`: an idea becomes a small task; the id does not change.
pub fn shape(call: &Call, id: &str, kind: &str, scope: &[String]) -> Result<Done, Failure> {
    creator(&call.actor, "item shape")?;
    let kind = parse_kind(kind)?;
    if kind == ItemKind::Idea {
        return Err(Failure::new(
            Code::InputInvalid,
            "item shape takes feature or bug; an idea stays an idea",
        ));
    }
    let scope = texts("--scope", scope)?;
    let lock = call.project.store.lock()?;
    let mut item = load(&lock, id)?;
    if item.status != ItemStatus::Open {
        return Err(not_open(&item));
    }
    if item.kind.is_small_task() {
        return Err(Failure::new(
            Code::ItemAlreadyShaped,
            format!(
                "{} is already a small task ({})",
                item.id,
                item.kind.as_str()
            ),
        )
        .remedy(format!("talita item promote {id}")));
    }
    item.shape(kind, scope)
        .map_err(|error| item_failure(id, error))?;
    lock.save_item(&item)?;
    Ok(Done {
        data: json!({
            "id": item.id,
            "kind": item.kind.as_str(),
            "scope": item.scope,
        }),
        wrote: vec![item_file(call, &item)],
        journal: Vec::new(),
    })
}

/// `item dismiss`: `dismissed`, keeping the reason.
pub fn dismiss(call: &Call, id: &str, reason: &str) -> Result<Done, Failure> {
    creator(&call.actor, "item dismiss")?;
    let reason = required_text("--reason", reason)?;
    let lock = call.project.store.lock()?;
    let mut item = load(&lock, id)?;
    if item.status != ItemStatus::Open {
        return Err(not_open(&item));
    }
    item.dismiss(reason)
        .map_err(|error| item_failure(id, error))?;
    lock.save_item(&item)?;
    Ok(Done {
        data: json!({
            "id": item.id,
            "status": item.status.as_str(),
            "dismissed_reason": item.dismissed_reason,
        }),
        wrote: vec![item_file(call, &item)],
        journal: Vec::new(),
    })
}

/// `item promote`: the item is written first, then the task, so a crash
/// cannot produce two tasks from one item.
pub fn promote(call: &Call, id: &str) -> Result<Done, Failure> {
    creator(&call.actor, "item promote")?;
    let lock = call.project.store.lock()?;
    let mut journal = read_journal(&lock)?;
    let mut item = load(&lock, id)?;
    if item.status != ItemStatus::Open {
        return Err(not_open(&item));
    }
    if item.kind == ItemKind::Idea {
        return Err(Failure::new(
            Code::ItemIsIdea,
            format!(
                "{} is an idea; it carries no acceptance criteria, and a task without them can never be signed off",
                item.id
            ),
        )
        .remedy(format!("talita item shape {id} --kind feature")));
    }
    let mut task = item
        .promote(lock.next_id()?)
        .map_err(|error| item_failure(id, error))?;
    task.notes.push(Note {
        id: "NOTE-01".to_string(),
        author: call.actor.id.clone(),
        body: format!("from {}", item.id),
        at: coding_crew_core::now_rfc3339(),
    });
    let item_path = item_file(call, &item);
    lock.save_item(&item)?;
    lock.save(&mut task)?;
    let mut wrote = vec![item_path, work_file(&lock.task_path(&task))];
    let kinds = append_journal(
        &lock,
        &mut journal,
        &task.id,
        &call.actor,
        &call.ran,
        &coding_crew_core::now_rfc3339(),
        vec![NewEvent::note(
            EventKind::Created,
            format!("from {}", item.id),
        )],
    )?;
    if !kinds.is_empty() {
        wrote.push(ACTIVITY_FILE.to_string());
    }
    Ok(Done {
        data: json!({
            "item": &item,
            "task": {
                "id": task.id,
                "status": task.status.as_str(),
                "kind": task.kind.as_str(),
                "promoted_from": task.promoted_from,
            },
        }),
        wrote,
        journal: kinds,
    })
}
