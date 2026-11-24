use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::actor::{Actor, ActorKind, Role};
use crate::task::{
    AcceptanceCriterion, CriterionStatus, ReReview, Round, RoundKind, StepStatus, Task, Verdict,
};

/// The canonical status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    #[default]
    Draft,
    Groomed,
    Approved,
    Executing,
    CodeReview,
    Remediating,
    NeedsHuman,
    Done,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Groomed => "groomed",
            Status::Approved => "approved",
            Status::Executing => "executing",
            Status::CodeReview => "code-review",
            Status::Remediating => "remediating",
            Status::NeedsHuman => "needs-human",
            Status::Done => "done",
        }
    }

    /// The lane the status is in. `draft` and `done` are in neither.
    pub fn lane(&self) -> Option<Lane> {
        match self {
            Status::Groomed | Status::CodeReview | Status::NeedsHuman => Some(Lane::Human),
            Status::Approved | Status::Executing | Status::Remediating => Some(Lane::Agent),
            Status::Draft | Status::Done => None,
        }
    }
}

/// Who owns a task's next action. A view of the status, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// The human lane: approve, review, decide.
    Human,
    /// The agent lane: claim, execute, remediate.
    Agent,
}

impl Lane {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lane::Human => "human",
            Lane::Agent => "agent",
        }
    }
}

/// Why a task waits on a human.
///
/// A reason is a phase in words: `groomed` waits for approval, `code-review`
/// for a review, and `needs-human` for a decision after an escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The task is groomed and waits for approval.
    Approve,
    /// The task is in review.
    Review,
    /// The review budget is spent and a human has to decide.
    Escalated,
}

impl Reason {
    /// Every reason, in the order the lifecycle reaches them.
    pub const ALL: [Reason; 3] = [Reason::Approve, Reason::Review, Reason::Escalated];

    pub fn label(&self) -> &'static str {
        match self {
            Reason::Approve => "approve",
            Reason::Review => "review",
            Reason::Escalated => "escalated",
        }
    }
}

/// An overlay: orthogonal to the status, applies to any phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlay {
    pub reason: String,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlays {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<Overlay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferred: Option<Overlay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<Overlay>,
}

impl Overlays {
    /// Whether any overlay is set.
    pub fn any(&self) -> bool {
        self.blocked.is_some() || self.deferred.is_some() || self.cancelled.is_some()
    }
}

/// The events that move a task forward or hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// draft → groomed
    Groom,
    /// groomed → approved
    Approve,
    /// approved → executing
    Start,
    /// executing → code-review
    Finish,
    /// code-review → done
    PassReview,
    /// code-review → remediating
    RequestChanges,
    /// remediating → code-review
    ReReview,
    /// code-review → needs-human
    Escalate,
    /// needs-human → executing
    ResumeWork,
    /// done → executing
    Reopen,
    /// hold the task
    Block,
    Unblock,
    Defer,
    Undefer,
    Cancel,
}

impl Event {
    /// The role allowed to perform this event.
    pub fn required_role(&self) -> Option<Role> {
        match self {
            Event::Groom | Event::Approve | Event::ResumeWork | Event::Reopen => {
                Some(Role::Creator)
            }
            Event::Start | Event::Finish | Event::ReReview => Some(Role::Runner),
            Event::PassReview | Event::RequestChanges | Event::Escalate => Some(Role::Reviewer),
            Event::Block | Event::Unblock | Event::Defer | Event::Undefer | Event::Cancel => None,
        }
    }
}

/// What the transition needs to know about the world outside the task.
#[derive(Clone, Copy, Default)]
pub struct TransitionContext<'a> {
    /// Whether a dependency task is done. Absent means "cannot check".
    pub dependency_done: Option<&'a dyn Fn(&str) -> bool>,
    /// Reason for an overlay event.
    pub reason: Option<&'a str>,
}

