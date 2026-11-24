//! Steps at work and the evidence (T9) through the real binary, against
//! scratch projects under the temp dir: `step begin|done|skip|fail`,
//! `criterion defer|undefer|explain` and `evidence run|override`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use coding_crew_core::{
    CriterionStatus, Evidence, Status, StepStatus, Store, Task, task_to_markdown,
};
use serde_json::{Value, json};

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

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-work-{name}-{}", std::process::id()));
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

/// A write talita has no verb for: a state the tests need as a fixture.
fn edit(dir: &Path, id: &str, change: impl FnOnce(&mut Task)) {
    let store = store(dir);
    let lock = store.lock().expect("the lock");
    let mut task = lock.load(id).expect("the task");
    change(&mut task);
    lock.save(&mut task).expect("the write");
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

fn criterion_add(dir: &Path, id: &str, description: &str, verification: &str) -> String {
    applied(
        talita(
            dir,
            &CREATOR,
            &[
                "criterion",
                "add",
                id,
                "--description",
                description,
                "--verification",
                verification,
            ],
        ),
        "criterion add",
    )["data"]["criterion"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn step_add(dir: &Path, id: &str, covers: &[&str]) {
    let mut args = vec![
        "step",
        "add",
        id,
        "--action",
        "do the work",
        "--done-when",
        "the work is done",
        "--covers",
    ];
    args.extend_from_slice(covers);
    applied(talita(dir, &CREATOR, &args), "step add");
}

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
    let done = applied(talita(dir, &CREATOR, &["task", "groom", id]), "task groom");
    assert_eq!(done["data"]["status"], "groomed", "{done}");
}

/// A task with one step covering every criterion, groomed and approved.
fn prepared(dir: &Path, title: &str, criteria: &[(&str, &str)]) -> String {
    let id = create(dir, title);
    let mut covers = Vec::new();
    for (description, verification) in criteria {
        covers.push(criterion_add(dir, &id, description, verification));
    }
    let covers: Vec<&str> = covers.iter().map(String::as_str).collect();
    step_add(dir, &id, &covers);
    groom(dir, &id);
    let done = applied(
        talita(dir, &CREATOR, &["task", "approve", &id]),
        "task approve",
    );
    assert_eq!(done["data"]["status"], "approved", "{done}");
    id
}

/// A task at work: approved, claimed by `runner-1` in `ses-runner`, executing.
fn at_work(dir: &Path, title: &str, criteria: &[(&str, &str)]) -> String {
    let id = prepared(dir, title, criteria);
    applied(talita(dir, &RUNNER, &["claim", "take", &id]), "claim take");
    let started = applied(talita(dir, &RUNNER, &["task", "start", &id]), "task start");
    assert_eq!(started["data"]["status"], "executing", "{started}");
    id
}

#[test]
fn evidence_run_records_the_real_exit_code_and_summary() {
    let dir = project("run");
    let id = at_work(
        &dir,
        "Prove the work",
        &[
            (
                "the output is read",
                "test -d coding-crew && printf 'building\\nall good\\n'",
            ),
            ("the code is read", "echo nope >&2; exit 3"),
            ("nothing said", "true"),
        ],
    );
    // From a subfolder: the command runs at the project root, not the caller's
    // directory (ADR 0019, `test -d coding-crew` would fail below it).
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).expect("a subfolder");

    let ran = applied(
        talita(&sub, &RUNNER, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
    );
    assert_eq!(ran["data"]["exit_code"], 0, "{ran}");
    assert_eq!(ran["data"]["status"], "passed", "{ran}");
    assert_eq!(
        ran["data"]["output"],
        json!(["building", "all good"]),
        "{ran}"
    );
    assert!(
        ran["journal"].as_array().unwrap().is_empty(),
        "evidence is content: it is not journaled"
    );
    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert_eq!(criterion.effective_status(), CriterionStatus::Passed);
    assert_eq!(criterion.status, CriterionStatus::Passed, "the mirror");
    let evidence = &criterion.evidence[0];
    assert_eq!(
        evidence.command,
        "test -d coding-crew && printf 'building\\nall good\\n'"
    );
    assert_eq!(evidence.exit_code, Some(0));
    assert_eq!(evidence.summary, "all good");
    assert_eq!(evidence.by, "runner-1");
    assert_eq!(evidence.session.as_deref(), Some("ses-runner"));
    assert!(!evidence.at.is_empty());
    assert!(!evidence.manual);

    // A non-zero code is the truth, whatever was printed.
    let failed = applied(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-02"]),
        "evidence run",
    );
    assert_eq!(failed["data"]["exit_code"], 3, "{failed}");
    assert_eq!(failed["data"]["status"], "failed", "{failed}");
    assert_eq!(failed["data"]["output"], json!(["nope"]), "{failed}");
    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-02").expect("the criterion");
    assert_eq!(criterion.effective_status(), CriterionStatus::Failed);
    assert_eq!(criterion.status, CriterionStatus::Failed, "the mirror");
    assert_eq!(criterion.evidence[0].exit_code, Some(3));
    assert_eq!(criterion.evidence[0].summary, "nope", "stderr speaks");

    // Exit 0 with nothing said is a failure.
    let silent = applied(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-03"]),
        "evidence run",
    );
    assert_eq!(silent["data"]["exit_code"], 0, "{silent}");
    assert_eq!(silent["data"]["status"], "failed", "{silent}");
    assert_eq!(
        silent["data"]["criterion"]["evidence"][0]["summary"], "",
        "{silent}"
    );
    let warning = &silent["data"]["warnings"][0];
    assert_eq!(warning["code"], "evidence.summary_empty", "{silent}");
    assert!(
        warning["message"].as_str().unwrap().contains("--summary"),
        "{silent}"
    );

    // --summary stands in for the command's words; the exit code still decides.
    let given = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "run",
                &id,
                "ACCEPT-03",
                "--summary",
                "it said nothing",
            ],
        ),
        "evidence run",
    );
    assert_eq!(given["data"]["status"], "passed", "{given}");
    assert!(given["data"]["warnings"].is_null(), "{given}");
    assert_eq!(
        given["data"]["criterion"]["evidence"][1]["summary"], "it said nothing",
        "{given}"
    );

    // A summary is cut at 200 characters, never inside one.
    let long = "é".repeat(210);
    let cut = applied(
        talita(
            &dir,
            &RUNNER,
            &["evidence", "run", &id, "ACCEPT-01", "--summary", &long],
        ),
        "evidence run",
    );
    let summary = cut["data"]["criterion"]["evidence"][1]["summary"]
        .as_str()
        .expect("the summary");
    assert_eq!(summary.chars().count(), 200);
    assert_eq!(summary, "é".repeat(200));
}

