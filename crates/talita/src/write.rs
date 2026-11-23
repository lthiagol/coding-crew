//! What every task write shares: identity, the lock, the revision check,
//! cancellation, ownership, the frozen sections, and text values.

use std::io::Read;

use coding_crew_core::ownership::authorize;
use coding_crew_core::store::{ACTIVITY_FILE, WORK_DIR};
use coding_crew_core::{
    Activity, ActivityError, ActivityEvent, Actor, EventKind, OwnershipError, ProjectLock, Section,
    Status, StoreError, Task,
};
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::project::Project;

/// One write call: the project, the writer, and `--expect-revision`.
pub struct Call<'a> {
    pub project: &'a Project,
    pub actor: Actor,
    pub expect_revision: Option<u32>,
}

/// An event a verb appends, before the write path stamps its task, actor and
/// instant.
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub kind: EventKind,
    pub note: Option<String>,
}

impl NewEvent {
    /// An event with no note.
    pub fn new(kind: EventKind) -> Self {
        Self { kind, note: None }
    }

    /// An event with the note ADR 0018 names.
    pub fn note(kind: EventKind, note: impl Into<String>) -> Self {
        Self {
            kind,
            note: Some(note.into()),
        }
    }
}

impl Call<'_> {
    /// Load `id` under the project lock, run `change` on it, and save it.
    /// `change` returns the verb's `data`; `id` and `revision` are added.
    pub fn task(
        &self,
        id: &str,
        change: impl FnOnce(&mut Task, &Actor) -> Result<Value, Failure>,
    ) -> Result<Done, Failure> {
        self.task_journaled(id, |task, actor| Ok((change(task, actor)?, Vec::new())))
    }

    /// The same write, appending the given events to the journal in the same
    /// locked call (ADR 0018). `change` returns the verb's `data` and its
    /// events.
    pub fn task_journaled(
        &self,
        id: &str,
        change: impl FnOnce(&mut Task, &Actor) -> Result<(Value, Vec<NewEvent>), Failure>,
    ) -> Result<Done, Failure> {
        let lock = self.project.store.lock()?;
        let mut journal = read_journal(&lock)?;
        let mut task = lock.load(id)?;
        if let Some(expected) = self.expect_revision
            && expected != task.revision
        {
            return Err(Failure::new(
                Code::ConflictRevision,
                format!(
                    "task revision {expected} is stale; the file holds {}",
                    task.revision
                ),
            )
            .remedy(format!("talita task show {id}")));
        }
        refuse_cancelled(&task)?;
        let (mut data, events) = change(&mut task, &self.actor)?;
        let now = coding_crew_core::now_rfc3339();
        task.updated = now.clone();
        lock.save(&mut task)?;
        let mut wrote = vec![work_file(&lock.task_path(&task))];
        let kinds = append_journal(&lock, &mut journal, &task.id, &self.actor, &now, events)?;
        if !kinds.is_empty() {
            wrote.push(ACTIVITY_FILE.to_string());
        }
        if let Value::Object(map) = &mut data {
            map.insert("id".into(), json!(task.id));
            map.insert("revision".into(), json!(task.revision));
        }
        Ok(Done {
            data,
            wrote,
            journal: kinds,
        })
    }
}

/// The project's journal under the lock.
///
/// A journal that does not parse, or that holds events this reader cannot
/// read, refuses the whole call before anything is written: rewriting it
/// would lose the events it cannot read (ADR 0018).
pub fn read_journal(lock: &ProjectLock<'_>) -> Result<Activity, Failure> {
    match Activity::read(&lock.activity_path()) {
        Ok(journal) if journal.unreadable > 0 => Err(Failure::from(StoreError::JournalUnreadable(
            journal.unreadable,
        ))),
        Ok(journal) => Ok(journal),
        Err(ActivityError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Activity::default())
        }
        Err(error) => Err(Failure::new(Code::StoreUnreadable, error.to_string())),
    }
}

/// Append a call's events to the journal and write it, atomically. Returns
/// the kinds appended, oldest first.
pub fn append_journal(
    lock: &ProjectLock<'_>,
    journal: &mut Activity,
    task: &str,
    actor: &Actor,
    at: &str,
    events: Vec<NewEvent>,
) -> Result<Vec<String>, Failure> {
    if events.is_empty() {
        return Ok(Vec::new());
    }
    let by = by(actor);
    let mut kinds = Vec::with_capacity(events.len());
    for event in events {
        kinds.push(event.kind.as_str().to_string());
        journal.events.push(ActivityEvent {
            at: at.to_string(),
            kind: event.kind,
            task: task.to_string(),
            by: by.clone(),
            note: event.note,
        });
    }
    lock.write_activity(journal)?;
    Ok(kinds)
}