/// A gate an event fails: its name in the task model (§ 3.3) and what was
/// found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GateFailure {
    pub gate: &'static str,
    pub message: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransitionError {
    #[error("{event:?} needs the {role:?} role")]
    WrongRole { event: Event, role: Role },
    #[error("only agents act on tasks")]
    NotAnAgent,
    #[error("cannot {event:?} from status {status:?}")]
    WrongStatus { event: Event, status: Status },
    #[error("task is cancelled")]
    Cancelled,
    /// Every gate the event fails, in the order the task model lists them.
    #[error("gate failed: {}", gate_messages(.0))]
    Gates(Vec<GateFailure>),
}

fn gate_messages(failures: &[GateFailure]) -> String {
    failures
        .iter()
        .map(|failure| failure.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Apply an event to a task, enforcing the gates.
///
/// The role, the cancellation and the status are checked first. Then every
/// gate runs, and a refusal carries all the failing ones (see [`check`]).
///
/// `now` is the instant the move happens and everything it records, as the
/// claim operations take it: the caller names the clock, so a test reads the
/// one it chose and a verb's records share one instant.
pub fn transition(
    task: &mut Task,
    event: Event,
    actor: &Actor,
    ctx: &TransitionContext<'_>,
    now: &str,
) -> Result<(), TransitionError> {
    if let Some(role) = event.required_role() {
        if actor.kind != ActorKind::Agent {
            return Err(TransitionError::NotAnAgent);
        }
        if actor.role != Some(role) {
            return Err(TransitionError::WrongRole { event, role });
        }
    }

    // A cancelled task refuses every event, overlays included: the
    // cancellation is a record of a decision, and nothing overwrites it.
    if task.overlays.cancelled.is_some() {
        return Err(TransitionError::Cancelled);
    }

    let Some((from, to)) = edge(event) else {
        // An overlay holds or releases the task and moves no status.
        let failures = check(task, event, actor, ctx, now);
        if !failures.is_empty() {
            return Err(TransitionError::Gates(failures));
        }
        apply_overlay(task, event, actor, ctx, now);
        return Ok(());
    };

    if task.status != from {
        return Err(TransitionError::WrongStatus {
            event,
            status: task.status,
        });
    }
    let failures = check(task, event, actor, ctx, now);
    if !failures.is_empty() {
        return Err(TransitionError::Gates(failures));
    }

    // The task enters its new status now: the wait in it begins now. The
    // records the move owes are written with it, under the same instant.
    match event {
        Event::Approve => task.planned_by = actor.session.clone(),
        Event::ReReview => task.review.re_reviews.push(ReReview {
            at: now.to_string(),
            after_round: latest_external(task).map_or(0, |round| round.n),
            by: actor.id.clone(),
            session: actor.session.clone(),
        }),
        _ => {}
    }
    task.status = to;
    task.status_since = now.to_string();
    task.updated = now.to_string();
    Ok(())
}

/// Every gate `event` fails on `task`, named as the task model names them
/// (§ 3.3). An empty list passes.
///
/// The role, the cancellation and the status are `transition`'s to check: this
/// assumes the task is in the status the event leaves. `now` is the instant
/// the gates are checked against, as it is for [`transition`].
pub fn check(
    task: &Task,
    event: Event,
    actor: &Actor,
    ctx: &TransitionContext<'_>,
    now: &str,
) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    match event {
        Event::Groom => {
            gate(
                &mut failures,
                "intent.outcome_present",
                !task.intent.outcome.trim().is_empty(),
                "intent.outcome is required",
            );
            gate(
                &mut failures,
                "criteria.present",
                !task.acceptance_criteria.is_empty(),
                "at least one acceptance criterion is required",
            );
            gate(
                &mut failures,
                "scope.out_of_scope_declared",
                !task.scope.out_of_scope.is_empty(),
                "at least one out_of_scope entry is required",
            );
        }
        Event::Approve => {
            gate(
                &mut failures,
                "questions.resolved",
                task.open_questions() == 0,
                "open questions must be resolved",
            );
            // The step rules do not apply to a small task.
            if !task.is_small_task() {
                gate(
                    &mut failures,
                    "criteria.covered",
                    task.criteria_covered(),
                    "every criterion needs a covering step",
                );
                gate(
                    &mut failures,
                    "steps.done_when_present",
                    task.steps_have_done_when(),
                    "every step needs a done_when",
                );
            }
        }
        Event::Start => {
            gate(
                &mut failures,
                "task.claimed",
                task.claim.is_some(),
                "task is not claimed",
            );
            if let Some(claim) = &task.claim {
                gate(
                    &mut failures,
                    "task.claimed",
                    claim.by == actor.id,
                    "the task is claimed by someone else",
                );
                gate(
                    &mut failures,
                    "task.claimed",
                    !claim.lease_expired(now),
                    "the claim's lease has expired",
                );
            }
            check_dependencies(&mut failures, task, ctx);
        }
        Event::Finish => {
            // The step rules do not apply to a small task.
            if !task.is_small_task() {
                gate(
                    &mut failures,
                    "steps.settled",
                    task.steps
                        .iter()
                        .all(|s| matches!(s.status, StepStatus::Done | StepStatus::Skipped)),
                    "every step must be settled",
                );
            }
            for criterion in &task.acceptance_criteria {
                check_criterion(&mut failures, criterion);
            }
            gate(
                &mut failures,
                "self_round.filed",
                task.self_rounds() > 0,
                "a self-review round is required",
            );
            check_holder(&mut failures, task, actor);
        }
        Event::PassReview => {
            gate(
                &mut failures,
                "review.verdict_ok",
                task.review.verdict == Some(Verdict::Ok),
                "the review verdict is not ok",
            );
            gate(
                &mut failures,
                "review.no_open_findings",
                task.open_findings() == 0,
                "open findings remain",
            );
            gate(
                &mut failures,
                "review.within_budget",
                task.external_rounds() as u32 <= task.review.max_rounds,
                "the review round budget is exhausted",
            );
            check_review(&mut failures, task, actor);
        }
        Event::RequestChanges => {
            gate(
                &mut failures,
                "review.verdict_changes_needed",
                task.review.verdict == Some(Verdict::ChangesNeeded),
                "the review verdict is not changes-needed",
            );
            gate(
                &mut failures,
                "review.findings_exist",
                task.open_findings() > 0,
                "no findings were opened",
            );
            gate(
                &mut failures,
                "review.budget_left",
                (task.external_rounds() as u32) < task.review.max_rounds,
                "the review round budget is exhausted",
            );
            check_review(&mut failures, task, actor);
        }
        Event::ReReview => {
            gate(
                &mut failures,
                "review.findings_closed",
                task.open_findings() == 0,
                "open findings remain",
            );
            check_remediation(&mut failures, task, actor);
        }
        Event::Escalate => {
            gate(
                &mut failures,
                "review.verdict_changes_needed",
                task.review.verdict == Some(Verdict::ChangesNeeded),
                "the review verdict is not changes-needed",
            );
            gate(
                &mut failures,
                "review.budget_exhausted",
                task.external_rounds() as u32 >= task.review.max_rounds,
                "the review round budget is not exhausted",
            );
            check_review(&mut failures, task, actor);
        }
        Event::ResumeWork | Event::Reopen => {}
        Event::Block | Event::Defer | Event::Cancel => gate(
            &mut failures,
            "overlay.reason_present",
            !ctx.reason.unwrap_or_default().trim().is_empty(),
            "a reason is required",
        ),
        Event::Unblock | Event::Undefer => {}
    }
    failures
}

/// The status an event leaves and the status it enters. `None` for an
/// overlay, which moves no status.
fn edge(event: Event) -> Option<(Status, Status)> {
    Some(match event {
        Event::Groom => (Status::Draft, Status::Groomed),
        Event::Approve => (Status::Groomed, Status::Approved),
        Event::Start => (Status::Approved, Status::Executing),
        Event::Finish => (Status::Executing, Status::CodeReview),
        Event::PassReview => (Status::CodeReview, Status::Done),
        Event::RequestChanges => (Status::CodeReview, Status::Remediating),
        Event::ReReview => (Status::Remediating, Status::CodeReview),
        Event::Escalate => (Status::CodeReview, Status::NeedsHuman),
        Event::ResumeWork => (Status::NeedsHuman, Status::Executing),
        Event::Reopen => (Status::Done, Status::Executing),
        Event::Block | Event::Unblock | Event::Defer | Event::Undefer | Event::Cancel => {
            return None;
        }
    })
}

fn apply_overlay(
    task: &mut Task,
    event: Event,
    actor: &Actor,
    ctx: &TransitionContext<'_>,
    now: &str,
) {
    let overlay = Overlay {
        reason: ctx.reason.unwrap_or_default().to_string(),
        by: actor.id.clone(),
        at: now.to_string(),
    };
    match event {
        Event::Block => task.overlays.blocked = Some(overlay),
        Event::Unblock => task.overlays.blocked = None,
        Event::Defer => task.overlays.deferred = Some(overlay),
        Event::Undefer => task.overlays.deferred = None,
        Event::Cancel => task.overlays.cancelled = Some(overlay),
        _ => unreachable!("only overlay events hold a task without a status"),
    }
    task.updated = now.to_string();
}

/// A criterion at finish: parked on purpose, or proven by its evidence.
fn check_criterion(failures: &mut Vec<GateFailure>, criterion: &AcceptanceCriterion) {
    let id = criterion.id.as_str();
    // A deferred criterion is parked on purpose: it needs no evidence.
    if criterion.deferral.is_some() {
        return;
    }
    if criterion.status == CriterionStatus::Deferred {
        fail(
            failures,
            "criteria.proven",
            &format!("{id} is deferred without a deferral"),
        );
        return;
    }
    if criterion.evidence.is_empty() {
        fail(
            failures,
            "evidence.attached",
            &format!("{id} has no evidence"),
        );
        return;
    }
    match criterion.effective_status() {
        CriterionStatus::Passed => {}
        CriterionStatus::Failed => gate(
            failures,
            "criteria.proven",
            !criterion.failure_reason.trim().is_empty(),
            &format!("{id} failed without a reason"),
        ),
        other => fail(
            failures,
            "criteria.proven",
            &format!("{id} is {}", other.as_str()),
        ),
    }
}

/// Only the claim holder, in the session that claimed, hands work to review.
/// The lease is not read here: an expired claim still belongs to its holder.
fn check_holder(failures: &mut Vec<GateFailure>, task: &Task, actor: &Actor) {
    gate(
        failures,
        "task.claimed",
        task.claim.is_some(),
        "task is not claimed",
    );
    if let Some(claim) = &task.claim {
        gate(
            failures,
            "task.claimed",
            claim.by == actor.id,
            "the task is claimed by someone else",
        );
        gate(
            failures,
            "task.claimed",
            same_session(claim.session.as_deref(), actor.session.as_deref()),
            "only the claiming session may finish the task",
        );
    }
}

fn check_dependencies(failures: &mut Vec<GateFailure>, task: &Task, ctx: &TransitionContext<'_>) {
    if task.depends_on.is_empty() {
        return;
    }
    let Some(is_done) = ctx.dependency_done else {
        fail(
            failures,
            "dependencies.satisfied",
            "dependencies cannot be checked without the plan",
        );
        return;
    };
    for dependency in &task.depends_on {
        gate(
            failures,
            "dependencies.satisfied",
            is_done(dependency),
            &format!("dependency {dependency} is not done"),
        );
    }
}

/// The reviewer's gates: the session apart from the planner and the executor,
/// and a verdict that the latest external round still holds.
fn check_review(failures: &mut Vec<GateFailure>, task: &Task, actor: &Actor) {
    check_independence(failures, task, actor);
    check_round_current(failures, task);
    check_round_owner(failures, task, actor);
}

/// The planning and the executing session never review the task. All three
/// sessions must be recorded: a check that cannot name one does not pass.
///
/// `round file` calls this at filing, as ADR 0017 requires; it is the same
/// rule the reviewer's transitions run.
pub fn check_independence(failures: &mut Vec<GateFailure>, task: &Task, actor: &Actor) {
    let planned = task.planned_by.as_deref();
    let executed = claim_session(task);
    let acting = actor.session.as_deref();
    gate(
        failures,
        "review.independent",
        planned.is_some(),
        "the planning session is not recorded",
    );
    gate(
        failures,
        "review.independent",
        executed.is_some(),
        "the executing session is not recorded",
    );
    gate(
        failures,
        "review.independent",
        acting.is_some(),
        "the reviewing session is not recorded",
    );
    gate(
        failures,
        "review.independent",
        !same_session(acting, planned),
        "the session that planned the task cannot review it",
    );
    gate(
        failures,
        "review.independent",
        !same_session(acting, executed),
        "the session that executed the task cannot review it",
    );
}

/// The latest external round was filed at or after the task entered
/// `code-review`. A verdict filed before that entry is a stale one.
fn check_round_current(failures: &mut Vec<GateFailure>, task: &Task) {
    let Some(round) = latest_external(task) else {
        fail(
            failures,
            "review.round_current",
            "no external round is filed",
        );
        return;
    };
    gate(
        failures,
        "review.round_current",
        filed_since_entry(task, round),
        "the latest external round was not filed since the task entered code-review",
    );
}

/// Whether `round` was filed at or after the task entered its status. A round
/// filed before that entry is stale.
fn filed_since_entry(task: &Task, round: &Round) -> bool {
    let entered = crate::parse_rfc3339(&task.status_since);
    let filed = crate::parse_rfc3339(&round.at);
    matches!((entered, filed), (Some(entered), Some(filed)) if filed >= entered)
}

/// Why the task waits on a human, if it does: the human lane, minus the tasks
/// carrying an overlay. A blocked, deferred or cancelled task is not waiting
/// on a human (`docs/tui/screens.md`, "Derived, never stored").
pub fn needs_you(task: &Task) -> Option<Reason> {
    if task.overlays.any() {
        return None;
    }
    match task.status {
        Status::Groomed => Some(Reason::Approve),
        Status::CodeReview => Some(Reason::Review),
        Status::NeedsHuman => Some(Reason::Escalated),
        _ => None,
    }
}

/// A task's review state, derived from its rounds and findings
/// (`docs/tasks/task-model.md` § 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// A finding is open.
    OpenFindings,
    /// In `code-review`, with no external round filed since the task entered it.
    PendingReview,
    /// The latest external round is filed, and no finding is open.
    Clean,
}

