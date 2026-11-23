use std::path::PathBuf;

use coding_crew_core::lifecycle::{Event, TransitionContext, TransitionError, check, transition};
use coding_crew_core::store::{DATA_DIR, WORK_DIR};
use coding_crew_core::task::{
    AcceptanceCriterion, Deferral, Evidence, Finding, FindingStatus, Focus, Round, RoundKind,
    Verdict,
};
use coding_crew_core::{Actor, CriterionStatus, Role, Step, StepStatus, Store, Task};

/// The clock the transitions of these tests are read with.
const NOW: &str = "2026-11-20T00:00:00Z";

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
            &TransitionContext::default(),
            NOW
        )
        .is_err()
    );

    task.intent.outcome = "something".to_string();
    assert!(
        transition(
            &mut task,
            Event::Groom,
            &creator(),
            &TransitionContext::default(),
            NOW
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
            &TransitionContext::default(),
            NOW
        )
        .is_err()
    );

    task.scope.out_of_scope.push("no more".to_string());
    transition(
        &mut task,
        Event::Groom,
        &creator(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
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
        NOW,
    )
    .unwrap();
    task.steps[0].done_when = String::new();
    assert!(
        transition(
            &mut task,
            Event::Approve,
            &creator(),
            &TransitionContext::default(),
            NOW
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
        NOW,
    )
    .expect_err("ACCEPT-02 has no covering step");
    assert!(error.to_string().contains("covering step"), "{error}");

    task.acceptance_criteria.pop();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();

    assert!(
        transition(
            &mut task,
            Event::Start,
            &runner(),
            &TransitionContext::default(),
            NOW
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
        NOW,
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
        NOW,
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
    )
    .unwrap();

    task.steps[0].status = StepStatus::Done;
    assert!(
        transition(
            &mut task,
            Event::Finish,
            &runner(),
            &TransitionContext::default(),
            NOW
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
        manual: false,
    });
    assert!(
        transition(
            &mut task,
            Event::Finish,
            &runner(),
            &TransitionContext::default(),
            NOW
        )
        .is_err()
    );

    task.review.rounds.push(Round {
        n: 1,
        kind: RoundKind::SelfReview,
        focus: Focus::SpecCompliance,
        reviewer: "runner-1".to_string(),
        session: None,
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
        NOW,
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
            manual: false,
        }],
        ..AcceptanceCriterion::default()
    };
    assert_eq!(criterion.effective_status(), CriterionStatus::Failed);
}

#[test]
fn five_external_rounds_escalate_to_a_human() {
    let mut task = code_review_task();
    changes_needed(&mut task);
    for n in 2..=5 {
        task.review
            .rounds
            .push(external_round(n, Verdict::ChangesNeeded));
    }

    transition(
        &mut task,
        Event::Escalate,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
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
        NOW,
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
        NOW,
    )
    .unwrap();
    assert_eq!(task.status_since, "2020-01-01T00:00:00Z");
    assert_ne!(task.updated, "2020-01-01T00:00:00Z");
}

#[test]
fn store_round_trips_tasks() {
    let dir = std::env::temp_dir().join(format!("coding-crew-store-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // The store's lock sits beside `work/`: keep the work folder inside the
    // scratch project, not the shared temp dir.
    let work = dir.join(DATA_DIR).join(WORK_DIR);
    std::fs::create_dir_all(&work).unwrap();
    let store = Store::new(&work);

    let mut task = store.create("Add lane filtering").unwrap();
    assert_eq!(task.id, "TSK01");
    assert_eq!(task.status, coding_crew_core::Status::Draft);
    task.status_since = "2026-11-13T12:00:00Z".to_string();
    store.lock().unwrap().save(&mut task).unwrap();

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
        NOW,
    )
    .unwrap();
    // No steps, no coverage, no done_when: a small task approves anyway.
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
    )
    .unwrap();
    task.steps[0].done_when = String::new();
    assert!(
        transition(
            &mut task,
            Event::Approve,
            &creator(),
            &TransitionContext::default(),
            NOW
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
        NOW,
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
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
        NOW,
    )
    .unwrap();
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
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
        session: None,
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
        manual: false,
    }
}

fn claim_by_runner(lease: &str) -> coding_crew_core::Claim {
    coding_crew_core::Claim {
        by: "runner-1".to_string(),
        session: Some("s-runner".to_string()),
        at: "now".to_string(),
        lease_expires_at: lease.to_string(),
    }
}

/// A task in code review: planned by `s-creator`, executed by `s-runner`. It
/// entered review on 2026-11-01, so its external rounds are filed after that.
fn code_review_task() -> Task {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::CodeReview;
    task.status_since = "2026-11-01T00:00:00Z".to_string();
    task.planned_by = Some("s-creator".to_string());
    task.claim = Some(claim_by_runner(""));
    task.review.verdict = Some(Verdict::Ok);
    task
}

/// An external round filed by `s-reviewer` on 2026-11-02, in review.
fn external_round(n: u32, verdict: Verdict) -> Round {
    Round {
        n,
        kind: RoundKind::External,
        focus: Focus::EdgeCases,
        reviewer: "reviewer-1".to_string(),
        session: Some("s-reviewer".to_string()),
        verdict,
        at: "2026-11-02T00:00:00Z".to_string(),
        notes: String::new(),
        findings: match verdict {
            Verdict::Ok => Vec::new(),
            Verdict::ChangesNeeded => vec!["FIND-01".to_string()],
        },
    }
}

/// The latest round asks for changes: the verdict, one open finding, and the
/// external round that filed both.
fn changes_needed(task: &mut Task) {
    task.review.verdict = Some(Verdict::ChangesNeeded);
    task.review.findings.push(Finding {
        id: "FIND-01".to_string(),
        status: FindingStatus::Open,
        ..Finding::default()
    });
    task.review
        .rounds
        .push(external_round(1, Verdict::ChangesNeeded));
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
    task.claim = Some(claim_by_runner(""));
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
        NOW,
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
        NOW,
    )
    .expect_err("a deferred status without a deferral");
    assert!(error.to_string().contains("without a deferral"), "{error}");
}

