//! The parts of a task's content: `scope`, `criterion add|revise|remove` and
//! its work verbs, `step add|revise|remove` and its work verbs, `question
//! ask|answer`, `note add`.

use coding_crew_core::{
    AcceptanceCriterion, Actor, ActorKind, CriterionStatus, Deferral, EventKind, Note, Question,
    QuestionStatus, Role, Section, Status, Step, StepStatus, Task,
};
use serde_json::json;

use crate::envelope::Done;
use crate::error::{Code, Failure, GateDetail};
use crate::task::word;
use crate::write::{
    Call, NewEvent, next_part_id, own, part_not_found, pre_approval, refuse_frozen, required_text,
};

/// A text flag the verb cannot do without.
fn needed(flag: &str, value: Option<&str>) -> Result<String, Failure> {
    value
        .map(|value| required_text(flag, value))
        .transpose()?
        .ok_or_else(|| Failure::new(Code::InputInvalid, format!("{flag} is required")))
}

// ---- scope

pub fn include(call: &Call, id: &str, item: &str) -> Result<Done, Failure> {
    scope_add(call, id, item, true)
}

pub fn exclude(call: &Call, id: &str, item: &str) -> Result<Done, Failure> {
    scope_add(call, id, item, false)
}

fn scope_add(call: &Call, id: &str, item: &str, inside: bool) -> Result<Done, Failure> {
    let item = required_text("--item", item)?;
    call.task(id, |task, actor| {
        own(Section::Scope, actor, task)?;
        refuse_frozen(task, Section::Scope)?;
        let list = if inside {
            &mut task.scope.in_scope
        } else {
            &mut task.scope.out_of_scope
        };
        list.push(item);
        Ok(json!({ "scope": task.scope }))
    })
}

/// `scope drop --in <n>` or `--out <n>`: entry `n`, 1-based.
pub fn drop(
    call: &Call,
    id: &str,
    inside: Option<usize>,
    outside: Option<usize>,
) -> Result<Done, Failure> {
    let (n, is_in) = match (inside, outside) {
        (Some(n), None) => (n, true),
        (None, Some(n)) => (n, false),
        _ => {
            return Err(Failure::new(
                Code::UsageInvalid,
                "scope drop needs exactly one of --in and --out",
            ));
        }
    };
    call.task(id, |task, actor| {
        own(Section::Scope, actor, task)?;
        refuse_frozen(task, Section::Scope)?;
        let (list, name) = if is_in {
            (&mut task.scope.in_scope, "in_scope")
        } else {
            (&mut task.scope.out_of_scope, "out_of_scope")
        };
        if n == 0 || n > list.len() {
            return Err(Failure::new(
                Code::PartNotFound,
                format!("{id} has {} {name} entries; there is no {n}", list.len()),
            )
            .remedy(format!("talita task show {id}")));
        }
        list.remove(n - 1);
        Ok(json!({ "scope": task.scope }))
    })
}

// ---- criterion

pub fn criterion_add(
    call: &Call,
    id: &str,
    description: &str,
    verification: &str,
) -> Result<Done, Failure> {
    let description = required_text("--description", description)?;
    let verification = required_text("--verification", verification)?;
    call.task(id, |task, actor| {
        own(Section::Criteria, actor, task)?;
        refuse_frozen(task, Section::Criteria)?;
        let cid = next_part_id(
            "ACCEPT",
            task.acceptance_criteria.iter().map(|c| c.id.as_str()),
        );
        task.acceptance_criteria.push(AcceptanceCriterion {
            id: cid.clone(),
            description,
            verification,
            ..AcceptanceCriterion::default()
        });
        Ok(json!({ "criterion": task.criterion(&cid) }))
    })
}

pub fn criterion_revise(
    call: &Call,
    id: &str,
    cid: &str,
    description: Option<&str>,
    verification: Option<&str>,
) -> Result<Done, Failure> {
    if description.is_none() && verification.is_none() {
        return Err(Failure::new(
            Code::UsageInvalid,
            "criterion revise needs --description or --verification",
        ));
    }
    let description = description
        .map(|v| required_text("--description", v))
        .transpose()?;
    let verification = verification
        .map(|v| required_text("--verification", v))
        .transpose()?;
    call.task(id, |task, actor| {
        own(Section::Criteria, actor, task)?;
        refuse_frozen(task, Section::Criteria)?;
        let found = task.criterion_mut(cid);
        let Some(criterion) = found else {
            return Err(part_not_found(task, cid));
        };
        if let Some(d) = description {
            criterion.description = d;
        }
        if let Some(v) = verification {
            criterion.verification = v;
        }
        Ok(json!({ "criterion": task.criterion(cid) }))
    })
}

