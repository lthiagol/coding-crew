//! Task lifecycle (T7) through the real binary, against scratch projects under
//! the temp dir: every transition, the overlays, `planned_by`, `--instructed-by`
//! and `--grant-rounds`.
//!
//! core's own claim, step, evidence and round writes stand in where talita has
//! no verb yet (claim is T8; the others are T9 and T10).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use coding_crew_core::claim::{self, DEFAULT_LEASE};
use coding_crew_core::{
    Actor, CriterionStatus, Evidence, Finding, FindingStatus, Focus, Role, Round, RoundKind,
    Severity, Status, StepStatus, Store, Task, Verdict,
};
use serde_json::Value;

const ENVS: [&str; 4] = [
    "TALITA_PROJECT",
    "TALITA_ACTOR",
    "TALITA_ROLE",
    "TALITA_SESSION",
];

const CREATOR: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "creator-1"),
    ("TALITA_ROLE", "creator"),
    ("TALITA_SESSION", "ses-creator"),
];

const RUNNER: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "runner-1"),
    ("TALITA_ROLE", "runner"),
    ("TALITA_SESSION", "ses-runner"),
];

const RUNNER2: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "runner-2"),
    ("TALITA_ROLE", "runner"),
    ("TALITA_SESSION", "ses-runner2"),
];

const REVIEWER: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "reviewer-1"),
    ("TALITA_ROLE", "reviewer"),
    ("TALITA_SESSION", "ses-reviewer"),
];

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-lifecycle-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    let dir = fs::canonicalize(&dir).expect("canonical scratch dir");
    applied(talita(&dir, &CREATOR, &["project", "init"]), "project init");
    dir
}

fn talita(cwd: &Path, env: &[(&str, &str)], args: &[&str]) -> (Value, i32) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_talita"));
    command.current_dir(cwd).args(args).stdin(Stdio::null());
    for name in ENVS {
        command.env_remove(name);
    }
    command.envs(env.iter().copied());
    let output = command.output().expect("talita runs");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(output.stderr.is_empty(), "{args:?}: stderr");
    assert_eq!(stdout.lines().count(), 1, "{args:?}: {stdout:?}");
    let envelope: Value = serde_json::from_str(&stdout).expect("JSON envelope");
    (envelope, output.status.code().expect("an exit code"))
}

fn applied(result: (Value, i32), command: &str) -> Value {
    let (envelope, status) = result;
    assert_eq!(envelope["ok"], true, "{envelope}");
    assert_eq!(envelope["command"], command);
    assert_eq!(status, 0);
    envelope
}

fn refused(result: (Value, i32), command: &str, code: &str, exit: i32) -> Value {
    let (envelope, status) = result;
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["command"], command, "{envelope}");
    assert_eq!(envelope["error"]["code"], code, "{envelope}");
    assert_eq!(status, exit, "{envelope}");
    envelope
}

/// The gate names of a refusal, in core's order.
fn gates(envelope: &Value) -> Vec<&str> {
    envelope["error"]["gates"]
        .as_array()
        .expect("a gate list")
        .iter()
        .map(|gate| gate["gate"].as_str().expect("a gate name"))
        .collect()
}

fn store(dir: &Path) -> Store {
    Store::for_project(dir)
}

fn read(dir: &Path, id: &str) -> Task {
    store(dir).load(id).expect("the task")
}

/// The one file of `id` in the scratch project.
fn file_of(dir: &Path, id: &str) -> PathBuf {
    let prefix = format!("{id}-");
    let mut found: Vec<PathBuf> = fs::read_dir(dir.join("coding-crew").join("work"))
        .expect("the work folder")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".json"))
        })
        .collect();
    assert_eq!(found.len(), 1, "{id}: {found:?}");
    found.pop().unwrap()
}

/// A write of what talita has no verb for yet: a core operation or a core
/// record, saved under the project lock.
fn edit(dir: &Path, id: &str, change: impl FnOnce(&mut Task)) {
    let store = store(dir);
    let lock = store.lock().expect("the lock");
    let mut task = lock.load(id).expect("the task");
    change(&mut task);
    lock.save(&mut task).expect("the write");
}