#[test]
fn pass_review_needs_the_reviewer_session_apart_from_the_planner() {
    let mut task = code_review_task();
    task.review.rounds.push(external_round(1, Verdict::Ok));
    transition(
        &mut task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Done);

    let mut task = code_review_task();
    task.review.rounds.push(external_round(1, Verdict::Ok));
    let error = transition(
        &mut task,
        Event::PassReview,
        &unrecorded(),
        &TransitionContext::default(),
        NOW,
    )
    .expect_err("a reviewer with no session");
    assert!(error.to_string().contains("not recorded"), "{error}");

    let mut task = code_review_task();
    task.review.rounds.push(external_round(1, Verdict::Ok));
    let planner = Actor::agent("creator-1", Role::Reviewer).in_session("s-creator");
    let error = transition(
        &mut task,
        Event::PassReview,
        &planner,
        &TransitionContext::default(),
        NOW,
    )
    .expect_err("the planning session reviews its own task");
    assert!(error.to_string().contains("cannot review"), "{error}");
}

#[test]
fn a_planner_left_unrecorded_cannot_be_checked() {
    let mut task = code_review_task();
    task.review.rounds.push(external_round(1, Verdict::Ok));
    task.planned_by = None;
    let error = transition(
        &mut task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
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
        NOW,
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
        NOW,
    )
    .expect_err("the planning session reviews its own task");
    assert!(error.to_string().contains("cannot review"), "{error}");
}

#[test]
fn escalate_needs_the_reviewer_session_apart_from_the_planner() {
    let spent = |task: &mut Task| {
        changes_needed(task);
        for n in 2..=5 {
            task.review
                .rounds
                .push(external_round(n, Verdict::ChangesNeeded));
        }
    };

    let mut task = code_review_task();
    spent(&mut task);
    let error = transition(
        &mut task,
        Event::Escalate,
        &unrecorded(),
        &TransitionContext::default(),
        NOW,
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
        NOW,
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
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);

    let mut task = remediating();
    let error = transition(
        &mut task,
        Event::ReReview,
        &runner(),
        &TransitionContext::default(),
        NOW,
    )
    .expect_err("the executing session remediates its own findings");
    assert!(error.to_string().contains("cannot remediate"), "{error}");

    let mut task = remediating();
    let error = transition(
        &mut task,
        Event::ReReview,
        &Actor::agent("runner-2", Role::Runner),
        &TransitionContext::default(),
        NOW,
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
        NOW,
    )
    .expect_err("an executing session that is not recorded");
    assert!(error.to_string().contains("not recorded"), "{error}");
}

