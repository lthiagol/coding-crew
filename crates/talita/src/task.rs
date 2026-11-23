//! `task create|show|list|retitle|describe|prioritize|estimate|tag|depend|undepend`.

use std::collections::{BTreeMap, BTreeSet};

use coding_crew_core::claim::claim_failures;
use coding_crew_core::lifecycle::{TransitionContext, check};
use coding_crew_core::store::by_number;
use coding_crew_core::task::{Effort, Priority, slugify};
use coding_crew_core::{Actor, ActorKind, Event, Role, Section, Status, Task, TaskKind};
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::project::Project;
use crate::write::{
    Call, own, refuse_cancelled, refuse_frozen, require_pre_approval, text, title, work_file,
};

/// The record's word for an enum value.
pub fn word<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(s)) => s,
        _ => String::new(),
    }
}

/// A creator agent, or the refusal.
fn creator(actor: &Actor) -> Result<(), Failure> {
    if actor.kind != ActorKind::Agent {
        return Err(
            Failure::new(Code::ActorNotAgent, "only a creator agent creates tasks")
                .remedy("a creator agent with --instructed-by records a human's decision"),
        );
    }
    if actor.role != Some(Role::Creator) {
        return Err(
            Failure::new(Code::RoleWrong, "task create needs the creator role")
                .remedy("act as --role creator"),
        );
    }
    Ok(())
}

pub fn parse_kind(value: &str) -> Result<TaskKind, Failure> {
    Ok(match value {
        "task" => TaskKind::Task,
        "bug" => TaskKind::Bug,
        "feature" => TaskKind::Feature,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("kind `{other}` is not one of task, bug, feature"),
            ));
        }
    })
}

pub fn parse_status(value: &str) -> Result<Status, Failure> {
    serde_json::from_value(json!(value)).map_err(|_| {
        Failure::new(
            Code::InputInvalid,
            format!(
                "status `{value}` is not one of draft, groomed, approved, executing, code-review, remediating, needs-human, done"
            ),
        )
    })
}

/// `task create`: a new task in `draft`; with `--parent`, the child `TSKnn.m`.
pub fn create(
    project: &Project,
    actor: &Actor,
    title_value: &str,
    kind: Option<&str>,
    parent: Option<&str>,
) -> Result<Done, Failure> {
    creator(actor)?;
    let title = title(title_value)?;
    let kind = kind.map(parse_kind).transpose()?.unwrap_or_default();
    let lock = project.store.lock()?;
    let mut wrote = Vec::new();
    let mut parent_task = match parent {
        Some(id) => {
            let task = lock.load(id)?;
            refuse_cancelled(&task)?;
            Some(task)
        }
        None => None,
    };
    let id = match &parent_task {
        Some(p) => next_child_id(project, p)?,
        None => lock.next_id()?,
    };
    let mut task = Task::new(id, title);
    task.kind = kind;
    if let Some(p) = &parent_task {
        task.parent = Some(p.id.clone());
    }
    lock.save(&mut task)?;
    wrote.push(work_file(&lock.task_path(&task)));
    if let Some(p) = &mut parent_task {
        p.children.push(task.id.clone());
        p.updated = coding_crew_core::now_rfc3339();
        lock.save(p)?;
        wrote.push(work_file(&lock.task_path(p)));
    }
    Ok(Done {
        data: json!({
            "id": task.id,
            "revision": task.revision,
            "title": task.title,
            "kind": word(&task.kind),
            "status": word(&task.status),
            "parent": task.parent,
        }),
        wrote,
        journal: Vec::new(),
    })
}

/// One past the highest child number the parent lists or the folder holds.
fn next_child_id(project: &Project, parent: &Task) -> Result<String, Failure> {
    let prefix = format!("{}.", parent.id);
    let mut highest = 0u32;
    let number = |id: &str| id.strip_prefix(&prefix).and_then(|n| n.parse::<u32>().ok());
    for child in &parent.children {
        highest = highest.max(number(child).unwrap_or(0));
    }
    if let Ok(entries) = std::fs::read_dir(project.store.root()) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some((id, _)) = name.split_once('-') {
                highest = highest.max(number(id).unwrap_or(0));
            }
        }
    }
    Ok(format!("{prefix}{}", highest + 1))
}

/// `task show`: the file as stored, and what core derives from it.
pub fn show(project: &Project, id: &str) -> Result<Done, Failure> {
    let task = project.store.load(id)?;
    let tasks = project.store.list()?;
    let statuses: BTreeMap<String, Status> =
        tasks.iter().map(|t| (t.id.clone(), t.status)).collect();
    let done = |dep: &str| statuses.get(dep) == Some(&Status::Done);
    let now = coding_crew_core::now_rfc3339();

    let criteria: BTreeMap<&str, String> = task
        .acceptance_criteria
        .iter()
        .map(|c| (c.id.as_str(), word(&c.effective_status())))
        .collect();
    let children_done = task
        .children
        .iter()
        .filter(|c| statuses.get(c.as_str()) == Some(&Status::Done))
        .count();
    Ok(Done::data(json!({
        "task": task,
        "derived": {
            "criteria": criteria,
            "claimable": claim_failures(&task, Some(&done), &now).is_empty(),
            "start_readiness": start_readiness(&task, &done),
            "children_progress": { "done": children_done, "total": task.children.len() },
        },
    })))
}