#[test]
fn a_timeout_is_recorded_unproven_and_refused() {
    let dir = project("timeout");
    let id = at_work(&dir, "Wait on it", &[("it finishes", "sleep 30")]);

    let refusal = refused(
        talita(
            &dir,
            &RUNNER,
            &["evidence", "run", &id, "ACCEPT-01", "--timeout", "1s"],
        ),
        "evidence run",
        "evidence.timeout",
        4,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("ACCEPT-01"),
        "{refusal}"
    );

    // The entry is on the record either way, with no exit code.
    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert_eq!(criterion.evidence.len(), 1);
    let evidence = &criterion.evidence[0];
    assert_eq!(evidence.command, "sleep 30");
    assert_eq!(evidence.exit_code, None);
    assert_eq!(evidence.by, "runner-1");
    assert_eq!(criterion.effective_status(), CriterionStatus::Unproven);
    assert_eq!(criterion.status, CriterionStatus::Unproven, "the mirror");
}

#[test]
fn an_override_is_a_manual_pass_raul_shows() {
    let dir = project("override");
    let id = at_work(&dir, "Judge it", &[("it reads well", "false")]);

    let passed = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "checked by reading the rendered page",
            ],
        ),
        "evidence override",
    );
    assert_eq!(passed["data"]["status"], "passed", "{passed}");

    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert_eq!(criterion.effective_status(), CriterionStatus::Passed);
    assert_eq!(criterion.status, CriterionStatus::Passed, "the mirror");
    let evidence = &criterion.evidence[0];
    assert!(evidence.manual, "the bypass is marked");
    assert_eq!(evidence.command, "");
    assert_eq!(evidence.exit_code, Some(0));
    assert_eq!(evidence.summary, "checked by reading the rendered page");

    // The projection raul renders carries the marker and the reason.
    let tasks = store(&dir).list().expect("the tasks");
    let doc = task_to_markdown(&task, &tasks);
    assert!(doc.contains("manual override"), "{doc}");
    assert!(
        doc.contains("checked by reading the rendered page"),
        "{doc}"
    );
}