#[test]
fn re_review_records_the_hand_back_in_the_same_move() {
    let mut task = code_review_task();
    task.status = coding_crew_core::Status::Remediating;
    task.review.verdict = Some(Verdict::ChangesNeeded);
    task.review
        .rounds
        .push(external_round(1, Verdict::ChangesNeeded));
    task.review.findings.push(Finding {
        id: "FIND-01".to_string(),
        status: FindingStatus::Fixed,
        ..Finding::default()
    });
    let fixer = Actor::agent("runner-2", Role::Runner).in_session("s-fixer");
    transition(
        &mut task,
        Event::ReReview,
        &fixer,
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();

    let [hand_back] = task.review.re_reviews.as_slice() else {
        panic!("one hand-back: {:?}", task.review.re_reviews);
    };
    assert_eq!(
        hand_back.at, task.status_since,
        "the same instant as the move"
    );
    assert_eq!(hand_back.after_round, 1);
    assert_eq!(hand_back.by, "runner-2");
    assert_eq!(hand_back.session.as_deref(), Some("s-fixer"));
}

#[test]
fn approve_records_the_planning_session() {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::Groomed;
    task.planned_by = None;
    transition(
        &mut task,
        Event::Approve,
        &creator(),
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Approved);
    assert_eq!(task.planned_by.as_deref(), Some("s-creator"));
}

#[test]
fn a_cancellation_keeps_its_first_reason() {
    let mut task = code_review_task();
    transition(
        &mut task,
        Event::Cancel,
        &creator(),
        &cancel_reason("out of scope"),
        NOW,
    )
    .unwrap();
    let first = task.overlays.cancelled.clone();

    let error = transition(
        &mut task,
        Event::Cancel,
        &creator(),
        &cancel_reason("second thoughts"),
        NOW,
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
        NOW,
    )
    .unwrap();

    for event in [Event::Block, Event::Unblock, Event::Defer, Event::Undefer] {
        let error = transition(&mut task, event, &creator(), &cancel_reason("later"), NOW)
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
        NOW,
    )
    .expect_err("a cancellation without a reason");
    assert!(error.to_string().contains("reason"), "{error}");
    assert_eq!(task.overlays.cancelled, None);
}

fn gate_names(failures: &[coding_crew_core::GateFailure]) -> Vec<&'static str> {
    failures.iter().map(|failure| failure.gate).collect()
}

