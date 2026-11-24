//! The runtime writer (T12) through the real binary, against scratch projects
//! under the temp dir: the `run`, `crew` and `queue` verbs, the derived fields
//! every write recomputes, the crew-row side effects, and the record the
//! mock's own state yields.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use coding_crew_core::{Evidence, Status, StepStatus, Store, Task};
use serde_json::{Value, json};

const ENVS: [&str; 6] = [
    "TALITA_PROJECT",
    "TALITA_ACTOR",
    "TALITA_ROLE",
    "TALITA_SESSION",
    "TALITA_MODEL",
    "TALITA_HARNESS",
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

const ORCH: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "orchestrator-1"),
    ("TALITA_ROLE", "orchestrator"),
    ("TALITA_SESSION", "ses-orchestrator"),
];

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-runtime-{name}-{}", std::process::id()));
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

/// The record as the file holds it.
fn record(dir: &Path) -> Value {
    let text = fs::read_to_string(store(dir).runtime_path()).expect("the runtime record");
    serde_json::from_str(&text).expect("a runtime record")
}

fn crew_row(dir: &Path, actor: &str) -> Value {
    record(dir)["crew"]
        .as_array()
        .expect("the crew")
        .iter()
        .find(|member| member["actor"] == actor)
        .cloned()
        .unwrap_or(Value::Null)
}

fn task_file(dir: &Path, id: &str) -> Task {
    store(dir).load(id).expect("the task")
}

/// A write of what talita has no verb for: a core operation or a core record,
/// saved under the project lock.
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

fn take(dir: &Path, id: &str) {
    take_as(dir, &RUNNER, id);
}

fn take_as(dir: &Path, env: &[(&str, &str)], id: &str) {
    applied(talita(dir, env, &["claim", "take", id]), "claim take");
}

/// The evidence `finish` needs.
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
fn self_round(dir: &Path, id: &str) {
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
    );
}

/// Write a round.json into the scratch project; returns its name.
fn write_round(dir: &Path, round: &Value) -> String {
    fs::write(
        dir.join("round.json"),
        serde_json::to_vec(round).expect("json"),
    )
    .expect("the round file");
    "round.json".to_string()
}

fn input_round(dir: &Path, id: &str, verdict: &str, findings: Value) {
    let round = json!({
        "kind": "external",
        "focus": "security",
        "verdict": verdict,
        "notes": "a look",
        "findings": findings,
    });
    let path = write_round(dir, &round);
    applied(
        talita(dir, &REVIEWER, &["round", "file", id, "--input", &path]),
        "round file",
    );
}

/// The record the mock state yields: the fixture's own work folder, driven by
/// the verbs the flow calls.
fn mock_state(dir: &Path) {
    applied(
        talita(
            dir,
            &ORCH,
            &[
                "run", "start", "--task", "TSK22", "--task", "TSK25", "--task", "TSK14", "--task",
                "TSK29", "--task", "TSK07",
            ],
        ),
        "run start",
    );
    for (actor, role, model) in [
        ("orchestrator-01", "orchestrator", "claude-opus-5-5"),
        ("runner-02", "builder", "claude-haiku-5-5"),
        ("reviewer-02", "reviewer", "claude-opus-5-5"),
    ] {
        applied(
            talita(
                dir,
                &ORCH,
                &["crew", "join", actor, "--role", role, "--model", model],
            ),
            "crew join",
        );
    }
    applied(
        talita(dir, &ORCH, &["crew", "assign", "runner-02", "TSK14"]),
        "crew assign",
    );
    applied(
        talita(dir, &ORCH, &["crew", "idle", "runner-02"]),
        "crew idle",
    );
    applied(
        talita(dir, &ORCH, &["crew", "assign", "reviewer-02", "TSK14"]),
        "crew assign",
    );
}

fn age_record(dir: &Path, by: Duration) {
    let path = store(dir).runtime_path();
    let at = SystemTime::now() - by;
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the record")
        .set_times(fs::FileTimes::new().set_modified(at))
        .expect("the times");
}

// ---- run