impl ReviewState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewState::OpenFindings => "open-findings",
            ReviewState::PendingReview => "pending-review",
            ReviewState::Clean => "clean",
        }
    }
}

/// The review state of `task`, or `None` when the task is not in review: no
/// external round is filed and no finding is open.
///
/// The rules are checked in order. An open finding is `open-findings`. A
/// `code-review` task whose latest external round predates its entry, or that
/// has none, is `pending-review`. A task whose latest external round is filed
/// is `clean`.
pub fn review_state(task: &Task) -> Option<ReviewState> {
    if task.open_findings() > 0 {
        return Some(ReviewState::OpenFindings);
    }
    let latest = latest_external(task);
    let current = latest.is_some_and(|round| filed_since_entry(task, round));
    if task.status == Status::CodeReview && !current {
        return Some(ReviewState::PendingReview);
    }
    latest.map(|_| ReviewState::Clean)
}

/// The session that filed the latest external round makes the move.
pub fn check_round_owner(failures: &mut Vec<GateFailure>, task: &Task, actor: &Actor) {
    let Some(round) = latest_external(task) else {
        fail(failures, "review.round_owner", "no external round is filed");
        return;
    };
    gate(
        failures,
        "review.round_owner",
        same_session(round.session.as_deref(), actor.session.as_deref()),
        "only the session that filed the latest external round may move the task",
    );
}