#[test]
fn a_verdict_from_before_review_cannot_move_the_task() {
    // The task entered review on 2026-11-01. A round filed before that is a
    // verdict on an earlier review, and it passes nothing.
    let mut task = code_review_task();
    let mut stale = external_round(1, Verdict::Ok);
    stale.at = "2026-10-30T00:00:00Z".to_string();
    task.review.rounds.push(stale);

    let failures = check(
        &task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(gate_names(&failures), vec!["review.round_current"]);
    assert!(
        transition(
            &mut task,
            Event::PassReview,
            &reviewer(),
            &TransitionContext::default(),
            NOW
        )
        .is_err()
    );

    // A round filed the moment the task entered review is current.
    task.review.rounds[0].at = "2026-11-01T00:00:00Z".to_string();
    transition(
        &mut task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Done);
}

#[test]
fn a_review_move_needs_an_external_round_behind_it() {
    let task = code_review_task();
    let failures = check(
        &task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(
        gate_names(&failures),
        vec!["review.round_current", "review.round_owner"]
    );
}

#[test]
fn only_the_session_that_filed_the_round_moves_it() {
    let mut task = code_review_task();
    let mut round = external_round(1, Verdict::Ok);
    round.session = Some("s-other-reviewer".to_string());
    task.review.rounds.push(round);

    let failures = check(
        &task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(gate_names(&failures), vec!["review.round_owner"]);
    let error = transition(
        &mut task,
        Event::PassReview,
        &reviewer(),
        &TransitionContext::default(),
        NOW,
    )
    .expect_err("a session that did not file the round");
    assert!(
        error.to_string().contains("latest external round"),
        "{error}"
    );
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);

    let filer = Actor::agent("reviewer-1", Role::Reviewer).in_session("s-other-reviewer");
    transition(
        &mut task,
        Event::PassReview,
        &filer,
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::Done);
}

#[test]
fn the_executing_session_never_reviews_its_task() {
    // The executing session filed the external round itself, so it owns the
    // round; the session apart from the executor is the only one that reviews.
    let mut task = code_review_task();
    let mut round = external_round(1, Verdict::Ok);
    round.session = Some("s-runner".to_string());
    task.review.rounds.push(round);

    let executor = Actor::agent("reviewer-1", Role::Reviewer).in_session("s-runner");
    let failures = check(
        &task,
        Event::PassReview,
        &executor,
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(gate_names(&failures), vec!["review.independent"]);
    assert!(failures[0].message.contains("executed"), "{failures:?}");
}

fn executing_with_claim(claim: coding_crew_core::Claim) -> Task {
    let mut task = groomed_task();
    task.status = coding_crew_core::Status::Executing;
    task.claim = Some(claim);
    task.steps[0].status = StepStatus::Done;
    task.review.rounds.push(self_round());
    task.acceptance_criteria[0].evidence.push(passed_evidence());
    task
}

#[test]
fn only_the_claim_holder_hands_the_work_to_review() {
    let mut task = executing_with_claim(claim_by_runner(""));

    // The same session under another actor, and the holder under another session.
    let other_actor = Actor::agent("runner-2", Role::Runner).in_session("s-runner");
    let failures = check(
        &task,
        Event::Finish,
        &other_actor,
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(gate_names(&failures), vec!["task.claimed"]);

    let other_session = Actor::agent("runner-1", Role::Runner).in_session("s-other");
    let failures = check(
        &task,
        Event::Finish,
        &other_session,
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(gate_names(&failures), vec!["task.claimed"]);
    assert_eq!(task.status, coding_crew_core::Status::Executing);

    transition(
        &mut task,
        Event::Finish,
        &runner(),
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);
}

#[test]
fn an_expired_lease_still_belongs_to_its_holder_at_finish() {
    // The lease guards the time before `finish`; the holder still hands over.
    let mut task = executing_with_claim(claim_by_runner("2020-01-01T01:00:00Z"));
    transition(
        &mut task,
        Event::Finish,
        &runner(),
        &TransitionContext::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);
}

#[test]
fn check_reports_every_failing_gate() {
    // A review with no planner, no external round, an open finding and a
    // reviewer who names no session: each failing gate is listed.
    let mut task = code_review_task();
    task.planned_by = None;
    task.review.verdict = Some(Verdict::ChangesNeeded);
    task.review.findings.push(Finding {
        id: "FIND-01".to_string(),
        status: FindingStatus::Open,
        ..Finding::default()
    });
    let expected = vec![
        "review.verdict_ok",
        "review.no_open_findings",
        "review.independent",
        "review.independent",
        "review.round_current",
        "review.round_owner",
    ];

    let failures = check(
        &task,
        Event::PassReview,
        &unrecorded(),
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(gate_names(&failures), expected);

    match transition(
        &mut task,
        Event::PassReview,
        &unrecorded(),
        &TransitionContext::default(),
        NOW,
    )
    .expect_err("every gate fails")
    {
        TransitionError::Gates(refused) => assert_eq!(gate_names(&refused), expected),
        other => panic!("expected every failing gate, got {other:?}"),
    }
    assert_eq!(task.status, coding_crew_core::Status::CodeReview);

    // Finish reports each criterion without evidence, and the missing self round.
    let mut task = executing_with_claim(claim_by_runner(""));
    task.review.rounds.clear();
    task.acceptance_criteria[0].evidence.clear();
    task.acceptance_criteria.push(AcceptanceCriterion {
        id: "ACCEPT-02".to_string(),
        ..AcceptanceCriterion::default()
    });
    let failures = check(
        &task,
        Event::Finish,
        &runner(),
        &TransitionContext::default(),
        NOW,
    );
    assert_eq!(
        gate_names(&failures),
        vec!["evidence.attached", "evidence.attached", "self_round.filed"]
    );
}

fn run(exit_code: i32, summary: &str) -> Evidence {
    Evidence {
        command: "cargo test".to_string(),
        exit_code: Some(exit_code),
        summary: summary.to_string(),
        ..Evidence::default()
    }
}

#[test]
fn a_stored_criterion_status_mirrors_the_evidence_and_the_deferral() {
    let mut criterion = AcceptanceCriterion {
        id: "ACCEPT-01".to_string(),
        ..AcceptanceCriterion::default()
    };
    assert_eq!(criterion.status, CriterionStatus::Pending);

    criterion.push_evidence(run(1, "2 failed"));
    assert_eq!(criterion.status, CriterionStatus::Failed);

    criterion.push_evidence(run(0, "3 passed"));
    assert_eq!(criterion.status, CriterionStatus::Passed);

    criterion.set_deferral(Some(Deferral {
        reason: "needs a device".to_string(),
        follow_up: "TSK09".to_string(),
    }));
    assert_eq!(criterion.status, CriterionStatus::Deferred);

    // Clearing the deferral gives the status back to the evidence.
    criterion.set_deferral(None);
    assert_eq!(criterion.status, CriterionStatus::Passed);
    assert_eq!(criterion.status, criterion.effective_status());
}

#[test]
fn an_unrun_criterion_mirrors_unproven() {
    let mut criterion = AcceptanceCriterion::default();
    criterion.push_evidence(Evidence {
        command: "cargo test".to_string(),
        exit_code: None,
        ..Evidence::default()
    });
    assert_eq!(criterion.status, CriterionStatus::Unproven);
}

#[test]
fn every_fixture_criterion_stores_the_status_it_derives() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("mock-data/coding-crew/work");
    for task in Store::new(root).list().expect("the mock tasks") {
        for criterion in &task.acceptance_criteria {
            assert_eq!(
                criterion.status,
                criterion.effective_status(),
                "{} {} stores a status it does not derive",
                task.id,
                criterion.id
            );
        }
    }
}
