//! `task groom|approve|start|finish|pass-review|request-changes|re-review|
//! escalate|resume|reopen`, and the overlays
//! `block|unblock|defer|undefer|cancel`. Every move runs core's gates and
//! reports each one that fails.

use std::collections::BTreeMap;

use coding_crew_core::lifecycle::{Event, TransitionContext, TransitionError, transition};
use coding_crew_core::{EventKind, Status, Task};
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure, GateDetail};
use crate::write::{Call, NewEvent, required_text, text};

/// One lifecycle move under the project lock: core's gates, talita's codes,
/// and the records the move owes.
fn apply(
    call: &Call,
    id: &str,
    verb: &str,
    event: Event,
    overlay_reason: Option<&str>,
    instructed_by: Option<&str>,
    grant_rounds: u32,
) -> Result<Done, Failure> {
    call.task_journaled(id, |task, actor| {
        match event {
            Event::Unblock if task.overlays.blocked.is_none() => {
                return Err(overlay_absent(id, "blocked"));
            }
            Event::Undefer if task.overlays.deferred.is_none() => {
                return Err(overlay_absent(id, "deferred"));
            }
            _ => {}
        }

        let now = coding_crew_core::now_rfc3339();
        let tasks = call.project.store.list()?;
        let statuses: BTreeMap<&str, Status> =
            tasks.iter().map(|t| (t.id.as_str(), t.status)).collect();
        let done = |dependency: &str| statuses.get(dependency) == Some(&Status::Done);
        let ctx = TransitionContext {
            dependency_done: Some(&done),
            reason: overlay_reason,
        };
        transition(task, event, actor, &ctx, &now)
            .map_err(|error| refused(error, verb, id, event))?;

        // `human.instructed` is the task model's gate, checked here: the flag
        // must name the human who decided (ADR 0015). The status is checked
        // first, inside `transition`, as every gate follows it.
        if matches!(event, Event::ResumeWork | Event::Reopen) && instructed_by.is_none() {
            return Err(gate_refusal(
                verb,
                id,
                vec![GateDetail {
                    gate: "human.instructed".to_string(),
                    detail: "the move carries no --instructed-by".to_string(),
                }],
            ));
        }

        if grant_rounds > 0 {
            task.review.max_rounds = task
                .review
                .max_rounds
                .checked_add(grant_rounds)
                .ok_or_else(|| {
                    Failure::new(
                        Code::InputInvalid,
                        "--grant-rounds overflows the round budget",
                    )
                })?;
        }
        // Cancel releases a live claim (ADR 0020); an expired one holds
        // nothing and stays as the record it is.
        let mut released = None;
        if matches!(event, Event::Cancel) {
            let live = task
                .claim
                .as_ref()
                .is_some_and(|claim| !claim.lease_expired(&now));
            if live {
                task.claim = None;
            }
            released = Some(live);
        }

        let events = events(task, event, overlay_reason, instructed_by, released);
        Ok((
            data(task, event, overlay_reason, instructed_by, released),
            events,
        ))
    })
}