pub fn criterion_remove(call: &Call, id: &str, cid: &str) -> Result<Done, Failure> {
    call.task(id, |task, actor| {
        own(Section::Criteria, actor, task)?;
        refuse_frozen(task, Section::Criteria)?;
        if task.criterion(cid).is_none() {
            return Err(part_not_found(task, cid));
        }
        let covering: Vec<&str> = task
            .steps
            .iter()
            .filter(|s| s.covers.iter().any(|c| c == cid))
            .map(|s| s.id.as_str())
            .collect();
        if !covering.is_empty() {
            return Err(Failure::new(
                Code::CriterionCovered,
                format!("{cid} is covered by {}", covering.join(", ")),
            )
            .remedy(format!(
                "talita step revise {id} {} --covers <other ACCEPT>",
                covering[0]
            )));
        }
        task.acceptance_criteria.retain(|c| c.id != cid);
        Ok(json!({ "removed": cid }))
    })
}

/// `criterion defer`: the criterion is parked on a follow-up task. The
/// status is core's mirror of the deferral (ADR 0019).
pub fn criterion_defer(
    call: &Call,
    id: &str,
    cid: &str,
    reason: Option<&str>,
    follow_up: Option<&str>,
) -> Result<Done, Failure> {
    let reason = needed("--reason", reason)?;
    let follow_up = needed("--follow-up", follow_up)?;
    call.task(id, |task, actor| {
        work_runner(actor, "criterion defer")?;
        working_runner(task, actor, "criteria")?;
        if task.criterion(cid).is_none() {
            return Err(part_not_found(task, cid));
        }
        call.project.store.load(&follow_up)?;
        let criterion = task.criterion_mut(cid).expect("checked above");
        criterion.set_deferral(Some(Deferral { reason, follow_up }));
        Ok(json!({ "criterion": task.criterion(cid) }))
    })
}

/// `criterion undefer`: clear the deferral; the status is derived again.
/// A cleared deferral with no evidence is `pending`.
pub fn criterion_undefer(call: &Call, id: &str, cid: &str) -> Result<Done, Failure> {
    call.task(id, |task, actor| {
        work_runner(actor, "criterion undefer")?;
        working_runner(task, actor, "criteria")?;
        let Some(criterion) = task.criterion(cid) else {
            return Err(part_not_found(task, cid));
        };
        if criterion.deferral.is_none() {
            return Err(Failure::new(
                Code::StatusWrong,
                format!(
                    "{cid} is not deferred: its status is {}",
                    word(&criterion.effective_status())
                ),
            )
            .remedy(format!("talita task show {id}")));
        }
        task.criterion_mut(cid)
            .expect("checked above")
            .set_deferral(None);
        Ok(json!({ "criterion": task.criterion(cid) }))
    })
}

/// `criterion explain`: why a criterion failed. Refused unless the derived
/// status is `failed` (`criterion.not_failed`).
pub fn criterion_explain(
    call: &Call,
    id: &str,
    cid: &str,
    failure_reason: Option<&str>,
) -> Result<Done, Failure> {
    let failure_reason = needed("--failure-reason", failure_reason)?;
    call.task(id, |task, actor| {
        work_runner(actor, "criterion explain")?;
        working_runner(task, actor, "criteria")?;
        let Some(criterion) = task.criterion(cid) else {
            return Err(part_not_found(task, cid));
        };
        let status = criterion.effective_status();
        if status != CriterionStatus::Failed {
            return Err(Failure::new(
                Code::CriterionNotFailed,
                format!(
                    "{cid} is {}, not failed; there is nothing to explain",
                    word(&status)
                ),
            )
            .remedy(format!("talita task show {id}")));
        }
        task.criterion_mut(cid)
            .expect("checked above")
            .failure_reason = failure_reason;
        Ok(json!({ "criterion": task.criterion(cid) }))
    })
}

// ---- step

