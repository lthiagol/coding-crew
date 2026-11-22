//! The committed runtime record tells a run the fixtures can hold.
//!
//! The record is provisional, but the mock must still tell the fixtures'
//! history: the queue is the run's tasks in order, with the statuses the
//! fixtures hold; the crew holds the task in flight, which is the one the
//! round is on; and the budget and the findings are that task's own.

use std::collections::HashMap;
use std::path::PathBuf;

use coding_crew_core::runtime::{AgentState, Composition, Severities};
use coding_crew_core::{
    Activity, FindingStatus, FlowRole, RoundKind, Runtime, Severity, Status, Store, Task,
    parse_rfc3339,
};

/// The task the run has in flight, and the run's queue in order.
const IN_FLIGHT: &str = "TSK14";
const RUN_QUEUE: [&str; 5] = ["TSK22", "TSK25", "TSK14", "TSK29", "TSK07"];

fn project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("mock-data/coding-crew")
}

fn store() -> Store {
    Store::new(project().join("work"))
}

fn runtime() -> Runtime {
    Runtime::read(&project().join("runtime.json")).expect("the runtime record")
}

fn tasks() -> HashMap<String, Task> {
    store()
        .list()
        .expect("the tasks")
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect()
}

fn moment(text: &str) -> i64 {
    parse_rfc3339(text).unwrap_or_else(|| panic!("unreadable stamp {text}"))
}

#[test]
fn the_record_sits_beside_the_journal_and_is_not_a_task() {
    let path = project().join("runtime.json");
    assert!(path.is_file(), "{} is not a file", path.display());
    assert!(
        !project().join("work/runtime.json").exists(),
        "the runtime record is inside the work folder"
    );
    assert_eq!(store().runtime_path(), path);
    // The store never reads it as a work unit.
    assert!(
        store()
            .list()
            .expect("the tasks")
            .iter()
            .all(|task| task.id != "runtime")
    );
}

#[test]
fn the_crew_holds_the_task_in_flight_and_each_hold_is_the_moment_its_task_names() {
    let runtime = runtime();
    let tasks = tasks();
    let task = &tasks[IN_FLIGHT];
    assert_eq!(task.status, Status::CodeReview, "{IN_FLIGHT}");

    // The flow is sequential: the builder and the reviewer both hold the task
    // in flight. The builder waits between its rounds, and the reviewer works.
    let builder = runtime
        .crew
        .iter()
        .find(|member| member.role == FlowRole::Builder)
        .expect("the builder");
    assert_eq!(builder.holds.as_deref(), Some(IN_FLIGHT));
    assert_eq!(builder.state, AgentState::Idle);
    let claim = task.claim.as_ref().expect("the builder's claim");
    assert!(claim.by.starts_with("runner-"), "{}", claim.by);
    assert_eq!(builder.since.as_deref(), Some(claim.at.as_str()));

    let reviewer = runtime
        .crew
        .iter()
        .find(|member| member.role == FlowRole::Reviewer)
        .expect("the reviewer");
    assert_eq!(reviewer.holds.as_deref(), Some(IN_FLIGHT));
    assert_eq!(reviewer.state, AgentState::Reviewing);
    assert_eq!(
        reviewer.since.as_deref(),
        Some(task.status_since.as_str()),
        "the reviewer took the review when it was requested"
    );

    let orchestrator = runtime
        .crew
        .iter()
        .find(|member| member.role == FlowRole::Orchestrator)
        .expect("the orchestrator");
    assert_eq!(orchestrator.holds, None);
    assert_eq!(orchestrator.state, AgentState::Idle);

    // Nothing else is held, and the crew is one per role.
    let roles: Vec<FlowRole> = runtime.crew.iter().map(|member| member.role).collect();
    assert_eq!(
        roles,
        [
            FlowRole::Orchestrator,
            FlowRole::Builder,
            FlowRole::Reviewer
        ],
        "the crew is not one per role"
    );
    assert_eq!(runtime.composition(), Composition::OnePerRole);
    assert_eq!(
        Composition::OnePerRole.says(),
        "one per role · the reviewer reviews"
    );
}