#[test]
fn run_start_writes_a_new_record_and_refuses_a_second() {
    let dir = project("start");
    let id = create(&dir, "A task");

    let done = applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    assert_eq!(done["wrote"], json!(["runtime.json"]), "{done}");
    let runtime = &done["data"]["runtime"];
    assert!(runtime["started_at"].is_string(), "{runtime}");
    assert_eq!(runtime["crew"], json!([]), "{runtime}");
    assert_eq!(runtime["stamp"], json!({"runs": true}), "{runtime}");
    assert_eq!(
        runtime["queue"],
        json!([{"id": id, "status": "draft", "claimable": false}]),
        "{runtime}"
    );
    assert_eq!(record(&dir), *runtime, "the file and the answer differ");

    refused(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
        "run.active",
        4,
    );

    // The unknown id is a fresh project's refusal: an active run refuses
    // first.
    let fresh = project("start-unknown");
    refused(
        talita(&fresh, &ORCH, &["run", "start", "--task", "TSK99"]),
        "run start",
        "task.not_found",
        3,
    );
}

#[test]
fn run_stop_stamps_and_idles_every_row_and_keeps_their_holds() {
    let dir = project("stop");
    refused(
        talita(&dir, &ORCH, &["run", "stop", "--why", "nothing yet"]),
        "run stop",
        "run.absent",
        3,
    );

    let id = create(&dir, "A task");
    applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew", "join", "runner-1", "--role", "builder", "--model", "M",
            ],
        ),
        "crew join",
    );
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "runner-1", &id]),
        "crew assign",
    );

    let done = applied(
        talita(&dir, &ORCH, &["run", "stop", "--why", "the queue is done"]),
        "run stop",
    );
    let runtime = &done["data"]["runtime"];
    assert_eq!(
        runtime["stamp"],
        json!({"runs": false, "why": "the queue is done"}),
        "{runtime}"
    );
    assert_eq!(runtime["crew"][0]["state"], "idle", "{runtime}");
    assert_eq!(runtime["crew"][0]["holds"], id, "{runtime}");
    assert_eq!(record(&dir), *runtime);

    // A stopped record starts again.
    applied(talita(&dir, &ORCH, &["run", "start"]), "run start");
}

#[test]
fn run_show_measures_staleness_against_the_longest_lease() {
    let dir = project("stale");
    refused(
        talita(&dir, &[], &["run", "show"]),
        "run show",
        "run.absent",
        3,
    );

    let id = create(&dir, "A task");
    groom(&dir, &id);
    approve(&dir, &id);
    applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew", "join", "runner-1", "--role", "builder", "--model", "M",
            ],
        ),
        "crew join",
    );
    take(&dir, &id);
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "runner-1", &id]),
        "crew assign",
    );

    let shown = applied(talita(&dir, &[], &["run", "show"]), "run show");
    assert_eq!(shown["data"]["stale"], false, "{shown}");
    assert_eq!(shown["data"]["runtime"]["crew"][0]["holds"], id, "{shown}");

    // The claim is two hours long: a record three hours old is stale.
    age_record(&dir, Duration::from_secs(3 * 3_600));
    let shown = applied(talita(&dir, &[], &["run", "show"]), "run show");
    assert_eq!(shown["data"]["stale"], true, "{shown}");

    // The next write renews it.
    applied(
        talita(&dir, &ORCH, &["crew", "idle", "runner-1"]),
        "crew idle",
    );
    let shown = applied(talita(&dir, &[], &["run", "show"]), "run show");
    assert_eq!(shown["data"]["stale"], false, "{shown}");

    // An expired lease still counts: a record older than its length is stale
    // even though nobody holds a live lease any more.
    edit(&dir, &id, |task| {
        let claim = task.claim.as_mut().expect("the claim");
        claim.at = "2026-11-01T09:00:00Z".to_string();
        claim.lease_expires_at = "2026-11-01T10:00:00Z".to_string();
    });
    age_record(&dir, Duration::from_secs(90 * 60));
    let shown = applied(talita(&dir, &[], &["run", "show"]), "run show");
    assert_eq!(shown["data"]["stale"], true, "{shown}");
}

// ---- crew

