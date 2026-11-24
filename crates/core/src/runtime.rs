//! The flow's runtime record: what the automation is doing, and who does it.
//!
//! The record is **provisional**: nothing writes the file yet, and the writer
//! settles the shape when the flow lands. It lives beside the journal at
//! `coding-crew/runtime.json`, not in the work folder, for the journal's
//! reason: the work folder holds one file per work unit.
//!
//! What it holds is settled in `docs/tasks/task-model.md` § 11.2: the run's
//! start and the record's last write, the crew, the run's queue, the task in
//! flight and its rounds, the round budget, the open findings and the stamp.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::actor::FlowRole;
use crate::lifecycle::Status;
use crate::task::RoundKind;

/// The record, as the file holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runtime {
    /// When this run began, RFC 3339.
    pub started_at: String,
    /// The record's last write, RFC 3339. Absent on a record written before
    /// the field: it has no known last write, and is never stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// One entry per agent of the flow.
    #[serde(default)]
    pub crew: Vec<CrewMember>,
    /// The run's tasks, in queue order: an id and a status, never the task's
    /// content. A held task is in the queue too; [`Runtime::waiting`] is the
    /// part the pane offers.
    #[serde(default)]
    pub queue: Vec<QueueItem>,
    /// The review in flight, when there is one: the task in flight and its
    /// rounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<RuntimeRound>,
    /// The rounds the task in flight has used, of its own limit.
    #[serde(default)]
    pub budget: Budget,
    /// The task in flight's open findings, as a count.
    #[serde(default)]
    pub findings: u32,
    /// The task in flight's open findings by severity, when the record holds
    /// them.
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

    /// The work waiting in the queue, in order: neither held by the crew nor
    /// done. This is what the pane draws and the orchestrator is handed.
    pub fn waiting(&self) -> impl Iterator<Item = &QueueItem> {
        self.queue
            .iter()
            .filter(|item| !self.is_held(&item.id) && item.status != Status::Done)
    }

    /// The waiting work an agent could claim now, in order: the part of the
    /// queue the pane lets a human pick.
    pub fn claimable(&self) -> impl Iterator<Item = &QueueItem> {
        self.waiting().filter(|item| item.claimable)
    }

    /// How the crew's size composes into who reviews.
    pub fn composition(&self) -> Composition {
        composition(&self.crew)
    }
}

/// One agent of the flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewMember {
    pub role: FlowRole,
    /// The model it runs, as its configuration names it.
    pub model: String,
    /// The actor id it acts as in task files, so a task verb finds the row.
    /// hugo does not read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
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
    /// The agent holds its task and cannot go on: it is not working, and the
    /// pane says so on the warning slot.
    Blocked,
}

impl AgentState {
    /// The word a screen writes.
    pub fn as_str(self) -> &'static str {
        match self {
            AgentState::Idle => "idle",
            AgentState::Executing => "executing",
            AgentState::Reviewing => "reviewing",
            AgentState::Blocked => "blocked",
        }
    }

    /// Whether the agent is doing something: executing or reviewing. A blocked
    /// agent holds its task and does nothing with it.
    pub fn is_working(self) -> bool {
        matches!(self, AgentState::Executing | AgentState::Reviewing)
    }
}

/// One item of the unclaimed queue: an id and a status, never a title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub status: Status,
    /// Whether an agent could claim the task now: the task model's claim rule,
    /// derived by the writer. A file that leaves it out says it is not, so
    /// nothing is offered that the record does not vouch for.
    #[serde(default)]
    pub claimable: bool,
}

/// The review in flight: the task in flight, its stage and its round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRound {
    /// The task under review.
    pub task: String,
    /// The stage the review is at.
    pub stage: RoundKind,
    /// The round's number.
    pub number: u32,
    /// The rounds before this one, on the same task, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub earlier: Vec<EarlierRound>,
}

/// A round the task in flight has finished: its build and its review, and the
/// findings the review raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EarlierRound {
    /// The round's number.
    pub number: u32,
    /// The findings the review raised, open or since resolved.
    pub raised: u32,
}

/// The rounds the task in flight has used, of its round limit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub spent: u32,
    pub total: u32,
}