fn runner_actor() -> Actor {
    Actor::agent("runner-1", Role::Runner).in_session("ses-runner")
}

/// `claim take` is T8: the claim reaches the task through core's operation.
fn claim(dir: &Path, id: &str) {
    edit(dir, id, |task| {
        claim::take(
            task,
            &runner_actor(),
            DEFAULT_LEASE,
            None,
            &coding_crew_core::now_rfc3339(),
        )
        .expect("the claim");
    });
}

fn create(dir: &Path, title: &str) -> String {
    applied(
        talita(dir, &CREATOR, &["task", "create", "--title", title]),
        "task create",
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// `draft` → `groomed`: the outcome, one criterion and the out-of-scope.
fn groom(dir: &Path, id: &str) {
    applied(
        talita(
            dir,
            &CREATOR,
            &["task", "describe", id, "--outcome", "it works"],
        ),
        "task describe",
    );
    applied(
        talita(
            dir,
            &CREATOR,
            &["scope", "exclude", id, "--item", "not this"],
        ),
        "scope exclude",
    );
    applied(
        talita(
            dir,
            &CREATOR,
            &[
                "criterion",
                "add",
                id,
                "--description",
                "it works",
                "--verification",
                "true",
            ],
        ),
        "criterion add",
    );
    let done = applied(talita(dir, &CREATOR, &["task", "groom", id]), "task groom");
    assert_eq!(done["data"]["status"], "groomed", "{done}");
}

/// `groomed` → `approved`: a covering step, then the approval.
fn approve(dir: &Path, id: &str) {
    applied(
        talita(
            dir,
            &CREATOR,
            &[
                "step",
                "add",
                id,
                "--action",
                "do the work",
                "--done-when",
                "the work is done",
                "--covers",
                "ACCEPT-01",
            ],
        ),
        "step add",
    );
    let done = applied(
        talita(
            dir,
            &CREATOR,
            &["task", "approve", id, "--instructed-by", "luiz"],
        ),
        "task approve",
    );
    assert_eq!(done["data"]["status"], "approved", "{done}");
    assert_eq!(done["data"]["planned_by"], "ses-creator", "{done}");
    assert_eq!(read(dir, id).planned_by.as_deref(), Some("ses-creator"));
}

/// `approved` → `executing`: the claim, then the start.
fn start(dir: &Path, id: &str) {
    claim(dir, id);
    let done = applied(talita(dir, &RUNNER, &["task", "start", id]), "task start");
    assert_eq!(done["data"]["status"], "executing", "{done}");
}

/// The work `executing` → `code-review` needs, through core where talita has no
/// verb yet: a settled step, the evidence and the self round.
fn settle(dir: &Path, id: &str) {
    let now = coding_crew_core::now_rfc3339();
    edit(dir, id, |task| {
        task.steps[0].status = StepStatus::Done;
        task.acceptance_criteria[0].push_evidence(Evidence {
            command: "true".to_string(),
            exit_code: Some(0),
            summary: "it works".to_string(),
            at: now.clone(),
            by: "runner-1".to_string(),
            session: Some("ses-runner".to_string()),
            manual: false,
        });
        task.review.rounds.push(Round {
            n: 0,
            kind: RoundKind::SelfReview,
            focus: Focus::SpecCompliance,
            reviewer: "runner-1".to_string(),
            session: Some("ses-runner".to_string()),
            verdict: Verdict::Ok,
            at: now,
            ..Round::default()
        });
    });
}

/// `executing` → `code-review`.
fn finish(dir: &Path, id: &str) {
    settle(dir, id);
    let done = applied(talita(dir, &RUNNER, &["task", "finish", id]), "task finish");
    assert_eq!(done["data"]["status"], "code-review", "{done}");
}

/// File an external round through core (round file is T10). The round is
/// stamped where the task entered review, so it is the current one.
fn file_round(dir: &Path, id: &str, n: u32, verdict: Verdict, finding: Option<&str>) {
    let at = read(dir, id).status_since;
    edit(dir, id, |task| {
        let mut findings = Vec::new();
        if let Some(finding) = finding {
            task.review.findings.push(Finding {
                id: finding.to_string(),
                severity: Severity::High,
                status: FindingStatus::Open,
                description: "a defect".to_string(),
                created: at.clone(),
                ..Finding::default()
            });
            findings.push(finding.to_string());
        }
        task.review.rounds.push(Round {
            n,
            kind: RoundKind::External,
            focus: Focus::EdgeCases,
            reviewer: "reviewer-1".to_string(),
            session: Some("ses-reviewer".to_string()),
            verdict,
            at: at.clone(),
            findings,
            ..Round::default()
        });
        task.review.verdict = Some(verdict);
    });
}

fn close_findings(dir: &Path, id: &str) {
    let now = coding_crew_core::now_rfc3339();
    edit(dir, id, |task| {
        for finding in &mut task.review.findings {
            if finding.status == FindingStatus::Open {
                finding.status = FindingStatus::Fixed;
                finding.resolved = now.clone();
                finding.resolution = "fixed".to_string();
            }
        }
    });
}

#[test]
fn usage_and_store_refusals() {
    let dir = project("inputs");
    let id = create(&dir, "Refusals of input");

    // `resume` needs its reason at the parser.
    refused(
        talita(&dir, &CREATOR, &["task", "resume", &id]),
        "task resume",
        "usage.invalid",
        2,
    );
    // An empty reason names no decision.
    refused(
        talita(&dir, &CREATOR, &["task", "reopen", &id, "--reason", ""]),
        "task reopen",
        "input.invalid",
        2,
    );
    // A task the project does not hold.
    refused(
        talita(&dir, &CREATOR, &["task", "groom", "TSK99"]),
        "task groom",
        "task.not_found",
        3,
    );

    // A stale revision refuses before the gates run.
    let revision = read(&dir, &id).revision;
    let stale = revision.saturating_sub(1).to_string();
    refused(
        talita(
            &dir,
            &CREATOR,
            &["--expect-revision", &stale, "task", "groom", &id],
        ),
        "task groom",
        "conflict.revision",
        6,
    );

    // A file that does not parse.
    fs::write(file_of(&dir, &id), "{ not json").expect("the broken file");
    refused(
        talita(&dir, &CREATOR, &["task", "groom", &id]),
        "task groom",
        "store.unreadable",
        7,
    );
}

#[test]
fn every_status_a_fixture_holds_is_reached_through_talita() {
    let dir = project("statuses");
    let c = &CREATOR;
    let mut reached: BTreeSet<String> = BTreeSet::new();

    // draft
    let id = create(&dir, "Walk the lifecycle");
    reached.insert("draft".to_string());

    // groomed
    groom(&dir, &id);
    reached.insert("groomed".to_string());

    // approved: planned_by is the approving creator's session, and the
    // content is frozen.
    approve(&dir, &id);
    refused(
        talita(
            &dir,
            c,
            &["task", "describe", &id, "--outcome", "something else"],
        ),
        "task describe",
        "section.frozen",
        4,
    );
    reached.insert("approved".to_string());

    // executing
    start(&dir, &id);
    reached.insert("executing".to_string());

    // code-review
    finish(&dir, &id);
    reached.insert("code-review".to_string());

    // remediating: an external round with one finding, then the request.
    file_round(&dir, &id, 1, Verdict::ChangesNeeded, Some("FIND-01"));
    let remediating = applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );
    assert_eq!(remediating["data"]["status"], "remediating");
    reached.insert("remediating".to_string());

    // code-review again: the finding closed in a session apart from the executor.
    close_findings(&dir, &id);
    let handed_back = applied(
        talita(&dir, &RUNNER2, &["task", "re-review", &id]),
        "task re-review",
    );
    assert_eq!(handed_back["data"]["status"], "code-review");
    assert_eq!(handed_back["data"]["after_round"], 1);
    assert_eq!(handed_back["data"]["re_reviews"], 1);

    // needs-human: five external rounds, the budget spent.
    for n in 2..=5 {
        file_round(
            &dir,
            &id,
            n,
            Verdict::ChangesNeeded,
            Some(&format!("FIND-0{n}")),
        );
    }
    let escalated = applied(
        talita(&dir, &REVIEWER, &["task", "escalate", &id]),
        "task escalate",
    );
    assert_eq!(escalated["data"]["status"], "needs-human");
    reached.insert("needs-human".to_string());

    // A human's decision, with two more rounds granted.
    let missing = refused(
        talita(
            &dir,
            c,
            &[
                "task",
                "resume",
                &id,
                "--reason",
                "carry on",
                "--grant-rounds",
                "2",
            ],
        ),
        "task resume",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&missing), vec!["human.instructed"]);
    assert_eq!(read(&dir, &id).status, Status::NeedsHuman);

    let overflow = refused(
        talita(
            &dir,
            c,
            &[
                "task",
                "resume",
                &id,
                "--instructed-by",
                "luiz",
                "--reason",
                "carry on",
                "--grant-rounds",
                "4294967295",
            ],
        ),
        "task resume",
        "input.invalid",
        2,
    );
    assert!(
        overflow["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--grant-rounds")
    );
    assert_eq!(read(&dir, &id).status, Status::NeedsHuman);

    let resumed = applied(
        talita(
            &dir,
            c,
            &[
                "task",
                "resume",
                &id,
                "--instructed-by",
                "luiz",
                "--reason",
                "carry on",
                "--grant-rounds",
                "2",
            ],
        ),
        "task resume",
    );
    assert_eq!(resumed["data"]["status"], "executing");
    assert_eq!(resumed["data"]["max_rounds"], 7);
    assert_eq!(read(&dir, &id).review.max_rounds, 7);
    assert!(read(&dir, &id).claim.is_none(), "resume releases the claim");
    reached.insert("executing".to_string());

    // done: the runner takes the resumed task again, the findings closed, the
    // work finished again, a sixth round ok.
    applied(talita(&dir, &RUNNER, &["claim", "take", &id]), "claim take");
    close_findings(&dir, &id);
    finish(&dir, &id);
    file_round(&dir, &id, 6, Verdict::Ok, None);
    let passed = applied(
        talita(&dir, &REVIEWER, &["task", "pass-review", &id]),
        "task pass-review",
    );
    assert_eq!(passed["data"]["status"], "done");
    reached.insert("done".to_string());

    // done → executing on a human's word.
    let reopened = applied(
        talita(
            &dir,
            c,
            &[
                "task",
                "reopen",
                &id,
                "--instructed-by",
                "luiz",
                "--reason",
                "a regression",
            ],
        ),
        "task reopen",
    );
    assert_eq!(reopened["data"]["status"], "executing");

    // Every status the mock fixtures hold was reached, and no other exists.
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work");
    let mut held = BTreeSet::new();
    for entry in fs::read_dir(fixtures).expect("the mock tasks") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.starts_with("TSK") || !name.ends_with(".json") {
            continue;
        }
        let task: Task =
            serde_json::from_str(&fs::read_to_string(&path).expect("the file")).expect("a task");
        held.insert(task.status.as_str().to_string());
    }
    assert_eq!(held, reached);
}

