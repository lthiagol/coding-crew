//! The flow's runtime record: what the automation is doing, and who does it.
//!
//! The record is **provisional**: nothing writes the file yet, and the writer
//! settles the shape when the flow lands. It lives beside the journal at
//! `coding-crew/runtime.json`, not in the work folder, for the journal's
//! reason: the work folder holds one file per work unit.
//!
//! What it holds is settled in `docs/tasks/task-model.md` § 11.2: the run's
//! start, the crew, the unclaimed queue, the review in flight, the round
//! budget, the open findings and the stamp.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::actor::Role;
use crate::lifecycle::Status;
use crate::task::RoundKind;

/// The record, as the file holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runtime {
    /// When this run began, RFC 3339.
    pub started_at: String,
    /// One entry per agent of the flow.
    #[serde(default)]
    pub crew: Vec<CrewMember>,
    /// The unclaimed work: an id and a status, never the task's content.
    #[serde(default)]
    pub queue: Vec<QueueItem>,
    /// The review in flight, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<RuntimeRound>,
    #[serde(default)]
    pub budget: Budget,
    /// The open findings, as a count.
    #[serde(default)]
    pub findings: u32,
    /// The open findings by severity, when the record holds them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severities: Option<Severities>,
    /// Whether the automation runs, and why when it does not.
    pub stamp: Stamp,
}

impl Runtime {
    /// Read a record from a file.
    pub fn read(path: &Path) -> Result<Self, RuntimeError> {
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Whether an agent of the crew holds a task.
    ///
    /// A held task belongs to the crew, not the queue: the queue paints the
    /// unclaimed work only, even when a writer left a held id in it.
    pub fn is_held(&self, id: &str) -> bool {
        self.crew
            .iter()
            .any(|member| member.holds.as_deref() == Some(id))
    }

    /// The queue as the pane paints it: the unclaimed work.
    pub fn unheld_queue(&self) -> impl Iterator<Item = &QueueItem> {
        self.queue.iter().filter(|item| !self.is_held(&item.id))
    }

    /// How the crew's size composes into who reviews.
    pub fn composition(&self) -> Composition {
        composition(&self.crew)
    }
}

/// One agent of the flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewMember {
    pub role: Role,
    /// The model it runs, as its configuration names it.
    pub model: String,
    /// The task it holds, when it holds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holds: Option<String>,
    /// When it took the task it holds, RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    #[serde(default)]
    pub state: AgentState,
}

/// What an agent of the crew is doing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    #[default]
    Idle,
    Executing,
    Reviewing,
}

impl AgentState {
    /// The word a screen writes.
    pub fn as_str(self) -> &'static str {
        match self {
            AgentState::Idle => "idle",
            AgentState::Executing => "executing",
            AgentState::Reviewing => "reviewing",
        }
    }

    /// Whether the agent is doing something: executing or reviewing.
    pub fn is_working(self) -> bool {
        !matches!(self, AgentState::Idle)
    }
}

/// One item of the unclaimed queue: an id and a status, never a title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub status: Status,
}

/// The review in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRound {
    /// The task under review.
    pub task: String,
    /// The stage the review is at.
    pub stage: RoundKind,
    /// The round's number.
    pub number: u32,
}

/// The external rounds spent of the round budget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub spent: u32,
    pub total: u32,
}

/// The open findings by severity. The total is the record's `findings`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Severities {
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

/// Whether the automation runs, and why when it does not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub runs: bool,
    /// Why it does not run. Present when `runs` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// How the crew's size composes into who reviews.
///
/// The size is a composition, not a number: three agents are one per role
/// and the reviewer reviews; two are an orchestrator and a worker, and the
/// orchestrator reviews; one is a worker, and the human reviews — an agent
/// cannot review itself. A crew with more than one agent in a role has no
/// rule for who reviews, and is named as what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Composition {
    /// Exactly one runner, one reviewer and one orchestrator.
    OnePerRole,
    /// A reviewer is in the crew, no role is doubled, and the three roles
    /// are not all there.
    ReviewerReviews,
    /// No reviewer and no doubled role: the orchestrator reviews.
    OrchestratorReviews,
    /// No reviewer, no orchestrator and no doubled role: the human reviews.
    HumanReviews,
    /// A role is filled more than once: no rule says who reviews.
    MoreThanOnePerRole,
}

impl Composition {
    /// The composition in words, as the pane writes it.
    ///
    /// It names who reviews; it is never a bare count of agents.
    pub fn says(self) -> &'static str {
        match self {
            Composition::OnePerRole => "one per role · the reviewer reviews",
            Composition::ReviewerReviews => "the reviewer reviews",
            Composition::OrchestratorReviews => "the orchestrator reviews",
            Composition::HumanReviews => "the human reviews",
            Composition::MoreThanOnePerRole => "more than one agent per role",
        }
    }
}

