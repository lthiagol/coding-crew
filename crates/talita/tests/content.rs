//! Task content (T6) through the real binary, against scratch projects under
//! the temp dir: `task`, `scope`, `criterion`, `step`, `question`, `note`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use coding_crew_core::lifecycle::{TransitionContext, check};
use coding_crew_core::{Actor, Event, Role, Store};
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

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-content-{name}-{}", std::process::id()));
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

fn work(dir: &Path) -> PathBuf {
    dir.join("coding-crew").join("work")
}

/// The one file of `id` in the scratch project.
fn file_of(dir: &Path, id: &str) -> PathBuf {
    let prefix = format!("{id}-");
    let mut found: Vec<PathBuf> = fs::read_dir(work(dir))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".json"))
        })
        .collect();
    assert_eq!(found.len(), 1, "{id}: {found:?}");
    found.pop().unwrap()
}

/// Put a scratch task into a state talita's T6 verbs cannot reach.
fn set(dir: &Path, id: &str, change: impl FnOnce(&mut Value)) {
    let path = file_of(dir, id);
    let mut task: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut task);
    fs::write(&path, serde_json::to_string_pretty(&task).unwrap()).unwrap();
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

#[test]
fn a_task_is_groomed_end_to_end_through_talita() {
    let dir = project("groom");
    let c = &CREATOR;
    let created = applied(
        talita(&dir, c, &["task", "create", "--title", "Parse the config"]),
        "task create",
    );
    assert_eq!(created["data"]["id"], "TSK01");
    assert_eq!(created["data"]["status"], "draft");
    assert_eq!(created["wrote"][0], "work/TSK01-parse-the-config.json");

    let outcome = dir.join("outcome.md");
    fs::write(&outcome, "The config parses.\n").unwrap();
    let at_outcome = format!("@{}", outcome.display());
    applied(
        talita(
            &dir,
            c,
            &[
                "task",
                "describe",
                "TSK01",
                "--outcome",
                &at_outcome,
                "--problem",
                "It does not.",
            ],
        ),
        "task describe",
    );
    applied(
        talita(
            &dir,
            c,
            &["scope", "include", "TSK01", "--item", "the parser"],
        ),
        "scope include",
    );
    applied(
        talita(&dir, c, &["scope", "include", "TSK01", "--item", "a typo"]),
        "scope include",
    );
    applied(
        talita(&dir, c, &["scope", "drop", "TSK01", "--in", "2"]),
        "scope drop",
    );
    applied(
        talita(
            &dir,
            c,
            &["scope", "exclude", "TSK01", "--item", "the writer"],
        ),
        "scope exclude",
    );
    let a1 = applied(
        talita(
            &dir,
            c,
            &[
                "criterion",
                "add",
                "TSK01",
                "--description",
                "parses",
                "--verification",
                "cargo test",
            ],
        ),
        "criterion add",
    );
    assert_eq!(a1["data"]["criterion"]["id"], "ACCEPT-01");
    assert_eq!(a1["data"]["criterion"]["status"], "pending");
    applied(
        talita(
            &dir,
            c,
            &[
                "criterion",
                "add",
                "TSK01",
                "--description",
                "drop me",
                "--verification",
                "true",
            ],
        ),
        "criterion add",
    );
    applied(
        talita(&dir, c, &["criterion", "remove", "TSK01", "ACCEPT-02"]),
        "criterion remove",
    );
    applied(
        talita(
            &dir,
            c,
            &[
                "criterion",
                "revise",
                "TSK01",
                "ACCEPT-01",
                "--verification",
                "cargo test config",
            ],
        ),
        "criterion revise",
    );
    let step = applied(
        talita(
            &dir,
            c,
            &[
                "step",
                "add",
                "TSK01",
                "--action",
                "write the parser",
                "--done-when",
                "tests pass",
                "--covers",
                "ACCEPT-01",
                "--files",
                "src/config.rs",
            ],
        ),
        "step add",
    );
    assert_eq!(step["data"]["step"]["id"], "STP-01");
    assert_eq!(step["data"]["step"]["order"], 1);
    applied(
        talita(
            &dir,
            c,
            &[
                "step",
                "add",
                "TSK01",
                "--action",
                "doc it",
                "--done-when",
                "x",
                "--after",
                "STP-01",
            ],
        ),
        "step add",
    );
    applied(
        talita(
            &dir,
            c,
            &[
                "step",
                "revise",
                "TSK01",
                "STP-02",
                "--done-when",
                "README names it",
            ],
        ),
        "step revise",
    );
    applied(
        talita(&dir, c, &["step", "remove", "TSK01", "STP-02"]),
        "step remove",
    );

    // Any role asks; the creator answers.
    let q = applied(
        talita(
            &dir,
            &RUNNER,
            &["question", "ask", "TSK01", "--question", "TOML or JSON?"],
        ),
        "question ask",
    );
    assert_eq!(q["data"]["question"]["id"], "QST-01");
    assert_eq!(q["data"]["question"]["raised_by"], "runner-1");
    let a = applied(
        talita(
            &dir,
            c,
            &[
                "question",
                "answer",
                "TSK01",
                "QST-01",
                "--answer",
                "TOML",
                "--instructed-by",
                "luiz",
            ],
        ),
        "question answer",
    );
    assert_eq!(a["data"]["question"]["status"], "resolved");
    assert_eq!(a["data"]["question"]["resolved_by"], "creator-1");
    let n = applied(
        talita(
            &dir,
            &HUMAN,
            &["note", "add", "TSK01", "--body", "serde has a toml crate"],
        ),
        "note add",
    );
    assert_eq!(n["data"]["note"]["id"], "NOTE-01");
    assert_eq!(n["data"]["note"]["author"], "luiz");

    applied(
        talita(
            &dir,
            c,
            &["task", "prioritize", "TSK01", "--priority", "high"],
        ),
        "task prioritize",
    );
    applied(
        talita(&dir, c, &["task", "estimate", "TSK01", "--effort", "S"]),
        "task estimate",
    );
    applied(
        talita(
            &dir,
            c,
            &[
                "task", "tag", "TSK01", "--add", "config", "--add", "x", "--remove", "x",
            ],
        ),
        "task tag",
    );
    let dep = create(&dir, "Read the config");
    applied(
        talita(&dir, c, &["task", "depend", "TSK01", "--on", &dep]),
        "task depend",
    );
    applied(
        talita(&dir, c, &["task", "undepend", "TSK01", "--on", &dep]),
        "task undepend",
    );
    let renamed = applied(
        talita(
            &dir,
            c,
            &[
                "task",
                "retitle",
                "TSK01",
                "--title",
                "Parse the TOML config",
            ],
        ),
        "task retitle",
    );
    assert_eq!(renamed["wrote"][0], "work/TSK01-parse-the-toml-config.json");
    assert!(!work(&dir).join("TSK01-parse-the-config.json").exists());

    let shown = applied(talita(&dir, &[], &["task", "show", "TSK01"]), "task show");
    let task = &shown["data"]["task"];
    assert_eq!(
        task["intent"]["outcome"], "The config parses.\n",
        "stored verbatim"
    );
    assert_eq!(task["scope"]["in_scope"], serde_json::json!(["the parser"]));
    assert_eq!(task["meta"]["priority"], "high");
    assert_eq!(task["meta"]["effort"], "S");
    assert_eq!(task["meta"]["tags"], serde_json::json!(["config"]));
    assert_eq!(shown["data"]["derived"]["criteria"]["ACCEPT-01"], "pending");
    assert_eq!(shown["data"]["derived"]["start_readiness"]["ok"], false);

    let listed = applied(
        talita(&dir, &[], &["task", "list", "--tag", "config"]),
        "task list",
    );
    assert_eq!(listed["data"].as_array().unwrap().len(), 1);
    let drafts = applied(
        talita(&dir, &[], &["task", "list", "--status", "draft"]),
        "task list",
    );
    assert_eq!(drafts["data"][0]["id"], "TSK01");
    assert_eq!(drafts["data"][1]["id"], "TSK02");

    // The groom and approve gates pass on what talita wrote.
    let store = Store::for_project(&dir);
    let task = store.load("TSK01").unwrap();
    let creator = Actor::agent("creator-1", Role::Creator).in_session("ses-creator");
    let ctx = TransitionContext::default();
    let now = coding_crew_core::now_rfc3339();
    assert!(check(&task, Event::Groom, &creator, &ctx, &now).is_empty());
    assert!(check(&task, Event::Approve, &creator, &ctx, &now).is_empty());
    assert!(task.revision > 10, "each write moved the revision");
}

