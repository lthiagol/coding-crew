//! The project's activity journal: what happened, in the record's order.
//!
//! The kinds are provisional: nothing writes the file yet, and the writer
//! settles them. The home is `coding-crew/activity.json`, beside `work/`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What happened to a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Created,
    Groomed,
    Approved,
    Claimed,
    Released,
    Started,
    RoundFiled,
    ReviewRequested,
    FindingRaised,
    FindingResolved,
    ChangesRequested,
    Remediated,
    Escalated,
    Completed,
    Resumed,
    Reopened,
    Blocked,
    Unblocked,
    Deferred,
    Undeferred,
    Cancelled,
    Commented,
}

impl EventKind {
    /// The word the journal stores and the Overview shows.
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Created => "created",
            EventKind::Groomed => "groomed",
            EventKind::Approved => "approved",
            EventKind::Claimed => "claimed",
            EventKind::Released => "released",
            EventKind::Started => "started",
            EventKind::RoundFiled => "round_filed",
            EventKind::ReviewRequested => "review_requested",
            EventKind::FindingRaised => "finding_raised",
            EventKind::FindingResolved => "finding_resolved",
            EventKind::ChangesRequested => "changes_requested",
            EventKind::Remediated => "remediated",
            EventKind::Escalated => "escalated",
            EventKind::Completed => "completed",
            EventKind::Resumed => "resumed",
            EventKind::Reopened => "reopened",
            EventKind::Blocked => "blocked",
            EventKind::Unblocked => "unblocked",
            EventKind::Deferred => "deferred",
            EventKind::Undeferred => "undeferred",
            EventKind::Cancelled => "cancelled",
            EventKind::Commented => "commented",
        }
    }
}

/// One line of the journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// When it happened, RFC 3339.
    pub at: String,
    pub kind: EventKind,
    /// The task id it happened to.
    pub task: String,
    /// The actor's role or kind: `runner`, `reviewer`, `human`, `orchestrator`.
    pub by: String,
    /// The model that ran it, when the call named one (ADR 0018).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The harness that ran it, when the call named one (ADR 0018).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    /// One line of free text, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The journal, as the file holds it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    #[serde(default)]
    pub events: Vec<ActivityEvent>,
    /// The events the journal holds that this reader could not read: an
    /// unknown kind, or a missing field. They are counted, not fatal.
    #[serde(default, skip)]
    pub unreadable: usize,
}

/// The journal before its events are read: the file's own list.
#[derive(Deserialize)]
struct RawActivity {
    #[serde(default)]
    events: Vec<serde_json::Value>,
}

impl Activity {
    /// Read a journal from a file.
    ///
    /// One bad event does not blank the history: the reader keeps the events
    /// it can read and counts the ones it cannot.
    pub fn read(path: &std::path::Path) -> Result<Self, ActivityError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json(&text)
    }

    /// Read a journal from its JSON text.
    pub fn from_json(text: &str) -> Result<Self, ActivityError> {
        let raw: RawActivity = serde_json::from_str(text)?;
        let mut events = Vec::new();
        let mut unreadable = 0;
        for value in raw.events {
            match serde_json::from_value(value) {
                Ok(event) => events.push(event),
                Err(_) => unreadable += 1,
            }
        }
        Ok(Activity { events, unreadable })
    }

    /// Whether the journal holds no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Whether the history is there to read.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty() || self.unreadable > 0
    }
}

/// Why a journal could not be read.
///
/// A file that cannot be read and a file that does not parse are different
/// failures, as they are for the runtime record.
#[derive(Debug, Error)]
pub enum ActivityError {
    #[error("failed to read the journal: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse the journal: {0}")]
    Parse(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_journal_reads_its_events_in_file_order() {
        let text = r#"{
          "events": [
            {"at": "2026-11-01T09:00Z", "kind": "created", "task": "TSK01", "by": "human"},
            {"at": "2026-11-02T09:00Z", "kind": "finding_raised", "task": "TSK01", "by": "reviewer", "note": "raised FIND-01"}
          ]
        }"#;
        let activity: Activity = serde_json::from_str(text).expect("a journal");
        assert_eq!(activity.events.len(), 2);
        assert_eq!(activity.events[0].kind, EventKind::Created);
        assert!(activity.events[0].note.is_none());
        assert_eq!(activity.events[1].note.as_deref(), Some("raised FIND-01"));
        assert_eq!(activity.events[1].kind.as_str(), "finding_raised");
    }

    #[test]
    fn an_event_without_a_model_or_a_harness_reads_and_keys_neither() {
        let event: ActivityEvent = serde_json::from_str(
            r#"{"at": "2026-11-01T09:00Z", "kind": "created", "task": "TSK01", "by": "human"}"#,
        )
        .expect("an event");
        assert!(event.model.is_none());
        assert!(event.harness.is_none());
        let json = serde_json::to_value(&event).expect("an event");
        assert!(json.get("model").is_none(), "{json}");
        assert!(json.get("harness").is_none(), "{json}");
    }