/// The word the journal stores as `by`: the actor's role, or `human`
/// (ADR 0018).
fn by(actor: &Actor) -> String {
    actor
        .role
        .map(|role| role.as_str().to_string())
        .unwrap_or_else(|| "human".to_string())
}

/// A work file's path relative to `coding-crew/`.
pub fn work_file(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    format!("{WORK_DIR}/{name}")
}

/// A cancelled task refuses every later write.
pub fn refuse_cancelled(task: &Task) -> Result<(), Failure> {
    if let Some(cancelled) = &task.overlays.cancelled {
        return Err(Failure::new(
            Code::TaskCancelled,
            format!("{} is cancelled: {}", task.id, cancelled.reason),
        ));
    }
    Ok(())
}

/// Section ownership (roles.md), as talita's codes.
pub fn own(section: Section, actor: &Actor, task: &Task) -> Result<(), Failure> {
    authorize(section, actor, task.status).map_err(|error| match error {
        OwnershipError::NoOwner(_) => Failure::new(Code::Internal, error.to_string()),
        OwnershipError::NotAnAgent(_) => Failure::new(Code::ActorNotAgent, error.to_string())
            .remedy("an agent with the owning role writes it"),
        OwnershipError::NotOwned { role, .. } => {
            Failure::new(Code::SectionNotOwned, error.to_string())
                .remedy(format!("act as --role {}", role.as_str()))
        }
    })
}

/// Whether the task is before approval: `draft` or `groomed`.
pub fn pre_approval(task: &Task) -> bool {
    matches!(task.status, Status::Draft | Status::Groomed)
}

/// Intent, scope and criteria are frozen from `approved` on (ADR 0013); the
/// remedy is a child task.
pub fn refuse_frozen(task: &Task, section: Section) -> Result<(), Failure> {
    if pre_approval(task) {
        return Ok(());
    }
    Err(Failure::new(
        Code::SectionFrozen,
        format!(
            "{} of {} is frozen: the task is {}",
            section.as_str(),
            task.id,
            task.status.as_str()
        ),
    )
    .remedy(format!(
        "talita task create --parent {} --title <t>",
        task.id
    )))
}

/// A verb that runs only before approval, on a section that is not frozen.
pub fn require_pre_approval(task: &Task, verb: &str) -> Result<(), Failure> {
    if pre_approval(task) {
        return Ok(());
    }
    Err(Failure::new(
        Code::StatusWrong,
        format!(
            "{verb} runs in draft or groomed; {} is {}",
            task.id,
            task.status.as_str()
        ),
    ))
}

/// A text value: `@<path>` reads a file, `@-` stdin, anything else is itself.
/// Stored verbatim (ADR 0009).
pub fn text(flag: &str, value: &str) -> Result<String, Failure> {
    let read = if value == "@-" {
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map(|_| buffer)
            .map_err(|e| format!("reading stdin: {e}"))
    } else if let Some(path) = value.strip_prefix('@') {
        std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))
    } else {
        Ok(value.to_string())
    };
    read.map_err(|message| Failure::new(Code::InputInvalid, format!("{flag}: {message}")))
}

/// A text value that must not be empty.
pub fn required_text(flag: &str, value: &str) -> Result<String, Failure> {
    let text = text(flag, value)?;
    if text.trim().is_empty() {
        return Err(Failure::new(Code::InputInvalid, format!("{flag} is empty")));
    }
    Ok(text)
}

/// A title: one non-empty line.
pub fn title(value: &str) -> Result<String, Failure> {
    let title = required_text("--title", value)?;
    let title = title.strip_suffix('\n').unwrap_or(&title).to_string();
    if title.contains(['\n', '\r']) {
        return Err(Failure::new(Code::InputInvalid, "--title is one line"));
    }
    Ok(title)
}

/// The next per-task part id: `<prefix>-nn`, one past the highest held.
pub fn next_part_id<'a>(prefix: &str, held: impl Iterator<Item = &'a str>) -> String {
    let highest = held
        .filter_map(|id| {
            id.strip_prefix(prefix)?
                .strip_prefix('-')?
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    format!("{prefix}-{:02}", highest + 1)
}

/// A part the task does not hold.
pub fn part_not_found(task: &Task, part: &str) -> Failure {
    Failure::new(Code::PartNotFound, format!("{} has no {part}", task.id))
        .remedy(format!("talita task show {}", task.id))
}