/// The session that executed a task never remediates its findings. Both
/// sessions must be recorded: a check that cannot name a session does not pass.
pub fn check_remediation(failures: &mut Vec<GateFailure>, task: &Task, actor: &Actor) {
    let executed = claim_session(task);
    let acting = actor.session.as_deref();
    gate(
        failures,
        "remediation.session_distinct",
        executed.is_some(),
        "the executing session is not recorded",
    );
    gate(
        failures,
        "remediation.session_distinct",
        acting.is_some(),
        "the remediating session is not recorded",
    );
    gate(
        failures,
        "remediation.session_distinct",
        !same_session(acting, executed),
        "the session that executed the task cannot remediate it",
    );
}

/// The latest external round: the verdict the reviewer's move rests on.
pub fn latest_external(task: &Task) -> Option<&Round> {
    task.review
        .rounds
        .iter()
        .filter(|round| round.kind == RoundKind::External)
        .max_by_key(|round| round.n)
}

fn claim_session(task: &Task) -> Option<&str> {
    task.claim
        .as_ref()
        .and_then(|claim| claim.session.as_deref())
}

/// Two recorded sessions that are the same. An unrecorded session is never the
/// same as anything, so a check that cannot name a session fails its own gate.
fn same_session(a: Option<&str>, b: Option<&str>) -> bool {
    a.is_some() && a == b
}