#[test]
fn the_crew_verbs_change_their_rows() {
    let dir = project("crew");
    applied(talita(&dir, &ORCH, &["run", "start"]), "run start");

    let joined = applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew",
                "join",
                "builder-1",
                "--role",
                "builder",
                "--model",
                "M1",
            ],
        ),
        "crew join",
    );
    assert_eq!(
        joined["data"]["crew"],
        json!([{"role": "builder", "model": "M1", "actor": "builder-1", "state": "idle"}]),
        "{joined}"
    );
    assert_eq!(joined["wrote"], json!(["runtime.json"]), "{joined}");
    refused(
        talita(
            &dir,
            &ORCH,
            &[
                "crew",
                "join",
                "builder-1",
                "--role",
                "reviewer",
                "--model",
                "M2",
            ],
        ),
        "crew join",
        "crew.exists",
        4,
    );
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew",
                "join",
                "reviewer-1",
                "--role",
                "reviewer",
                "--model",
                "M2",
            ],
        ),
        "crew join",
    );

    let id = create(&dir, "A task");
    let assigned = applied(
        talita(&dir, &ORCH, &["crew", "assign", "builder-1", &id]),
        "crew assign",
    );
    assert_eq!(assigned["data"]["crew"][0]["holds"], id, "{assigned}");
    assert!(
        assigned["data"]["crew"][0]["since"].is_string(),
        "{assigned}"
    );
    assert_eq!(
        assigned["data"]["crew"][0]["state"], "executing",
        "{assigned}"
    );
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "reviewer-1", &id]),
        "crew assign",
    );
    assert_eq!(crew_row(&dir, "reviewer-1")["state"], "reviewing");
    assert_eq!(crew_row(&dir, "reviewer-1")["holds"], id);

    let idle = applied(
        talita(&dir, &ORCH, &["crew", "idle", "builder-1"]),
        "crew idle",
    );
    assert_eq!(idle["data"]["crew"][0]["state"], "idle", "{idle}");
    assert_eq!(idle["data"]["crew"][0]["holds"], id, "{idle}");

    let stall = applied(
        talita(&dir, &ORCH, &["crew", "stall", "builder-1"]),
        "crew stall",
    );
    assert_eq!(stall["data"]["crew"][0]["state"], "blocked", "{stall}");
    assert_eq!(stall["data"]["crew"][0]["holds"], id, "{stall}");

    let left = applied(
        talita(&dir, &ORCH, &["crew", "leave", "reviewer-1"]),
        "crew leave",
    );
    assert_eq!(
        left["data"]["crew"].as_array().expect("the crew").len(),
        1,
        "{left}"
    );

    refused(
        talita(&dir, &ORCH, &["crew", "idle", "nobody"]),
        "crew idle",
        "crew.unknown",
        3,
    );
    refused(
        talita(&dir, &ORCH, &["crew", "stall", "nobody"]),
        "crew stall",
        "crew.unknown",
        3,
    );
    refused(
        talita(&dir, &ORCH, &["crew", "leave", "nobody"]),
        "crew leave",
        "crew.unknown",
        3,
    );
    refused(
        talita(&dir, &ORCH, &["crew", "assign", "nobody", &id]),
        "crew assign",
        "crew.unknown",
        3,
    );
    refused(
        talita(&dir, &ORCH, &["crew", "assign", "builder-1", "TSK99"]),
        "crew assign",
        "task.not_found",
        3,
    );

    // A project with no record has no crew.
    let fresh = project("crew-absent");
    refused(
        talita(
            &fresh,
            &ORCH,
            &[
                "crew",
                "join",
                "builder-1",
                "--role",
                "builder",
                "--model",
                "M1",
            ],
        ),
        "crew join",
        "run.absent",
        3,
    );
}

// ---- queue

#[test]
fn the_queue_verbs_add_insert_skip_and_drop() {
    let dir = project("queue");
    applied(talita(&dir, &ORCH, &["run", "start"]), "run start");
    let a = create(&dir, "First");
    let b = create(&dir, "Second");
    let c = create(&dir, "Third");

    let added = applied(talita(&dir, &ORCH, &["queue", "add", &a, &b]), "queue add");
    assert_eq!(
        ids(&added["data"]["queue"]),
        vec![a.clone(), b.clone()],
        "{added}"
    );
    assert_eq!(added["wrote"], json!(["runtime.json"]), "{added}");

    let again = applied(talita(&dir, &ORCH, &["queue", "add", &a]), "queue add");
    assert_eq!(
        ids(&again["data"]["queue"]),
        vec![a.clone(), b.clone()],
        "an id the queue holds was added twice: {again}"
    );

    let inserted = applied(
        talita(&dir, &ORCH, &["queue", "add", &c, "--before", &b]),
        "queue add",
    );
    assert_eq!(
        ids(&inserted["data"]["queue"]),
        vec![a.clone(), c.clone(), b.clone()],
        "{inserted}"
    );

    let dropped = applied(
        talita(&dir, &ORCH, &["queue", "drop", &a, &b]),
        "queue drop",
    );
    assert_eq!(ids(&dropped["data"]["queue"]), vec![c.clone()], "{dropped}");

    let noop = applied(
        talita(&dir, &ORCH, &["queue", "drop", "TSK99"]),
        "queue drop",
    );
    assert_eq!(ids(&noop["data"]["queue"]), vec![c.clone()], "{noop}");

    // The show is the record's queue; a groomed task is approved and
    // claimable on the next write.
    let d = create(&dir, "Fourth");
    groom(&dir, &d);
    approve(&dir, &d);
    applied(talita(&dir, &ORCH, &["queue", "add", &d]), "queue add");
    let shown = applied(talita(&dir, &[], &["queue", "show"]), "queue show");
    let queue = shown["data"]["queue"].as_array().expect("the queue");
    let fourth = queue.iter().find(|item| item["id"] == d).expect("the task");
    assert_eq!(fourth["status"], "approved", "{shown}");
    assert_eq!(fourth["claimable"], true, "{shown}");

    refused(
        talita(&dir, &ORCH, &["queue", "add", "TSK99"]),
        "queue add",
        "task.not_found",
        3,
    );
    refused(
        talita(&dir, &ORCH, &["queue", "add", &c, "--before", "TSK99"]),
        "queue add",
        "task.not_found",
        3,
    );

    // A project with no record has an empty queue and no add.
    let fresh = project("queue-absent");
    let shown = applied(talita(&fresh, &[], &["queue", "show"]), "queue show");
    assert_eq!(shown["data"]["queue"], json!([]), "{shown}");
    refused(
        talita(&fresh, &ORCH, &["queue", "add", "TSK99"]),
        "queue add",
        "run.absent",
        3,
    );
}