/// Would `start` be accepted now, and if not, which gates refuse it. Judged
/// for the claim's holder; with no claim, `task.claimed` fails.
fn start_readiness(task: &Task, done: &dyn Fn(&str) -> bool) -> Value {
    let mut gates: Vec<Value> = Vec::new();
    if task.status != Status::Approved {
        gates.push(json!({
            "gate": "status",
            "detail": format!("the task is {}, not approved", task.status.as_str()),
        }));
    }
    let holder = Actor::agent(
        task.claim
            .as_ref()
            .map(|c| c.by.clone())
            .unwrap_or_default(),
        Role::Runner,
    );
    let ctx = TransitionContext {
        dependency_done: Some(done),
        reason: None,
    };
    for failure in check(task, Event::Start, &holder, &ctx) {
        gates.push(json!({ "gate": failure.gate, "detail": failure.message }));
    }
    json!({ "ok": gates.is_empty(), "gates": gates })
}

/// The filters of `task list`.
pub struct ListFilter {
    pub statuses: Vec<String>,
    pub kind: Option<String>,
    pub claimable: bool,
    pub tags: Vec<String>,
}

/// `task list`: summaries in numeric id order.
pub fn list(project: &Project, filter: &ListFilter) -> Result<Done, Failure> {
    let statuses: Vec<Status> = filter
        .statuses
        .iter()
        .map(|s| parse_status(s))
        .collect::<Result<_, _>>()?;
    let kind = filter.kind.as_deref().map(parse_kind).transpose()?;
    let mut tasks = project.store.list()?;
    tasks.sort_by(|a, b| by_number(&a.id, &b.id));
    let done_ids: BTreeSet<String> = tasks
        .iter()
        .filter(|t| t.status == Status::Done)
        .map(|t| t.id.clone())
        .collect();
    let done = |dep: &str| done_ids.contains(dep);
    let now = coding_crew_core::now_rfc3339();
    let rows: Vec<Value> = tasks
        .iter()
        .filter(|t| statuses.is_empty() || statuses.contains(&t.status))
        .filter(|t| kind.is_none_or(|k| t.kind == k))
        .filter(|t| filter.tags.iter().all(|tag| t.meta.tags.contains(tag)))
        .filter_map(|t| {
            let claimable = claim_failures(t, Some(&done), &now).is_empty();
            if filter.claimable && !claimable {
                return None;
            }
            let mut row = json!({
                "id": t.id,
                "title": t.title,
                "status": word(&t.status),
                "overlays": t.overlays,
                "kind": word(&t.kind),
                "claimable": claimable,
            });
            if let Some(claim) = &t.claim {
                row["claim"] = json!(claim);
            }
            Some(row)
        })
        .collect();
    Ok(Done::data(Value::Array(rows)))
}

/// `task retitle`: title, slug, and the file's name.
pub fn retitle(call: &Call, id: &str, value: &str) -> Result<Done, Failure> {
    let title = title(value)?;
    call.task(id, |task, actor| {
        own(Section::Intent, actor, task)?;
        require_pre_approval(task, "task retitle")?;
        task.slug = slugify(&title);
        task.title = title;
        Ok(json!({ "title": task.title, "slug": task.slug }))
    })
}

/// `task describe`: the intent's outcome and problem.
pub fn describe(
    call: &Call,
    id: &str,
    outcome: Option<&str>,
    problem: Option<&str>,
) -> Result<Done, Failure> {
    if outcome.is_none() && problem.is_none() {
        return Err(Failure::new(
            Code::UsageInvalid,
            "task describe needs --outcome or --problem",
        ));
    }
    let outcome = outcome.map(|v| text("--outcome", v)).transpose()?;
    let problem = problem.map(|v| text("--problem", v)).transpose()?;
    call.task(id, |task, actor| {
        own(Section::Intent, actor, task)?;
        refuse_frozen(task, Section::Intent)?;
        if let Some(outcome) = outcome {
            task.intent.outcome = outcome;
        }
        if let Some(problem) = problem {
            task.intent.problem = problem;
        }
        Ok(json!({ "intent": task.intent }))
    })
}

fn meta_write(
    call: &Call,
    id: &str,
    verb: &str,
    change: impl FnOnce(&mut Task) -> Value,
) -> Result<Done, Failure> {
    call.task(id, |task, actor| {
        own(Section::Meta, actor, task)?;
        require_pre_approval(task, verb)?;
        Ok(change(task))
    })
}

