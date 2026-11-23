//! The review record (T10) through the real binary, against scratch projects
//! under the temp dir: `round file|show`, `finding open|resolve|dismiss`,
//! `comment add`, the verdict rules and `re_reviews[]`.
//!
//! core's claim and evidence writes stand in where talita has no verb yet
//! (claim is T8; evidence is T9).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use coding_crew_core::claim::{self, DEFAULT_LEASE};
use coding_crew_core::{
    Actor, Evidence, FindingStatus, Focus, Role, Round, RoundKind, Status, StepStatus, Store, Task,
    Verdict,
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

const REVIEWER: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "reviewer-1"),
    ("TALITA_ROLE", "reviewer"),
    ("TALITA_SESSION", "ses-reviewer"),
];

const REVIEWER2: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "reviewer-2"),
    ("TALITA_ROLE", "reviewer"),
    ("TALITA_SESSION", "ses-reviewer2"),
];

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-review-{name}-{}", std::process::id()));
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
        talita(dir, &CREATOR, &["task", "approve", id]),
        "task approve",
    );
    assert_eq!(done["data"]["status"], "approved", "{done}");
}

/// A task `executing`, claimed by `runner-1` in `ses-runner`.
fn prepared(dir: &Path) -> String {
    let id = create(dir, "Walk the review");
    groom(dir, &id);
    approve(dir, &id);
    claim(dir, &id);
    let done = applied(talita(dir, &RUNNER, &["task", "start", &id]), "task start");
    assert_eq!(done["data"]["status"], "executing", "{done}");
    id
}

/// The evidence `finish` needs; `evidence` is T9.
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
    });
}