#[test]
fn the_queue_is_the_run_in_order_with_the_fixtures_statuses() {
    let runtime = runtime();
    let tasks = tasks();
    let queued: Vec<&str> = runtime.queue.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(queued, RUN_QUEUE, "the run's queue, in order");
    for item in &runtime.queue {
        let task = tasks
            .get(&item.id)
            .unwrap_or_else(|| panic!("{} names no task", item.id));
        assert_eq!(item.status, task.status, "{} status", item.id);
    }

    // Two are done, one is held by the crew, and two wait.
    let done: Vec<&str> = runtime
        .queue
        .iter()
        .filter(|item| item.status == Status::Done)
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(done, ["TSK22", "TSK25"]);
    let waiting: Vec<&str> = runtime.waiting().map(|item| item.id.as_str()).collect();
    assert_eq!(waiting, ["TSK29", "TSK07"]);
    for id in &waiting {
        let task = &tasks[*id];
        assert_eq!(task.status, Status::Approved, "{id}");
        assert!(task.overlays.blocked.is_none(), "{id} is blocked");
        assert!(!runtime.is_held(id), "{id} is held");
    }
    assert!(runtime.is_held(IN_FLIGHT));
    assert!(
        !runtime.waiting().any(|item| item.id == IN_FLIGHT),
        "a held task is in the crew, not the queue"
    );
}

#[test]
fn the_round_is_the_task_in_flights_next_review_after_the_rounds_before_it() {
    let runtime = runtime();
    let round = runtime.round.as_ref().expect("a review in flight");
    assert_eq!(round.task, IN_FLIGHT);
    assert_eq!(round.stage, RoundKind::External);
    assert_eq!(round.number, 2, "the second round");
    assert_eq!(
        round.earlier.len() as u32 + 1,
        round.number,
        "the rounds before it are named"
    );
    assert_eq!(
        round.earlier,
        [coding_crew_core::runtime::EarlierRound {
            number: 1,
            raised: 3,
        }],
        "round 1 raised three findings"
    );
    let reviewer = runtime
        .crew
        .iter()
        .find(|member| member.role == FlowRole::Reviewer)
        .expect("the reviewer");
    assert_eq!(reviewer.holds.as_deref(), Some(round.task.as_str()));
}

#[test]
fn the_budget_is_the_rounds_the_task_in_flight_has_used_of_its_limit() {
    let runtime = runtime();
    let tasks = tasks();
    let task = &tasks[IN_FLIGHT];
    assert_eq!(
        runtime.budget.spent,
        runtime.round.as_ref().expect("a round").number
    );
    assert_eq!(runtime.budget.total, task.review.max_rounds);
    assert!(runtime.budget.spent < runtime.budget.total);
}

#[test]
fn the_findings_are_the_task_in_flights_open_findings_by_their_own_severity() {
    let runtime = runtime();
    let tasks = tasks();
    let task = &tasks[IN_FLIGHT];
    assert_eq!(runtime.findings, task.open_findings() as u32);
    assert!(
        runtime.findings > 0,
        "the task in flight has an open finding"
    );

    let mut counted = Severities::default();
    for finding in task
        .review
        .findings
        .iter()
        .filter(|finding| finding.status == FindingStatus::Open)
    {
        match finding.severity {
            Severity::High => counted.high += 1,
            Severity::Medium => counted.medium += 1,
            Severity::Low => counted.low += 1,
        }
    }
    let severities = runtime
        .severities
        .expect("the mock holds the severity breakdown");
    assert_eq!(severities, counted);
    assert_eq!(
        severities.high + severities.medium + severities.low,
        runtime.findings
    );
}

#[test]
fn the_run_began_before_its_first_claim_and_no_event_falls_between() {
    let runtime = runtime();
    let tasks = tasks();
    let started = moment(&runtime.started_at);
    let holds: Vec<i64> = runtime
        .crew
        .iter()
        .filter_map(|member| member.since.as_deref())
        .map(moment)
        .collect();
    assert!(!holds.is_empty(), "no agent holds a task");
    assert!(
        holds.iter().all(|at| started < *at),
        "{}",
        runtime.started_at
    );

    // The run starts half an hour before the first task it claims.
    let first = tasks[RUN_QUEUE[0]]
        .claim
        .as_ref()
        .expect("the first task's claim")
        .at
        .clone();
    assert_eq!(moment(&runtime.started_at) + 30 * 60, moment(&first));

    // No fixture event falls between the run's start and its first claim: the
    // run begins after the events that came before it, as the journal does.
    let journal = Activity::read(&project().join("activity.json")).expect("the journal");
    for event in journal.events {
        let at = moment(&event.at);
        assert!(
            at <= started || at >= moment(&first),
            "{} happens between the run's start and its first claim",
            event.at
        );
    }
}