pub fn prioritize(call: &Call, id: &str, value: &str) -> Result<Done, Failure> {
    let priority: Priority = serde_json::from_value(json!(value)).map_err(|_| {
        Failure::new(
            Code::InputInvalid,
            format!("priority `{value}` is not one of urgent, high, normal, low"),
        )
    })?;
    meta_write(call, id, "task prioritize", |task| {
        task.meta.priority = priority;
        json!({ "priority": word(&priority) })
    })
}

pub fn estimate(call: &Call, id: &str, value: &str) -> Result<Done, Failure> {
    let effort: Effort = serde_json::from_value(json!(value)).map_err(|_| {
        Failure::new(
            Code::InputInvalid,
            format!("effort `{value}` is not one of S, M, L"),
        )
    })?;
    meta_write(call, id, "task estimate", |task| {
        task.meta.effort = effort;
        json!({ "effort": word(&effort) })
    })
}

pub fn tag(call: &Call, id: &str, add: &[String], remove: &[String]) -> Result<Done, Failure> {
    if add.is_empty() && remove.is_empty() {
        return Err(Failure::new(
            Code::UsageInvalid,
            "task tag needs --add or --remove",
        ));
    }
    if add.iter().chain(remove).any(|t| t.trim().is_empty()) {
        return Err(Failure::new(Code::InputInvalid, "a tag is empty"));
    }
    meta_write(call, id, "task tag", |task| {
        for t in add {
            if !task.meta.tags.contains(t) {
                task.meta.tags.push(t.clone());
            }
        }
        task.meta.tags.retain(|t| !remove.contains(t));
        json!({ "tags": task.meta.tags })
    })
}

/// `depend` and `undepend` run in draft, groomed, and approved while unclaimed.
fn dependable(task: &Task, verb: &str) -> Result<(), Failure> {
    let ok = match task.status {
        Status::Draft | Status::Groomed => true,
        Status::Approved => task.claim.is_none(),
        _ => false,
    };
    if ok {
        return Ok(());
    }
    let claimed = if task.claim.is_some() {
        ", claimed"
    } else {
        ""
    };
    Err(Failure::new(
        Code::StatusWrong,
        format!(
            "{verb} runs in draft, groomed, or approved while unclaimed; {} is {}{claimed}",
            task.id,
            task.status.as_str()
        ),
    ))
}

pub fn depend(call: &Call, id: &str, on: &[String]) -> Result<Done, Failure> {
    if on.is_empty() {
        return Err(Failure::new(Code::UsageInvalid, "task depend needs --on"));
    }
    call.task(id, |task, actor| {
        own(Section::DependsOn, actor, task)?;
        dependable(task, "task depend")?;
        // The graph is read under the lock, so a concurrent depend cannot
        // slip a cycle past the check.
        let tasks = call.project.store.list()?;
        let mut edges: BTreeMap<String, Vec<String>> = tasks
            .iter()
            .map(|t| (t.id.clone(), t.depends_on.clone()))
            .collect();
        for dep in on {
            if !edges.contains_key(dep) {
                call.project.store.load(dep)?;
            }
        }
        for dep in on {
            if !task.depends_on.contains(dep) {
                task.depends_on.push(dep.clone());
            }
        }
        edges.insert(task.id.clone(), task.depends_on.clone());
        if let Some(path) = cycle_from(&task.id, &edges) {
            return Err(Failure::new(
                Code::DependsCycle,
                format!("the dependency closes a cycle: {}", path.join(" → ")),
            ));
        }
        Ok(json!({ "depends_on": task.depends_on }))
    })
}

/// A path from `start` back to itself through `edges`, if one exists.
fn cycle_from(start: &str, edges: &BTreeMap<String, Vec<String>>) -> Option<Vec<String>> {
    fn walk(
        at: &str,
        start: &str,
        edges: &BTreeMap<String, Vec<String>>,
        seen: &mut BTreeSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        for next in edges.get(at).into_iter().flatten() {
            path.push(next.clone());
            if next == start {
                return true;
            }
            if seen.insert(next.clone()) && walk(next, start, edges, seen, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = vec![start.to_string()];
    let mut seen = BTreeSet::new();
    walk(start, start, edges, &mut seen, &mut path).then_some(path)
}

pub fn undepend(call: &Call, id: &str, on: &[String]) -> Result<Done, Failure> {
    if on.is_empty() {
        return Err(Failure::new(Code::UsageInvalid, "task undepend needs --on"));
    }
    call.task(id, |task, actor| {
        own(Section::DependsOn, actor, task)?;
        dependable(task, "task undepend")?;
        for dep in on {
            if !task.depends_on.contains(dep) {
                return Err(Failure::new(
                    Code::PartNotFound,
                    format!("{} does not depend on {dep}", task.id),
                ));
            }
        }
        task.depends_on.retain(|d| !on.contains(d));
        Ok(json!({ "depends_on": task.depends_on }))
    })
}