#[test]
fn every_refusal_names_its_gates() {
    let dir = project("gates");

    // groom: every content gate, not only the first.
    let id = create(&dir, "An empty draft");
    let refusal = refused(
        talita(&dir, &CREATOR, &["task", "groom", &id]),
        "task groom",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec![
            "intent.outcome_present",
            "criteria.present",
            "scope.out_of_scope_declared"
        ]
    );
    assert!(
        refusal["error"]["remedy"]
            .as_str()
            .unwrap()
            .contains("task describe")
    );

    // approve: the open question and the uncovered criterion together.
    groom(&dir, &id);
    applied(
        talita(
            &dir,
            &CREATOR,
            &["question", "ask", &id, "--question", "which?"],
        ),
        "question ask",
    );
    let refusal = refused(
        talita(&dir, &CREATOR, &["task", "approve", &id]),
        "task approve",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec!["questions.resolved", "criteria.covered"]
    );

    // start: the task carries no claim.
    applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "step",
                "add",
                &id,
                "--action",
                "do",
                "--done-when",
                "done",
                "--covers",
                "ACCEPT-01",
            ],
        ),
        "step add",
    );
    applied(
        talita(
            &dir,
            &CREATOR,
            &["question", "answer", &id, "QST-01", "--answer", "this"],
        ),
        "question answer",
    );
    applied(
        talita(&dir, &CREATOR, &["task", "approve", &id]),
        "task approve",
    );
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "start", &id]),
        "task start",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["task.claimed"]);

    // finish: no evidence and no self round (the step is settled).
    claim(&dir, &id);
    applied(talita(&dir, &RUNNER, &["task", "start", &id]), "task start");
    edit(&dir, &id, |task| task.steps[0].status = StepStatus::Done);
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec!["evidence.attached", "self_round.filed"]
    );

    // code-review with nothing filed: the review gates.
    settle(&dir, &id);
    applied(
        talita(&dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
    );
    let refusal = refused(
        talita(&dir, &REVIEWER, &["task", "pass-review", &id]),
        "task pass-review",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec![
            "review.verdict_ok",
            "review.round_current",
            "review.round_owner"
        ]
    );

    let refusal = refused(
        talita(&dir, &REVIEWER, &["task", "escalate", &id]),
        "task escalate",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec![
            "review.verdict_changes_needed",
            "review.budget_exhausted",
            "review.round_current",
            "review.round_owner"
        ]
    );

    // request-changes: an ok round with no finding.
    file_round(&dir, &id, 1, Verdict::Ok, None);
    let refusal = refused(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec!["review.verdict_changes_needed", "review.findings_exist"]
    );

    // re-review from the execution session, with the finding still open.
    file_round(&dir, &id, 2, Verdict::ChangesNeeded, Some("FIND-01"));
    applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "re-review", &id]),
        "task re-review",
        "gate.refused",
        4,
    );
    assert_eq!(
        gates(&refusal),
        vec!["review.findings_closed", "remediation.session_distinct"]
    );

    // The distinct-session gate alone; its remedy mints a runner, because
    // re-review is the runner's verb.
    close_findings(&dir, &id);
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "re-review", &id]),
        "task re-review",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["remediation.session_distinct"]);
    assert_eq!(
        refusal["error"]["remedy"],
        "talita session new --as <actor> --role runner"
    );

    // block: the reason gate.
    let refusal = refused(
        talita(&dir, &HUMAN, &["task", "block", &id]),
        "task block",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["overlay.reason_present"]);
}