fn gate(failures: &mut Vec<GateFailure>, gate: &'static str, holds: bool, message: &str) {
    if !holds {
        fail(failures, gate, message);
    }
}

fn fail(failures: &mut Vec<GateFailure>, gate: &'static str, message: &str) {
    failures.push(GateFailure {
        gate,
        message: message.to_string(),
    });
}

impl CriterionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CriterionStatus::Pending => "pending",
            CriterionStatus::Passed => "passed",
            CriterionStatus::Failed => "failed",
            CriterionStatus::Unproven => "unproven",
            CriterionStatus::Deferred => "deferred",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Finding, FindingStatus, Round, RoundKind, Severity, Task};

    const ENTERED: &str = "2026-11-20T10:00:00Z";
    const BEFORE: &str = "2026-11-19T10:00:00Z";
    const AFTER: &str = "2026-11-21T10:00:00Z";

    fn task(status: Status) -> Task {
        let mut task = Task::new("TSK01", "a task");
        task.status = status;
        task.status_since = ENTERED.to_string();
        task
    }

    fn round(kind: RoundKind, at: &str) -> Round {
        Round {
            n: 1,
            kind,
            at: at.to_string(),
            ..Round::default()
        }
    }

    fn finding(status: FindingStatus) -> Finding {
        Finding {
            id: "FIND-01".to_string(),
            severity: Severity::High,
            status,
            ..Finding::default()
        }
    }

    fn overlay() -> Option<Overlay> {
        Some(Overlay {
            reason: "waiting".to_string(),
            by: "luiz".to_string(),
            at: AFTER.to_string(),
        })
    }

    #[test]
    fn each_status_is_in_the_lane_the_model_names() {
        for status in [Status::Groomed, Status::CodeReview, Status::NeedsHuman] {
            assert_eq!(status.lane(), Some(Lane::Human), "{status:?}");
        }
        for status in [Status::Approved, Status::Executing, Status::Remediating] {
            assert_eq!(status.lane(), Some(Lane::Agent), "{status:?}");
        }
        assert_eq!(Status::Draft.lane(), None);
        assert_eq!(Status::Done.lane(), None);
    }

    #[test]
    fn a_lane_has_its_word() {
        assert_eq!(Lane::Human.as_str(), "human");
        assert_eq!(Lane::Agent.as_str(), "agent");
    }

    #[test]
    fn an_overlay_is_any_of_the_three() {
        assert!(!Overlays::default().any());
        for overlays in [
            Overlays {
                blocked: overlay(),
                ..Overlays::default()
            },
            Overlays {
                deferred: overlay(),
                ..Overlays::default()
            },
            Overlays {
                cancelled: overlay(),
                ..Overlays::default()
            },
        ] {
            assert!(overlays.any(), "{overlays:?}");
        }
    }

    #[test]
    fn each_human_status_needs_you_for_its_reason() {
        assert_eq!(needs_you(&task(Status::Groomed)), Some(Reason::Approve));
        assert_eq!(needs_you(&task(Status::CodeReview)), Some(Reason::Review));
        assert_eq!(
            needs_you(&task(Status::NeedsHuman)),
            Some(Reason::Escalated)
        );
    }

    #[test]
    fn an_agent_or_parked_status_does_not_need_you() {
        for status in [
            Status::Draft,
            Status::Approved,
            Status::Executing,
            Status::Remediating,
            Status::Done,
        ] {
            assert_eq!(needs_you(&task(status)), None, "{status:?}");
        }
    }

    #[test]
    fn a_task_with_an_overlay_does_not_need_you() {
        let overlays = [
            Overlays {
                blocked: overlay(),
                ..Overlays::default()
            },
            Overlays {
                deferred: overlay(),
                ..Overlays::default()
            },
            Overlays {
                cancelled: overlay(),
                ..Overlays::default()
            },
        ];
        for overlays in overlays {
            let mut groomed = task(Status::Groomed);
            groomed.overlays = overlays;
            assert_eq!(needs_you(&groomed), None, "{:?}", groomed.overlays);
        }
    }

    #[test]
    fn a_task_outside_review_has_no_review_state() {
        assert_eq!(review_state(&task(Status::Draft)), None);
        assert_eq!(review_state(&task(Status::Executing)), None);
        assert_eq!(review_state(&task(Status::Done)), None);
    }

    #[test]
    fn a_code_review_with_no_external_round_is_pending() {
        assert_eq!(
            review_state(&task(Status::CodeReview)),
            Some(ReviewState::PendingReview)
        );
    }

    #[test]
    fn a_self_round_does_not_end_the_wait() {
        let mut pending = task(Status::CodeReview);
        pending
            .review
            .rounds
            .push(round(RoundKind::SelfReview, AFTER));
        assert_eq!(review_state(&pending), Some(ReviewState::PendingReview));
    }

    #[test]
    fn a_round_filed_before_the_entry_is_stale() {
        let mut stale = task(Status::CodeReview);
        stale.review.rounds.push(round(RoundKind::External, BEFORE));
        assert_eq!(review_state(&stale), Some(ReviewState::PendingReview));
    }

    #[test]
    fn a_round_filed_since_the_entry_is_clean() {
        let mut filed = task(Status::CodeReview);
        filed.review.rounds.push(round(RoundKind::External, AFTER));
        assert_eq!(review_state(&filed), Some(ReviewState::Clean));
    }

    #[test]
    fn a_round_filed_at_the_entry_is_current() {
        let mut same = task(Status::CodeReview);
        same.review.rounds.push(round(RoundKind::External, ENTERED));
        assert_eq!(review_state(&same), Some(ReviewState::Clean));
    }

    #[test]
    fn a_round_with_no_readable_time_is_not_current() {
        let mut unread = task(Status::CodeReview);
        unread.review.rounds.push(round(RoundKind::External, ""));
        assert_eq!(review_state(&unread), Some(ReviewState::PendingReview));
    }

    #[test]
    fn an_open_finding_is_open_findings_whatever_the_status() {
        let mut open = task(Status::Remediating);
        open.review.findings.push(finding(FindingStatus::Open));
        assert_eq!(review_state(&open), Some(ReviewState::OpenFindings));

        let mut stale = task(Status::CodeReview);
        stale.review.rounds.push(round(RoundKind::External, BEFORE));
        stale.review.findings.push(finding(FindingStatus::Open));
        assert_eq!(review_state(&stale), Some(ReviewState::OpenFindings));
    }

    #[test]
    fn a_fixed_or_dismissed_finding_is_not_open() {
        let mut fixed = task(Status::CodeReview);
        fixed.review.rounds.push(round(RoundKind::External, AFTER));
        fixed.review.findings.push(finding(FindingStatus::Fixed));
        fixed
            .review
            .findings
            .push(finding(FindingStatus::Dismissed));
        assert_eq!(review_state(&fixed), Some(ReviewState::Clean));
    }

    #[test]
    fn a_filed_round_with_no_open_finding_is_clean_outside_code_review() {
        let mut done = task(Status::Done);
        done.review.rounds.push(round(RoundKind::External, BEFORE));
        assert_eq!(review_state(&done), Some(ReviewState::Clean));
    }

    #[test]
    fn each_review_state_has_its_word() {
        assert_eq!(ReviewState::OpenFindings.as_str(), "open-findings");
        assert_eq!(ReviewState::PendingReview.as_str(), "pending-review");
        assert_eq!(ReviewState::Clean.as_str(), "clean");
    }
}