fn ids(queue: &Value) -> Vec<String> {
    queue
        .as_array()
        .expect("the queue")
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect()
}

// ---- the crew-row side effects

#[test]
fn the_task_verbs_make_their_crew_rows_certain() {
    let dir = project("effects");
    let id = create(&dir, "Walk the review");
    groom(&dir, &id);
    approve(&dir, &id);
    applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew", "join", "runner-1", "--role", "builder", "--model", "M1",
            ],
        ),
        "crew join",
    );
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew",
                "join",
                "reviewer-1",
                "--role",
                "reviewer",
                "--model",
                "M2",
            ],
        ),
        "crew join",
    );

    // `claim take`: the actor's row holds the task, since now, executing.
    take(&dir, &id);
    let row = crew_row(&dir, "runner-1");
    assert_eq!(row["holds"], id, "{row}");
    assert!(row["since"].is_string(), "{row}");
    assert_eq!(row["state"], "executing", "{row}");

    // `task start` leaves the row as the claim left it.
    applied(talita(&dir, &RUNNER, &["task", "start", &id]), "task start");
    assert_eq!(crew_row(&dir, "runner-1")["state"], "executing");
    assert_eq!(crew_row(&dir, "runner-1")["holds"], id);

    // A self round changes no row.
    settle(&dir, &id);
    self_round(&dir, &id);
    assert_eq!(crew_row(&dir, "runner-1")["state"], "executing");
    assert_eq!(crew_row(&dir, "runner-1")["holds"], id);

    // `task finish`: idle, keeps the hold.
    applied(
        talita(&dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
    );
    let row = crew_row(&dir, "runner-1");
    assert_eq!(row["state"], "idle", "{row}");
    assert_eq!(row["holds"], id, "{row}");

    // The reviewer holds it while it reviews: external, round 1 of 5.
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "reviewer-1", &id]),
        "crew assign",
    );
    assert_eq!(crew_row(&dir, "reviewer-1")["state"], "reviewing");
    let runtime = record(&dir);
    assert_eq!(runtime["round"]["stage"], "external", "{runtime}");
    assert_eq!(runtime["round"]["number"], 1, "{runtime}");
    assert_eq!(runtime["budget"], json!({"spent": 1, "total": 5}));

    // `round file` (external): idle, keeps the hold.
    input_round(
        &dir,
        &id,
        "changes-needed",
        json!([{"ref": "D1", "severity": "high", "description": "a defect"}]),
    );
    let row = crew_row(&dir, "reviewer-1");
    assert_eq!(row["state"], "idle", "{row}");
    assert_eq!(row["holds"], id, "{row}");

    applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );

    // A second builder remediates; `task re-review` idles its row.
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew", "join", "runner-2", "--role", "builder", "--model", "M3",
            ],
        ),
        "crew join",
    );
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "runner-2", &id]),
        "crew assign",
    );
    assert_eq!(crew_row(&dir, "runner-2")["state"], "executing");
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
    applied(
        talita(&dir, &RUNNER2, &["task", "re-review", &id]),
        "task re-review",
    );
    let row = crew_row(&dir, "runner-2");
    assert_eq!(row["state"], "idle", "{row}");
    assert_eq!(row["holds"], id, "{row}");

    // The second external round passes, and every row holding the task is
    // released and idle.
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "reviewer-1", &id]),
        "crew assign",
    );
    applied(
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
    applied(
        talita(&dir, &REVIEWER, &["task", "pass-review", &id]),
        "task pass-review",
    );
    let runtime = record(&dir);
    for member in runtime["crew"].as_array().expect("the crew") {
        assert!(member.get("holds").is_none(), "{runtime}");
        assert!(member.get("since").is_none(), "{runtime}");
        assert_eq!(member["state"], "idle", "{runtime}");
    }
    assert_eq!(runtime["queue"][0]["status"], "done", "{runtime}");
}

