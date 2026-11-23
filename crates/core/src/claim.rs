//! Claims: a runner's hold on a task, with a lease (ADR 0020).
//!
//! A claim changes no status, so it is an operation, not an
//! [`Event`](crate::lifecycle::Event). The rule is in `docs/tasks/task-model.md`
//! § 3.6. Every operation takes `now` as an RFC 3339 stamp, so a test reads the
//! clock it chooses.
//!

use std::time::Duration;

use thiserror::Error;

use crate::actor::{Actor, ActorKind, Role};
use crate::clock::{format_rfc3339, parse_rfc3339};
use crate::lifecycle::{GateFailure, Status};
use crate::task::{Claim, Task};

/// The lease a take gets when the caller names none.
pub const DEFAULT_LEASE: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClaimError {
    #[error("only agents hold claims")]
    NotAnAgent,
    #[error("the claim needs the {} role", .0.as_str())]
    WrongRole(Role),
    #[error("the claim needs a session")]
    NoSession,
    #[error("task is cancelled")]
    Cancelled,
    #[error("a claim cannot change a task in {}", .0.as_str())]
    WrongStatus(Status),
    /// A live lease held by another actor. Wait for it to expire, or ask the
    /// holder to release it.
    #[error("the task is held by a live claim of {by}")]
    Held { by: String },
    /// Every condition the take fails, as the gates name them.
    #[error("the task cannot be claimed: {}", failures_text(.0))]
    NotClaimable(Vec<GateFailure>),
    #[error("only the holder, in its session, may do this")]
    NotHolder,
    #[error("a release needs a reason")]
    ReasonMissing,
    #[error("{0:?} is not an RFC 3339 stamp")]
    BadStamp(String),
}

