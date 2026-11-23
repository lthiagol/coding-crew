//! `journal show`: the project's record of moves, oldest first.

use coding_crew_core::{Activity, ActivityError, ActivityEvent, EventKind};
use serde_json::json;

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::project::Project;

/// The filters of `journal show`. An empty `kinds` keeps every kind.
pub struct Filter<'a> {
    pub task: Option<&'a str>,
    pub kinds: &'a [String],
    pub since: Option<&'a str>,
    pub limit: Option<usize>,
}

/// `journal show`: the events oldest first, and how many this reader could
/// not read. A project with no journal has an empty one.
pub fn show(project: &Project, filter: &Filter<'_>) -> Result<Done, Failure> {
    let kinds = filter
        .kinds
        .iter()
        .map(|kind| parse_kind(kind))
        .collect::<Result<Vec<_>, _>>()?;
    let since = filter.since.map(parse_since).transpose()?;
    let journal = read(project)?;
    let unreadable = journal.unreadable;
    let mut events: Vec<ActivityEvent> = journal
        .events
        .into_iter()
        .filter(|event| filter.task.is_none_or(|task| event.task == task))
        .filter(|event| kinds.is_empty() || kinds.contains(&event.kind))
        .filter(|event| {
            since.is_none_or(|since| {
                coding_crew_core::parse_rfc3339(&event.at).is_some_and(|at| at >= since)
            })
        })
        .collect();
    if let Some(limit) = filter.limit
        && events.len() > limit
    {
        events.drain(..events.len() - limit);
    }
    Ok(Done::data(json!({
        "events": events,
        "unreadable": unreadable,
    })))
}

/// The journal, or an empty one when the project has none.
fn read(project: &Project) -> Result<Activity, Failure> {
    match Activity::read(&project.store.activity_path()) {
        Ok(journal) => Ok(journal),
        Err(ActivityError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Activity::default())
        }
        Err(error) => Err(Failure::new(Code::StoreUnreadable, error.to_string())),
    }
}

fn parse_kind(value: &str) -> Result<EventKind, Failure> {
    serde_json::from_value(json!(value)).map_err(|_| {
        Failure::new(
            Code::InputInvalid,
            format!("kind `{value}` is not a journal kind"),
        )
    })
}

fn parse_since(value: &str) -> Result<i64, Failure> {
    coding_crew_core::parse_rfc3339(value).ok_or_else(|| {
        Failure::new(
            Code::InputInvalid,
            format!("--since `{value}` is not an RFC 3339 instant"),
        )
    })
}