#[test]
fn the_step_work_verbs_write_the_status_they_name() {
    let dir = project("steps");
    let id = at_work(&dir, "Step through", &[("it works", "true")]);

    let began = applied(
        talita(&dir, &RUNNER, &["step", "begin", &id, "STP-01"]),
        "step begin",
    );
    assert_eq!(began["data"]["step"]["status"], "in-progress", "{began}");
    assert!(
        began["journal"].as_array().unwrap().is_empty(),
        "a step is content: it is not journaled"
    );

    let done = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "step",
                "done",
                &id,
                "STP-01",
                "--deviation",
                "did it a little differently",
            ],
        ),
        "step done",
    );
    assert_eq!(done["data"]["step"]["status"], "done", "{done}");
    assert_eq!(
        done["data"]["step"]["deviation"], "did it a little differently",
        "{done}"
    );

    let skipped = applied(
        talita(
            &dir,
            &RUNNER,
            &["step", "skip", &id, "STP-01", "--deviation", "not needed"],
        ),
        "step skip",
    );
    assert_eq!(skipped["data"]["step"]["status"], "skipped", "{skipped}");
    assert_eq!(skipped["data"]["step"]["deviation"], "not needed");

    let failed = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "step",
                "fail",
                &id,
                "STP-01",
                "--deviation",
                "the tool is missing",
            ],
        ),
        "step fail",
    );
    assert_eq!(failed["data"]["step"]["status"], "failed", "{failed}");

    let task = read(&dir, &id);
    let step = task.step("STP-01").expect("the step");
    assert_eq!(step.status, StepStatus::Failed);
    assert_eq!(step.deviation, "the tool is missing");
}

#[test]
fn a_step_write_needs_the_working_runner_and_a_deviation() {
    let dir = project("step-refusals");
    let id = at_work(&dir, "Guard the steps", &[("it works", "true")]);

    refused(
        talita(&dir, &RUNNER, &["step", "skip", &id, "STP-01"]),
        "step skip",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, &RUNNER, &["step", "fail", &id, "STP-01"]),
        "step fail",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, &RUNNER, &["step", "begin", &id, "STP-09"]),
        "step begin",
        "part.not_found",
        3,
    );
    refused(
        talita(&dir, &RUNNER2, &["step", "begin", &id, "STP-01"]),
        "step begin",
        "claim.not_holder",
        5,
    );
    refused(
        talita(&dir, &CREATOR, &["step", "begin", &id, "STP-01"]),
        "step begin",
        "role.wrong",
        5,
    );
    refused(
        talita(&dir, &HUMAN, &["step", "begin", &id, "STP-01"]),
        "step begin",
        "actor.not_agent",
        5,
    );
    assert_eq!(
        read(&dir, &id).step("STP-01").expect("the step").status,
        StepStatus::Pending,
        "a refusal writes nothing"
    );

    // The work verbs run at work only.
    let approved = prepared(&dir, "Not started", &[("it works", "true")]);
    refused(
        talita(&dir, &RUNNER, &["step", "begin", &approved, "STP-01"]),
        "step begin",
        "status.wrong",
        4,
    );
}

#[test]
fn a_deferral_is_mirrored_and_undefer_re_derives_it() {
    let dir = project("deferral");
    let id = at_work(&dir, "Park a criterion", &[("it works", "true")]);
    let follow_up = create(&dir, "The follow-up");

    let deferred = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "defer",
                &id,
                "ACCEPT-01",
                "--reason",
                "needs an upstream change",
                "--follow-up",
                &follow_up,
            ],
        ),
        "criterion defer",
    );
    assert_eq!(
        deferred["data"]["criterion"]["deferral"]["reason"], "needs an upstream change",
        "{deferred}"
    );
    assert_eq!(
        deferred["data"]["criterion"]["deferral"]["follow_up"], follow_up,
        "{deferred}"
    );
    assert_eq!(deferred["data"]["criterion"]["status"], "deferred");
    assert!(
        deferred["journal"].as_array().unwrap().is_empty(),
        "a deferral is content: it is not journaled"
    );

    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert_eq!(criterion.effective_status(), CriterionStatus::Deferred);
    assert_eq!(criterion.status, CriterionStatus::Deferred, "the mirror");

    // A deferred criterion refuses the run.
    refused(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
        "criterion.deferred",
        4,
    );

    // A deferred criterion refuses the override too: a manual pass the
    // criterion cannot show would be recorded and masked (round 1, D1).
    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "checked by hand",
            ],
        ),
        "evidence override",
        "criterion.deferred",
        4,
    );
    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert!(
        criterion.evidence.is_empty(),
        "the refusal wrote no evidence"
    );

    let cleared = applied(
        talita(&dir, &RUNNER, &["criterion", "undefer", &id, "ACCEPT-01"]),
        "criterion undefer",
    );
    assert_eq!(
        cleared["data"]["criterion"]["status"], "pending",
        "{cleared}"
    );
    assert!(
        cleared["data"]["criterion"]["deferral"].is_null(),
        "{cleared}"
    );
    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert!(criterion.deferral.is_none());
    assert_eq!(
        criterion.effective_status(),
        CriterionStatus::Pending,
        "a cleared deferral does not stick"
    );
    assert_eq!(criterion.status, CriterionStatus::Pending, "the mirror");

    refused(
        talita(&dir, &RUNNER, &["criterion", "undefer", &id, "ACCEPT-01"]),
        "criterion undefer",
        "status.wrong",
        4,
    );
    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "defer",
                &id,
                "ACCEPT-01",
                "--reason",
                "somewhere else",
                "--follow-up",
                "TSK99",
            ],
        ),
        "criterion defer",
        "task.not_found",
        3,
    );
}