/// The fields `step add` and `step revise` take.
#[derive(Default)]
pub struct StepFields {
    pub action: Option<String>,
    pub done_when: Option<String>,
    pub covers: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub after: Option<Vec<String>>,
    pub deviation: Option<String>,
}

impl StepFields {
    fn read(&mut self) -> Result<(), Failure> {
        for (flag, value) in [
            ("--action", &mut self.action),
            ("--done-when", &mut self.done_when),
            ("--deviation", &mut self.deviation),
        ] {
            if let Some(v) = value.take() {
                *value = Some(required_text(flag, &v)?);
            }
        }
        Ok(())
    }
}

/// Who may write steps now: the creator before approval (ownership), the
/// working runner after. Returns whether the write is after approval, where
/// a deviation is required.
fn step_writer(task: &Task, actor: &Actor) -> Result<bool, Failure> {
    own(Section::Steps, actor, task)?;
    if pre_approval(task) {
        return Ok(false);
    }
    working_runner(task, actor, "steps")?;
    Ok(true)
}

/// The working runner (commands.md, Conventions): in `executing`, the claim's
/// holder; in `remediating`, a session that is not the claim's. `subject` names
/// what the caller writes, for the refusal's message.
pub(crate) fn working_runner(task: &Task, actor: &Actor, subject: &str) -> Result<(), Failure> {
    match task.status {
        Status::Executing => {
            let holds = task
                .claim
                .as_ref()
                .is_some_and(|c| c.by == actor.id && c.session == actor.session);
            if !holds {
                return Err(Failure::new(
                    Code::ClaimNotHolder,
                    format!(
                        "only the claim's holder, in its session, writes {}'s {subject}",
                        task.id
                    ),
                )
                .remedy(format!("talita claim show {}", task.id)));
            }
            Ok(())
        }
        Status::Remediating => {
            let same = task
                .claim
                .as_ref()
                .is_some_and(|c| c.session.is_some() && c.session == actor.session);
            if same {
                let mut failure = Failure::new(
                    Code::GateRefused,
                    "the remediating session must not be the execution session",
                );
                failure.gates.push(GateDetail {
                    gate: "remediation.session_distinct".into(),
                    detail: "the acting session is claim.session".into(),
                });
                return Err(failure);
            }
            Ok(())
        }
        other => Err(Failure::new(
            Code::StatusWrong,
            format!(
                "after approval, {subject} are written in executing or remediating; {} is {}",
                task.id,
                other.as_str()
            ),
        )),
    }
}

/// The runner a work verb takes (commands.md, Conventions: the working
/// runner). A criterion's deferral and failure reason have no owning section
/// — ADR 0013 keeps them writable while the criterion itself is frozen — so
/// the role is checked here; the session rule is `working_runner`.
pub(crate) fn work_runner(actor: &Actor, verb: &str) -> Result<(), Failure> {
    if actor.kind != ActorKind::Agent {
        return Err(
            Failure::new(Code::ActorNotAgent, format!("{verb} is a runner's act"))
                .remedy("act as --role runner"),
        );
    }
    if actor.role != Some(Role::Runner) {
        return Err(
            Failure::new(Code::RoleWrong, format!("{verb} needs the runner role"))
                .remedy("act as --role runner"),
        );
    }
    Ok(())
}

/// `--covers` and `--after` refuse ids the task does not hold.
fn check_refs(task: &Task, fields: &StepFields, own_id: Option<&str>) -> Result<(), Failure> {
    for c in fields.covers.iter().flatten() {
        if task.criterion(c).is_none() {
            return Err(part_not_found(task, c));
        }
    }
    for s in fields.after.iter().flatten() {
        if task.step(s).is_none() || Some(s.as_str()) == own_id {
            return Err(part_not_found(task, s));
        }
    }
    Ok(())
}

fn deviation_required(after_approval: bool, fields: &StepFields) -> Result<(), Failure> {
    if after_approval && fields.deviation.is_none() {
        return Err(Failure::new(
            Code::UsageInvalid,
            "after approval a step write is a deviation: --deviation is required",
        ));
    }
    Ok(())
}