/// The journal events this move appends (ADR 0018): its kind, and the note
/// the decision names.
fn events(
    task: &Task,
    event: Event,
    reason: Option<&str>,
    instructed_by: Option<&str>,
    released: Option<bool>,
) -> Vec<NewEvent> {
    let note = |kind, text: String| NewEvent::note(kind, text);
    match event {
        Event::Groom => vec![NewEvent::new(EventKind::Groomed)],
        Event::Approve => vec![instructed(EventKind::Approved, instructed_by, None)],
        Event::Start => vec![NewEvent::new(EventKind::Started)],
        Event::Finish => vec![NewEvent::new(EventKind::ReviewRequested)],
        Event::PassReview => vec![NewEvent::new(EventKind::Completed)],
        Event::RequestChanges => vec![NewEvent::new(EventKind::ChangesRequested)],
        Event::ReReview => {
            let after = task
                .review
                .re_reviews
                .last()
                .map(|record| record.after_round)
                .unwrap_or(0);
            vec![note(EventKind::Remediated, format!("after round {after}"))]
        }
        Event::Escalate => vec![NewEvent::new(EventKind::Escalated)],
        Event::ResumeWork => vec![instructed(EventKind::Resumed, instructed_by, reason)],
        Event::Reopen => vec![instructed(EventKind::Reopened, instructed_by, reason)],
        Event::Block => vec![note(
            EventKind::Blocked,
            reason.unwrap_or_default().to_string(),
        )],
        Event::Unblock => vec![NewEvent::new(EventKind::Unblocked)],
        Event::Defer => vec![note(
            EventKind::Deferred,
            reason.unwrap_or_default().to_string(),
        )],
        Event::Undefer => vec![NewEvent::new(EventKind::Undeferred)],
        Event::Cancel => {
            let mut events = vec![note(
                EventKind::Cancelled,
                reason.unwrap_or_default().to_string(),
            )];
            if released == Some(true) {
                events.push(note(
                    EventKind::Released,
                    reason.unwrap_or_default().to_string(),
                ));
            }
            events
        }
    }
}

/// The event a human's decision appends (ADR 0015): its note reads
/// `instructed by <human>`, with the move's reason after it when the move has
/// one. No flag, no note.
fn instructed(kind: EventKind, instructed_by: Option<&str>, reason: Option<&str>) -> NewEvent {
    let Some(human) = instructed_by else {
        return NewEvent::new(kind);
    };
    match reason {
        Some(reason) => NewEvent::note(kind, format!("instructed by {human}: {reason}")),
        None => NewEvent::note(kind, format!("instructed by {human}")),
    }
}

/// What the verb returns: the status it moved to, and what the move wrote.
fn data(
    task: &Task,
    event: Event,
    reason: Option<&str>,
    instructed_by: Option<&str>,
    released: Option<bool>,
) -> Value {
    let mut data = json!({
        "status": task.status.as_str(),
        "status_since": task.status_since,
    });
    if let Some(human) = instructed_by {
        data["instructed_by"] = json!(human);
    }
    if let Some(released) = released {
        data["released"] = json!(released);
    }
    if let Some(reason) =
        reason.filter(|_| matches!(event, Event::Block | Event::Defer | Event::Cancel))
    {
        data["reason"] = json!(reason);
    }
    match event {
        Event::Approve => data["planned_by"] = json!(task.planned_by),
        Event::ResumeWork => data["max_rounds"] = json!(task.review.max_rounds),
        Event::ReReview => {
            data["after_round"] = json!(
                task.review
                    .re_reviews
                    .last()
                    .map(|record| record.after_round)
            );
            data["re_reviews"] = json!(task.review.re_reviews.len());
        }
        Event::Block | Event::Unblock | Event::Defer | Event::Undefer | Event::Cancel => {
            data["overlays"] = json!(task.overlays);
        }
        _ => {}
    }
    data
}

/// The failure of a refused transition, with the role or every failing gate.
fn refused(error: TransitionError, verb: &str, id: &str, event: Event) -> Failure {
    match error {
        TransitionError::WrongRole { role, .. } => Failure::new(
            Code::RoleWrong,
            format!("{verb} needs the {} role", role.as_str()),
        )
        .remedy(format!("act as --role {}", role.as_str())),
        TransitionError::NotAnAgent => {
            Failure::new(Code::ActorNotAgent, format!("{verb} is an agent's act"))
                .remedy(human_route(id, event))
        }
        TransitionError::WrongStatus { status, .. } => Failure::new(
            Code::StatusWrong,
            format!("{verb} cannot move {id} from {}", status.as_str()),
        )
        .remedy(format!("talita task show {id}")),
        TransitionError::Cancelled => {
            Failure::new(Code::TaskCancelled, format!("{id} is cancelled"))
        }
        TransitionError::Gates(failures) => gate_refusal(
            verb,
            id,
            failures
                .into_iter()
                .map(|failure| GateDetail {
                    gate: failure.gate.to_string(),
                    detail: failure.message,
                })
                .collect(),
        ),
    }
}