#[test]
fn identity_status_and_state_refusals() {
    let dir = project("refusals");
    let id = create(&dir, "A draft with the wrong actor");

    // approve a draft: the status is wrong.
    let refusal = refused(
        talita(&dir, &CREATOR, &["task", "approve", &id]),
        "task approve",
        "status.wrong",
        4,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("draft")
    );

    // groom as a runner: the role is wrong.
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "groom", &id]),
        "task groom",
        "role.wrong",
        5,
    );
    assert!(
        refusal["error"]["remedy"]
            .as_str()
            .unwrap()
            .contains("creator")
    );

    // groom as a human: agents act on tasks.
    refused(
        talita(&dir, &HUMAN, &["task", "groom", &id]),
        "task groom",
        "actor.not_agent",
        5,
    );

    // clearing an overlay that is not set.
    refused(
        talita(&dir, &HUMAN, &["task", "unblock", &id]),
        "task unblock",
        "overlay.absent",
        4,
    );
    refused(
        talita(&dir, &HUMAN, &["task", "undefer", &id]),
        "task undefer",
        "overlay.absent",
        4,
    );

    // the status is checked before the human.instructed gate.
    refused(
        talita(
            &dir,
            &CREATOR,
            &["task", "resume", &id, "--reason", "too early"],
        ),
        "task resume",
        "status.wrong",
        4,
    );

    // a cancelled task refuses every later write.
    let cancelled = applied(
        talita(
            &dir,
            &HUMAN,
            &["task", "cancel", &id, "--reason", "dropped"],
        ),
        "task cancel",
    );
    assert_eq!(
        cancelled["data"]["overlays"]["cancelled"]["reason"],
        "dropped"
    );
    assert_eq!(cancelled["data"]["released"], false);
    for (args, command) in [
        (vec!["task", "groom", &id], "task groom"),
        (vec!["task", "defer", &id, "--reason", "x"], "task defer"),
        (vec!["task", "undefer", &id], "task undefer"),
    ] {
        refused(talita(&dir, &CREATOR, &args), command, "task.cancelled", 4);
    }
}

