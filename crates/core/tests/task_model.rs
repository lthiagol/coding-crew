use coding_crew_core::lifecycle::{Event, TransitionContext, TransitionError, transition};
use coding_crew_core::task::{
    AcceptanceCriterion, Deferral, Evidence, Finding, FindingStatus, Focus, Round, RoundKind,
    Verdict,
};
use coding_crew_core::{Actor, CriterionStatus, Role, Step, StepStatus, Store, Task};

fn creator() -> Actor {
    Actor::agent("creator-1", Role::Creator).in_session("s-creator")
}

fn runner() -> Actor {
    Actor::agent("runner-1", Role::Runner).in_session("s-runner")
}

fn reviewer() -> Actor {
    Actor::agent("reviewer-1", Role::Reviewer).in_session("s-reviewer")
}

fn groomed_task() -> Task {
    let mut task = Task::new("TSK01", "Add lane filtering");
    task.intent.outcome = "raul filters lanes".to_string();
    task.scope.out_of_scope = vec!["no new keys".to_string()];
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        description: "filtering works".to_string(),
        verification: "cargo test".to_string(),
        ..AcceptanceCriterion::default()
    }];
    task.steps = vec![Step {
        id: "STP-01".to_string(),
        order: 1,
        action: "read the filter module".to_string(),
        done_when: "the module is read".to_string(),
        covers: vec!["ACCEPT-01".to_string()],
        ..Step::default()
    }];
    task
}