/// A human's route to a role-gated verb (ADR 0015): the creator agent records
/// the decision with the human's name. The verbs that take `--instructed-by`
/// name that call; every other verb names the role that owns it.
fn human_route(id: &str, event: Event) -> String {
    match event {
        Event::Approve => {
            format!("a creator agent records it: talita task approve {id} --instructed-by <human>")
        }
        Event::ResumeWork => format!(
            "a creator agent records it: talita task resume {id} --instructed-by <human> --reason <t>"
        ),
        Event::Reopen => format!(
            "a creator agent records it: talita task reopen {id} --instructed-by <human> --reason <t>"
        ),
        _ => event
            .required_role()
            .map(|role| format!("act as --role {}", role.as_str()))
            .unwrap_or_else(|| format!("talita task show {id}")),
    }
}

fn gate_refusal(verb: &str, id: &str, gates: Vec<GateDetail>) -> Failure {
    let count = gates.len();
    let remedy = gates
        .first()
        .map(|gate| gate_remedy(verb, id, gate))
        .unwrap_or_else(|| format!("talita task show {id}"));
    let mut failure = Failure::new(
        Code::GateRefused,
        format!(
            "{verb} is refused by {count} gate{}",
            if count == 1 { "" } else { "s" }
        ),
    )
    .remedy(remedy);
    failure.gates = gates;
    failure
}

/// The command that moves a failing gate forward, when one exists. A remedy
/// names the call that unblocks, never the verb that was just refused.
fn gate_remedy(verb: &str, id: &str, gate: &GateDetail) -> String {
    let criterion = criterion_of(&gate.detail).unwrap_or("<ACCEPT>");
    match gate.gate.as_str() {
        "intent.outcome_present" => format!("talita task describe {id} --outcome <t>"),
        "criteria.present" => {
            format!("talita criterion add {id} --description <t> --verification <t>")
        }
        "scope.out_of_scope_declared" => format!("talita scope exclude {id} --item <t>"),
        "questions.resolved" => format!("talita question answer {id} <QST> --answer <t>"),
        "criteria.covered" => {
            format!("talita step add {id} --action <t> --done-when <t> --covers <ACCEPT>")
        }
        "steps.done_when_present" => format!("talita step revise {id} <STP> --done-when <t>"),
        "task.claimed" => format!("talita claim take {id}"),
        "steps.settled" => format!("talita step done {id} <STP>"),
        "criteria.proven" if gate.detail.contains("without a reason") => {
            format!("talita criterion explain {id} {criterion} --failure-reason <t>")
        }
        "criteria.proven" if gate.detail.contains("without a deferral") => {
            format!("talita criterion undefer {id} {criterion}")
        }
        "evidence.attached" | "criteria.proven" => {
            format!("talita evidence run {id} {criterion}")
        }
        "self_round.filed" => {
            format!("talita round file {id} --kind self --focus spec-compliance --verdict ok")
        }
        "review.findings_exist" => {
            format!("talita finding open {id} --severity <s> --description <t>")
        }
        "review.findings_closed" | "review.no_open_findings" => {
            format!("talita finding resolve {id} <FIND> --resolution <t>")
        }
        "review.verdict_changes_needed"
        | "review.budget_exhausted"
        | "review.round_current"
        | "review.round_owner" => format!("talita round file {id} --input <round>"),
        "review.verdict_ok" => {
            format!("talita round file {id} --kind external --focus <f> --verdict ok")
        }
        "review.independent" => "talita session new --as <actor> --role reviewer".to_string(),
        "remediation.session_distinct" => {
            "talita session new --as <actor> --role runner".to_string()
        }
        "overlay.reason_present" => format!("talita {verb} {id} --reason <t>"),
        "human.instructed" => {
            format!("talita {verb} {id} --instructed-by <human> --reason <t>")
        }
        _ => format!("talita task show {id}"),
    }
}