#[test]
fn overlays_ride_beside_the_status_and_cancel_releases_a_live_claim() {
    let dir = project("overlays");
    let id = create(&dir, "Carries overlays");
    groom(&dir, &id);
    approve(&dir, &id);

    let blocked = applied(
        talita(
            &dir,
            &HUMAN,
            &["task", "block", &id, "--reason", "waiting on a key"],
        ),
        "task block",
    );
    assert_eq!(blocked["data"]["status"], "approved");
    assert_eq!(
        blocked["data"]["overlays"]["blocked"]["reason"],
        "waiting on a key"
    );
    assert_eq!(blocked["data"]["overlays"]["blocked"]["by"], "luiz");
    assert_eq!(blocked["data"]["reason"], "waiting on a key");

    applied(
        talita(&dir, &HUMAN, &["task", "unblock", &id]),
        "task unblock",
    );
    assert!(read(&dir, &id).overlays.blocked.is_none());

    applied(
        talita(&dir, &HUMAN, &["task", "defer", &id, "--reason", "later"]),
        "task defer",
    );
    assert_eq!(
        read(&dir, &id).overlays.deferred.as_ref().unwrap().reason,
        "later"
    );
    applied(
        talita(&dir, &HUMAN, &["task", "undefer", &id]),
        "task undefer",
    );
    assert!(read(&dir, &id).overlays.deferred.is_none());

    // A live claim goes with the cancellation; the reason stays on the record.
    claim(&dir, &id);
    let cancelled = applied(
        talita(
            &dir,
            &HUMAN,
            &["task", "cancel", &id, "--reason", "superseded"],
        ),
        "task cancel",
    );
    assert_eq!(cancelled["data"]["released"], true);
    let task = read(&dir, &id);
    assert!(task.claim.is_none());
    assert_eq!(task.status, Status::Approved, "the overlay moved no status");
    assert_eq!(
        task.overlays.cancelled.as_ref().unwrap().reason,
        "superseded"
    );
}