#[test]
fn a_frozen_section_is_refused_with_the_child_task_remedy() {
    let dir = project("frozen");
    let c = &CREATOR;
    create(&dir, "Frozen");
    applied(
        talita(
            &dir,
            c,
            &[
                "criterion",
                "add",
                "TSK01",
                "--description",
                "d",
                "--verification",
                "v",
            ],
        ),
        "criterion add",
    );
    applied(
        talita(&dir, c, &["scope", "exclude", "TSK01", "--item", "x"]),
        "scope exclude",
    );
    set(&dir, "TSK01", |t| t["status"] = "approved".into());

    let frozen: [(&[&str], &str); 7] = [
        (
            &["task", "describe", "TSK01", "--outcome", "o"],
            "task describe",
        ),
        (
            &["scope", "include", "TSK01", "--item", "i"],
            "scope include",
        ),
        (
            &["scope", "exclude", "TSK01", "--item", "i"],
            "scope exclude",
        ),
        (&["scope", "drop", "TSK01", "--out", "1"], "scope drop"),
        (
            &[
                "criterion",
                "add",
                "TSK01",
                "--description",
                "d",
                "--verification",
                "v",
            ],
            "criterion add",
        ),
        (
            &[
                "criterion",
                "revise",
                "TSK01",
                "ACCEPT-01",
                "--description",
                "typo",
            ],
            "criterion revise",
        ),
        (
            &["criterion", "remove", "TSK01", "ACCEPT-01"],
            "criterion remove",
        ),
    ];
    for (args, command) in frozen {
        let e = refused(talita(&dir, c, args), command, "section.frozen", 4);
        assert_eq!(
            e["error"]["remedy"], "talita task create --parent TSK01 --title <t>",
            "{command}"
        );
    }

    // Not frozen, but pre-approval only.
    for (args, command) in [
        (
            &["task", "retitle", "TSK01", "--title", "t"][..],
            "task retitle",
        ),
        (
            &["task", "prioritize", "TSK01", "--priority", "low"][..],
            "task prioritize",
        ),
        (
            &["task", "estimate", "TSK01", "--effort", "L"][..],
            "task estimate",
        ),
        (&["task", "tag", "TSK01", "--add", "t"][..], "task tag"),
    ] {
        refused(talita(&dir, c, args), command, "status.wrong", 4);
    }
    // Questions and notes stay open.
    applied(
        talita(
            &dir,
            &RUNNER,
            &["question", "ask", "TSK01", "--question", "q"],
        ),
        "question ask",
    );
    applied(
        talita(&dir, &RUNNER, &["note", "add", "TSK01", "--body", "b"]),
        "note add",
    );

    // The remedy works: a child task, linked both ways.
    let child = applied(
        talita(
            &dir,
            c,
            &[
                "task",
                "create",
                "--parent",
                "TSK01",
                "--title",
                "Fix the typo",
                "--kind",
                "bug",
            ],
        ),
        "task create",
    );
    assert_eq!(child["data"]["id"], "TSK01.1");
    assert_eq!(child["data"]["kind"], "bug");
    assert_eq!(
        child["wrote"],
        json!([
            "work/TSK01.1-fix-the-typo.json",
            "work/TSK01-frozen.json",
            "activity.json"
        ]),
        "the child, the parent, then the journal"
    );
    let second = applied(
        talita(
            &dir,
            c,
            &["task", "create", "--parent", "TSK01", "--title", "Another"],
        ),
        "task create",
    );
    assert_eq!(second["data"]["id"], "TSK01.2");
    let parent = Store::for_project(&dir).load("TSK01").unwrap();
    assert_eq!(parent.children, ["TSK01.1", "TSK01.2"]);
    assert_eq!(
        create(&dir, "Next plain"),
        "TSK02",
        "child ids are not plain numbers"
    );
}