fn failures_text(failures: &[GateFailure]) -> String {
    failures
        .iter()
        .map(|failure| failure.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Every condition that stops `task` being claimable now: approved, no
/// overlay, every dependency done, and no live claim. An empty list means
/// claimable. An expired claim does not hold the task.
///
/// `dependency_done` is the store's answer. Absent means "cannot check", which
/// fails the dependency gate, as it does for `start`.
pub fn claim_failures(
    task: &Task,
    dependency_done: Option<&dyn Fn(&str) -> bool>,
    now: &str,
) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    if task.status != Status::Approved {
        failures.push(gate(
            "claim.status",
            format!("the task is {}, not approved", task.status.as_str()),
        ));
    }
    let overlays = [
        ("blocked", task.overlays.blocked.is_some()),
        ("deferred", task.overlays.deferred.is_some()),
        ("cancelled", task.overlays.cancelled.is_some()),
    ];
    for (kind, set) in overlays {
        if set {
            failures.push(gate("overlay.clear", format!("the task is {kind}")));
        }
    }
    if let Some(held) = live_claim(task, now) {
        failures.push(gate(
            "claim.held",
            format!("the task is held by a live claim of {}", held.by),
        ));
    }
    if !task.depends_on.is_empty() {
        match dependency_done {
            None => failures.push(gate(
                "dependencies.satisfied",
                "dependencies cannot be checked without the plan".to_string(),
            )),
            Some(is_done) => {
                for dependency in &task.depends_on {
                    if !is_done(dependency) {
                        failures.push(gate(
                            "dependencies.satisfied",
                            format!("dependency {dependency} is not done"),
                        ));
                    }
                }
            }
        }
    }
    failures
}

/// Whether an agent could claim `task` now. See [`claim_failures`].
pub fn claimable(task: &Task, dependency_done: Option<&dyn Fn(&str) -> bool>, now: &str) -> bool {
    claim_failures(task, dependency_done, now).is_empty()
}

/// Take a claim on the task, as a runner.
///
/// In `approved` the task must be claimable; an expired claim is replaced. In
/// `executing` only an expired claim is taken over: the new holder and session
/// become the execution session, so the holder cannot remediate later.
pub fn take(
    task: &mut Task,
    actor: &Actor,
    lease: Duration,
    dependency_done: Option<&dyn Fn(&str) -> bool>,
    now: &str,
) -> Result<(), ClaimError> {
    let session = runner_session(task, actor)?;
    match task.status {
        Status::Approved => {
            if let Some(held) = live_claim(task, now) {
                return Err(ClaimError::Held {
                    by: held.by.clone(),
                });
            }
            let failures = claim_failures(task, dependency_done, now);
            if !failures.is_empty() {
                return Err(ClaimError::NotClaimable(failures));
            }
        }
        Status::Executing => {
            let Some(claim) = &task.claim else {
                return Err(ClaimError::NotClaimable(vec![gate(
                    "claim.takeover",
                    "there is no claim to take over".to_string(),
                )]));
            };
            if !claim.lease_expired(now) {
                return Err(ClaimError::Held {
                    by: claim.by.clone(),
                });
            }
        }
        other => return Err(ClaimError::WrongStatus(other)),
    }
    task.claim = Some(Claim {
        by: actor.id.clone(),
        session: Some(session.to_string()),
        at: now.to_string(),
        lease_expires_at: lease_end(now, lease)?,
    });
    task.updated = now.to_string();
    Ok(())
}

/// Extend the holder's lease. The holder is the same actor in the same
/// session. The lease may already have expired: the holder renews before
/// `start`, and an expired claim that nobody took is still its holder's.
pub fn renew(task: &mut Task, actor: &Actor, lease: Duration, now: &str) -> Result<(), ClaimError> {
    runner_session(task, actor)?;
    require_status(task, &[Status::Approved, Status::Executing])?;
    require_holder(task, actor)?;
    let expires = lease_end(now, lease)?;
    if let Some(claim) = task.claim.as_mut() {
        claim.lease_expires_at = expires;
    }
    task.updated = now.to_string();
    Ok(())
}

/// Give the claim up, as its holder. `reason` is the caller's to record; the
/// task file keeps no reason for a release.
pub fn release(task: &mut Task, actor: &Actor, reason: &str, now: &str) -> Result<(), ClaimError> {
    runner_session(task, actor)?;
    if reason.trim().is_empty() {
        return Err(ClaimError::ReasonMissing);
    }
    require_status(task, &[Status::Approved, Status::Executing])?;
    require_holder(task, actor)?;
    task.claim = None;
    task.updated = now.to_string();
    Ok(())
}

/// The session a runner acts in. Claims are checked in the runner's role, in
/// a session, on a task that is not cancelled.
fn runner_session<'a>(task: &Task, actor: &'a Actor) -> Result<&'a str, ClaimError> {
    if actor.kind != ActorKind::Agent {
        return Err(ClaimError::NotAnAgent);
    }
    if actor.role != Some(Role::Runner) {
        return Err(ClaimError::WrongRole(Role::Runner));
    }
    if task.overlays.cancelled.is_some() {
        return Err(ClaimError::Cancelled);
    }
    actor.session.as_deref().ok_or(ClaimError::NoSession)
}

fn require_status(task: &Task, allowed: &[Status]) -> Result<(), ClaimError> {
    if allowed.contains(&task.status) {
        Ok(())
    } else {
        Err(ClaimError::WrongStatus(task.status))
    }
}

/// The holder is the same actor in the same session. An unrecorded session
/// never matches, so a claim without one cannot be renewed or released.
fn require_holder(task: &Task, actor: &Actor) -> Result<(), ClaimError> {
    let holder = match &task.claim {
        Some(claim) => {
            claim.by == actor.id
                && claim.session.is_some()
                && claim.session.as_deref() == actor.session.as_deref()
        }
        None => false,
    };
    if holder {
        Ok(())
    } else {
        Err(ClaimError::NotHolder)
    }
}

/// The claim on `task`, when its lease is live at `now`.
fn live_claim<'a>(task: &'a Task, now: &str) -> Option<&'a Claim> {
    task.claim
        .as_ref()
        .filter(|claim| !claim.lease_expired(now))
}

/// `now` plus the lease, as an RFC 3339 stamp.
fn lease_end(now: &str, lease: Duration) -> Result<String, ClaimError> {
    let start = parse_rfc3339(now).ok_or_else(|| ClaimError::BadStamp(now.to_string()))?;
    Ok(format_rfc3339(start + lease.as_secs() as i64))
}