#[test]
fn claim_release_and_task_cancel_release_the_rows() {
    let dir = project("release");
    applied(talita(&dir, &ORCH, &["run", "start"]), "run start");
    for (actor, role) in [("runner-1", "builder"), ("reviewer-1", "reviewer")] {
        applied(
            talita(
                &dir,
                &ORCH,
                &["crew", "join", actor, "--role", role, "--model", "M"],
            ),
            "crew join",
        );
    }

    // `claim release`: every row holding the task is released.
    let first = create(&dir, "Released hold");
    groom(&dir, &first);
    approve(&dir, &first);
    take(&dir, &first);
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "reviewer-1", &first]),
        "crew assign",
    );
    assert_eq!(crew_row(&dir, "reviewer-1")["holds"], first);
    applied(
        talita(
            &dir,
            &RUNNER,
            &["claim", "release", &first, "--reason", "the plan changed"],
        ),
        "claim release",
    );
    for actor in ["runner-1", "reviewer-1"] {
        let row = crew_row(&dir, actor);
        assert!(row.get("holds").is_none(), "{actor}: {row}");
        assert!(row.get("since").is_none(), "{actor}: {row}");
    }

    // `task cancel`: the same release.
    let second = create(&dir, "Cancelled hold");
    groom(&dir, &second);
    approve(&dir, &second);
    take(&dir, &second);
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "reviewer-1", &second]),
        "crew assign",
    );
    applied(
        talita(
            &dir,
            &HUMAN,
            &["task", "cancel", &second, "--reason", "the plan is gone"],
        ),
        "task cancel",
    );
    for actor in ["runner-1", "reviewer-1"] {
        let row = crew_row(&dir, actor);
        assert!(row.get("holds").is_none(), "{actor}: {row}");
        assert!(row.get("since").is_none(), "{actor}: {row}");
    }
}

#[test]
fn a_task_verb_by_an_actor_with_no_row_changes_nothing() {
    let dir = project("no-row");
    applied(talita(&dir, &ORCH, &["run", "start"]), "run start");
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew", "join", "runner-1", "--role", "builder", "--model", "M",
            ],
        ),
        "crew join",
    );
    let theirs = create(&dir, "Theirs");
    groom(&dir, &theirs);
    approve(&dir, &theirs);
    let mine = create(&dir, "Mine");
    groom(&dir, &mine);
    approve(&dir, &mine);

    take(&dir, &theirs);
    let before = record(&dir);
    // runner-2 has no row: the take lands on the task, and no row.
    take_as(&dir, &RUNNER2, &mine);
    let after = record(&dir);
    assert_eq!(after["crew"], before["crew"], "{after}");
    assert_eq!(
        task_file(&dir, &mine).claim.expect("the claim").by,
        "runner-2"
    );
}

// ---- the derived fields