pub fn step_add(call: &Call, id: &str, mut fields: StepFields) -> Result<Done, Failure> {
    fields.read()?;
    let (Some(action), Some(done_when)) = (fields.action.clone(), fields.done_when.clone()) else {
        return Err(Failure::new(
            Code::UsageInvalid,
            "step add needs --action and --done-when",
        ));
    };
    call.task(id, |task, actor| {
        let after_approval = step_writer(task, actor)?;
        deviation_required(after_approval, &fields)?;
        check_refs(task, &fields, None)?;
        let sid = next_part_id("STP", task.steps.iter().map(|s| s.id.as_str()));
        let order = task.steps.iter().map(|s| s.order).max().unwrap_or(0) + 1;
        task.steps.push(Step {
            id: sid.clone(),
            order,
            action,
            done_when,
            files: fields.files.unwrap_or_default(),
            status: StepStatus::Pending,
            covers: fields.covers.unwrap_or_default(),
            depends_on: fields.after.unwrap_or_default(),
            deviation: fields.deviation.unwrap_or_default(),
        });
        Ok(json!({ "step": task.step(&sid) }))
    })
}

pub fn step_revise(
    call: &Call,
    id: &str,
    sid: &str,
    mut fields: StepFields,
) -> Result<Done, Failure> {
    fields.read()?;
    let StepFields {
        action,
        done_when,
        covers,
        files,
        after,
        deviation,
    } = &fields;
    if action.is_none()
        && done_when.is_none()
        && covers.is_none()
        && files.is_none()
        && after.is_none()
    {
        return Err(Failure::new(
            Code::UsageInvalid,
            "step revise needs a field to change",
        ));
    }
    call.task(id, |task, actor| {
        let after_approval = step_writer(task, actor)?;
        deviation_required(after_approval, &fields)?;
        if task.step(sid).is_none() {
            return Err(part_not_found(task, sid));
        }
        check_refs(task, &fields, Some(sid))?;
        let step = task.step_mut(sid).expect("checked above");
        if let Some(v) = action {
            step.action = v.clone();
        }
        if let Some(v) = done_when {
            step.done_when = v.clone();
        }
        if let Some(v) = covers {
            step.covers = v.clone();
        }
        if let Some(v) = files {
            step.files = v.clone();
        }
        if let Some(v) = after {
            step.depends_on = v.clone();
        }
        if let Some(v) = deviation {
            step.deviation = v.clone();
        }
        Ok(json!({ "step": task.step(sid) }))
    })
}

pub fn step_remove(call: &Call, id: &str, sid: &str) -> Result<Done, Failure> {
    call.task(id, |task, actor| {
        own(Section::Steps, actor, task)?;
        if !pre_approval(task) {
            return Err(Failure::new(
                Code::StatusWrong,
                format!(
                    "a step is removed in draft or groomed; {} is {}",
                    task.id,
                    task.status.as_str()
                ),
            )
            .remedy(format!("talita step skip {id} {sid} --deviation <why>")));
        }
        if task.step(sid).is_none() {
            return Err(part_not_found(task, sid));
        }
        task.steps.retain(|s| s.id != sid);
        for step in &mut task.steps {
            step.depends_on.retain(|d| d != sid);
        }
        Ok(json!({ "removed": sid }))
    })
}

/// `step begin`: the runner starts the step.
pub fn step_begin(call: &Call, id: &str, sid: &str) -> Result<Done, Failure> {
    step_status(
        call,
        id,
        sid,
        "step begin",
        StepStatus::InProgress,
        None,
        false,
    )
}

/// `step done`: done, with the deviation when the step went differently.
pub fn step_done(
    call: &Call,
    id: &str,
    sid: &str,
    deviation: Option<&str>,
) -> Result<Done, Failure> {
    step_status(
        call,
        id,
        sid,
        "step done",
        StepStatus::Done,
        deviation,
        false,
    )
}

/// `step skip`: skipped; the deviation is the why.
pub fn step_skip(
    call: &Call,
    id: &str,
    sid: &str,
    deviation: Option<&str>,
) -> Result<Done, Failure> {
    step_status(
        call,
        id,
        sid,
        "step skip",
        StepStatus::Skipped,
        deviation,
        true,
    )
}

/// `step fail`: failed; the deviation is the why.
pub fn step_fail(
    call: &Call,
    id: &str,
    sid: &str,
    deviation: Option<&str>,
) -> Result<Done, Failure> {
    step_status(
        call,
        id,
        sid,
        "step fail",
        StepStatus::Failed,
        deviation,
        true,
    )
}