fn gate(gate: &'static str, message: String) -> GateFailure {
    GateFailure { gate, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-11-19T08:00:00Z";
    const LATER: &str = "2026-11-19T11:00:00Z";

    fn runner() -> Actor {
        Actor::agent("runner-1", Role::Runner).in_session("s-runner")
    }

    fn approved() -> Task {
        let mut task = Task::new("TSK01", "a task");
        task.status = Status::Approved;
        task
    }

    #[test]
    fn an_approved_task_with_no_claim_is_claimable() {
        assert!(claimable(&approved(), None, NOW));
    }

    #[test]
    fn a_live_claim_holds_the_task() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        assert!(!claimable(&task, None, NOW));
    }

    #[test]
    fn an_expired_claim_frees_the_task() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        assert!(claimable(&task, None, LATER));
    }

    #[test]
    fn a_task_that_is_not_approved_is_not_claimable() {
        let mut task = approved();
        task.status = Status::Groomed;
        let failures = claim_failures(&task, None, NOW);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].gate, "claim.status");
    }

    #[test]
    fn an_overlay_holds_the_task() {
        let mut task = approved();
        task.overlays.deferred = Some(crate::Overlay {
            reason: "later".to_string(),
            by: "creator-1".to_string(),
            at: NOW.to_string(),
        });
        assert_eq!(claim_failures(&task, None, NOW)[0].gate, "overlay.clear");
    }

    #[test]
    fn a_dependency_that_is_not_done_holds_the_task() {
        let mut task = approved();
        task.depends_on = vec!["TSK02".to_string()];
        let not_done = |_: &str| false;
        assert!(!claimable(&task, Some(&not_done), NOW));
        assert!(!claimable(&task, None, NOW), "unchecked is refused");
        let done = |_: &str| true;
        assert!(claimable(&task, Some(&done), NOW));
    }

    #[test]
    fn take_sets_the_holder_session_and_a_lease_of_the_default_length() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        let claim = task.claim.expect("the claim");
        assert_eq!(claim.by, "runner-1");
        assert_eq!(claim.session.as_deref(), Some("s-runner"));
        assert_eq!(claim.at, NOW);
        assert_eq!(claim.lease_expires_at, "2026-11-19T10:00:00Z");
    }

    #[test]
    fn take_refuses_a_live_claim_held_by_another() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        let other = Actor::agent("runner-2", Role::Runner).in_session("s-other");
        let error = take(&mut task, &other, DEFAULT_LEASE, None, NOW).unwrap_err();
        assert_eq!(
            error,
            ClaimError::Held {
                by: "runner-1".to_string()
            }
        );
        assert_eq!(task.claim.expect("the claim").by, "runner-1");
    }

    #[test]
    fn an_expired_claim_in_approved_is_replaced() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        let other = Actor::agent("runner-2", Role::Runner).in_session("s-other");
        take(&mut task, &other, DEFAULT_LEASE, None, LATER).unwrap();
        let claim = task.claim.expect("the claim");
        assert_eq!(claim.by, "runner-2");
        assert_eq!(claim.session.as_deref(), Some("s-other"));
    }

    #[test]
    fn an_expired_lease_is_taken_over_in_executing() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        task.status = Status::Executing;

        let other = Actor::agent("runner-2", Role::Runner).in_session("s-other");
        assert!(matches!(
            take(&mut task, &other, DEFAULT_LEASE, None, NOW),
            Err(ClaimError::Held { .. })
        ));

        take(&mut task, &other, DEFAULT_LEASE, None, LATER).unwrap();
        let claim = task.claim.expect("the claim");
        assert_eq!(claim.by, "runner-2");
        assert_eq!(claim.session.as_deref(), Some("s-other"));
    }

    #[test]
    fn take_in_executing_needs_a_claim_to_take_over() {
        let mut task = approved();
        task.status = Status::Executing;
        let error = take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap_err();
        assert!(matches!(error, ClaimError::NotClaimable(_)));
    }

    #[test]
    fn take_refuses_other_statuses() {
        let mut task = approved();
        task.status = Status::CodeReview;
        assert_eq!(
            take(&mut task, &runner(), DEFAULT_LEASE, None, NOW),
            Err(ClaimError::WrongStatus(Status::CodeReview))
        );
    }

    #[test]
    fn only_a_runner_in_a_session_takes_a_claim() {
        let mut task = approved();
        let reviewer = Actor::agent("reviewer-1", Role::Reviewer).in_session("s-r");
        assert_eq!(
            take(&mut task, &reviewer, DEFAULT_LEASE, None, NOW),
            Err(ClaimError::WrongRole(Role::Runner))
        );
        let sessionless = Actor::agent("runner-1", Role::Runner);
        assert_eq!(
            take(&mut task, &sessionless, DEFAULT_LEASE, None, NOW),
            Err(ClaimError::NoSession)
        );
        assert_eq!(
            take(&mut task, &Actor::human("luiz"), DEFAULT_LEASE, None, NOW),
            Err(ClaimError::NotAnAgent)
        );
        assert!(task.claim.is_none());
    }

    #[test]
    fn renew_extends_the_holders_lease_even_after_it_expired() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        renew(&mut task, &runner(), DEFAULT_LEASE, LATER).unwrap();
        let claim = task.claim.expect("the claim");
        assert_eq!(claim.lease_expires_at, "2026-11-19T13:00:00Z");
        assert_eq!(claim.by, "runner-1");
    }

    #[test]
    fn renew_refuses_anyone_but_the_holder_in_its_session() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        let other_session = Actor::agent("runner-1", Role::Runner).in_session("s-new");
        assert_eq!(
            renew(&mut task, &other_session, DEFAULT_LEASE, NOW),
            Err(ClaimError::NotHolder)
        );
        let other = Actor::agent("runner-2", Role::Runner).in_session("s-runner");
        assert_eq!(
            renew(&mut task, &other, DEFAULT_LEASE, NOW),
            Err(ClaimError::NotHolder)
        );
    }

    #[test]
    fn renew_needs_a_claim_and_a_status_that_holds_one() {
        let mut task = approved();
        assert_eq!(
            renew(&mut task, &runner(), DEFAULT_LEASE, NOW),
            Err(ClaimError::NotHolder)
        );
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        task.status = Status::Remediating;
        assert_eq!(
            renew(&mut task, &runner(), DEFAULT_LEASE, NOW),
            Err(ClaimError::WrongStatus(Status::Remediating))
        );
    }

    #[test]
    fn release_removes_the_claim_for_the_holder() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        assert_eq!(
            release(&mut task, &runner(), "  ", NOW),
            Err(ClaimError::ReasonMissing)
        );
        release(&mut task, &runner(), "the builder is stopping", NOW).unwrap();
        assert!(task.claim.is_none());
    }

    #[test]
    fn release_refuses_a_stranger() {
        let mut task = approved();
        take(&mut task, &runner(), DEFAULT_LEASE, None, NOW).unwrap();
        let other = Actor::agent("runner-2", Role::Runner).in_session("s-other");
        assert_eq!(
            release(&mut task, &other, "not mine", NOW),
            Err(ClaimError::NotHolder)
        );
        assert!(task.claim.is_some());
    }

    #[test]
    fn a_claim_on_a_cancelled_task_is_refused() {
        let mut task = approved();
        task.overlays.cancelled = Some(crate::Overlay {
            reason: "dropped".to_string(),
            by: "creator-1".to_string(),
            at: NOW.to_string(),
        });
        assert_eq!(
            take(&mut task, &runner(), DEFAULT_LEASE, None, NOW),
            Err(ClaimError::Cancelled)
        );
    }

    #[test]
    fn a_stamp_that_is_not_one_is_refused_before_a_lease_is_written() {
        let mut task = approved();
        assert!(matches!(
            take(&mut task, &runner(), DEFAULT_LEASE, None, "soon"),
            Err(ClaimError::BadStamp(_))
        ));
        assert!(task.claim.is_none());
    }
}