/// The task in flight's open findings by severity. The total is the record's
/// `findings`.
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
/// The size is a composition, not a number: the crew is the roles present.
/// Three agents are one per role and the reviewer reviews; an orchestrator and
/// a builder, or the orchestrator alone, and the orchestrator reviews. A crew
/// with more than one agent in a role has no rule for who reviews, and is
/// named as what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Composition {
    /// Exactly one builder, one reviewer and one orchestrator.
    OnePerRole,
    /// A reviewer is in the crew, no role is doubled, and the three roles
    /// are not all there.
    ReviewerReviews,
    /// No reviewer and no doubled role: the orchestrator reviews.
    OrchestratorReviews,
    /// No reviewer, no orchestrator and no doubled role: the human reviews.
    /// The configuration cannot produce this crew, since the orchestrator is
    /// always present; a runtime record with no orchestrator is named so.
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
    let count = |role: FlowRole| crew.iter().filter(|member| member.role == role).count();
    let orchestrators = count(FlowRole::Orchestrator);
    let builders = count(FlowRole::Builder);
    let reviewers = count(FlowRole::Reviewer);
    if orchestrators == 1 && builders == 1 && reviewers == 1 {
        Composition::OnePerRole
    } else if orchestrators > 1 || builders > 1 || reviewers > 1 {
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

    fn member(role: FlowRole, holds: Option<&str>, state: AgentState) -> CrewMember {
        CrewMember {
            role,
            model: "opencode-go/model#high".to_string(),
            actor: None,
            holds: holds.map(str::to_string),
            since: holds.map(|_| "2026-11-16T12:00Z".to_string()),
            state,
        }
    }

    fn record(crew: Vec<CrewMember>, queue: Vec<QueueItem>) -> Runtime {
        Runtime {
            started_at: "2026-11-16T12:00Z".to_string(),
            updated_at: None,
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
          "updated_at": "2026-11-16T13:40Z",
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
        assert_eq!(runtime.updated_at.as_deref(), Some("2026-11-16T13:40Z"));
        assert_eq!(runtime.crew[0].role, FlowRole::Orchestrator);
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
        // A record written before the field has no known last write.
        assert_eq!(runtime.updated_at, None);
        // The stamp carries no why when it runs.
        let json = serde_json::to_string(&runtime).expect("a record");
        assert!(!json.contains("why"), "{json}");
        // Nor does the record name a last write it does not hold.
        assert!(!json.contains("updated_at"), "{json}");
    }

    #[test]
    fn a_held_or_done_task_is_not_waiting() {
        // A held task belongs to the crew, not the queue, even when a writer
        // left it in both: the pane paints it once. A done task is in the
        // run's queue, and is not waiting either.
        let runtime = record(
            vec![member(
                FlowRole::Builder,
                Some("TSK10"),
                AgentState::Executing,
            )],
            vec![
                QueueItem {
                    id: "TSK22".to_string(),
                    status: Status::Done,
                    claimable: false,
                },
                QueueItem {
                    id: "TSK10".to_string(),
                    status: Status::Approved,
                    claimable: false,
                },
                QueueItem {
                    id: "TSK29".to_string(),
                    status: Status::Approved,
                    claimable: true,
                },
            ],
        );
        assert!(runtime.is_held("TSK10"));
        let waiting: Vec<&str> = runtime.waiting().map(|item| item.id.as_str()).collect();
        assert_eq!(waiting, ["TSK29"]);
    }

    #[test]
    fn a_waiting_task_the_record_does_not_call_claimable_is_not_offered() {
        // The queue shows what waits; only a claimable waiting task is offered
        // for a pick.
        let runtime = record(
            Vec::new(),
            vec![
                QueueItem {
                    id: "TSK07".to_string(),
                    status: Status::Approved,
                    claimable: false,
                },
                QueueItem {
                    id: "TSK29".to_string(),
                    status: Status::Approved,
                    claimable: true,
                },
            ],
        );
        let waiting: Vec<&str> = runtime.waiting().map(|item| item.id.as_str()).collect();
        assert_eq!(waiting, ["TSK07", "TSK29"]);
        let offered: Vec<&str> = runtime.claimable().map(|item| item.id.as_str()).collect();
        assert_eq!(offered, ["TSK29"]);
    }

    #[test]
    fn a_round_names_the_rounds_before_it_when_it_has_them() {
        let text = r#"{
          "started_at": "2026-11-16T12:00Z",
          "round": {
            "task": "TSK14", "stage": "external", "number": 2,
            "earlier": [{"number": 1, "raised": 3}]
          },
          "stamp": {"runs": true}
        }"#;
        let runtime: Runtime = serde_json::from_str(text).expect("a runtime record");
        let round = runtime.round.expect("a round");
        assert_eq!(
            round.earlier,
            [EarlierRound {
                number: 1,
                raised: 3,
            }]
        );
        // A first round has no earlier rounds, and the record leaves them out.
        let first = RuntimeRound {
            earlier: Vec::new(),
            ..round
        };
        let json = serde_json::to_string(&first).expect("a round");
        assert!(!json.contains("earlier"), "{json}");
    }

    #[test]
    fn three_agents_are_one_per_role_and_the_reviewer_reviews() {
        let crew = vec![
            member(FlowRole::Orchestrator, None, AgentState::Idle),
            member(FlowRole::Builder, Some("TSK10"), AgentState::Executing),
            member(FlowRole::Reviewer, Some("TSK13"), AgentState::Reviewing),
        ];
        assert_eq!(composition(&crew), Composition::OnePerRole);
        let says = Composition::OnePerRole.says();
        assert!(says.contains("one per role"), "{says}");
        assert!(says.contains("reviewer reviews"), "{says}");
    }

    #[test]
    fn two_agents_mean_the_orchestrator_reviews() {
        let crew = vec![
            member(FlowRole::Orchestrator, None, AgentState::Idle),
            member(FlowRole::Builder, Some("TSK10"), AgentState::Executing),
        ];
        assert_eq!(composition(&crew), Composition::OrchestratorReviews);
        assert_eq!(
            Composition::OrchestratorReviews.says(),
            "the orchestrator reviews"
        );
    }

    #[test]
    fn one_agent_means_the_human_reviews() {
        let crew = vec![member(
            FlowRole::Builder,
            Some("TSK10"),
            AgentState::Executing,
        )];
        assert_eq!(composition(&crew), Composition::HumanReviews);
        assert_eq!(Composition::HumanReviews.says(), "the human reviews");
    }

    #[test]
    fn a_reviewer_reviews_without_the_three_roles() {
        let crew = vec![
            member(FlowRole::Builder, Some("TSK10"), AgentState::Executing),
            member(FlowRole::Reviewer, Some("TSK13"), AgentState::Reviewing),
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
                member(FlowRole::Orchestrator, None, AgentState::Idle),
                member(FlowRole::Builder, Some("TSK10"), AgentState::Executing),
                member(FlowRole::Builder, Some("TSK11"), AgentState::Executing),
                member(FlowRole::Reviewer, Some("TSK13"), AgentState::Reviewing),
            ],
            vec![
                member(FlowRole::Builder, Some("TSK10"), AgentState::Executing),
                member(FlowRole::Builder, Some("TSK11"), AgentState::Executing),
                member(FlowRole::Reviewer, Some("TSK13"), AgentState::Reviewing),
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
    fn a_blocked_agent_is_not_working() {
        // Blocked holds its task but does nothing with it: no spinner, no
        // working state.
        assert!(!AgentState::Blocked.is_working());
    }

    #[test]
    fn a_blocked_agent_keeps_its_stored_word() {
        let json = serde_json::to_string(&AgentState::Blocked).expect("a state");
        assert_eq!(json, "\"blocked\"");
        let read: AgentState = serde_json::from_str("\"blocked\"").expect("the stored word");
        assert_eq!(read, AgentState::Blocked);
    }

    #[test]
    fn a_queue_item_the_record_leaves_unclaimable_is_not_offered() {
        // A record written before the flag, or by a writer that leaves it out,
        // names no task an agent may take.
        let item: QueueItem =
            serde_json::from_str(r#"{"id":"TSK01","status":"approved"}"#).expect("an item");
        assert!(!item.claimable);
        let item: QueueItem =
            serde_json::from_str(r#"{"id":"TSK01","status":"approved","claimable":true}"#)
                .expect("an item");
        assert!(item.claimable);
    }

    #[test]
    fn every_agent_state_keeps_its_stored_word() {
        for (state, word) in [
            (AgentState::Idle, "idle"),
            (AgentState::Executing, "executing"),
            (AgentState::Reviewing, "reviewing"),
            (AgentState::Blocked, "blocked"),
        ] {
            assert_eq!(state.as_str(), word);
            let json = serde_json::to_string(&state).expect("a state");
            assert_eq!(json, format!("\"{word}\""));
        }
    }
}