#[test]
fn explain_writes_a_reason_only_for_a_failed_criterion() {
    let dir = project("explain");
    let id = at_work(
        &dir,
        "Explain a failure",
        &[("it fails", "exit 3"), ("it works", "printf 'ok\\n'")],
    );

    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "explain",
                &id,
                "ACCEPT-02",
                "--failure-reason",
                "nothing yet",
            ],
        ),
        "criterion explain",
        "criterion.not_failed",
        4,
    );

    applied(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
    );
    let explained = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "explain",
                &id,
                "ACCEPT-01",
                "--failure-reason",
                "the tool is missing",
            ],
        ),
        "criterion explain",
    );
    assert_eq!(
        explained["data"]["criterion"]["failure_reason"], "the tool is missing",
        "{explained}"
    );
    assert_eq!(explained["data"]["criterion"]["status"], "failed");
    assert_eq!(
        read(&dir, &id)
            .criterion("ACCEPT-01")
            .expect("the criterion")
            .failure_reason,
        "the tool is missing"
    );

    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "explain",
                &id,
                "ACCEPT-02",
                "--failure-reason",
                "still nothing",
            ],
        ),
        "criterion explain",
        "criterion.not_failed",
        4,
    );
}

#[test]
fn a_stale_revision_refuses_before_the_command_runs() {
    let dir = project("stale-revision");
    let id = at_work(
        &dir,
        "Check the revision first",
        &[("it runs", "touch ran.txt")],
    );
    let stale = (read(&dir, &id).revision + 1).to_string();

    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "run",
                &id,
                "ACCEPT-01",
                "--expect-revision",
                &stale,
            ],
        ),
        "evidence run",
        "conflict.revision",
        6,
    );

    assert!(
        !dir.join("ran.txt").exists(),
        "a stale revision costs a file read, not a run"
    );
    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert!(criterion.evidence.is_empty(), "no evidence is recorded");
    assert_eq!(criterion.effective_status(), CriterionStatus::Pending);
}

