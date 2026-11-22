//! The committed runtime record is computable from the fixtures, in their time.
//!
//! The record is provisional, but the mock must still tell the fixtures'
//! history: every held task exists, the queue is the claimable work nobody
//! holds, the round is the review in flight, and the counts are the
//! fixtures' own.

use std::collections::HashMap;
use std::path::PathBuf;

use coding_crew_core::runtime::{AgentState, Composition, Severities};
use coding_crew_core::{
    Activity, FindingStatus, Role, Runtime, Severity, Status, Store, Task, parse_rfc3339,
};

/// The instant the fixtures are read with, as in `mock_data.rs`.
const NOW: &str = "2026-11-19T08:00:00Z";

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

/// Whether an agent could claim the task at the fixtures' clock.
fn claimable(task: &Task, tasks: &HashMap<String, Task>) -> bool {
    task.status == Status::Approved
        && task.overlays.blocked.is_none()
        && task.overlays.deferred.is_none()
        && task.overlays.cancelled.is_none()
        && task
            .depends_on
            .iter()
            .all(|id| tasks[id].status == Status::Done)
        && !task
            .claim
            .as_ref()
            .is_some_and(|claim| !claim.lease_expired(NOW))
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
fn every_held_task_exists_and_is_the_live_work_the_agent_owns() {
    let runtime = runtime();
    let tasks = tasks();
    for member in &runtime.crew {
        let Some(id) = &member.holds else {
            assert_eq!(member.state, AgentState::Idle, "{member:?}");
            assert!(member.since.is_none(), "{member:?}");
            continue;
        };
        let task = tasks
            .get(id)
            .unwrap_or_else(|| panic!("{} holds unknown task {id}", member.role.as_str()));
        match member.role {
            Role::Runner => {
                assert_eq!(task.status, Status::Executing, "{id}");
                let claim = task.claim.as_ref().expect("the runner's claim");
                assert!(claim.by.starts_with("runner-"), "{id}: {}", claim.by);
                assert_eq!(member.since.as_deref(), Some(claim.at.as_str()), "{id}");
                assert_eq!(member.state, AgentState::Executing, "{id}");
            }
            Role::Reviewer => {
                assert_eq!(task.status, Status::CodeReview, "{id}");
                assert_eq!(
                    member.since.as_deref(),
                    Some(task.status_since.as_str()),
                    "{id}: the reviewer took the review when it was requested"
                );
                assert_eq!(member.state, AgentState::Reviewing, "{id}");
            }
            other => panic!("the crew holds a task under {other:?}"),
        }
    }
    // No task is held twice.
    let held: Vec<&str> = runtime
        .crew
        .iter()
        .filter_map(|member| member.holds.as_deref())
        .collect();
    let unique: std::collections::HashSet<&str> = held.iter().copied().collect();
    assert_eq!(held.len(), unique.len(), "{held:?}");
}

#[test]
fn the_crew_is_one_per_role_and_the_reviewer_reviews() {
    let runtime = runtime();
    let roles: Vec<Role> = runtime.crew.iter().map(|member| member.role).collect();
    assert_eq!(
        roles,
        [Role::Orchestrator, Role::Runner, Role::Reviewer],
        "the crew is not one per role"
    );
    assert_eq!(runtime.composition(), Composition::OnePerRole);
    assert_eq!(
        Composition::OnePerRole.says(),
        "one per role · the reviewer reviews"
    );
    // The one reviewing holds the round's task.
    let round = runtime.round.as_ref().expect("a review in flight");
    let reviewer = runtime
        .crew
        .iter()
        .find(|member| member.role == Role::Reviewer)
        .expect("the reviewer");
    assert_eq!(reviewer.holds.as_deref(), Some(round.task.as_str()));
}

#[test]
fn the_queue_is_the_claimable_work_nobody_holds() {
    let runtime = runtime();
    let tasks = tasks();
    assert!(!runtime.queue.is_empty(), "the queue is empty");
    for item in &runtime.queue {
        let task = tasks
            .get(&item.id)
            .unwrap_or_else(|| panic!("{} names no task", item.id));
        assert_eq!(item.status, task.status, "{} status", item.id);
        assert!(claimable(task, &tasks), "{} is not claimable", item.id);
        assert!(!runtime.is_held(&item.id), "{} is held", item.id);
    }
    // A held task belongs to the crew, never to the queue.
    for member in &runtime.crew {
        if let Some(id) = &member.holds {
            assert!(
                !runtime.queue.iter().any(|item| &item.id == id),
                "{id} is in both the crew and the queue"
            );
        }
    }
    // The mock holds the one claimable task the fixtures have, so the
    // queue's count is a fact of the set rather than of the record.
    let claimable_ids: Vec<String> = tasks
        .values()
        .filter(|task| claimable(task, &tasks))
        .map(|task| task.id.clone())
        .collect();
    let queued: Vec<String> = runtime.queue.iter().map(|item| item.id.clone()).collect();
    assert_eq!(queued, claimable_ids);
    assert_eq!(queued, ["TSK29"]);
}

#[test]
fn the_round_is_the_review_in_flight() {
    let runtime = runtime();
    let tasks = tasks();
    let round = runtime.round.as_ref().expect("a review in flight");
    let task = &tasks[&round.task];
    assert_eq!(task.status, Status::CodeReview, "{}", round.task);
    let external = task
        .review
        .rounds
        .iter()
        .filter(|round| round.kind == coding_crew_core::RoundKind::External)
        .count();
    assert_eq!(round.number, external as u32 + 1, "the next round");
    assert_eq!(round.stage, coding_crew_core::RoundKind::External);
    assert_eq!(
        round.number, 1,
        "the fixture has a self pre-screen and no external round yet"
    );
}

#[test]
fn the_budget_and_the_findings_are_the_fixtures_own_counts() {
    let runtime = runtime();
    let tasks = tasks();
    let live: Vec<&Task> = tasks
        .values()
        .filter(|task| !task.review.rounds.is_empty() && task.status != Status::Done)
        .collect();

    let spent: u32 = live
        .iter()
        .flat_map(|task| task.review.rounds.iter())
        .filter(|round| round.kind == coding_crew_core::RoundKind::External)
        .count() as u32;
    let total: u32 = live.iter().map(|task| task.review.max_rounds).sum();
    assert_eq!(runtime.budget.spent, spent);
    assert_eq!(runtime.budget.total, total);
    assert!(runtime.budget.spent < runtime.budget.total);

    let open: u32 = tasks.values().map(|task| task.open_findings() as u32).sum();
    assert_eq!(runtime.findings, open);
    assert!(runtime.findings > 0, "no fixture carries an open finding");
}

#[test]
fn the_severities_break_the_open_findings_down_by_their_own_severity() {
    let runtime = runtime();
    let tasks = tasks();
    let severities = runtime
        .severities
        .expect("the mock holds the severity breakdown");
    let mut counted = Severities::default();
    for finding in tasks
        .values()
        .flat_map(|task| task.review.findings.iter())
        .filter(|finding| finding.status == FindingStatus::Open)
    {
        match finding.severity {
            Severity::High => counted.high += 1,
            Severity::Medium => counted.medium += 1,
            Severity::Low => counted.low += 1,
        }
    }
    assert_eq!(severities, counted);
    assert_eq!(
        severities.high + severities.medium + severities.low,
        runtime.findings
    );
}

#[test]
fn the_run_began_before_the_first_hold_and_after_the_event_before_it() {
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
    let earliest = *holds.iter().min().expect("a hold");
    assert!(started < earliest, "{}", runtime.started_at);

    // Every hold is the moment its task names: the claim for the runner,
    // the review request for the reviewer.
    for member in &runtime.crew {
        if let (Some(id), Some(since)) = (&member.holds, &member.since) {
            let task = &tasks[id];
            let named = match member.role {
                Role::Runner => task.claim.as_ref().expect("a claim").at.clone(),
                _ => task.status_since.clone(),
            };
            assert_eq!(*since, named, "{id}");
        }
    }

    // No fixture event falls between the run's start and the first hold: the
    // run begins after the events that came before it, as the journal does.
    let journal = Activity::read(&project().join("activity.json")).expect("the journal");
    for event in journal.events {
        let at = moment(&event.at);
        assert!(
            at <= started || at >= earliest,
            "{} happens between the run's start and the first hold",
            event.at
        );
    }
    assert_eq!(moment(&runtime.started_at), moment("2026-11-09T08:30Z"));
}
