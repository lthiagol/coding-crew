//! Derived lane and review state (TM § 12) through the real binary, against
//! scratch projects under the temp dir: `task show`'s `derived.lane` and
//! `derived.review_state`, and `task list --lane` and `--needs-you`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

const ENTERED: &str = "2026-11-20T10:00:00Z";
const BEFORE: &str = "2026-11-19T10:00:00Z";
const AFTER: &str = "2026-11-21T10:00:00Z";

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-derived-{name}-{}", std::process::id()));
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

/// Put a scratch task into a state talita's verbs do not reach directly.
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

/// Put `id` in `status`, entered at `since`.
fn enter(dir: &Path, id: &str, status: &str, since: &str) {
    set(dir, id, |task| {
        task["status"] = json!(status);
        task["status_since"] = json!(since);
    });
}

fn round(n: u32, kind: &str, at: &str) -> Value {
    json!({
        "n": n,
        "kind": kind,
        "focus": "spec-compliance",
        "reviewer": "reviewer-1",
        "verdict": "changes-needed",
        "at": at,
        "notes": "",
        "findings": [],
    })
}

fn finding(status: &str) -> Value {
    json!({
        "id": "FIND-01",
        "severity": "high",
        "status": status,
        "description": "the parser drops a key",
        "created": ENTERED,
        "resolved": "",
        "resolution": "",
        "comments": [],
    })
}

/// Put `id`'s rounds and findings in place.
fn review(dir: &Path, id: &str, rounds: Vec<Value>, findings: Vec<Value>) {
    set(dir, id, |task| {
        task["review"]["rounds"] = json!(rounds);
        task["review"]["findings"] = json!(findings);
    });
}

/// The derived fields `task show` gives for `id`.
fn derived(dir: &Path, id: &str) -> Value {
    applied(talita(dir, &[], &["task", "show", id]), "task show")["data"]["derived"].clone()
}

/// The ids `task list` returns for the given filter, in order.
fn listed(dir: &Path, filter: &[&str]) -> Vec<String> {
    let mut args = vec!["task", "list"];
    args.extend_from_slice(filter);
    applied(talita(dir, &[], &args), "task list")["data"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|row| row["id"].as_str().expect("an id").to_string())
        .collect()
}

#[test]
fn show_derives_the_lane_and_the_review_state() {
    let dir = project("show");
    let id = create(&dir, "Derived");

    // A draft and a done task are in neither lane.
    let draft = derived(&dir, &id);
    assert!(draft["lane"].is_null(), "{draft}");
    assert!(draft["review_state"].is_null(), "{draft}");

    enter(&dir, &id, "groomed", ENTERED);
    let groomed = derived(&dir, &id);
    assert_eq!(groomed["lane"], "human", "{groomed}");
    assert!(groomed["review_state"].is_null(), "{groomed}");

    enter(&dir, &id, "approved", ENTERED);
    assert_eq!(derived(&dir, &id)["lane"], "agent");

    enter(&dir, &id, "executing", ENTERED);
    assert_eq!(derived(&dir, &id)["lane"], "agent");

    // In review with no external round since the entry: pending.
    enter(&dir, &id, "code-review", ENTERED);
    let pending = derived(&dir, &id);
    assert_eq!(pending["lane"], "human", "{pending}");
    assert_eq!(pending["review_state"], "pending-review", "{pending}");

    // A self round is a pre-screen, not a verdict: still pending.
    review(&dir, &id, vec![round(1, "self", AFTER)], vec![]);
    assert_eq!(derived(&dir, &id)["review_state"], "pending-review");

    // An external round filed before the entry is stale: still pending.
    review(&dir, &id, vec![round(1, "external", BEFORE)], vec![]);
    assert_eq!(derived(&dir, &id)["review_state"], "pending-review");

    // Filed since the entry, nothing open: clean.
    review(&dir, &id, vec![round(1, "external", AFTER)], vec![]);
    assert_eq!(derived(&dir, &id)["review_state"], "clean");

    // An open finding wins, whatever the status.
    review(
        &dir,
        &id,
        vec![round(1, "external", AFTER)],
        vec![finding("open")],
    );
    let open = derived(&dir, &id);
    assert_eq!(open["review_state"], "open-findings", "{open}");
    assert_eq!(open["lane"], "human");

    enter(&dir, &id, "remediating", AFTER);
    let remediating = derived(&dir, &id);
    assert_eq!(remediating["lane"], "agent", "{remediating}");
    assert_eq!(
        remediating["review_state"], "open-findings",
        "{remediating}"
    );

    // A fixed finding is not open. Outside code-review a filed round is clean.
    review(
        &dir,
        &id,
        vec![round(1, "external", BEFORE)],
        vec![finding("fixed")],
    );
    enter(&dir, &id, "done", AFTER);
    let done = derived(&dir, &id);
    assert!(done["lane"].is_null(), "{done}");
    assert_eq!(done["review_state"], "clean", "{done}");
}

