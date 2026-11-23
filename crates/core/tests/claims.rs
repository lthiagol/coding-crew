//! The claim operations (ADR 0020) against the fixtures and in sequence.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use coding_crew_core::claim::{self, ClaimError, DEFAULT_LEASE};
use coding_crew_core::lifecycle::{Event, TransitionContext, check, transition};
use coding_crew_core::{Actor, Role, Runtime, Status, Store, Task, now_rfc3339};

/// The clock the fixtures are read with (`mock-data/generate.py`, FIXTURE_NOW).
const FIXTURE_NOW: &str = "2026-11-19T08:00:00Z";
const T0: &str = "2026-11-19T09:00:00Z";
const T1: &str = "2026-11-19T11:00:00Z";

fn project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew")
}

fn runner(id: &str, session: &str) -> Actor {
    Actor::agent(id, Role::Runner).in_session(session)
}

fn approved(id: &str) -> Task {
    let mut task = Task::new(id, "a task");
    task.status = Status::Approved;
    task
}

#[test]
fn the_claim_rule_is_the_queue_record_claimable_flag() {
    // The record's flag is the generator's rule. The core rule must give the
    // same answer for every task the queue names, at the fixtures' clock.
    let tasks: HashMap<String, Task> = Store::new(project().join("work"))
        .list()
        .expect("the mock tasks")
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect();
    let done = |id: &str| tasks.get(id).is_some_and(|t| t.status == Status::Done);
    let record = Runtime::read(&project().join("runtime.json")).expect("the runtime record");

    assert!(!record.queue.is_empty());
    for item in &record.queue {
        let task = &tasks[&item.id];
        assert_eq!(
            claim::claimable(task, Some(&done), FIXTURE_NOW),
            item.claimable,
            "{} disagrees with the record",
            item.id
        );
    }
}

#[test]
fn only_one_fixture_is_claimable_at_the_fixtures_clock() {
    // TSK09 holds a live claim and TSK07 waits on work not yet done; TSK29 is
    // the one approved task with nothing in its way.
    let tasks: Vec<Task> = Store::new(project().join("work")).list().expect("tasks");
    let done_ids: Vec<String> = tasks
        .iter()
        .filter(|t| t.status == Status::Done)
        .map(|t| t.id.clone())
        .collect();
    let done = |id: &str| done_ids.iter().any(|d| d == id);
    let claimable: Vec<&str> = tasks
        .iter()
        .filter(|task| claim::claimable(task, Some(&done), FIXTURE_NOW))
        .map(|task| task.id.as_str())
        .collect();
    assert_eq!(claimable, ["TSK29"]);
}

#[test]
fn an_expired_lease_is_taken_over_by_another_runner() {
    let mut task = approved("TSK01");
    claim::take(
        &mut task,
        &runner("runner-1", "s-1"),
        DEFAULT_LEASE,
        None,
        T0,
    )
    .unwrap();
    assert_eq!(
        claim::take(
            &mut task,
            &runner("runner-2", "s-2"),
            DEFAULT_LEASE,
            None,
            T0
        ),
        Err(ClaimError::Held {
            by: "runner-1".to_string()
        })
    );

    // Two hours on, the lease has run out: runner-2 takes it.
    claim::take(
        &mut task,
        &runner("runner-2", "s-2"),
        DEFAULT_LEASE,
        None,
        T1,
    )
    .unwrap();
    let holder = task.claim.as_ref().expect("the claim");
    assert_eq!(
        (holder.by.as_str(), holder.session.as_deref()),
        ("runner-2", Some("s-2"))
    );

    // The old holder no longer holds it.
    assert_eq!(
        claim::renew(&mut task, &runner("runner-1", "s-1"), DEFAULT_LEASE, T1),
        Err(ClaimError::NotHolder)
    );
}

#[test]
fn a_holder_renews_its_live_claim_rather_than_taking_it_again() {
    let mut task = approved("TSK01");
    let me = runner("runner-1", "s-1");
    claim::take(&mut task, &me, DEFAULT_LEASE, None, T0).unwrap();

    assert_eq!(
        claim::take(&mut task, &me, DEFAULT_LEASE, None, T0),
        Err(ClaimError::Held {
            by: "runner-1".to_string()
        })
    );
    claim::renew(&mut task, &me, DEFAULT_LEASE, T1).unwrap();
    let holder = task.claim.as_ref().expect("the claim");
    assert_eq!(holder.by, "runner-1");
}

#[test]
fn start_refuses_an_expired_lease_until_its_holder_renews() {
    let mut task = approved("TSK01");
    // A claim taken long ago: its lease is past at the real clock.
    claim::take(
        &mut task,
        &runner("runner-1", "s-1"),
        Duration::from_secs(60),
        None,
        "2020-01-01T00:00:00Z",
    )
    .unwrap();

    let me = runner("runner-1", "s-1");
    let refused = transition(&mut task, Event::Start, &me, &TransitionContext::default());
    assert!(refused.is_err(), "an expired lease must not start the task");
    assert_eq!(task.status, Status::Approved);

    claim::renew(&mut task, &me, DEFAULT_LEASE, &now_rfc3339()).unwrap();
    transition(&mut task, Event::Start, &me, &TransitionContext::default())
        .expect("a renewed lease starts the task");
    assert_eq!(task.status, Status::Executing);
}

#[test]
fn a_takeover_in_executing_moves_the_execution_session() {
    let first = runner("runner-1", "s-1");
    let second = runner("runner-2", "s-2");
    let mut task = approved("TSK01");
    claim::take(&mut task, &first, DEFAULT_LEASE, None, &now_rfc3339()).unwrap();
    transition(
        &mut task,
        Event::Start,
        &first,
        &TransitionContext::default(),
    )
    .unwrap();

    // The first builder dies: its lease runs out while the task is executing.
    task.claim.as_mut().expect("the claim").lease_expires_at = "2020-01-01T01:00:00Z".to_string();
    claim::take(&mut task, &second, DEFAULT_LEASE, None, &now_rfc3339()).unwrap();
    assert_eq!(
        task.claim.as_ref().and_then(|c| c.session.as_deref()),
        Some("s-2")
    );

    // The execution session is now the second: it may not remediate, and the
    // first may, since it did not finish the work.
    task.status = Status::Remediating;
    let ctx = TransitionContext::default();
    assert!(!check(&task, Event::ReReview, &second, &ctx).is_empty());
    assert!(check(&task, Event::ReReview, &first, &ctx).is_empty());
}

#[test]
fn a_claim_is_released_only_by_its_holder_with_a_reason() {
    let mut task = approved("TSK01");
    claim::take(
        &mut task,
        &runner("runner-1", "s-1"),
        DEFAULT_LEASE,
        None,
        T0,
    )
    .unwrap();
    claim::release(&mut task, &runner("runner-1", "s-1"), "stopping", T0).unwrap();
    assert!(task.claim.is_none());
    assert!(claim::claimable(&task, None, T0));
}