/// A task at work with its step settled and a self round filed; the evidence
/// is the test's to write.
fn at_work_with_a_round(dir: &Path, title: &str) -> String {
    let id = create(dir, title);
    groom(dir, &id);
    approve(dir, &id);
    start(dir, &id);
    edit(dir, &id, |task| task.steps[0].status = StepStatus::Done);
    applied(
        talita(
            dir,
            &RUNNER,
            &[
                "round",
                "file",
                &id,
                "--kind",
                "self",
                "--focus",
                "spec-compliance",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
    );
    id
}

#[test]
fn task_finish_names_the_call_that_clears_its_gate() {
    let dir = project("finish-remedy");

    // A criterion that failed without a reason: `criterion explain`, naming
    // the criterion the gate's detail names. The evidence was already run.
    let failed = at_work_with_a_round(&dir, "Fail without a reason");
    edit(&dir, &failed, |task| {
        task.acceptance_criteria[0].verification = "false".to_string();
    });
    applied(
        talita(&dir, &RUNNER, &["evidence", "run", &failed, "ACCEPT-01"]),
        "evidence run",
    );
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "finish", &failed]),
        "task finish",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["criteria.proven"]);
    let remedy = refusal["error"]["remedy"].as_str().expect("a remedy");
    assert!(
        remedy.contains("talita criterion explain") && remedy.contains("ACCEPT-01"),
        "{remedy}"
    );
    assert!(remedy.contains("--failure-reason"), "{remedy}");
    assert!(
        !remedy.contains("evidence run"),
        "the run already happened: {remedy}"
    );

    // A criterion with no evidence: `evidence run`, with the criterion's id.
    let bare = at_work_with_a_round(&dir, "No evidence");
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "finish", &bare]),
        "task finish",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["evidence.attached"]);
    let remedy = refusal["error"]["remedy"].as_str().expect("a remedy");
    assert_eq!(
        remedy,
        format!("talita evidence run {bare} ACCEPT-01"),
        "{refusal}"
    );

    // A criterion deferred without a deferral: `criterion undefer`.
    let parked = at_work_with_a_round(&dir, "Deferred without a deferral");
    edit(&dir, &parked, |task| {
        task.acceptance_criteria[0].status = CriterionStatus::Deferred;
    });
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "finish", &parked]),
        "task finish",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["criteria.proven"]);
    let remedy = refusal["error"]["remedy"].as_str().expect("a remedy");
    assert_eq!(
        remedy,
        format!("talita criterion undefer {parked} ACCEPT-01"),
        "{refusal}"
    );

    // A failed step and a criterion failed without a reason: each gate's call,
    // with the step's id where the gate's detail does not carry it.
    let both = at_work_with_a_round(&dir, "Two gates");
    edit(&dir, &both, |task| {
        task.acceptance_criteria[0].verification = "false".to_string();
        task.steps[0].status = StepStatus::Failed;
    });
    applied(
        talita(&dir, &RUNNER, &["evidence", "run", &both, "ACCEPT-01"]),
        "evidence run",
    );
    let refusal = refused(
        talita(&dir, &RUNNER, &["task", "finish", &both]),
        "task finish",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["steps.settled", "criteria.proven"]);
    assert_eq!(
        refusal["error"]["remedy"],
        format!(
            "talita step begin {both} STP-01 (retry); talita step skip {both} STP-01 --deviation <why>; talita criterion explain {both} ACCEPT-01 --failure-reason <t>"
        ),
        "{refusal}"
    );
}