/// The composition of a crew.
///
/// The rule is the roles', not the count's: a reviewer reviews when there is
/// one; without a reviewer the orchestrator does; without either, the human
/// does. A crew with exactly one agent in each role is stated as one per
/// role. A crew with more than one agent in a role is not one per role, and
/// has no rule: it is named as what it is.
pub fn composition(crew: &[CrewMember]) -> Composition {
    let count = |role: Role| crew.iter().filter(|member| member.role == role).count();
    let orchestrators = count(Role::Orchestrator);
    let runners = count(Role::Runner);
    let reviewers = count(Role::Reviewer);
    if orchestrators == 1 && runners == 1 && reviewers == 1 {
        Composition::OnePerRole
    } else if orchestrators > 1 || runners > 1 || reviewers > 1 {
        Composition::MoreThanOnePerRole
    } else if reviewers == 1 {
        Composition::ReviewerReviews
    } else if orchestrators == 1 {
        Composition::OrchestratorReviews
    } else {
        Composition::HumanReviews
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to read the runtime record: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse the runtime record: {0}")]
    Parse(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(role: Role, holds: Option<&str>, state: AgentState) -> CrewMember {
        CrewMember {
            role,
            model: "opencode-go/model#high".to_string(),
            holds: holds.map(str::to_string),
            since: holds.map(|_| "2026-11-16T12:00Z".to_string()),
            state,
        }
    }

    fn record(crew: Vec<CrewMember>, queue: Vec<QueueItem>) -> Runtime {
        Runtime {
            started_at: "2026-11-16T12:00Z".to_string(),
            crew,
            queue,
            round: None,
            budget: Budget { spent: 0, total: 5 },
            findings: 0,
            severities: None,
            stamp: Stamp {
                runs: true,
                why: None,
            },
        }
    }

    #[test]
    fn a_record_reads_its_parts_in_file_order() {
        let text = r#"{
          "started_at": "2026-11-16T12:00Z",
          "crew": [
            {"role": "orchestrator", "model": "opencode-go/glm-5.3#high", "state": "idle"},
            {"role": "runner", "model": "opencode-go/deepseek-v4.1-flash#high",
             "holds": "TSK10", "since": "2026-11-16T12:00Z", "state": "executing"}
          ],
          "queue": [{"id": "TSK29", "status": "approved"}],
          "round": {"task": "TSK13", "stage": "external", "number": 2},
          "budget": {"spent": 18, "total": 45},
          "findings": 21,
          "stamp": {"runs": false, "why": "the review budget is spent"}
        }"#;
        let runtime: Runtime = serde_json::from_str(text).expect("a runtime record");
        assert_eq!(runtime.crew.len(), 2);
        assert_eq!(runtime.crew[0].role, Role::Orchestrator);
        assert_eq!(runtime.crew[0].holds, None);
        assert_eq!(runtime.crew[1].state, AgentState::Executing);
        assert_eq!(runtime.queue[0].status, Status::Approved);
        let round = runtime.round.expect("a round");
        assert_eq!(round.stage, RoundKind::External);
        assert_eq!(round.number, 2);
        assert_eq!(runtime.budget.spent, 18);
        assert_eq!(runtime.findings, 21);
        assert!(!runtime.stamp.runs);
        assert_eq!(
            runtime.stamp.why.as_deref(),
            Some("the review budget is spent")
        );
    }

    #[test]
    fn the_optional_parts_are_absent_not_empty() {
        let text = r#"{
          "started_at": "2026-11-16T12:00Z",
          "stamp": {"runs": true}
        }"#;
        let runtime: Runtime = serde_json::from_str(text).expect("a runtime record");
        assert!(runtime.crew.is_empty());
        assert!(runtime.queue.is_empty());
        assert_eq!(runtime.round, None);
        assert_eq!(runtime.findings, 0);
        // The stamp carries no why when it runs.
        let json = serde_json::to_string(&runtime).expect("a record");
        assert!(!json.contains("why"), "{json}");
    }

    #[test]
    fn a_held_task_is_not_in_the_queue() {
        // A held task belongs to the crew, not the queue, even when a writer
        // left it in both: the pane paints it once.
        let runtime = record(
            vec![member(Role::Runner, Some("TSK10"), AgentState::Executing)],
            vec![
                QueueItem {
                    id: "TSK10".to_string(),
                    status: Status::Approved,
                },
                QueueItem {
                    id: "TSK29".to_string(),
                    status: Status::Approved,
                },
            ],
        );
        assert!(runtime.is_held("TSK10"));
        let queue: Vec<&str> = runtime
            .unheld_queue()
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(queue, ["TSK29"]);
    }

    #[test]
    fn three_agents_are_one_per_role_and_the_reviewer_reviews() {
        let crew = vec![
            member(Role::Orchestrator, None, AgentState::Idle),
            member(Role::Runner, Some("TSK10"), AgentState::Executing),
            member(Role::Reviewer, Some("TSK13"), AgentState::Reviewing),
        ];
        assert_eq!(composition(&crew), Composition::OnePerRole);
        let says = Composition::OnePerRole.says();
        assert!(says.contains("one per role"), "{says}");
        assert!(says.contains("reviewer reviews"), "{says}");
    }

    #[test]
    fn two_agents_mean_the_orchestrator_reviews() {
        let crew = vec![
            member(Role::Orchestrator, None, AgentState::Idle),
            member(Role::Runner, Some("TSK10"), AgentState::Executing),
        ];
        assert_eq!(composition(&crew), Composition::OrchestratorReviews);
        assert_eq!(
            Composition::OrchestratorReviews.says(),
            "the orchestrator reviews"
        );
    }

    #[test]
    fn one_agent_means_the_human_reviews() {
        let crew = vec![member(Role::Runner, Some("TSK10"), AgentState::Executing)];
        assert_eq!(composition(&crew), Composition::HumanReviews);
        assert_eq!(Composition::HumanReviews.says(), "the human reviews");
    }

    #[test]
    fn a_reviewer_reviews_without_the_three_roles() {
        let crew = vec![
            member(Role::Runner, Some("TSK10"), AgentState::Executing),
            member(Role::Reviewer, Some("TSK13"), AgentState::Reviewing),
        ];
        assert_eq!(composition(&crew), Composition::ReviewerReviews);
        assert_eq!(Composition::ReviewerReviews.says(), "the reviewer reviews");
    }

    #[test]
    fn a_crew_with_a_role_filled_twice_is_named_as_what_it_is() {
        // Four agents are not one per role, and neither are three with a
        // duplicated role: no rule says who reviews them.
        for crew in [
            vec![
                member(Role::Orchestrator, None, AgentState::Idle),
                member(Role::Runner, Some("TSK10"), AgentState::Executing),
                member(Role::Runner, Some("TSK11"), AgentState::Executing),
                member(Role::Reviewer, Some("TSK13"), AgentState::Reviewing),
            ],
            vec![
                member(Role::Runner, Some("TSK10"), AgentState::Executing),
                member(Role::Runner, Some("TSK11"), AgentState::Executing),
                member(Role::Reviewer, Some("TSK13"), AgentState::Reviewing),
            ],
        ] {
            assert_eq!(composition(&crew), Composition::MoreThanOnePerRole);
        }
        assert_eq!(
            Composition::MoreThanOnePerRole.says(),
            "more than one agent per role"
        );
    }

    #[test]
    fn a_crew_with_no_agent_leaves_the_review_to_the_human() {
        assert_eq!(composition(&[]), Composition::HumanReviews);
    }

    #[test]
    fn severities_are_optional_and_the_count_stays_the_total() {
        let text = r#"{
          "started_at": "2026-11-16T12:00Z",
          "findings": 17,
          "severities": {"high": 8, "medium": 8, "low": 1},
          "stamp": {"runs": true}
        }"#;
        let runtime: Runtime = serde_json::from_str(text).expect("a runtime record");
        assert_eq!(runtime.findings, 17);
        assert_eq!(
            runtime.severities,
            Some(Severities {
                high: 8,
                medium: 8,
                low: 1,
            })
        );
        // A record that holds no breakdown says so by leaving it out.
        let plain = record(Vec::new(), Vec::new());
        let json = serde_json::to_string(&plain).expect("a record");
        assert!(!json.contains("severities"), "{json}");
    }

    #[test]
    fn an_agent_that_executes_or_reviews_is_working() {
        assert!(!AgentState::Idle.is_working());
        assert!(AgentState::Executing.is_working());
        assert!(AgentState::Reviewing.is_working());
    }

    #[test]
    fn every_agent_state_keeps_its_stored_word() {
        for (state, word) in [
            (AgentState::Idle, "idle"),
            (AgentState::Executing, "executing"),
            (AgentState::Reviewing, "reviewing"),
        ] {
            assert_eq!(state.as_str(), word);
            let json = serde_json::to_string(&state).expect("a state");
            assert_eq!(json, format!("\"{word}\""));
        }
    }
}