    #[test]
    fn an_event_with_a_model_and_a_harness_round_trips() {
        let event = ActivityEvent {
            at: "2026-11-01T09:00Z".to_string(),
            kind: EventKind::Completed,
            task: "TSK01".to_string(),
            by: "runner".to_string(),
            model: Some("deepseek-v4.1-flash".to_string()),
            harness: Some("opencode".to_string()),
            note: None,
        };
        let text = serde_json::to_string(&event).expect("an event");
        let read: ActivityEvent = serde_json::from_str(&text).expect("an event");
        assert_eq!(read, event);
    }

    #[test]
    fn an_empty_journal_has_no_events() {
        let activity: Activity = serde_json::from_str("{}").expect("a journal");
        assert!(activity.is_empty());
    }

    #[test]
    fn every_kind_keeps_its_stored_word() {
        let words = [
            (EventKind::Created, "created"),
            (EventKind::Groomed, "groomed"),
            (EventKind::Approved, "approved"),
            (EventKind::Claimed, "claimed"),
            (EventKind::Released, "released"),
            (EventKind::Started, "started"),
            (EventKind::RoundFiled, "round_filed"),
            (EventKind::ReviewRequested, "review_requested"),
            (EventKind::FindingRaised, "finding_raised"),
            (EventKind::FindingResolved, "finding_resolved"),
            (EventKind::ChangesRequested, "changes_requested"),
            (EventKind::Remediated, "remediated"),
            (EventKind::Escalated, "escalated"),
            (EventKind::Completed, "completed"),
            (EventKind::Resumed, "resumed"),
            (EventKind::Reopened, "reopened"),
            (EventKind::Blocked, "blocked"),
            (EventKind::Unblocked, "unblocked"),
            (EventKind::Deferred, "deferred"),
            (EventKind::Undeferred, "undeferred"),
            (EventKind::Cancelled, "cancelled"),
            (EventKind::Commented, "commented"),
        ];
        for (kind, word) in words {
            assert_eq!(kind.as_str(), word);
            let json = serde_json::to_string(&kind).expect("a kind");
            assert_eq!(json, format!("\"{word}\""));
        }
    }

    #[test]
    fn a_journal_that_does_not_parse_is_a_parse_failure() {
        let dir =
            std::env::temp_dir().join(format!("coding-crew-broken-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("activity.json");
        std::fs::write(&path, "{ not json").expect("a broken journal");
        let error = Activity::read(&path).expect_err("a broken journal");
        assert!(matches!(&error, ActivityError::Parse(_)), "{error:?}");
        assert!(error.to_string().contains("parse"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_event_is_counted_and_the_readable_events_stay() {
        let text = r#"{
          "events": [
            {"at": "2026-11-01T09:00Z", "kind": "created", "task": "TSK01", "by": "human"},
            {"at": "2026-11-02T09:00Z", "kind": "not_a_kind", "task": "TSK01", "by": "human"},
            {"at": "2026-11-03T09:00Z", "kind": "claimed", "by": "runner"},
            {"at": "2026-11-04T09:00Z", "kind": "completed", "task": "TSK01", "by": "reviewer"}
          ]
        }"#;
        let activity = Activity::from_json(text).expect("a journal with bad events");
        assert_eq!(activity.events.len(), 2);
        assert_eq!(activity.events[0].kind, EventKind::Created);
        assert_eq!(activity.events[1].kind, EventKind::Completed);
        assert_eq!(activity.unreadable, 2);
        assert!(activity.has_events());
        assert!(!activity.is_empty());
    }

    #[test]
    fn a_journal_whose_events_are_not_a_list_does_not_parse() {
        let error = Activity::from_json(r#"{"events": "not a list"}"#)
            .expect_err("the journal has no list of events");
        assert!(matches!(&error, ActivityError::Parse(_)), "{error:?}");
    }

    #[test]
    fn a_journal_that_is_not_there_is_an_io_failure() {
        let path = std::env::temp_dir()
            .join(format!("coding-crew-no-journal-{}", std::process::id()))
            .join("activity.json");
        let error = Activity::read(&path).expect_err("no journal");
        assert!(matches!(&error, ActivityError::Io(_)), "{error:?}");
        assert!(error.to_string().contains("read"), "{error}");
    }

    #[test]
    fn a_parse_failure_is_a_parse_failure_in_both_readers() {
        // The journal's reader and the runtime record's reader agree in
        // kind: a file that does not parse is a parse failure in both.
        let dir =
            std::env::temp_dir().join(format!("coding-crew-parse-failure-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let journal = dir.join("activity.json");
        let record = dir.join("runtime.json");
        std::fs::write(&journal, "{").expect("a broken journal");
        std::fs::write(&record, "{").expect("a broken record");
        let journal = Activity::read(&journal).expect_err("a broken journal");
        let record = crate::runtime::Runtime::read(&record).expect_err("a broken runtime record");
        assert!(matches!(&journal, ActivityError::Parse(_)), "{journal:?}");
        assert!(
            matches!(&record, crate::RuntimeError::Parse(_)),
            "{record:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