/// A self round through talita, as the worker files it before `finish`.
fn self_round(dir: &Path, id: &str) -> Value {
    applied(
        talita(
            dir,
            &RUNNER,
            &[
                "round",
                "file",
                id,
                "--kind",
                "self",
                "--focus",
                "spec-compliance",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
    )
}

/// `executing` → `code-review`: the evidence, the self round, `finish`.
fn to_review(dir: &Path) -> String {
    let id = prepared(dir);
    settle(dir, &id);
    let filed = self_round(dir, &id);
    assert_eq!(filed["data"]["round"]["n"], 0, "{filed}");
    let done = applied(
        talita(dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
    );
    assert_eq!(done["data"]["status"], "code-review", "{done}");
    id
}

/// Write a round.json into the scratch project; returns its name.
fn write_round(dir: &Path, name: &str, round: &Value) -> String {
    fs::write(dir.join(name), serde_json::to_vec(round).expect("json")).expect("the round file");
    name.to_string()
}

/// An external round through `--input`, filed by `reviewer-1`.
fn input_round(dir: &Path, id: &str, focus: &str, verdict: &str, findings: Value) -> Value {
    let round = json!({
        "kind": "external",
        "focus": focus,
        "verdict": verdict,
        "notes": "a look",
        "findings": findings,
    });
    let path = write_round(dir, "round.json", &round);
    applied(
        talita(dir, &REVIEWER, &["round", "file", id, "--input", &path]),
        "round file",
    )
}

#[test]
fn the_readme_unit_runs_through_talita() {
    let dir = project("unit");
    let id = prepared(&dir);

    let filed = self_round(&dir, &id);
    assert_eq!(filed["data"]["round"]["kind"], "self", "{filed}");
    assert_eq!(filed["data"]["round"]["reviewer"], "runner-1");
    assert_eq!(filed["data"]["round"]["session"], "ses-runner");
    assert_eq!(filed["data"]["round"]["verdict"], "ok");
    assert!(filed["data"]["warnings"].as_array().unwrap().is_empty());

    settle(&dir, &id);
    let done = applied(
        talita(&dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
    );
    assert_eq!(done["data"]["status"], "code-review", "{done}");

    // External round 1, filed whole with its two findings.
    let filed = input_round(
        &dir,
        &id,
        "security",
        "changes-needed",
        json!([
            {"ref": "D1", "severity": "high", "description": "the lock is not released"},
            {"ref": "D2", "severity": "low", "description": "the name is vague"},
        ]),
    );
    assert_eq!(filed["data"]["round"]["n"], 1, "{filed}");
    assert_eq!(filed["data"]["round"]["session"], "ses-reviewer");
    assert_eq!(
        filed["data"]["round"]["findings"],
        json!(["FIND-01", "FIND-02"])
    );
    assert_eq!(filed["data"]["refs"]["D1"], "FIND-01");
    assert_eq!(filed["data"]["refs"]["D2"], "FIND-02");
    assert_eq!(filed["data"]["round"]["notes"], "a look");
    assert!(filed["data"]["warnings"].as_array().unwrap().is_empty());

    let task = read(&dir, &id);
    assert_eq!(task.review.verdict, Some(Verdict::ChangesNeeded));
    assert_eq!(task.review.findings[0].status, FindingStatus::Open);
    assert!(task.review.findings[0].description.starts_with("the lock"));

    let changes = applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );
    assert_eq!(changes["data"]["status"], "remediating", "{changes}");

    // Remediation happens in a fresh session.
    let resolved = applied(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "resolve",
                &id,
                "FIND-01",
                "--resolution",
                "the lock is dropped now",
            ],
        ),
        "finding resolve",
    );
    assert_eq!(resolved["data"]["finding"]["status"], "fixed");
    assert_eq!(
        resolved["data"]["finding"]["resolution"],
        "the lock is dropped now"
    );
    let dismissed = applied(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "dismiss",
                &id,
                "FIND-02",
                "--reason",
                "it is the crate's name",
            ],
        ),
        "finding dismiss",
    );
    assert_eq!(dismissed["data"]["finding"]["status"], "dismissed");
    assert_eq!(
        dismissed["data"]["finding"]["resolution"],
        "it is the crate's name"
    );
    assert!(
        !dismissed["data"]["finding"]["resolved"]
            .as_str()
            .unwrap()
            .is_empty()
    );

    let handed = applied(
        talita(&dir, &RUNNER2, &["task", "re-review", &id]),
        "task re-review",
    );
    assert_eq!(handed["data"]["status"], "code-review", "{handed}");
    let task = read(&dir, &id);
    assert_eq!(task.review.re_reviews.len(), 1, "{task:?}");
    let back = &task.review.re_reviews[0];
    assert_eq!(back.after_round, 1);
    assert_eq!(back.by, "runner-2");
    assert_eq!(back.session.as_deref(), Some("ses-runner2"));
    assert!(!back.at.is_empty());

    let second = applied(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &id,
                "--kind",
                "external",
                "--focus",
                "edge-cases",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
    );
    assert_eq!(second["data"]["round"]["n"], 2, "{second}");
    assert_eq!(second["data"]["round"]["findings"], json!([]));
    assert!(second["data"]["warnings"].as_array().unwrap().is_empty());

    let passed = applied(
        talita(&dir, &REVIEWER, &["task", "pass-review", &id]),
        "task pass-review",
    );
    assert_eq!(passed["data"]["status"], "done", "{passed}");

    // The whole record, each round with the findings it holds.
    let shown = applied(talita(&dir, &[], &["round", "show", &id]), "round show");
    let rounds = shown["data"]["rounds"].as_array().expect("the rounds");
    assert_eq!(rounds.len(), 3, "{shown}");
    assert_eq!(rounds[0]["round"]["kind"], "self");
    assert_eq!(rounds[1]["round"]["n"], 1);
    assert_eq!(
        rounds[1]["round"]["findings"],
        json!(["FIND-01", "FIND-02"])
    );
    assert_eq!(rounds[1]["findings"][0]["id"], "FIND-01");
    assert_eq!(rounds[1]["findings"][0]["status"], "fixed");
    assert_eq!(rounds[1]["findings"][1]["status"], "dismissed");
    assert_eq!(rounds[2]["round"]["n"], 2);
    assert_eq!(rounds[2]["round"]["verdict"], "ok");

    let one = applied(
        talita(&dir, &[], &["round", "show", &id, "1"]),
        "round show",
    );
    assert_eq!(one["data"]["rounds"].as_array().unwrap().len(), 1);
    assert_eq!(one["data"]["rounds"][0]["round"]["n"], 1);

    refused(
        talita(&dir, &[], &["round", "show", &id, "9"]),
        "round show",
        "part.not_found",
        3,
    );
}