#[test]
fn evidence_run_needs_the_working_runner_a_command_and_no_deferral() {
    let dir = project("evidence-refusals");
    let id = at_work(&dir, "Guard the run", &[("it works", "true")]);

    refused(
        talita(&dir, &RUNNER2, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
        "claim.not_holder",
        5,
    );
    refused(
        talita(&dir, &CREATOR, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
        "role.wrong",
        5,
    );
    refused(
        talita(&dir, &HUMAN, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
        "actor.not_agent",
        5,
    );
    refused(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-09"]),
        "evidence run",
        "part.not_found",
        3,
    );

    // The work verbs run at work only.
    let approved = prepared(&dir, "Not started", &[("it works", "true")]);
    refused(
        talita(&dir, &RUNNER, &["evidence", "run", &approved, "ACCEPT-01"]),
        "evidence run",
        "status.wrong",
        4,
    );

    // Remediating: the execution session cannot prove its own fix.
    edit(&dir, &id, |task| task.status = Status::Remediating);
    let refusal = refused(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["remediation.session_distinct"]);
    applied(
        talita(&dir, &RUNNER2, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
    );

    // A criterion with no verification has nothing to run.
    let unverified = at_work(&dir, "No command", &[("it works", "true")]);
    edit(&dir, &unverified, |task| {
        task.criterion_mut("ACCEPT-01")
            .expect("the criterion")
            .verification
            .clear();
    });
    let refusal = refused(
        talita(
            &dir,
            &RUNNER,
            &["evidence", "run", &unverified, "ACCEPT-01"],
        ),
        "evidence run",
        "input.invalid",
        2,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("ACCEPT-01"),
        "{refusal}"
    );
    assert_eq!(
        refusal["error"]["remedy"],
        format!("talita criterion revise {unverified} ACCEPT-01 --verification <cmd>"),
        "{refusal}"
    );
}

#[test]
fn a_failed_step_is_retried_or_skipped_before_step_done_settles_it() {
    let dir = project("step-failed");
    let id = at_work(&dir, "Retry a step", &[("it works", "true")]);
    applied(
        talita(
            &dir,
            &RUNNER,
            &["step", "fail", &id, "STP-01", "--deviation", "broke"],
        ),
        "step fail",
    );

    let refusal = refused(
        talita(&dir, &RUNNER, &["step", "done", &id, "STP-01"]),
        "step done",
        "status.wrong",
        4,
    );
    assert_eq!(
        refusal["error"]["remedy"],
        format!(
            "talita step begin {id} STP-01 (retry); talita step skip {id} STP-01 --deviation <why>"
        ),
        "{refusal}"
    );
    assert_eq!(
        read(&dir, &id).step("STP-01").expect("the step").status,
        StepStatus::Failed,
        "a refusal writes nothing"
    );

    // The retry path: begin, then done.
    applied(
        talita(&dir, &RUNNER, &["step", "begin", &id, "STP-01"]),
        "step begin",
    );
    let done = applied(
        talita(&dir, &RUNNER, &["step", "done", &id, "STP-01"]),
        "step done",
    );
    assert_eq!(done["data"]["step"]["status"], "done", "{done}");
}

#[test]
fn an_override_does_not_mask_a_criterion_the_command_proved() {
    let dir = project("override-proven");
    let id = at_work(&dir, "Proven by its command", &[("it works", "echo ok")]);
    applied(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
    );

    let refusal = refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "manual check",
            ],
        ),
        "evidence override",
        "status.wrong",
        4,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("already proven"),
        "{refusal}"
    );

    let task = read(&dir, &id);
    let criterion = task.criterion("ACCEPT-01").expect("the criterion");
    assert_eq!(criterion.evidence.len(), 1, "the refusal wrote no evidence");
    assert!(
        !criterion.evidence[0].manual,
        "the command's proof stays last"
    );
    assert_eq!(criterion.effective_status(), CriterionStatus::Passed);

    // A bypass already appended after the command's proof (written before this
    // refusal existed) does not reopen the door.
    edit(&dir, &id, |task| {
        task.criterion_mut("ACCEPT-01")
            .expect("the criterion")
            .push_evidence(Evidence {
                command: String::new(),
                exit_code: Some(0),
                summary: "checked by hand".to_string(),
                manual: true,
                ..Evidence::default()
            });
    });
    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "again",
            ],
        ),
        "evidence override",
        "status.wrong",
        4,
    );

    // A later command run that fails leaves the bypass as the only proof: the
    // criterion is no longer proven by its command, so the override is allowed
    // again, and a second one after it is too.
    edit(&dir, &id, |task| {
        task.criterion_mut("ACCEPT-01")
            .expect("the criterion")
            .push_evidence(Evidence {
                command: "echo ok".to_string(),
                exit_code: Some(1),
                summary: "broke".to_string(),
                ..Evidence::default()
            });
    });
    applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "checked after the failure",
            ],
        ),
        "evidence override",
    );
    applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "and again",
            ],
        ),
        "evidence override",
    );
}

#[test]
fn a_passing_criterion_clears_the_failure_reason_it_carried() {
    let dir = project("reason-clears");
    let id = at_work(&dir, "Pass after a failure", &[("it fails", "exit 3")]);
    applied(
        talita(&dir, &RUNNER, &["evidence", "run", &id, "ACCEPT-01"]),
        "evidence run",
    );
    applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "explain",
                &id,
                "ACCEPT-01",
                "--failure-reason",
                "the tool is missing",
            ],
        ),
        "criterion explain",
    );

    let passed = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "evidence",
                "override",
                &id,
                "ACCEPT-01",
                "--reason",
                "checked by hand",
            ],
        ),
        "evidence override",
    );
    assert_eq!(passed["data"]["status"], "passed", "{passed}");
    assert_eq!(
        passed["data"]["criterion"]["failure_reason"], "",
        "{passed}"
    );
    assert_eq!(
        read(&dir, &id)
            .criterion("ACCEPT-01")
            .expect("the criterion")
            .failure_reason,
        "",
        "the stored reason clears with the pass"
    );
}
