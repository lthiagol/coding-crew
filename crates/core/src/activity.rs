//! The project's activity journal: what happened, in the record's order.
//!
//! The kinds are provisional: nothing writes the file yet, and the writer
//! settles them. The home is `coding-crew/activity.json`, beside `work/`.

use serde::{Deserialize, Serialize};

/// What happened to a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Created,
    Approved,
    Claimed,
    ReviewRequested,
    FindingRaised,
    Remediated,
    Escalated,
    Completed,
    Commented,
}

impl EventKind {
    /// The word the journal stores and the Overview shows.
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Created => "created",
            EventKind::Approved => "approved",
            EventKind::Claimed => "claimed",
            EventKind::ReviewRequested => "review_requested",
            EventKind::FindingRaised => "finding_raised",
            EventKind::Remediated => "remediated",
            EventKind::Escalated => "escalated",
            EventKind::Completed => "completed",
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
    /// One line of free text, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The journal, as the file holds it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    #[serde(default)]
    pub events: Vec<ActivityEvent>,
}

impl Activity {
    /// Whether the journal holds no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
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
    fn an_empty_journal_has_no_events() {
        let activity: Activity = serde_json::from_str("{}").expect("a journal");
        assert!(activity.is_empty());
    }

    #[test]
    fn every_kind_keeps_its_stored_word() {
        let words = [
            (EventKind::Created, "created"),
            (EventKind::Approved, "approved"),
            (EventKind::Claimed, "claimed"),
            (EventKind::ReviewRequested, "review_requested"),
            (EventKind::FindingRaised, "finding_raised"),
            (EventKind::Remediated, "remediated"),
            (EventKind::Escalated, "escalated"),
            (EventKind::Completed, "completed"),
            (EventKind::Commented, "commented"),
        ];
        for (kind, word) in words {
            assert_eq!(kind.as_str(), word);
            let json = serde_json::to_string(&kind).expect("a kind");
            assert_eq!(json, format!("\"{word}\""));
        }
    }
}