#[test]
fn the_verdict_rules_refuse_both_mismatches() {
    let dir = project("verdicts");
    let id = to_review(&dir);

    // changes-needed with no finding in the round.
    let revision = read(&dir, &id).revision;
    let refusal = refused(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &id,
                "--kind",
                "external",
                "--focus",
                "edge-cases",
                "--verdict",
                "changes-needed",
            ],
        ),
        "round file",
        "review.verdict_mismatch",
        4,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no finding")
    );
    assert_eq!(read(&dir, &id).revision, revision, "nothing was written");

    // ok while a finding stays open.
    input_round(
        &dir,
        &id,
        "security",
        "changes-needed",
        json!([{"ref": "D1", "severity": "high", "description": "a defect"}]),
    );
    let refusal = refused(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &id,
                "--kind",
                "external",
                "--focus",
                "design",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
        "review.verdict_mismatch",
        4,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("open")
    );
    let task = read(&dir, &id);
    assert_eq!(task.external_rounds(), 1, "the refused round was not filed");
    assert_eq!(task.review.findings.len(), 1);
}

#[test]
fn round_file_checks_the_status_the_session_the_budget_and_the_focus() {
    let dir = project("filing");

    // An external round in executing: the wrong status.
    let executing = prepared(&dir);
    refused(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &executing,
                "--kind",
                "external",
                "--focus",
                "security",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
        "status.wrong",
        4,
    );
    // A self round in executing: the role's.
    refused(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &executing,
                "--kind",
                "self",
                "--focus",
                "security",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
        "role.wrong",
        5,
    );
    refused(
        talita(
            &dir,
            &HUMAN,
            &[
                "round",
                "file",
                &executing,
                "--kind",
                "self",
                "--focus",
                "security",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
        "actor.not_agent",
        5,
    );

    // An external round from the claim's own session: independence.
    let id = to_review(&dir);
    let own_session = [
        ("TALITA_ACTOR", "runner-1"),
        ("TALITA_ROLE", "reviewer"),
        ("TALITA_SESSION", "ses-runner"),
    ];
    let refusal = refused(
        talita(
            &dir,
            &own_session,
            &[
                "round",
                "file",
                &id,
                "--kind",
                "external",
                "--focus",
                "security",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["review.independent"]);

    // Five external rounds filed: the budget is spent.
    edit(&dir, &id, |task| {
        for n in 1..=5 {
            task.review.rounds.push(Round {
                n,
                kind: RoundKind::External,
                focus: Focus::Security,
                reviewer: "reviewer-1".to_string(),
                session: Some("ses-reviewer".to_string()),
                verdict: Verdict::Ok,
                at: task.status_since.clone(),
                ..Round::default()
            });
        }
    });
    refused(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &id,
                "--kind",
                "external",
                "--focus",
                "design",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
        "review.budget_spent",
        4,
    );

    // A repeated focus is a warning, not a refusal.
    let repeated = to_review(&dir);
    edit(&dir, &repeated, |task| {
        task.review.rounds.push(Round {
            n: 1,
            kind: RoundKind::External,
            focus: Focus::EdgeCases,
            reviewer: "reviewer-1".to_string(),
            session: Some("ses-reviewer".to_string()),
            verdict: Verdict::Ok,
            at: task.status_since.clone(),
            ..Round::default()
        });
    });
    let filed = applied(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &repeated,
                "--kind",
                "external",
                "--focus",
                "edge-cases",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
    );
    assert_eq!(filed["data"]["round"]["n"], 2, "{filed}");
    assert_eq!(
        filed["data"]["warnings"][0]["code"], "review.focus_repeated",
        "{filed}"
    );
}

#[test]
fn round_file_reads_a_closed_input_schema() {
    let dir = project("input");
    let id = to_review(&dir);

    let unknown = json!({
        "kind": "external",
        "focus": "security",
        "verdict": "ok",
        "title": "not a field",
    });
    let path = write_round(&dir, "unknown.json", &unknown);
    refused(
        talita(&dir, &REVIEWER, &["round", "file", &id, "--input", &path]),
        "round file",
        "input.invalid",
        2,
    );

    let bad_focus = json!({"kind": "external", "focus": "style", "verdict": "ok"});
    let path = write_round(&dir, "focus.json", &bad_focus);
    refused(
        talita(&dir, &REVIEWER, &["round", "file", &id, "--input", &path]),
        "round file",
        "input.invalid",
        2,
    );

    let bad_severity = json!({
        "kind": "external",
        "focus": "security",
        "verdict": "changes-needed",
        "findings": [{"ref": "D1", "severity": "critical", "description": "x"}],
    });
    let path = write_round(&dir, "severity.json", &bad_severity);
    refused(
        talita(&dir, &REVIEWER, &["round", "file", &id, "--input", &path]),
        "round file",
        "input.invalid",
        2,
    );

    // A self round never carries findings; the remedy names the flag form.
    let self_with = json!({
        "kind": "self",
        "focus": "security",
        "verdict": "ok",
        "findings": [{"ref": "D1", "severity": "high", "description": "x"}],
    });
    let path = write_round(&dir, "self.json", &self_with);
    let refusal = refused(
        talita(&dir, &REVIEWER, &["round", "file", &id, "--input", &path]),
        "round file",
        "input.invalid",
        2,
    );
    assert!(
        refusal["error"]["remedy"]
            .as_str()
            .unwrap()
            .contains("--kind self")
    );

    // The flag form and --input do not mix, and the flags are required alone.
    let path = write_round(
        &dir,
        "ok.json",
        &json!({"kind": "external", "focus": "security", "verdict": "ok"}),
    );
    refused(
        talita(
            &dir,
            &REVIEWER,
            &["round", "file", &id, "--kind", "external", "--input", &path],
        ),
        "round file",
        "usage.invalid",
        2,
    );
    refused(
        talita(&dir, &REVIEWER, &["round", "file", &id]),
        "round file",
        "usage.invalid",
        2,
    );
}

#[test]
fn finding_open_is_the_reviewers_and_adds_to_its_round() {
    let dir = project("findings");
    let id = to_review(&dir);
    input_round(
        &dir,
        &id,
        "security",
        "changes-needed",
        json!([{"ref": "D1", "severity": "high", "description": "a defect"}]),
    );

    let opened = applied(
        talita(
            &dir,
            &REVIEWER,
            &[
                "finding",
                "open",
                &id,
                "--severity",
                "medium",
                "--description",
                "another defect",
            ],
        ),
        "finding open",
    );
    assert_eq!(opened["data"]["finding"]["id"], "FIND-02");
    assert_eq!(opened["data"]["finding"]["status"], "open");
    assert_eq!(opened["data"]["finding"]["severity"], "medium");
    let task = read(&dir, &id);
    assert_eq!(task.review.rounds[1].findings, vec!["FIND-01", "FIND-02"]);

    // Only the session that filed the round adds to it.
    let refusal = refused(
        talita(
            &dir,
            &REVIEWER2,
            &[
                "finding",
                "open",
                &id,
                "--severity",
                "low",
                "--description",
                "not mine",
            ],
        ),
        "finding open",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["review.round_owner"]);
    refused(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "open",
                &id,
                "--severity",
                "low",
                "--description",
                "not a reviewer",
            ],
        ),
        "finding open",
        "role.wrong",
        5,
    );
    refused(
        talita(
            &dir,
            &HUMAN,
            &[
                "finding",
                "open",
                &id,
                "--severity",
                "low",
                "--description",
                "not an agent",
            ],
        ),
        "finding open",
        "actor.not_agent",
        5,
    );

    // A finding needs a changes-needed round.
    let clean = to_review(&dir);
    applied(
        talita(
            &dir,
            &REVIEWER,
            &[
                "round",
                "file",
                &clean,
                "--kind",
                "external",
                "--focus",
                "design",
                "--verdict",
                "ok",
            ],
        ),
        "round file",
    );
    refused(
        talita(
            &dir,
            &REVIEWER,
            &[
                "finding",
                "open",
                &clean,
                "--severity",
                "low",
                "--description",
                "late",
            ],
        ),
        "finding open",
        "review.verdict_mismatch",
        4,
    );
}

#[test]
fn remediation_closes_each_finding_once_in_another_session() {
    let dir = project("remediation");
    let id = to_review(&dir);

    refused(
        talita(
            &dir,
            &RUNNER2,
            &["finding", "resolve", &id, "FIND-01", "--resolution", "x"],
        ),
        "finding resolve",
        "status.wrong",
        4,
    );

    input_round(
        &dir,
        &id,
        "security",
        "changes-needed",
        json!([
            {"ref": "D1", "severity": "high", "description": "a defect"},
            {"ref": "D2", "severity": "low", "description": "another"},
        ]),
    );
    applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );

    // The execution session cannot close its own findings.
    let refusal = refused(
        talita(
            &dir,
            &RUNNER,
            &["finding", "resolve", &id, "FIND-01", "--resolution", "x"],
        ),
        "finding resolve",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["remediation.session_distinct"]);
    let refusal = refused(
        talita(
            &dir,
            &RUNNER,
            &["finding", "dismiss", &id, "FIND-02", "--reason", "x"],
        ),
        "finding dismiss",
        "gate.refused",
        4,
    );
    assert_eq!(gates(&refusal), vec!["remediation.session_distinct"]);

    refused(
        talita(
            &dir,
            &REVIEWER,
            &["finding", "resolve", &id, "FIND-01", "--resolution", "x"],
        ),
        "finding resolve",
        "role.wrong",
        5,
    );
    refused(
        talita(
            &dir,
            &HUMAN,
            &["finding", "resolve", &id, "FIND-01", "--resolution", "x"],
        ),
        "finding resolve",
        "actor.not_agent",
        5,
    );
    refused(
        talita(
            &dir,
            &RUNNER2,
            &["finding", "resolve", &id, "FIND-09", "--resolution", "x"],
        ),
        "finding resolve",
        "part.not_found",
        3,
    );

    let resolved = applied(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "resolve",
                &id,
                "FIND-01",
                "--resolution",
                "the lock is dropped now",
            ],
        ),
        "finding resolve",
    );
    assert_eq!(resolved["data"]["finding"]["status"], "fixed");
    refused(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "resolve",
                &id,
                "FIND-01",
                "--resolution",
                "again",
            ],
        ),
        "finding resolve",
        "finding.closed",
        4,
    );

    let dismissed = applied(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "dismiss",
                &id,
                "FIND-02",
                "--reason",
                "it is the crate's name",
            ],
        ),
        "finding dismiss",
    );
    assert_eq!(dismissed["data"]["finding"]["status"], "dismissed");
    refused(
        talita(
            &dir,
            &RUNNER2,
            &["finding", "dismiss", &id, "FIND-02", "--reason", "again"],
        ),
        "finding dismiss",
        "finding.closed",
        4,
    );

    let task = read(&dir, &id);
    assert_eq!(task.status, Status::Remediating);
    assert_eq!(task.review.findings[0].status, FindingStatus::Fixed);
    assert_eq!(task.review.findings[1].status, FindingStatus::Dismissed);
    assert_eq!(task.review.findings[1].resolution, "it is the crate's name");
    assert!(!task.review.findings[0].resolved.is_empty());
}