#[test]
fn draft_needs_outcome_criteria_and_scope() {
    let mut task = Task::new("TSK01", "empty");
    assert!(
        transition(
            &mut task,
            Event::Groom,
            &creator(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.intent.outcome = "something".to_string();
    assert!(
        transition(
            &mut task,
            Event::Groom,
            &creator(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.acceptance_criteria.push(AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        ..AcceptanceCriterion::default()
    });
    assert!(
        transition(
            &mut task,
            Event::Groom,
            &creator(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.scope.out_of_scope.push("no more".to_string());
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Groomed);
}

#[test]
fn only_the_creator_grooms() {
    let mut task = groomed_task();
    let error = transition(
        &mut task,
        Event::Groom,
        &runner(),
        &TransitionContext::default(),
    )
    .expect_err("runner cannot groom");
    assert!(matches!(error, TransitionError::WrongRole { .. }));
}

#[test]
fn approval_needs_coverage_and_done_when() {
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    task.steps[0].done_when = String::new();
    assert!(
        transition(
            &mut task,
            Event::Approve,
            &creator(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.steps[0].done_when = "read".to_string();
    task.acceptance_criteria.push(AcceptanceCriterion {
        id: "ACCEPT-02".to_string(),
        ..AcceptanceCriterion::default()
    });
    let error = transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .expect_err("ACCEPT-02 has no covering step");
    assert!(error.to_string().contains("covering step"), "{error}");

    task.acceptance_criteria.pop();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Approved);
}

#[test]
fn start_needs_a_claim() {
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();

    assert!(
        transition(
            &mut task,
            Event::Start,
            &runner(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.claim = Some(coding_crew_core::Claim {
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
        at: "now".to_string(),
        lease_expires_at: String::new(),
    });
    transition(
        &mut task,
        Event::Start,
        &runner(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Executing);
}

#[test]
fn finish_needs_evidence_and_a_self_round() {
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    task.claim = Some(coding_crew_core::Claim {
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
        at: "now".to_string(),
        lease_expires_at: String::new(),
    });
    transition(
        &mut task,
        Event::Start,
        &runner(),
        &TransitionContext::default(),
    )
    .unwrap();

    task.steps[0].status = StepStatus::Done;
    assert!(
        transition(
            &mut task,
            Event::Finish,
            &runner(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.acceptance_criteria[0].evidence.push(Evidence {
        command: "cargo test".to_string(),
        exit_code: Some(0),
        summary: "3 passed".to_string(),
        at: "now".to_string(),
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
    });
    assert!(
        transition(
            &mut task,
            Event::Finish,
            &runner(),
            &TransitionContext::default()
        )
        .is_err()
    );

    task.review.rounds.push(Round {
        n: 1,
        kind: RoundKind::SelfReview,
        focus: Focus::SpecCompliance,
        reviewer: "runner-1".to_string(),
        verdict: Verdict::Ok,
        at: "now".to_string(),
        notes: String::new(),
        findings: Vec::new(),
    });
    transition(
        &mut task,
        Event::Finish,
        &runner(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);
}

#[test]
fn criterion_status_comes_from_evidence() {
    let criterion = AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        evidence: vec![Evidence {
            command: "cargo test".to_string(),
            exit_code: Some(1),
            summary: "1 failed".to_string(),
            at: "now".to_string(),
            by: "runner-1".to_string(),
            session: None,
        }],
        ..AcceptanceCriterion::default()
    };
    assert_eq!(criterion.effective_status(), CriterionStatus::Failed);
}

#[test]
fn five_external_rounds_escalate_to_a_human() {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::CodeReview;
    task.planned_by = Some("s-creator".to_string());
    task.review.verdict = Some(Verdict::ChangesNeeded);
    task.review.findings.push(Finding {
        id: "FIND-01".to_string(),
        status: FindingStatus::Open,
        ..Finding::default()
    });
    for n in 1..=5 {
        task.review.rounds.push(Round {
            n,
            kind: RoundKind::External,
            focus: Focus::EdgeCases,
            reviewer: "reviewer-1".to_string(),
            verdict: Verdict::ChangesNeeded,
            at: "now".to_string(),
            notes: String::new(),
            findings: vec!["FIND-01".to_string()],
        });
    }

    transition(
        &mut task,
        Event::Escalate,
        &reviewer(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::NeedsHuman);
}

#[test]
fn a_transition_stamps_the_moment_the_status_began() {
    let mut task = groomed_task();
    task.status_since = "2020-01-01T00:00:00Z".to_string();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Groomed);
    assert_ne!(task.status_since, "2020-01-01T00:00:00Z");
}

#[test]
fn an_overlay_does_not_move_the_status_stamp() {
    // A write that does not change the status does not move the moment
    // the task entered it: the wait goes on.
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    task.status_since = "2020-01-01T00:00:00Z".to_string();
    task.updated = "2020-01-01T00:00:00Z".to_string();
    transition(
        &mut task,
        Event::Block,
        &creator(),
        &TransitionContext {
            reason: Some("waiting for the plan"),
            ..TransitionContext::default()
        },
    )
    .unwrap();
    assert_eq!(task.status_since, "2020-01-01T00:00:00Z");
    assert_ne!(task.updated, "2020-01-01T00:00:00Z");
}

#[test]
fn store_round_trips_tasks() {
    let dir = std::env::temp_dir().join(format!("coding-crew-store-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::new(&dir);

    let mut task = store.create("Add lane filtering").unwrap();
    assert_eq!(task.id, "TSK01");
    assert_eq!(task.status, coding_crew_core::Status::Draft);
    task.status_since = "2026-11-13T12:00:00Z".to_string();
    store.save(&task).unwrap();

    let second = store.create("Second task").unwrap();
    assert_eq!(second.id, "TSK02");

    assert_eq!(store.list().unwrap().len(), 2);
    assert_eq!(store.load("TSK02").unwrap().title, "Second task");

    // The moment the task entered its status survives the save and the load.
    assert_eq!(
        store.load("TSK01").unwrap().status_since,
        "2026-11-13T12:00:00Z"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_small_task_skips_the_step_rules() {
    let mut task = Task::new("TSK01", "Fix the lorem pane");
    task.kind = coding_crew_core::TaskKind::Bug;
    task.intent.outcome = "the lorem pane works".to_string();
    task.scope.out_of_scope = vec!["no unrelated refactors".to_string()];
    task.acceptance_criteria = vec![AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        description: "the lorem pane works".to_string(),
        verification: "cargo test".to_string(),
        ..AcceptanceCriterion::default()
    }];

    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    // No steps, no coverage, no done_when: a small task approves anyway.
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Approved);
}

#[test]
fn a_full_task_still_needs_its_steps() {
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    task.steps[0].done_when = String::new();
    assert!(
        transition(
            &mut task,
            Event::Approve,
            &creator(),
            &TransitionContext::default()
        )
        .is_err()
    );
}

#[test]
fn an_expired_lease_cannot_start() {
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();

    task.claim = Some(coding_crew_core::Claim {
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
        at: "2020-01-01T00:00:00Z".to_string(),
        lease_expires_at: "2020-01-01T01:00:00Z".to_string(),
    });
    let error = transition(
        &mut task,
        Event::Start,
        &runner(),
        &TransitionContext::default(),
    )
    .expect_err("the lease expired");
    assert!(error.to_string().contains("lease"));
}

#[test]
fn a_live_lease_starts_the_task() {
    let mut task = groomed_task();
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
    )
    .unwrap();

    task.claim = Some(coding_crew_core::Claim {
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
        at: "now".to_string(),
        lease_expires_at: "2999-01-01T00:00:00Z".to_string(),
    });
    transition(
        &mut task,
        Event::Start,
        &runner(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Executing);
}

fn self_round() -> Round {
    Round {
        n: 1,
        kind: RoundKind::SelfReview,
        focus: Focus::SpecCompliance,
        reviewer: "runner-1".to_string(),
        verdict: Verdict::Ok,
        at: "now".to_string(),
        notes: String::new(),
        findings: Vec::new(),
    }
}

fn passed_evidence() -> Evidence {
    Evidence {
        command: "cargo test".to_string(),
        exit_code: Some(0),
        summary: "3 passed".to_string(),
        at: "now".to_string(),
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
    }
}

/// A task in code review: planned by `s-creator`, executed by `s-runner`.
fn code_review_task() -> Task {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::CodeReview;
    task.planned_by = Some("s-creator".to_string());
    task.claim = Some(coding_crew_core::Claim {
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
        at: "now".to_string(),
        lease_expires_at: String::new(),
    });
    task.review.verdict = Some(Verdict::Ok);
    task
}

fn changes_needed(task: &mut Task) {
    task.review.verdict = Some(Verdict::ChangesNeeded);
    task.review.findings.push(Finding {
        id: "FIND-01".to_string(),
        status: FindingStatus::Open,
        ..Finding::default()
    });
}

fn unrecorded() -> Actor {
    Actor::agent("reviewer-1", Role::Reviewer)
}

fn cancel_reason(reason: &str) -> TransitionContext<'_> {
    TransitionContext {
        reason: Some(reason),
        ..TransitionContext::default()
    }
}

#[test]
fn a_deferred_criterion_finishes_without_evidence() {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::Executing;
    task.steps[0].status = StepStatus::Done;
    task.review.rounds.push(self_round());
    task.acceptance_criteria[0].evidence.push(passed_evidence());
    task.acceptance_criteria.push(AcceptanceCriterion {
        id: "ACCEPT-02".to_string(),
        status: CriterionStatus::Deferred,
        deferral: Some(Deferral {
            reason: "needs an upstream API change".to_string(),
            follow_up: "TSK03".to_string(),
        }),
        ..AcceptanceCriterion::default()
    });

    transition(
        &mut task,
        Event::Finish,
        &runner(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);
}

#[test]
fn a_deferral_is_the_criterion_status() {
    let criterion = AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        evidence: vec![passed_evidence()],
        deferral: Some(Deferral {
            reason: "parked".to_string(),
            follow_up: String::new(),
        }),
        ..AcceptanceCriterion::default()
    };
    assert_eq!(criterion.effective_status(), CriterionStatus::Deferred);
}

#[test]
fn a_deferred_status_needs_its_deferral() {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::Executing;
    task.steps[0].status = StepStatus::Done;
    task.review.rounds.push(self_round());
    task.acceptance_criteria[0].evidence.push(passed_evidence());
    task.acceptance_criteria[0].status = CriterionStatus::Deferred;

    let error = transition(
        &mut task,
        Event::Finish,
        &runner(),
        &TransitionContext::default(),
    )
    .expect_err("a deferred status without a deferral");
    assert!(error.to_string().contains("without a deferral"), "{error}");
}

#[test]
fn pass_review_needs_the_reviewer_session_apart_from_the_planner() {
    let mut task = code_review_task();
    task.review.rounds.push(self_round());
    transition(
        &mut task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Done);

    let mut task = code_review_task();
    let error = transition(
        &mut task,
        Event::PassReview,
        &unrecorded(),
        &TransitionContext::default(),
    )
    .expect_err("a reviewer with no session");
    assert!(error.to_string().contains("not recorded"), "{error}");

    let mut task = code_review_task();
    let planner = Actor::agent("creator-1", Role::Reviewer).in_session("s-creator");
    let error = transition(
        &mut task,
        Event::PassReview,
        &planner,
        &TransitionContext::default(),
    )
    .expect_err("the planning session reviews its own task");
    assert!(error.to_string().contains("cannot review"), "{error}");
}

#[test]
fn a_planner_left_unrecorded_cannot_be_checked() {
    let mut task = code_review_task();
    task.planned_by = None;
    let error = transition(
        &mut task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
    )
    .expect_err("no planning session to check against");
    assert!(error.to_string().contains("not recorded"), "{error}");
}

#[test]
fn request_changes_needs_the_reviewer_session_apart_from_the_planner() {
    let mut task = code_review_task();
    changes_needed(&mut task);
    transition(
        &mut task,
        Event::RequestChanges,
        &reviewer(),
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Remediating);

    let mut task = code_review_task();
    changes_needed(&mut task);
    let error = transition(
        &mut task,
        Event::RequestChanges,
        &unrecorded(),
        &TransitionContext::default(),
    )
    .expect_err("a reviewer with no session");
    assert!(error.to_string().contains("not recorded"), "{error}");

    let mut task = code_review_task();
    changes_needed(&mut task);
    let planner = Actor::agent("creator-1", Role::Reviewer).in_session("s-creator");
    let error = transition(
        &mut task,
        Event::RequestChanges,
        &planner,
        &TransitionContext::default(),
    )
    .expect_err("the planning session reviews its own task");
    assert!(error.to_string().contains("cannot review"), "{error}");
}

#[test]
fn escalate_needs_the_reviewer_session_apart_from_the_planner() {
    let spent = |task: &mut Task| {
        changes_needed(task);
        for n in 1..=5 {
            task.review.rounds.push(Round {
                n,
                kind: RoundKind::External,
                focus: Focus::EdgeCases,
                reviewer: "reviewer-1".to_string(),
                verdict: Verdict::ChangesNeeded,
                at: "now".to_string(),
                notes: String::new(),
                findings: vec!["FIND-01".to_string()],
            });
        }
    };

    let mut task = code_review_task();
    spent(&mut task);
    let error = transition(
        &mut task,
        Event::Escalate,
        &unrecorded(),
        &TransitionContext::default(),
    )
    .expect_err("a reviewer with no session");
    assert!(error.to_string().contains("not recorded"), "{error}");

    let mut task = code_review_task();
    spent(&mut task);
    let planner = Actor::agent("creator-1", Role::Reviewer).in_session("s-creator");
    let error = transition(
        &mut task,
        Event::Escalate,
        &planner,
        &TransitionContext::default(),
    )
    .expect_err("the planning session escalates its own task");
    assert!(error.to_string().contains("cannot review"), "{error}");
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);
}

#[test]
fn re_review_needs_a_remediating_session_apart_from_the_executor() {
    let remediating = || {
        let mut task = code_review_task();
        task.status = coding_crew_core::Status::Remediating;
        task.review.verdict = Some(Verdict::ChangesNeeded);
        task.review.findings.push(Finding {
            id: "FIND-01".to_string(),
            status: FindingStatus::Fixed,
            ..Finding::default()
        });
        task
    };
    let fixer = Actor::agent("runner-2", Role::Runner).in_session("s-fixer");

    let mut task = remediating();
    transition(
        &mut task,
        Event::ReReview,
        &fixer,
        &TransitionContext::default(),
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);

    let mut task = remediating();
    let error = transition(
        &mut task,
        Event::ReReview,
        &runner(),
        &TransitionContext::default(),
    )
    .expect_err("the executing session remediates its own findings");
    assert!(error.to_string().contains("cannot remediate"), "{error}");

    let mut task = remediating();
    let error = transition(
        &mut task,
        Event::ReReview,
        &Actor::agent("runner-2", Role::Runner),
        &TransitionContext::default(),
    )
    .expect_err("a remediating session that is not recorded");
    assert!(error.to_string().contains("not recorded"), "{error}");

    let mut task = remediating();
    task.claim.as_mut().unwrap().session = None;
    let error = transition(
        &mut task,
        Event::ReReview,
        &fixer,
        &TransitionContext::default(),
    )
    .expect_err("an executing session that is not recorded");
    assert!(error.to_string().contains("not recorded"), "{error}");
}

#[test]
fn a_cancellation_keeps_its_first_reason() {
    let mut task = code_review_task();
    transition(
        &mut task,
        Event::Cancel,
        &creator(),
        &cancel_reason("out of scope"),
    )
    .unwrap();
    let first = task.overlays.cancelled.clone();

    let error = transition(
        &mut task,
        Event::Cancel,
        &creator(),
        &cancel_reason("second thoughts"),
    )
    .expect_err("a cancelled task is not cancelled again");
    assert!(error.to_string().contains("cancelled"), "{error}");
    assert_eq!(task.overlays.cancelled, first);
}

#[test]
fn a_cancelled_task_refuses_every_event() {
    let mut task = code_review_task();
    transition(
        &mut task,
        Event::Cancel,
        &creator(),
        &cancel_reason("out of scope"),
    )
    .unwrap();

    for event in [Event::Block, Event::Unblock, Event::Defer, Event::Undefer] {
        let error = transition(&mut task, event, &creator(), &cancel_reason("later"))
            .expect_err("a cancelled task takes no overlay");
        assert!(error.to_string().contains("cancelled"), "{error:?}");
    }
    assert_eq!(task.overlays.blocked, None);
    assert_eq!(task.overlays.deferred, None);
}

#[test]
fn cancel_needs_a_reason() {
    let mut task = code_review_task();
    let error = transition(
        &mut task,
        Event::Cancel,
        &creator(),
        &TransitionContext::default(),
    )
    .expect_err("a cancellation without a reason");
    assert!(error.to_string().contains("reason"), "{error}");
    assert_eq!(task.overlays.cancelled, None);
}
