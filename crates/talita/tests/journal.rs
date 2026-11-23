//! The journal writer (T11) through the real binary, against scratch projects
//! under the temp dir: every task verb's event, `journal show`, and raul's
//! activity block over the T10 run.
//!
//! core's claim and evidence writes stand in where talita has no verb yet
//! (claim is T8; evidence is T9).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use coding_crew_core::claim::{self, DEFAULT_LEASE};
use coding_crew_core::{
    Activity, ActivityEvent, Actor, EventKind, Evidence, Role, Status, StepStatus, Store, Task,
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

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-journal-{name}-{}", std::process::id()));
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

/// The journal as raul's reader reads it.
fn journal(dir: &Path) -> Activity {
    Activity::read(&store(dir).activity_path()).expect("the journal")
}

fn journal_path(dir: &Path) -> PathBuf {
    store(dir).activity_path()
}

fn kinds(dir: &Path) -> Vec<String> {
    journal(dir)
        .events
        .iter()
        .map(|event| event.kind.as_str().to_string())
        .collect()
}

fn notes(dir: &Path) -> Vec<Option<String>> {
    journal(dir)
        .events
        .iter()
        .map(|event| event.note.clone())
        .collect()
}

/// The whole T10 run: the README's sequence's build and review halves on one
/// task, with the claim, steps and evidence through core.
fn the_t10_run(dir: &Path) -> String {
    let id = to_review(dir);
    let filed = input_round(
        dir,
        &id,
        "security",
        "changes-needed",
        json!([
            {"ref": "D1", "severity": "high", "description": "the lock is not released"},
            {"ref": "D2", "severity": "low", "description": "the name is vague"},
        ]),
    );
    assert_eq!(
        filed["journal"],
        json!(["round_filed", "finding_raised", "finding_raised"]),
        "{filed}"
    );
    assert_eq!(
        filed["wrote"].as_array().expect("the writes").last(),
        Some(&json!("activity.json")),
        "{filed}"
    );
    applied(
        talita(dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );
    applied(
        talita(
            dir,
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
    applied(
        talita(
            dir,
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
    applied(
        talita(dir, &RUNNER2, &["task", "re-review", &id]),
        "task re-review",
    );
    applied(
        talita(
            dir,
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
    applied(
        talita(dir, &REVIEWER, &["task", "pass-review", &id]),
        "task pass-review",
    );
    id
}

#[test]
fn the_t10_run_is_the_journal_in_order() {
    let dir = project("run");
    let id = the_t10_run(&dir);

    let activity = journal(&dir);
    assert_eq!(
        activity.unreadable, 0,
        "the journal holds unreadable events"
    );
    let expected = [
        "created",
        "groomed",
        "approved",
        "started",
        "round_filed",
        "review_requested",
        "round_filed",
        "finding_raised",
        "finding_raised",
        "changes_requested",
        "finding_resolved",
        "finding_resolved",
        "remediated",
        "round_filed",
        "completed",
    ];
    let words: Vec<&str> = activity
        .events
        .iter()
        .map(|event| event.kind.as_str())
        .collect();
    assert_eq!(words, expected, "{activity:?}");

    let by: Vec<&str> = activity
        .events
        .iter()
        .map(|event| event.by.as_str())
        .collect();
    assert_eq!(
        by,
        [
            "creator", "creator", "creator", "runner", "runner", "runner", "reviewer", "reviewer",
            "reviewer", "reviewer", "runner", "runner", "runner", "reviewer", "reviewer",
        ],
        "{activity:?}"
    );

    let expected_notes: [Option<&str>; 15] = [
        None,
        None,
        None,
        None,
        Some("self, spec-compliance, ok"),
        None,
        Some("external 1, security, changes-needed"),
        Some("raised FIND-01"),
        Some("raised FIND-02"),
        None,
        Some("fixed FIND-01"),
        Some("dismissed FIND-02"),
        Some("after round 1"),
        Some("external 2, edge-cases, ok"),
        None,
    ];
    let notes: Vec<Option<&str>> = activity
        .events
        .iter()
        .map(|event| event.note.as_deref())
        .collect();
    assert_eq!(notes, expected_notes, "{activity:?}");

    for event in &activity.events {
        assert_eq!(event.task, id, "{event:?}");
        assert!(!event.at.is_empty(), "{event:?}");
    }

    // The envelope says what each call appended, and that it wrote the
    // journal after the task file.
    let other = create(&dir, "A second task");
    let retitled = applied(
        talita(
            &dir,
            &CREATOR,
            &["task", "retitle", &other, "--title", "Another title"],
        ),
        "task retitle",
    );
    assert_eq!(
        retitled["journal"],
        json!([]),
        "a content edit is journaled"
    );
    assert_eq!(
        retitled["wrote"],
        json!([format!("work/{other}-another-title.json")]),
        "a content edit rewrote the journal"
    );
}

#[test]
fn the_t10_run_reaches_rauls_activity_block() {
    let dir = project("raul");
    the_t10_run(&dir);

    let screen = raul(store(&dir).root());
    assert!(screen.contains("activity"), "{screen}");
    // The block shows the last five of the fifteen moves and says how many
    // earlier ones it left out.
    assert!(screen.contains("+10 earlier events"), "{screen}");
    let order = [
        "fixed FIND-01",
        "dismissed FIND-02",
        "after round 1",
        "external 2, edge-cases, ok",
        "TSK01 completed",
    ];
    let positions: Vec<usize> = order
        .iter()
        .map(|part| {
            screen
                .find(part)
                .unwrap_or_else(|| panic!("{part} is not drawn: {screen}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "the moves are out of order: {screen}"
    );
}

/// raul's binary beside this test binary, as `cargo test --workspace` builds
/// it. A missing raul fails the test: the gate must not pass silently.
fn raul(work: &Path) -> String {
    let binary = raul_binary();
    let output = Command::new(&binary)
        .args(["--render", "--plain", "--width", "200", "--height", "80"])
        .arg(work)
        .output()
        .expect("raul runs");
    assert!(
        output.status.success(),
        "raul failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8")
}

/// The nearest `raul` above the test binary in the target directory: beside
/// it in `deps/`, or above the build folder this cargo writes test binaries
/// to.
fn raul_binary() -> PathBuf {
    let name = format!("raul{}", std::env::consts::EXE_SUFFIX);
    let mut dir = std::env::current_exe().expect("the test binary");
    dir.pop();
    loop {
        let candidate = dir.join(&name);
        if candidate.is_file() {
            return candidate;
        }
        if !dir.pop() {
            panic!("raul is not built in the target directory above the test binary");
        }
    }
}

#[test]
fn the_overlays_journal_their_reason_and_cancel_releases_the_claim() {
    let dir = project("overlays");
    let held = create(&dir, "Hold me");

    for (args, command) in [
        (
            vec!["task", "block", &held, "--reason", "the design is open"],
            "task block",
        ),
        (vec!["task", "unblock", held.as_str()], "task unblock"),
        (
            vec!["task", "defer", &held, "--reason", "after the release"],
            "task defer",
        ),
        (vec!["task", "undefer", held.as_str()], "task undefer"),
    ] {
        let done = applied(talita(&dir, &HUMAN, &args), command);
        assert_eq!(done["journal"].as_array().unwrap().len(), 1, "{done}");
    }
    assert_eq!(
        kinds(&dir),
        ["created", "blocked", "unblocked", "deferred", "undeferred"]
    );
    assert_eq!(
        notes(&dir),
        [
            None,
            Some("the design is open".to_string()),
            None,
            Some("after the release".to_string()),
            None,
        ]
    );
    assert_eq!(journal(&dir).events[1].by, "human");

    // Cancel on a live claim appends its release, both with the reason.
    let running = prepared(&dir);
    let cancelled = applied(
        talita(
            &dir,
            &HUMAN,
            &["task", "cancel", &running, "--reason", "the plan changed"],
        ),
        "task cancel",
    );
    assert_eq!(
        cancelled["journal"],
        json!(["cancelled", "released"]),
        "{cancelled}"
    );
    let events = journal(&dir).events;
    let tail = &events[events.len() - 2..];
    assert_eq!(tail[0].kind, EventKind::Cancelled);
    assert_eq!(tail[0].by, "human");
    assert_eq!(tail[0].note.as_deref(), Some("the plan changed"));
    assert_eq!(tail[1].kind, EventKind::Released);
    assert_eq!(tail[1].note.as_deref(), Some("the plan changed"));
    assert!(
        read(&dir, &running).claim.is_none(),
        "the claim was not released"
    );
}

#[test]
fn notes_and_comments_journal_commented() {
    let dir = project("comments");
    let id = prepared(&dir);
    let done = applied(
        talita(&dir, &RUNNER, &["note", "add", &id, "--body", "a learning"]),
        "note add",
    );
    assert_eq!(done["journal"], json!(["commented"]), "{done}");
    assert_eq!(kinds(&dir).last().map(String::as_str), Some("commented"));
    assert_eq!(journal(&dir).events.last().unwrap().by, "runner");

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
    let done = applied(
        talita(
            &dir,
            &REVIEWER,
            &["comment", "add", &id, "FIND-01", "--body", "noted"],
        ),
        "comment add",
    );
    assert_eq!(done["journal"], json!(["commented"]), "{done}");
    let last = journal(&dir).events.last().cloned().expect("an event");
    assert_eq!(last.kind, EventKind::Commented);
    assert_eq!(last.by, "reviewer");
    assert!(last.note.is_none());
}

#[test]
fn escalating_resuming_and_reopening_carry_their_note() {
    let dir = project("resume");
    let id = to_review(&dir);
    edit(&dir, &id, |task| task.review.max_rounds = 1);
    input_round(
        &dir,
        &id,
        "security",
        "changes-needed",
        json!([{"ref": "D1", "severity": "high", "description": "a defect"}]),
    );

    let escalated = applied(
        talita(&dir, &REVIEWER, &["task", "escalate", &id]),
        "task escalate",
    );
    assert_eq!(escalated["journal"], json!(["escalated"]), "{escalated}");
    let last = journal(&dir).events.last().cloned().expect("an event");
    assert_eq!(last.kind, EventKind::Escalated);
    assert_eq!(last.by, "reviewer");
    assert!(last.note.is_none());

    let resumed = applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "task",
                "resume",
                &id,
                "--instructed-by",
                "luiz",
                "--reason",
                "the budget is raised",
                "--grant-rounds",
                "1",
            ],
        ),
        "task resume",
    );
    assert_eq!(resumed["journal"], json!(["resumed"]), "{resumed}");
    let last = journal(&dir).events.last().cloned().expect("an event");
    assert_eq!(last.kind, EventKind::Resumed);
    assert_eq!(last.by, "creator");
    assert_eq!(
        last.note.as_deref(),
        Some("instructed by luiz: the budget is raised")
    );

    edit(&dir, &id, |task| task.status = Status::Done);
    let reopened = applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "task",
                "reopen",
                &id,
                "--instructed-by",
                "luiz",
                "--reason",
                "the count is wrong",
            ],
        ),
        "task reopen",
    );
    assert_eq!(reopened["journal"], json!(["reopened"]), "{reopened}");
    let last = journal(&dir).events.last().cloned().expect("an event");
    assert_eq!(last.kind, EventKind::Reopened);
    assert_eq!(last.by, "creator");
    assert_eq!(
        last.note.as_deref(),
        Some("instructed by luiz: the count is wrong")
    );
}

#[test]
fn a_promoted_item_journals_created_on_its_task() {
    let dir = project("promote");
    let item = applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "item",
                "create",
                "--kind",
                "idea",
                "--title",
                "a small feature",
            ],
        ),
        "item create",
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    applied(
        talita(
            &dir,
            &CREATOR,
            &["item", "shape", &item, "--kind", "feature"],
        ),
        "item shape",
    );
    assert!(
        !journal_path(&dir).exists(),
        "an item write created a journal: {:?}",
        kinds(&dir)
    );

    let promoted = applied(
        talita(&dir, &CREATOR, &["item", "promote", &item]),
        "item promote",
    );
    assert_eq!(promoted["journal"], json!(["created"]), "{promoted}");
    let task = promoted["data"]["task"]["id"].as_str().unwrap().to_string();
    let events = journal(&dir).events;
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].kind, EventKind::Created);
    assert_eq!(events[0].task, task);
    assert_eq!(events[0].by, "creator");
    assert_eq!(events[0].note.as_deref(), Some("from ITM01"));
    assert_eq!(
        promoted["wrote"],
        json!([
            format!("work/{item}-a-small-feature.json"),
            format!("work/{task}-a-small-feature.json"),
            "activity.json"
        ]),
        "{promoted}"
    );
}

#[test]
fn content_and_part_writes_are_not_journaled() {
    let dir = project("content");
    let id = create(&dir, "Written content");
    let before = fs::read(journal_path(&dir)).expect("the journal");

    let writes: Vec<(&str, Vec<&str>)> = vec![
        (
            "task retitle",
            vec!["task", "retitle", &id, "--title", "Another title"],
        ),
        (
            "task describe",
            vec!["task", "describe", &id, "--problem", "a problem"],
        ),
        (
            "task prioritize",
            vec!["task", "prioritize", &id, "--priority", "high"],
        ),
        (
            "task estimate",
            vec!["task", "estimate", &id, "--effort", "M"],
        ),
        ("task tag", vec!["task", "tag", &id, "--add", "t"]),
        (
            "scope include",
            vec!["scope", "include", &id, "--item", "this"],
        ),
        (
            "criterion add",
            vec![
                "criterion",
                "add",
                &id,
                "--description",
                "works",
                "--verification",
                "true",
            ],
        ),
        (
            "step add",
            vec![
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
        (
            "question ask",
            vec!["question", "ask", &id, "--question", "why"],
        ),
        (
            "question answer",
            vec!["question", "answer", &id, "QST-01", "--answer", "because"],
        ),
    ];
    for (command, args) in writes {
        let done = applied(talita(&dir, &CREATOR, &args), command);
        assert_eq!(done["journal"], json!([]), "{command} journaled: {done}");
        assert_eq!(
            fs::read(journal_path(&dir)).expect("the journal"),
            before,
            "{command} wrote the journal"
        );
    }

    // The item verbs other than promote are not journaled either.
    let item = applied(
        talita(
            &dir,
            &CREATOR,
            &["item", "create", "--kind", "idea", "--title", "small"],
        ),
        "item create",
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    applied(
        talita(&dir, &CREATOR, &["item", "shape", &item, "--kind", "bug"]),
        "item shape",
    );
    assert_eq!(
        fs::read(journal_path(&dir)).expect("the journal"),
        before,
        "an item verb wrote the journal"
    );
    applied(
        talita(
            &dir,
            &CREATOR,
            &["item", "dismiss", &item, "--reason", "not now"],
        ),
        "item dismiss",
    );
    assert_eq!(
        fs::read(journal_path(&dir)).expect("the journal"),
        before,
        "an item verb wrote the journal"
    );
}

#[test]
fn journal_show_filters_keeps_the_order_and_counts_the_unreadable() {
    let dir = project("show");
    let journal = Activity {
        events: vec![
            event(
                "2026-11-01T09:00:00Z",
                EventKind::Created,
                "TSK01",
                "human",
                None,
            ),
            event(
                "2026-11-02T09:00:00Z",
                EventKind::Groomed,
                "TSK01",
                "creator",
                None,
            ),
            event(
                "2026-11-03T09:00:00Z",
                EventKind::Created,
                "TSK02",
                "human",
                Some("from ITM01"),
            ),
            event(
                "2026-11-04T09:00:00Z",
                EventKind::Approved,
                "TSK01",
                "creator",
                None,
            ),
            event(
                "2026-11-05T09:00:00Z",
                EventKind::Completed,
                "TSK01",
                "reviewer",
                None,
            ),
        ],
        ..Activity::default()
    };
    store(&dir)
        .lock()
        .expect("the lock")
        .write_activity(&journal)
        .expect("the journal");

    let shown = applied(talita(&dir, &[], &["journal", "show"]), "journal show");
    assert_eq!(shown["data"]["unreadable"], 0, "{shown}");
    assert_eq!(shown["data"]["events"].as_array().unwrap().len(), 5);
    assert_eq!(shown["data"]["events"][0]["kind"], "created");
    assert_eq!(shown["data"]["events"][4]["kind"], "completed");

    let one = applied(
        talita(&dir, &[], &["journal", "show", "--task", "TSK02"]),
        "journal show",
    );
    assert_eq!(one["data"]["events"].as_array().unwrap().len(), 1, "{one}");
    assert_eq!(one["data"]["events"][0]["note"], "from ITM01");

    let two = applied(
        talita(
            &dir,
            &[],
            &[
                "journal",
                "show",
                "--kind",
                "groomed",
                "--kind",
                "completed",
            ],
        ),
        "journal show",
    );
    let kinds: Vec<&str> = two["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["groomed", "completed"], "{two}");

    let since = applied(
        talita(
            &dir,
            &[],
            &["journal", "show", "--since", "2026-11-03T09:00:00Z"],
        ),
        "journal show",
    );
    let kinds: Vec<&str> = since["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["created", "approved", "completed"], "{since}");

    let limited = applied(
        talita(&dir, &[], &["journal", "show", "--limit", "2"]),
        "journal show",
    );
    let kinds: Vec<&str> = limited["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["approved", "completed"], "{limited}");

    refused(
        talita(&dir, &[], &["journal", "show", "--kind", "teleported"]),
        "journal show",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, &[], &["journal", "show", "--since", "yesterday"]),
        "journal show",
        "input.invalid",
        2,
    );

    // One event this reader cannot read is counted, and the rest stay.
    let text = r#"{
      "events": [
        {"at": "2026-11-01T09:00Z", "kind": "created", "task": "TSK01", "by": "human"},
        {"at": "2026-11-02T09:00Z", "kind": "teleported", "task": "TSK01", "by": "human"},
        {"at": "2026-11-03T09:00Z", "kind": "completed", "task": "TSK01", "by": "reviewer"}
      ]
    }"#;
    fs::write(journal_path(&dir), text).expect("a journal with one bad event");
    let counted = applied(talita(&dir, &[], &["journal", "show"]), "journal show");
    assert_eq!(counted["data"]["events"].as_array().unwrap().len(), 2);
    assert_eq!(counted["data"]["unreadable"], 1, "{counted}");

    // A journal that does not parse is the damaged store.
    fs::write(journal_path(&dir), "{ not json").expect("a broken journal");
    refused(
        talita(&dir, &[], &["journal", "show"]),
        "journal show",
        "store.unreadable",
        7,
    );

    // A project with no journal has an empty one, not an error.
    let empty = project("empty");
    let shown = applied(talita(&empty, &[], &["journal", "show"]), "journal show");
    assert_eq!(shown["data"]["events"], json!([]), "{shown}");
    assert_eq!(shown["data"]["unreadable"], 0, "{shown}");
}

fn event(at: &str, kind: EventKind, task: &str, by: &str, note: Option<&str>) -> ActivityEvent {
    ActivityEvent {
        at: at.to_string(),
        kind,
        task: task.to_string(),
        by: by.to_string(),
        note: note.map(str::to_string),
    }
}

#[test]
fn an_unreadable_journal_refuses_every_write() {
    let dir = project("refusals");
    let id = create(&dir, "Untouched");
    let task_path = store(&dir).task_path(&read(&dir, &id));
    let task_before = fs::read(&task_path).expect("the task file");

    // A journal that holds an event this reader cannot read.
    let text = r#"{
      "events": [
        {"at": "2026-11-01T09:00Z", "kind": "created", "task": "TSK01", "by": "creator"},
        {"at": "2026-11-02T09:00Z", "kind": "teleported", "task": "TSK01", "by": "runner"}
      ]
    }"#;
    fs::write(journal_path(&dir), text).expect("an unreadable journal");
    let refusal = refused(
        talita(
            &dir,
            &CREATOR,
            &["task", "retitle", &id, "--title", "Must not land"],
        ),
        "task retitle",
        "store.unreadable",
        7,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cannot read"),
        "{refusal}"
    );
    assert_eq!(
        fs::read(&task_path).expect("the task file"),
        task_before,
        "the refused write changed the task"
    );
    assert_eq!(
        fs::read_to_string(journal_path(&dir)).expect("the journal"),
        text,
        "the refused write changed the journal"
    );

    // A journal that does not parse refuses the create door too, and no task
    // file appears.
    fs::write(journal_path(&dir), "{ not json").expect("a broken journal");
    refused(
        talita(&dir, &CREATOR, &["task", "create", "--title", "Ghost"]),
        "task create",
        "store.unreadable",
        7,
    );
    assert!(
        read(&dir, "TSK01").id == id,
        "the refused create wrote a task"
    );
    let files: Vec<String> = fs::read_dir(store(&dir).root())
        .expect("the work folder")
        .filter_map(|entry| {
            entry
                .ok()
                .and_then(|entry| entry.file_name().to_str().map(str::to_string))
        })
        .collect();
    assert_eq!(files.len(), 1, "a refused create wrote a file: {files:?}");
    assert_eq!(
        fs::read(journal_path(&dir)).expect("the journal"),
        b"{ not json",
        "the refused create wrote the journal"
    );
}