#[test]
fn the_derived_fields_follow_the_task_files() {
    let dir = project("derived");
    let id = create(&dir, "Measured");
    groom(&dir, &id);
    approve(&dir, &id);
    applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    for (actor, role) in [("runner-1", "builder"), ("reviewer-1", "reviewer")] {
        applied(
            talita(
                &dir,
                &ORCH,
                &["crew", "join", actor, "--role", role, "--model", "M"],
            ),
            "crew join",
        );
    }

    // The queue's status and claimable flag are the task's, recomputed on
    // every write.
    assert_eq!(record(&dir)["queue"][0]["status"], "approved");
    assert_eq!(record(&dir)["queue"][0]["claimable"], true);
    take(&dir, &id);
    assert_eq!(record(&dir)["queue"][0]["claimable"], false);
    applied(talita(&dir, &RUNNER, &["task", "start", &id]), "task start");
    assert_eq!(record(&dir)["queue"][0]["status"], "executing");

    settle(&dir, &id);
    self_round(&dir, &id);
    applied(
        talita(&dir, &RUNNER, &["task", "finish", &id]),
        "task finish",
    );
    let runtime = record(&dir);
    assert_eq!(runtime["queue"][0]["status"], "code-review", "{runtime}");
    // The builder holds it and is idle: a self stage, no budget spent.
    assert_eq!(
        runtime["round"],
        json!({"task": id, "stage": "self", "number": 0}),
        "{runtime}"
    );
    assert_eq!(runtime["budget"], json!({"spent": 0, "total": 5}));
    assert_eq!(runtime["findings"], 0);
    assert_eq!(
        runtime["severities"],
        json!({"high": 0, "medium": 0, "low": 0})
    );

    // A reviewer row reviewing it makes the stage external, round 1.
    applied(
        talita(&dir, &ORCH, &["crew", "assign", "reviewer-1", &id]),
        "crew assign",
    );
    let runtime = record(&dir);
    assert_eq!(runtime["round"]["stage"], "external", "{runtime}");
    assert_eq!(runtime["round"]["number"], 1, "{runtime}");

    // The filing idles the row: the stage is self again, and the filed round
    // is the budget's spend.
    input_round(
        &dir,
        &id,
        "changes-needed",
        json!([
            {"ref": "D1", "severity": "high", "description": "a defect"},
            {"ref": "D2", "severity": "low", "description": "a nit"},
        ]),
    );
    let runtime = record(&dir);
    assert_eq!(runtime["round"]["stage"], "self", "{runtime}");
    assert_eq!(runtime["round"]["number"], 0, "{runtime}");
    assert_eq!(runtime["budget"], json!({"spent": 1, "total": 5}));
    assert_eq!(runtime["findings"], 2, "{runtime}");
    assert_eq!(
        runtime["severities"],
        json!({"high": 1, "medium": 0, "low": 1}),
        "{runtime}"
    );

    // Remediating is external: the next round, after the filed one.
    applied(
        talita(&dir, &REVIEWER, &["task", "request-changes", &id]),
        "task request-changes",
    );
    let runtime = record(&dir);
    assert_eq!(
        runtime["round"],
        json!({
            "task": id,
            "stage": "external",
            "number": 2,
            "earlier": [{"number": 1, "raised": 2}],
        }),
        "{runtime}"
    );
    assert_eq!(runtime["budget"], json!({"spent": 2, "total": 5}));

    // Closing the findings empties the count and the breakdown.
    applied(
        talita(
            &dir,
            &RUNNER2,
            &[
                "finding",
                "dismiss",
                &id,
                "FIND-02",
                "--reason",
                "it is the house style",
            ],
        ),
        "finding dismiss",
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
    let runtime = record(&dir);
    assert_eq!(runtime["findings"], 0, "{runtime}");
    assert_eq!(
        runtime["severities"],
        json!({"high": 0, "medium": 0, "low": 0}),
        "{runtime}"
    );
}

#[test]
fn every_task_write_refreshes_the_record_and_no_record_stays_missing() {
    // A project without a record gets none.
    let none = project("no-record");
    let bare = create(&none, "No run");
    assert!(
        !store(&none).runtime_path().exists(),
        "a task write created a record"
    );
    applied(
        talita(
            &none,
            &CREATOR,
            &["scope", "include", &bare, "--item", "this"],
        ),
        "scope include",
    );
    assert!(!store(&none).runtime_path().exists());

    // A project with one keeps it current through every task write.
    let dir = project("refresh");
    let id = create(&dir, "Current");
    groom(&dir, &id);
    assert_eq!(task_file(&dir, &id).status, Status::Groomed);
    applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    assert_eq!(record(&dir)["queue"][0]["status"], "groomed");
    applied(
        talita(
            &dir,
            &CREATOR,
            &[
                "step",
                "add",
                &id,
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
    let approved = applied(
        talita(&dir, &CREATOR, &["task", "approve", &id]),
        "task approve",
    );
    assert_eq!(
        approved["wrote"],
        json!([
            format!("work/{id}-current.json"),
            "activity.json",
            "runtime.json"
        ]),
        "{approved}"
    );
    let runtime = record(&dir);
    assert_eq!(runtime["queue"][0]["status"], "approved", "{runtime}");
    assert_eq!(runtime["queue"][0]["claimable"], true, "{runtime}");

    // The create door and the item verbs refresh it too.
    let created = applied(
        talita(&dir, &CREATOR, &["task", "create", "--title", "Another"]),
        "task create",
    );
    assert_eq!(
        created["wrote"].as_array().expect("the writes").last(),
        Some(&json!("runtime.json")),
        "{created}"
    );
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
                "a small thing",
            ],
        ),
        "item create",
    );
    assert_eq!(
        item["wrote"].as_array().expect("the writes").last(),
        Some(&json!("runtime.json")),
        "{item}"
    );
}

#[test]
fn run_refresh_rebuilds_the_derived_fields_and_keeps_the_written_ones() {
    let dir = project("refresh-verb");
    let id = create(&dir, "Moved behind the record");
    applied(
        talita(&dir, &ORCH, &["run", "start", "--task", &id]),
        "run start",
    );
    applied(
        talita(
            &dir,
            &ORCH,
            &[
                "crew", "join", "runner-1", "--role", "builder", "--model", "M",
            ],
        ),
        "crew join",
    );
    let before = record(&dir);

    // A write outside talita leaves the record behind; the refresh repairs
    // it, and leaves the written fields alone.
    edit(&dir, &id, |task| task.status = Status::Approved);
    let done = applied(talita(&dir, &ORCH, &["run", "refresh"]), "run refresh");
    assert_eq!(done["wrote"], json!(["runtime.json"]), "{done}");
    let runtime = &done["data"]["runtime"];
    assert_eq!(runtime["queue"][0]["status"], "approved", "{runtime}");
    assert_eq!(runtime["queue"][0]["claimable"], true, "{runtime}");
    assert_eq!(runtime["started_at"], before["started_at"], "{runtime}");
    assert_eq!(runtime["crew"], before["crew"], "{runtime}");
    assert_eq!(runtime["stamp"], before["stamp"], "{runtime}");
    assert_eq!(record(&dir), *runtime);
}

#[test]
fn an_unreadable_record_refuses_the_write() {
    let dir = project("unreadable");
    let id = create(&dir, "Untouched");
    let path = store(&dir).runtime_path();
    fs::write(&path, "{ not json").expect("a broken record");

    // The write refuses after the task is on disk, the window ADR 0018
    // accepts for the journal.
    refused(
        talita(
            &dir,
            &CREATOR,
            &["task", "retitle", &id, "--title", "Changed"],
        ),
        "task retitle",
        "store.unreadable",
        7,
    );
    assert_eq!(task_file(&dir, &id).title, "Changed");
    assert_eq!(
        fs::read_to_string(&path).expect("the record"),
        "{ not json",
        "the refused write overwrote the record"
    );

    for (args, command) in [
        (vec!["run", "refresh"], "run refresh"),
        (
            vec![
                "crew",
                "join",
                "builder-1",
                "--role",
                "builder",
                "--model",
                "M",
            ],
            "crew join",
        ),
        (vec!["run", "start"], "run start"),
        (vec!["queue", "show"], "queue show"),
        (vec!["run", "show"], "run show"),
    ] {
        refused(talita(&dir, &ORCH, &args), command, "store.unreadable", 7);
    }
    assert_eq!(
        fs::read_to_string(&path).expect("the record"),
        "{ not json",
        "a refused call overwrote the record"
    );
}

#[test]
fn the_flow_verbs_are_the_orchestrators() {
    let dir = project("roles");
    refused(
        talita(&dir, &RUNNER, &["run", "start"]),
        "run start",
        "role.wrong",
        5,
    );
    refused(
        talita(&dir, &HUMAN, &["run", "start"]),
        "run start",
        "actor.not_agent",
        5,
    );
    refused(
        talita(
            &dir,
            &RUNNER,
            &[
                "crew",
                "join",
                "builder-1",
                "--role",
                "builder",
                "--model",
                "M",
            ],
        ),
        "crew join",
        "role.wrong",
        5,
    );

    // The reads need no identity.
    applied(talita(&dir, &[], &["queue", "show"]), "queue show");
    refused(
        talita(&dir, &[], &["run", "show"]),
        "run show",
        "run.absent",
        3,
    );
}

#[test]
fn crew_join_resolves_the_callers_role_from_the_flag_before_the_noun() {
    let dir = project("crew-prenoun");
    applied(talita(&dir, &ORCH, &["run", "start"]), "run start");

    // The crew's role is the subcommand's flag; the caller's role is the
    // global flag before the noun. Flags beat env.
    let joined = applied(
        talita(
            &dir,
            &[("TALITA_ROLE", "builder")],
            &[
                "--as",
                "orchestrator-9",
                "--role",
                "orchestrator",
                "--session",
                "ses-orchestrator-9",
                "crew",
                "join",
                "builder-1",
                "--role",
                "builder",
                "--model",
                "M1",
            ],
        ),
        "crew join",
    );
    assert_eq!(
        joined["data"]["crew"],
        json!([{"role": "builder", "model": "M1", "actor": "builder-1", "state": "idle"}]),
        "{joined}"
    );

    // The row's flag alone is not the caller's: the env still decides.
    applied(
        talita(
            &dir,
            &[("TALITA_ROLE", "orchestrator")],
            &[
                "--as",
                "orchestrator-8",
                "--session",
                "ses-orchestrator-8",
                "crew",
                "join",
                "builder-2",
                "--role",
                "builder",
                "--model",
                "M2",
            ],
        ),
        "crew join",
    );

    refused(
        talita(
            &dir,
            &[("TALITA_ROLE", "orchestrator")],
            &[
                "--as",
                "runner-9",
                "--role",
                "builder",
                "--session",
                "ses-runner-9",
                "crew",
                "join",
                "builder-3",
                "--role",
                "builder",
                "--model",
                "M3",
            ],
        ),
        "crew join",
        "role.wrong",
        5,
    );
}

// ---- the mock

#[test]
fn the_mock_states_record_is_the_mocks_record() {
    let dir = project("mock");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew");
    copy_work(&fixture.join("work"), store(&dir).root());
    mock_state(&dir);

    let written = record(&dir);
    let mut mock: Value =
        serde_json::from_str(&fs::read_to_string(fixture.join("runtime.json")).expect("the mock"))
            .expect("a runtime record");
    // The run's start and the rows' since are this run's instants; every
    // other field is the mock's.
    let mut written = written;
    for value in [&mut written, &mut mock] {
        value["started_at"] = json!("");
        for member in value["crew"].as_array_mut().expect("the crew") {
            member.as_object_mut().expect("a row").remove("since");
        }
    }
    assert_eq!(written, mock);

    // The crew the mock names, field for field.
    let runtime = record(&dir);
    let crew = runtime["crew"].as_array().expect("the crew");
    let summary: Vec<(String, String, String, Option<String>, String)> = crew
        .iter()
        .map(|member| {
            (
                member["role"].as_str().unwrap().to_string(),
                member["model"].as_str().unwrap().to_string(),
                member["actor"].as_str().unwrap().to_string(),
                member["holds"].as_str().map(str::to_string),
                member["state"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            (
                "orchestrator".to_string(),
                "claude-opus-5-5".to_string(),
                "orchestrator-01".to_string(),
                None,
                "idle".to_string(),
            ),
            (
                "builder".to_string(),
                "claude-haiku-5-5".to_string(),
                "runner-02".to_string(),
                Some("TSK14".to_string()),
                "idle".to_string(),
            ),
            (
                "reviewer".to_string(),
                "claude-opus-5-5".to_string(),
                "reviewer-02".to_string(),
                Some("TSK14".to_string()),
                "reviewing".to_string(),
            ),
        ]
    );
    assert_eq!(
        runtime["queue"]
            .as_array()
            .expect("the queue")
            .iter()
            .map(|item| (
                item["id"].as_str().unwrap(),
                item["status"].as_str().unwrap(),
                item["claimable"].as_bool().unwrap()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("TSK22", "done", false),
            ("TSK25", "done", false),
            ("TSK14", "code-review", false),
            ("TSK29", "approved", true),
            ("TSK07", "approved", false),
        ]
    );
    assert_eq!(
        runtime["round"],
        json!({
            "task": "TSK14",
            "stage": "external",
            "number": 2,
            "earlier": [{"number": 1, "raised": 3}],
        }),
        "{runtime}"
    );
    assert_eq!(runtime["budget"], json!({"spent": 2, "total": 5}));
    assert_eq!(runtime["findings"], 1);
    assert_eq!(
        runtime["severities"],
        json!({"high": 1, "medium": 0, "low": 0})
    );
    assert_eq!(runtime["stamp"], json!({"runs": true}));
}

fn copy_work(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the work folder");
    for entry in fs::read_dir(from).expect("the fixture's work folder") {
        let entry = entry.expect("an entry");
        if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
            fs::copy(entry.path(), to.join(entry.file_name())).expect("a work file");
        }
    }
}