#[test]
fn groom_refuses_a_criterion_no_command_can_prove() {
    let dir = project("groom-verification");
    let id = create(&dir, "Unverified");
    applied(
        talita(
            &dir,
            &CREATOR,
            &["task", "describe", &id, "--outcome", "it works"],
        ),
        "task describe",
    );
    applied(
        talita(
            &dir,
            &CREATOR,
            &["scope", "exclude", &id, "--item", "not this"],
        ),
        "scope exclude",
    );
    applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "criterion",
                "add",
                &id,
                "--description",
                "it works",
                "--verification",
                "true",
            ],
        ),
        "criterion add",
    );
    // A promoted item's placeholder carries no verification (ITM promote).
    edit(&dir, &id, |task| {
        task.acceptance_criteria[0].verification.clear();
    });

    let refusal = refused(
        talita(&dir, &CREATOR, &["task", "groom", &id]),
        "task groom",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["criteria.verified"]);
    assert_eq!(
        refusal["error"]["gates"][0]["detail"], "ACCEPT-01 has no verification",
        "{refusal}"
    );
    assert_eq!(
        refusal["error"]["remedy"],
        format!("talita criterion revise {id} ACCEPT-01 --verification <cmd>"),
        "{refusal}"
    );
    assert_eq!(
        read(&dir, &id).status,
        Status::Draft,
        "a refusal writes nothing"
    );

    applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "criterion",
                "revise",
                &id,
                "ACCEPT-01",
                "--verification",
                "true",
            ],
        ),
        "criterion revise",
    );
    let done = applied(
        talita(&dir, &CREATOR, &["task", "groom", &id]),
        "task groom",
    );
    assert_eq!(done["data"]["status"], "groomed", "{done}");
}

