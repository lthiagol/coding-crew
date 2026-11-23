//! `claim take|renew|release|show`: a runner's hold on a task, with a lease
//! (ADR 0020). A claim changes no status, so it is an operation, not a
//! lifecycle event. The rule is core's; the verbs call it and map its errors.

use std::collections::BTreeMap;
use std::time::Duration;

use coding_crew_core::claim::{self, ClaimError, DEFAULT_LEASE};
use coding_crew_core::{EventKind, Status, Task};
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure, GateDetail};
use crate::project::Project;
use crate::write::{Call, NewEvent, duration, required_text};

/// `claim take`: take a claimable task, or take over an expired claim. The
/// takeover is journaled with the holder it replaced.
pub fn take(call: &Call, id: &str, lease: Option<&str>) -> Result<Done, Failure> {
    let lease = lease_arg(lease)?;
    call.task_journaled(id, |task, actor| {
        let now = coding_crew_core::now_rfc3339();
        let before = task.claim.clone();
        // The dependency gate reads the plan, as `task start` does.
        let tasks = call.project.store.list()?;
        let statuses: BTreeMap<&str, Status> = tasks
            .iter()
            .map(|task| (task.id.as_str(), task.status))
            .collect();
        let done = |dependency: &str| statuses.get(dependency) == Some(&Status::Done);
        claim::take(task, actor, lease, Some(&done), &now)
            .map_err(|error| refused(error, "claim take", id))?;
        let event = match before {
            Some(claim) => {
                NewEvent::note(EventKind::Claimed, format!("took over from {}", claim.by))
            }
            None => NewEvent::new(EventKind::Claimed),
        };
        Ok((held(task, &now), vec![event]))
    })
}

/// `claim renew`: extend the holder's lease.
pub fn renew(call: &Call, id: &str, lease: Option<&str>) -> Result<Done, Failure> {
    let lease = lease_arg(lease)?;
    call.task(id, |task, actor| {
        let now = coding_crew_core::now_rfc3339();
        claim::renew(task, actor, lease, &now)
            .map_err(|error| refused(error, "claim renew", id))?;
        Ok(held(task, &now))
    })
}

/// `claim release`: give the claim up. The reason goes to the journal; the
/// task file keeps none.
pub fn release(call: &Call, id: &str, reason: Option<&str>) -> Result<Done, Failure> {
    let reason = reason
        .map(|value| required_text("--reason", value))
        .transpose()?
        .ok_or_else(|| Failure::new(Code::InputInvalid, "--reason is required"))?;
    call.task_journaled(id, |task, actor| {
        let now = coding_crew_core::now_rfc3339();
        claim::release(task, actor, &reason, &now)
            .map_err(|error| refused(error, "claim release", id))?;
        Ok((
            held(task, &now),
            vec![NewEvent::note(EventKind::Released, reason.clone())],
        ))
    })
}

/// `claim show`: the claim and whether its lease is live. A read.
pub fn show(project: &Project, id: &str) -> Result<Done, Failure> {
    let task = project.store.load(id)?;
    Ok(Done::data(held(&task, &coding_crew_core::now_rfc3339())))
}

/// The lease the call names, or core's default.
fn lease_arg(lease: Option<&str>) -> Result<Duration, Failure> {
    match lease {
        Some(value) => duration("--lease", value),
        None => Ok(DEFAULT_LEASE),
    }
}

/// What the four verbs return: the claim, if any, and its liveness.
fn held(task: &Task, now: &str) -> Value {
    let live = task
        .claim
        .as_ref()
        .is_some_and(|claim| !claim.lease_expired(now));
    json!({ "claim": task.claim, "live": live })
}

/// core's refusal, as talita's code (ADR 0014).
fn refused(error: ClaimError, verb: &str, id: &str) -> Failure {
    let message = error.to_string();
    match error {
        ClaimError::NotAnAgent => {
            Failure::new(Code::ActorNotAgent, message).remedy("act as --role runner")
        }
        ClaimError::WrongRole(role) => Failure::new(Code::RoleWrong, message)
            .remedy(format!("act as --role {}", role.as_str())),
        ClaimError::NoSession => Failure::new(Code::IdentityMissing, message)
            .remedy("talita session new --as <actor> --role runner"),
        ClaimError::Cancelled => Failure::new(Code::TaskCancelled, message),
        ClaimError::WrongStatus(status) => Failure::new(
            Code::StatusWrong,
            format!("{verb} cannot hold {id} in {}", status.as_str()),
        )
        .remedy(format!("talita task show {id}")),
        ClaimError::Held { by } => Failure::new(Code::ClaimHeld, message).remedy(format!(
            "wait for {by}'s lease to expire, or ask {by} to release it"
        )),
        ClaimError::NotClaimable(failures) => {
            let details = failures
                .iter()
                .map(|failure| failure.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let mut failure = Failure::new(
                Code::ClaimNotClaimable,
                format!("{id} cannot be claimed: {details}"),
            );
            failure.gates = failures
                .into_iter()
                .map(|gate| GateDetail {
                    gate: gate.gate.to_string(),
                    detail: gate.message,
                })
                .collect();
            failure
        }
        ClaimError::NotHolder => {
            Failure::new(Code::ClaimNotHolder, message).remedy(format!("talita claim take {id}"))
        }
        ClaimError::ReasonMissing => Failure::new(Code::InputInvalid, message),
        ClaimError::BadStamp(_) => Failure::new(Code::Internal, message),
        ClaimError::LeaseTooLong => Failure::new(Code::InputInvalid, message),
    }
}
