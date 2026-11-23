//! What every task write shares: identity, the lock, the revision check,
//! cancellation, ownership, the frozen sections, and text values.

use std::io::Read;

use coding_crew_core::ownership::authorize;
use coding_crew_core::store::WORK_DIR;
use coding_crew_core::{Actor, OwnershipError, Section, Status, Task};
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

impl Call<'_> {
    /// Load `id` under the project lock, run `change` on it, and save it.
    /// `change` returns the verb's `data`; `id` and `revision` are added.
    pub fn task(
        &self,
        id: &str,
        change: impl FnOnce(&mut Task, &Actor) -> Result<Value, Failure>,
    ) -> Result<Done, Failure> {
        let lock = self.project.store.lock()?;
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
        let mut data = change(&mut task, &self.actor)?;
        task.updated = coding_crew_core::now_rfc3339();
        lock.save(&mut task)?;
        if let Value::Object(map) = &mut data {
            map.insert("id".into(), json!(task.id));
            map.insert("revision".into(), json!(task.revision));
        }
        Ok(Done {
            data,
            wrote: vec![work_file(&lock.task_path(&task))],
            journal: Vec::new(),
        })
    }
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