#[test]
fn comments_are_any_actor_and_outlive_a_closed_finding() {
    let dir = project("comments");
    let id = to_review(&dir);
    input_round(
        &dir,
        &id,
        "security",
        "changes-needed",
        json!([{"ref": "D1", "severity": "high", "description": "a defect"}]),
    );
    applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );
    applied(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "resolve",
                &id,
                "FIND-01",
                "--resolution",
                "fixed",
            ],
        ),
        "finding resolve",
    );

    let human = applied(
        talita(
            &dir,
            &HUMAN,
            &["comment", "add", &id, "FIND-01", "--body", "thank you"],
        ),
        "comment add",
    );
    assert_eq!(human["data"]["comment"]["id"], "CMT-01");
    assert_eq!(human["data"]["comment"]["author"], "luiz");
    assert_eq!(human["data"]["comment"]["body"], "thank you");
    let discussed = applied(
        talita(
            &dir,
            &REVIEWER,
            &["comment", "add", &id, "FIND-01", "--body", "noted"],
        ),
        "comment add",
    );
    assert_eq!(discussed["data"]["comment"]["id"], "CMT-02");

    let task = read(&dir, &id);
    assert_eq!(task.review.findings[0].comments.len(), 2);
    assert_eq!(task.review.findings[0].comments[0].author, "luiz");
    assert_eq!(task.review.findings[0].comments[1].id, "CMT-02");

    refused(
        talita(
            &dir,
            &RUNNER2,
            &["comment", "add", &id, "FIND-09", "--body", "x"],
        ),
        "comment add",
        "part.not_found",
        3,
    );
}