#[test]
fn steps_after_approval_need_the_working_runner_and_a_deviation() {
    let dir = project("steps-after");
    create(&dir, "Work");
    set(&dir, "TSK01", |t| {
        t["status"] = "approved".into();
    });
    let add = [
        "step",
        "add",
        "TSK01",
        "--action",
        "a",
        "--done-when",
        "d",
        "--deviation",
        "found more",
    ];
    refused(
        talita(&dir, &CREATOR, &add),
        "step add",
        "section.not_owned",
        5,
    );
    refused(talita(&dir, &RUNNER, &add), "step add", "status.wrong", 4);

    set(&dir, "TSK01", |t| {
        t["status"] = "executing".into();
        t["claim"] = serde_json::json!({
            "by": "runner-1", "session": "ses-runner", "at": "2026-11-23T00:00:00Z",
            "lease_expires_at": "2999-01-01T00:00:00Z"
        });
    });
    refused(
        talita(
            &dir,
            &RUNNER,
            &["step", "add", "TSK01", "--action", "a", "--done-when", "d"],
        ),
        "step add",
        "usage.invalid",
        2,
    );
    let other = [
        ("TALITA_ACTOR", "runner-1"),
        ("TALITA_ROLE", "runner"),
        ("TALITA_SESSION", "ses-other"),
    ];
    refused(
        talita(&dir, &other, &add),
        "step add",
        "claim.not_holder",
        5,
    );
    let ok = applied(talita(&dir, &RUNNER, &add), "step add");
    assert_eq!(ok["data"]["step"]["deviation"], "found more");
    refused(
        talita(&dir, &RUNNER, &["step", "remove", "TSK01", "STP-01"]),
        "step remove",
        "status.wrong",
        4,
    );

    set(&dir, "TSK01", |t| t["status"] = "remediating".into());
    let e = refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "step",
                "revise",
                "TSK01",
                "STP-01",
                "--action",
                "b",
                "--deviation",
                "y",
            ],
        ),
        "step revise",
        "gate.refused",
        4,
    );
    assert_eq!(
        e["error"]["gates"][0]["gate"],
        "remediation.session_distinct"
    );
    applied(
        talita(
            &dir,
            &other,
            &[
                "step",
                "revise",
                "TSK01",
                "STP-01",
                "--action",
                "b",
                "--deviation",
                "y",
            ],
        ),
        "step revise",
    );
}