/// The one write behind the four work verbs. The docs name no transition
/// rule, so the verb writes the status it names.
fn step_status(
    call: &Call,
    id: &str,
    sid: &str,
    verb: &str,
    status: StepStatus,
    deviation: Option<&str>,
    deviation_needed: bool,
) -> Result<Done, Failure> {
    let deviation = deviation
        .map(|value| required_text("--deviation", value))
        .transpose()?;
    if deviation_needed && deviation.is_none() {
        return Err(Failure::new(
            Code::InputInvalid,
            "--deviation is required: why the step did not go as written",
        ));
    }
    call.task(id, |task, actor| {
        work_runner(actor, verb)?;
        working_runner(task, actor, "steps")?;
        let Some(step) = task.step_mut(sid) else {
            return Err(part_not_found(task, sid));
        };
        // A failure stays on the record until the step is retried or skipped.
        if status == StepStatus::Done && step.status == StepStatus::Failed {
            return Err(Failure::new(
                Code::StatusWrong,
                format!("{sid} failed: step done is refused until it is retried or skipped"),
            )
            .remedy(settle_calls(id, sid)));
        }
        step.status = status;
        if let Some(deviation) = deviation {
            step.deviation = deviation;
        }
        Ok(json!({ "step": task.step(sid) }))
    })
}

/// The calls that settle a failed step: a retry, or a skip with the why.
pub(crate) fn settle_calls(id: &str, sid: &str) -> String {
    format!("talita step begin {id} {sid} (retry); talita step skip {id} {sid} --deviation <why>")
}

// ---- question

pub fn ask(call: &Call, id: &str, question: &str) -> Result<Done, Failure> {
    let question = required_text("--question", question)?;
    call.task(id, |task, actor| {
        own(Section::QuestionAsk, actor, task)?;
        let qid = next_part_id("QST", task.questions.iter().map(|q| q.id.as_str()));
        task.questions.push(Question {
            id: qid.clone(),
            question,
            status: QuestionStatus::Open,
            raised_by: actor.id.clone(),
            raised_at: coding_crew_core::now_rfc3339(),
            ..Question::default()
        });
        Ok(json!({ "question": task.questions.iter().find(|q| q.id == qid) }))
    })
}

pub fn answer(
    call: &Call,
    id: &str,
    qid: &str,
    answer: &str,
    instructed_by: Option<&str>,
) -> Result<Done, Failure> {
    let answer = required_text("--answer", answer)?;
    if instructed_by.is_some_and(|h| h.trim().is_empty()) {
        return Err(Failure::new(Code::InputInvalid, "--instructed-by is empty"));
    }
    call.task(id, |task, actor| {
        if let Err(mut failure) = own(Section::QuestionAnswer, actor, task) {
            // A human's route is the creator's, with the human's name (ADR 0015).
            if failure.code == Code::ActorNotAgent {
                failure.remedy = Some(format!(
                    "a creator agent records it: talita question answer {id} {qid} --answer <t> --instructed-by <human>"
                ));
            }
            return Err(failure);
        }
        let Some(question) = task.questions.iter_mut().find(|q| q.id == qid) else {
            return Err(part_not_found(task, qid));
        };
        if question.status == QuestionStatus::Resolved {
            return Err(
                Failure::new(Code::QuestionResolved, format!("{qid} is already answered"))
                    .remedy(format!("talita question ask {id} --question <t>")),
            );
        }
        question.status = QuestionStatus::Resolved;
        question.answer = answer;
        question.resolved_by = actor.id.clone();
        question.resolved_at = coding_crew_core::now_rfc3339();
        question.instructed_by = instructed_by.map(str::to_string);
        Ok(json!({ "question": question, "instructed_by": instructed_by }))
    })
}

// ---- note

pub fn note(call: &Call, id: &str, body: &str) -> Result<Done, Failure> {
    let body = required_text("--body", body)?;
    call.task_journaled(id, |task, actor| {
        own(Section::Notes, actor, task)?;
        let nid = next_part_id("NOTE", task.notes.iter().map(|n| n.id.as_str()));
        task.notes.push(Note {
            id: nid.clone(),
            author: actor.id.clone(),
            body,
            at: coding_crew_core::now_rfc3339(),
        });
        Ok((
            json!({ "note": task.notes.iter().find(|n| n.id == nid) }),
            vec![NewEvent::new(EventKind::Commented)],
        ))
    })
}