#[test]
fn task_escalate_names_the_round_to_file_not_itself() {
    let dir = project("escalate-remedy");
    let id = create(&dir, "Escalate too early");
    groom(&dir, &id);
    approve(&dir, &id);
    start(&dir, &id);
    finish(&dir, &id);
    // One changes-needed round: the verdict is set, so the budget is the only
    // gate that refuses the escalation.
    file_round(&dir, &id, 1, Verdict::ChangesNeeded, Some("FIND-01"));

    let refusal = refused(
        talita(&dir, &REVIEWER, &["task", "escalate", &id]),
        "task escalate",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["review.budget_exhausted"]);
    let remedy = refusal["error"]["remedy"].as_str().expect("a remedy");
    assert!(
        remedy.contains("talita round file") && remedy.contains("--input"),
        "{remedy}"
    );
    assert!(
        !remedy.contains("task escalate"),
        "the remedy names the call that was refused: {remedy}"
    );
}

#[test]
fn a_human_is_pointed_at_the_creators_route() {
    let dir = project("human-route");
    let id = create(&dir, "A human's decision");
    groom(&dir, &id);
    let question = applied(
        talita(
            &dir,
            &CREATOR,
            &["question", "ask", &id, "--question", "which?"],
        ),
        "question ask",
    )["data"]["question"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // `question answer` is the creator's, with the human's name.
    let refusal = refused(
        talita(
            &dir,
            &HUMAN,
            &["question", "answer", &id, &question, "--answer", "this"],
        ),
        "question answer",
        "actor.not_agent",
        5,
    );
    let remedy = refusal["error"]["remedy"].as_str().expect("a remedy");
    assert!(
        remedy.contains("talita question answer")
            && remedy.contains("--instructed-by")
            && remedy.contains(&question),
        "{remedy}"
    );

    // `task approve`: the creator agent acts with the human's name.
    let refusal = refused(
        talita(&dir, &HUMAN, &["task", "approve", &id]),
        "task approve",
        "actor.not_agent",
        5,
    );
    let remedy = refusal["error"]["remedy"].as_str().expect("a remedy");
    assert!(
        remedy.contains("talita task approve")
            && remedy.contains(&id)
            && remedy.contains("--instructed-by"),
        "{remedy}"
    );
    assert!(
        !remedy.contains("act as an agent"),
        "the human is told to be what was refused: {remedy}"
    );

    // `task start` has no human route: it names the role that owns it.
    applied(
        talita(
            &dir,
            &CREATOR,
            &["question", "answer", &id, &question, "--answer", "this"],
        ),
        "question answer",
    );
    approve(&dir, &id);
    let refusal = refused(
        talita(&dir, &HUMAN, &["task", "start", &id]),
        "task start",
        "actor.not_agent",
        5,
    );
    assert_eq!(
        refusal["error"]["remedy"], "act as --role runner",
        "{refusal}"
    );
}

#[test]
fn a_non_holders_finish_is_the_holders_refusal_not_the_gates() {
    let dir = project("finish-holder");
    let id = at_work_with_a_round(&dir, "Held by the runner");

    // Another runner, and the holder's own actor in another session: both
    // are refused as non-holders, naming the holder, with no gate list.
    let other = refused(
        talita(&dir, &RUNNER2, &["task", "finish", &id]),
        "task finish",
        "claim.not_holder",
        5,
    );
    let message = other["error"]["message"].as_str().unwrap();
    assert!(message.contains("held by runner-1"), "{other}");
    assert_eq!(other["error"]["remedy"], format!("talita claim show {id}"));
    assert!(other["error"]["gates"].is_null(), "{other}");

    let elsewhere = refused(
        talita(
            &dir,
            &[
                ("TALITA_ACTOR", "runner-1"),
                ("TALITA_ROLE", "runner"),
                ("TALITA_SESSION", "ses-elsewhere"),
            ],
            &["task", "finish", &id],
        ),
        "task finish",
        "claim.not_holder",
        5,
    );
    assert!(
        elsewhere["error"]["message"]
            .as_str()
            .unwrap()
            .contains("held by runner-1"),
        "{elsewhere}"
    );

    // The holder still meets the gates: no evidence yet.
    refused(
        talita(&dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
        "gate.refused",
        4,
    );
    // The role comes first: a reviewer is told its role, not the holder.
    refused(
        talita(&dir, &REVIEWER, &["task", "finish", &id]),
        "task finish",
        "role.wrong",
        5,
    );
}