#[test]
fn a_task_outside_review_has_no_review_state() {
    let dir = project("no-review");
    let id = create(&dir, "Not in review");
    enter(&dir, &id, "executing", ENTERED);
    let shown = derived(&dir, &id);
    assert!(shown["review_state"].is_null(), "{shown}");
    assert_eq!(shown["lane"], "agent", "{shown}");
}

#[test]
fn the_list_filters_on_the_lane_and_on_needs_you() {
    let dir = project("lane-filter");
    for title in [
        "Draft",
        "Groomed",
        "Blocked",
        "Approved",
        "Executing",
        "Review",
        "Escalated",
        "Done",
    ] {
        create(&dir, title);
    }
    enter(&dir, "TSK02", "groomed", ENTERED);
    enter(&dir, "TSK03", "groomed", ENTERED);
    set(&dir, "TSK03", |task| {
        task["overlays"]["blocked"] = json!({
            "reason": "waiting on a key",
            "by": "luiz",
            "at": AFTER,
        });
    });
    enter(&dir, "TSK04", "approved", ENTERED);
    enter(&dir, "TSK05", "executing", ENTERED);
    enter(&dir, "TSK06", "code-review", ENTERED);
    enter(&dir, "TSK07", "needs-human", ENTERED);
    enter(&dir, "TSK08", "done", ENTERED);

    // The human lane holds the overlaid task too: an overlay parks a task
    // out of Needs you, not out of its lane.
    assert_eq!(
        listed(&dir, &["--lane", "human"]),
        ["TSK02", "TSK03", "TSK06", "TSK07"]
    );
    assert_eq!(listed(&dir, &["--lane", "agent"]), ["TSK04", "TSK05"]);

    // Needs you is the human lane minus the overlaid tasks.
    assert_eq!(listed(&dir, &["--needs-you"]), ["TSK02", "TSK06", "TSK07"]);
    assert_eq!(
        listed(&dir, &["--lane", "human", "--needs-you"]),
        ["TSK02", "TSK06", "TSK07"]
    );
    assert!(listed(&dir, &["--lane", "agent", "--needs-you"]).is_empty());

    // The filters combine with the others.
    assert_eq!(
        listed(&dir, &["--lane", "human", "--overlay", "blocked"]),
        ["TSK03"]
    );

    let unknown = refused(
        talita(&dir, &[], &["task", "list", "--lane", "sideways"]),
        "task list",
        "input.invalid",
        2,
    );
    let remedy = unknown["error"]["remedy"].as_str().expect("a remedy");
    assert!(remedy.contains("human|agent"), "{remedy}");
}

#[test]
fn the_lane_filter_is_read_only_and_needs_no_identity() {
    let dir = project("read-only");
    let id = create(&dir, "Read");
    enter(&dir, &id, "groomed", ENTERED);
    let before = fs::read_to_string(file_of(&dir, &id)).unwrap();
    assert_eq!(listed(&dir, &["--needs-you"]), [id.as_str()]);
    assert_eq!(fs::read_to_string(file_of(&dir, &id)).unwrap(), before);
}