/// The criterion a gate's detail names, when it names one. Core's details
/// begin with the criterion's id.
fn criterion_of(detail: &str) -> Option<&str> {
    let first = detail.split_whitespace().next()?;
    let number = first.strip_prefix("ACCEPT-")?;
    (number.chars().all(|c| c.is_ascii_digit()) && !number.is_empty()).then_some(first)
}

fn overlay_absent(id: &str, overlay: &str) -> Failure {
    Failure::new(
        Code::OverlayAbsent,
        format!("{id} carries no {overlay} overlay"),
    )
    .remedy(format!("talita task show {id}"))
}

// ---- transitions

pub fn groom(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task groom", Event::Groom, None, None, 0)
}

pub fn approve(call: &Call, id: &str, instructed_by: Option<&str>) -> Result<Done, Failure> {
    let instructed = instructed_by
        .map(|human| required_text("--instructed-by", human))
        .transpose()?;
    apply(
        call,
        id,
        "task approve",
        Event::Approve,
        None,
        instructed.as_deref(),
        0,
    )
}

pub fn start(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task start", Event::Start, None, None, 0)
}

pub fn finish(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task finish", Event::Finish, None, None, 0)
}

pub fn pass_review(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(
        call,
        id,
        "task pass-review",
        Event::PassReview,
        None,
        None,
        0,
    )
}

pub fn request_changes(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(
        call,
        id,
        "task request-changes",
        Event::RequestChanges,
        None,
        None,
        0,
    )
}

pub fn re_review(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task re-review", Event::ReReview, None, None, 0)
}

pub fn escalate(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task escalate", Event::Escalate, None, None, 0)
}

pub fn resume(
    call: &Call,
    id: &str,
    instructed_by: Option<&str>,
    reason: &str,
    grant_rounds: Option<u32>,
) -> Result<Done, Failure> {
    // The journal's `resumed` event records the reason (ADR 0018).
    required_text("--reason", reason)?;
    let instructed = instructed_by
        .map(|human| required_text("--instructed-by", human))
        .transpose()?;
    apply(
        call,
        id,
        "task resume",
        Event::ResumeWork,
        Some(reason),
        instructed.as_deref(),
        grant_rounds.unwrap_or(0),
    )
}

pub fn reopen(
    call: &Call,
    id: &str,
    instructed_by: Option<&str>,
    reason: &str,
) -> Result<Done, Failure> {
    // The journal's `reopened` event records the reason (ADR 0018).
    required_text("--reason", reason)?;
    let instructed = instructed_by
        .map(|human| required_text("--instructed-by", human))
        .transpose()?;
    apply(
        call,
        id,
        "task reopen",
        Event::Reopen,
        Some(reason),
        instructed.as_deref(),
        0,
    )
}

// ---- overlays

pub fn block(call: &Call, id: &str, reason: Option<&str>) -> Result<Done, Failure> {
    let reason = reason.map(|value| text("--reason", value)).transpose()?;
    apply(
        call,
        id,
        "task block",
        Event::Block,
        reason.as_deref(),
        None,
        0,
    )
}

pub fn unblock(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task unblock", Event::Unblock, None, None, 0)
}

pub fn defer(call: &Call, id: &str, reason: Option<&str>) -> Result<Done, Failure> {
    let reason = reason.map(|value| text("--reason", value)).transpose()?;
    apply(
        call,
        id,
        "task defer",
        Event::Defer,
        reason.as_deref(),
        None,
        0,
    )
}

pub fn undefer(call: &Call, id: &str) -> Result<Done, Failure> {
    apply(call, id, "task undefer", Event::Undefer, None, None, 0)
}

pub fn cancel(call: &Call, id: &str, reason: Option<&str>) -> Result<Done, Failure> {
    let reason = reason.map(|value| text("--reason", value)).transpose()?;
    apply(
        call,
        id,
        "task cancel",
        Event::Cancel,
        reason.as_deref(),
        None,
        0,
    )
}