#[test]
fn each_refusal_carries_its_code_and_exit() {
    let dir = project("refusals");
    let c = &CREATOR;
    create(&dir, "One");
    create(&dir, "Two");

    // Identity and ownership.
    refused(
        talita(
            &dir,
            &[("TALITA_ACTOR", "a")],
            &["task", "describe", "TSK01", "--outcome", "o"],
        ),
        "task describe",
        "identity.missing",
        5,
    );
    refused(
        talita(
            &dir,
            &HUMAN,
            &["task", "describe", "TSK01", "--outcome", "o"],
        ),
        "task describe",
        "actor.not_agent",
        5,
    );
    refused(
        talita(&dir, &HUMAN, &["task", "create", "--title", "t"]),
        "task create",
        "actor.not_agent",
        5,
    );
    refused(
        talita(&dir, &RUNNER, &["task", "create", "--title", "t"]),
        "task create",
        "role.wrong",
        5,
    );
    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "criterion",
                "add",
                "TSK01",
                "--description",
                "d",
                "--verification",
                "v",
            ],
        ),
        "criterion add",
        "section.not_owned",
        5,
    );
    refused(
        talita(
            &dir,
            &RUNNER,
            &["question", "answer", "TSK01", "QST-01", "--answer", "a"],
        ),
        "question answer",
        "section.not_owned",
        5,
    );

    // Not found.
    refused(
        talita(&dir, &[], &["task", "show", "TSK09"]),
        "task show",
        "task.not_found",
        3,
    );
    refused(
        talita(
            &dir,
            c,
            &["task", "create", "--parent", "TSK09", "--title", "t"],
        ),
        "task create",
        "task.not_found",
        3,
    );
    refused(
        talita(&dir, c, &["task", "depend", "TSK01", "--on", "TSK09"]),
        "task depend",
        "task.not_found",
        3,
    );
    refused(
        talita(
            &dir,
            c,
            &[
                "criterion",
                "revise",
                "TSK01",
                "ACCEPT-09",
                "--description",
                "d",
            ],
        ),
        "criterion revise",
        "part.not_found",
        3,
    );
    refused(
        talita(
            &dir,
            c,
            &[
                "step",
                "add",
                "TSK01",
                "--action",
                "a",
                "--done-when",
                "d",
                "--covers",
                "ACCEPT-09",
            ],
        ),
        "step add",
        "part.not_found",
        3,
    );
    refused(
        talita(&dir, c, &["scope", "drop", "TSK01", "--in", "1"]),
        "scope drop",
        "part.not_found",
        3,
    );
    refused(
        talita(&dir, c, &["task", "undepend", "TSK01", "--on", "TSK02"]),
        "task undepend",
        "part.not_found",
        3,
    );
    refused(
        talita(
            &std::env::temp_dir(),
            &[("TALITA_PROJECT", "/nonexistent-talita")],
            &["task", "list"],
        ),
        "task list",
        "project.not_found",
        3,
    );

    // Input and usage.
    refused(
        talita(&dir, c, &["task", "create", "--title", "two\nlines"]),
        "task create",
        "input.invalid",
        2,
    );
    refused(
        talita(
            &dir,
            c,
            &["task", "prioritize", "TSK01", "--priority", "now"],
        ),
        "task prioritize",
        "input.invalid",
        2,
    );
    refused(
        talita(
            &dir,
            c,
            &[
                "task",
                "describe",
                "TSK01",
                "--outcome",
                "@/nonexistent-file",
            ],
        ),
        "task describe",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, c, &["task", "describe", "TSK01"]),
        "task describe",
        "usage.invalid",
        2,
    );
    refused(
        talita(
            &dir,
            c,
            &["scope", "drop", "TSK01", "--in", "1", "--out", "1"],
        ),
        "scope drop",
        "usage.invalid",
        2,
    );

    // Rules.
    applied(
        talita(
            &dir,
            c,
            &[
                "criterion",
                "add",
                "TSK01",
                "--description",
                "d",
                "--verification",
                "v",
            ],
        ),
        "criterion add",
    );
    applied(
        talita(
            &dir,
            c,
            &[
                "step",
                "add",
                "TSK01",
                "--action",
                "a",
                "--done-when",
                "d",
                "--covers",
                "ACCEPT-01",
            ],
        ),
        "step add",
    );
    let e = refused(
        talita(&dir, c, &["criterion", "remove", "TSK01", "ACCEPT-01"]),
        "criterion remove",
        "criterion.covered",
        4,
    );
    assert!(e["error"]["remedy"].as_str().unwrap().contains("STP-01"));
    applied(
        talita(&dir, c, &["question", "ask", "TSK01", "--question", "q"]),
        "question ask",
    );
    applied(
        talita(
            &dir,
            c,
            &["question", "answer", "TSK01", "QST-01", "--answer", "a"],
        ),
        "question answer",
    );
    refused(
        talita(
            &dir,
            c,
            &["question", "answer", "TSK01", "QST-01", "--answer", "b"],
        ),
        "question answer",
        "question.resolved",
        4,
    );
    applied(
        talita(&dir, c, &["task", "depend", "TSK01", "--on", "TSK02"]),
        "task depend",
    );
    refused(
        talita(&dir, c, &["task", "depend", "TSK02", "--on", "TSK01"]),
        "task depend",
        "depends.cycle",
        4,
    );
    refused(
        talita(&dir, c, &["task", "depend", "TSK02", "--on", "TSK02"]),
        "task depend",
        "depends.cycle",
        4,
    );
    set(&dir, "TSK02", |t| {
        t["status"] = "approved".into();
        t["claim"] = serde_json::json!({"by": "runner-1", "at": "2026-11-23T00:00:00Z"});
    });
    refused(
        talita(&dir, c, &["task", "undepend", "TSK02", "--on", "TSK01"]),
        "task undepend",
        "status.wrong",
        4,
    );

    // Conflict.
    let revision = Store::for_project(&dir)
        .load("TSK01")
        .unwrap()
        .revision
        .to_string();
    let stale = (revision.parse::<u32>().unwrap() - 1).to_string();
    refused(
        talita(
            &dir,
            c,
            &[
                "--expect-revision",
                &stale,
                "note",
                "add",
                "TSK01",
                "--body",
                "b",
            ],
        ),
        "note add",
        "conflict.revision",
        6,
    );
    applied(
        talita(
            &dir,
            c,
            &[
                "--expect-revision",
                &revision,
                "note",
                "add",
                "TSK01",
                "--body",
                "b",
            ],
        ),
        "note add",
    );

    // Cancelled, then damaged.
    set(&dir, "TSK01", |t| {
        t["overlays"]["cancelled"] =
            serde_json::json!({"reason": "dropped", "by": "luiz", "at": "x"});
    });
    refused(
        talita(&dir, &HUMAN, &["note", "add", "TSK01", "--body", "b"]),
        "note add",
        "task.cancelled",
        4,
    );
    fs::write(file_of(&dir, "TSK01"), "{ not json").unwrap();
    refused(
        talita(&dir, c, &["note", "add", "TSK01", "--body", "b"]),
        "note add",
        "store.unreadable",
        7,
    );
}
