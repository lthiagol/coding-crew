//! Claims (T8) through the real binary, against scratch projects under the
//! temp dir: `claim take|renew|release|show`, the takeover, the journal and
//! the refusals.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use coding_crew_core::{Status, Store, Task};
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
    let dir = std::env::temp_dir().join(format!("talita-claim-{name}-{}", std::process::id()));
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

/// A task an agent could claim now: created, groomed and approved.
fn ready(dir: &Path, title: &str) -> String {
    let id = create(dir, title);
    groom(dir, &id);
    approve(dir, &id);
    id
}

#[test]
fn a_runner_claims_starts_and_another_takes_over_after_the_lease() {
    let dir = project("takeover");
    let id = ready(&dir, "Own the claim");

    let taken = applied(
        talita(&dir, &RUNNER, &["claim", "take", &id, "--lease", "2s"]),
        "claim take",
    );
    assert_eq!(taken["data"]["claim"]["by"], "runner-1", "{taken}");
    assert_eq!(taken["data"]["claim"]["session"], "ses-runner");
    assert_eq!(taken["data"]["live"], true);
    assert_eq!(taken["journal"], json!(["claimed"]));
    let claim = read(&dir, &id).claim.expect("the claim");
    assert_eq!(claim.by, "runner-1");
    assert_eq!(claim.session.as_deref(), Some("ses-runner"));
    assert!(claim.lease_expires_at.starts_with("20"), "{claim:?}");

    let started = applied(talita(&dir, &RUNNER, &["task", "start", &id]), "task start");
    assert_eq!(started["data"]["status"], "executing", "{started}");

    // The lease runs out with the task executing; the runner is gone.
    std::thread::sleep(Duration::from_millis(2_500));

    let taken = applied(
        talita(&dir, &RUNNER2, &["claim", "take", &id]),
        "claim take",
    );
    assert_eq!(taken["data"]["claim"]["by"], "runner-2", "{taken}");
    assert_eq!(taken["data"]["claim"]["session"], "ses-runner2");
    assert_eq!(taken["data"]["live"], true);
    let task = read(&dir, &id);
    assert_eq!(task.status, Status::Executing, "a take changes no status");
    let claim = task.claim.expect("the claim");
    assert_eq!(
        (claim.by.as_str(), claim.session.as_deref()),
        ("runner-2", Some("ses-runner2"))
    );

    let shown = applied(
        talita(
            &dir,
            &[],
            &["journal", "show", "--task", &id, "--kind", "claimed"],
        ),
        "journal show",
    );
    let events = shown["data"]["events"].as_array().expect("the events");
    assert_eq!(events.len(), 2, "{shown}");
    assert_eq!(events[0]["note"], Value::Null);
    assert_eq!(events[1]["note"], "took over from runner-1");
    assert_eq!(events[1]["by"], "runner");
}

#[test]
fn a_take_refuses_a_live_claim_a_wrong_status_and_an_unclaimable_task() {
    let dir = project("refusals");

    // Another runner's live lease holds the task.
    let live = ready(&dir, "Held by another");
    applied(
        talita(&dir, &RUNNER, &["claim", "take", &live]),
        "claim take",
    );
    let refusal = refused(
        talita(&dir, &RUNNER2, &["claim", "take", &live]),
        "claim take",
        "claim.held",
        4,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("runner-1"),
        "{refusal}"
    );
    assert_eq!(read(&dir, &live).claim.expect("the claim").by, "runner-1");

    // A blocked overlay holds an approved task.
    let blocked = ready(&dir, "Blocked");
    applied(
        talita(
            &dir,
            &CREATOR,
            &["task", "block", &blocked, "--reason", "waiting on a key"],
        ),
        "task block",
    );
    let refusal = refused(
        talita(&dir, &RUNNER, &["claim", "take", &blocked]),
        "claim take",
        "claim.not_claimable",
        4,
    );
    assert_eq!(gates(&refusal), vec!["overlay.clear"]);
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("blocked"),
        "{refusal}"
    );

    // A dependency that is not done holds it too.
    let dependency = create(&dir, "Not done yet");
    let waiting = ready(&dir, "Waiting on it");
    applied(
        talita(
            &dir,
            &CREATOR,
            &["task", "depend", &waiting, "--on", &dependency],
        ),
        "task depend",
    );
    let refusal = refused(
        talita(&dir, &RUNNER, &["claim", "take", &waiting]),
        "claim take",
        "claim.not_claimable",
        4,
    );
    assert_eq!(gates(&refusal), vec!["dependencies.satisfied"]);
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&dependency),
        "{refusal}"
    );

    // A draft task has no claim to take.
    let draft = create(&dir, "Still a draft");
    refused(
        talita(&dir, &RUNNER, &["claim", "take", &draft]),
        "claim take",
        "status.wrong",
        4,
    );

    // Only a runner in a session claims.
    refused(
        talita(&dir, &REVIEWER, &["claim", "take", &draft]),
        "claim take",
        "role.wrong",
        5,
    );
    refused(
        talita(&dir, &HUMAN, &["claim", "take", &draft]),
        "claim take",
        "actor.not_agent",
        5,
    );

    // A lease that does not parse is a usage error, and writes nothing.
    let free = ready(&dir, "Free");
    let refusal = refused(
        talita(&dir, &RUNNER, &["claim", "take", &free, "--lease", "soon"]),
        "claim take",
        "input.invalid",
        2,
    );
    assert!(
        refusal["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--lease"),
        "{refusal}"
    );
    assert!(read(&dir, &free).claim.is_none());
}

#[test]
fn only_the_holder_renews_or_releases() {
    let dir = project("holder");
    let id = ready(&dir, "Held by the runner");

    let taken = applied(
        talita(&dir, &RUNNER, &["claim", "take", &id, "--lease", "1h"]),
        "claim take",
    );
    let first = taken["data"]["claim"]["lease_expires_at"]
        .as_str()
        .unwrap()
        .to_string();

    let renewed = applied(
        talita(&dir, &RUNNER, &["claim", "renew", &id, "--lease", "2h"]),
        "claim renew",
    );
    let second = renewed["data"]["claim"]["lease_expires_at"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(second > first, "{second} does not follow {first}");
    assert_eq!(renewed["data"]["claim"]["by"], "runner-1");
    assert_eq!(renewed["journal"], json!([]), "a renew journals nothing");

    refused(
        talita(&dir, &RUNNER2, &["claim", "renew", &id]),
        "claim renew",
        "claim.not_holder",
        5,
    );
    refused(
        talita(
            &dir,
            &RUNNER2,
            &["claim", "release", &id, "--reason", "not mine"],
        ),
        "claim release",
        "claim.not_holder",
        5,
    );
    refused(
        talita(&dir, &RUNNER, &["claim", "release", &id]),
        "claim release",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, &RUNNER, &["claim", "release", &id, "--reason", "   "]),
        "claim release",
        "input.invalid",
        2,
    );
    assert!(read(&dir, &id).claim.is_some(), "a refusal keeps the claim");

    let released = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "claim",
                "release",
                &id,
                "--reason",
                "the builder is stopping",
            ],
        ),
        "claim release",
    );
    assert_eq!(released["data"]["claim"], Value::Null, "{released}");
    assert_eq!(released["data"]["live"], false);
    assert!(read(&dir, &id).claim.is_none());

    // A released task is claimable again.
    let again = applied(
        talita(&dir, &RUNNER2, &["claim", "take", &id]),
        "claim take",
    );
    assert_eq!(again["data"]["claim"]["by"], "runner-2", "{again}");
}

#[test]
fn the_journal_records_claimed_and_released_but_not_renewed() {
    let dir = project("journal");
    let id = ready(&dir, "Recorded");

    let taken = applied(talita(&dir, &RUNNER, &["claim", "take", &id]), "claim take");
    assert_eq!(taken["journal"], json!(["claimed"]));
    let renewed = applied(
        talita(&dir, &RUNNER, &["claim", "renew", &id]),
        "claim renew",
    );
    assert_eq!(renewed["journal"], json!([]));
    let released = applied(
        talita(
            &dir,
            &RUNNER,
            &[
                "claim",
                "release",
                &id,
                "--reason",
                "the builder is stopping",
            ],
        ),
        "claim release",
    );
    assert_eq!(released["journal"], json!(["released"]));

    let shown = applied(
        talita(
            &dir,
            &[],
            &[
                "journal", "show", "--task", &id, "--kind", "claimed", "--kind", "released",
            ],
        ),
        "journal show",
    );
    let events = shown["data"]["events"].as_array().expect("the events");
    let kinds: Vec<&str> = events
        .iter()
        .map(|event| event["kind"].as_str().expect("a kind"))
        .collect();
    assert_eq!(kinds, ["claimed", "released"], "{shown}");
    assert_eq!(events[0]["task"], id);
    assert_eq!(events[0]["by"], "runner");
    assert_eq!(events[0]["note"], Value::Null);
    assert_eq!(events[1]["note"], "the builder is stopping");
}

#[test]
fn claim_show_reads_the_claim_and_its_liveness() {
    let dir = project("show");
    let id = ready(&dir, "Shown");

    let empty = applied(talita(&dir, &[], &["claim", "show", &id]), "claim show");
    assert_eq!(empty["data"]["claim"], Value::Null);
    assert_eq!(empty["data"]["live"], false);

    applied(
        talita(&dir, &RUNNER, &["claim", "take", &id, "--lease", "1s"]),
        "claim take",
    );
    let shown = applied(talita(&dir, &[], &["claim", "show", &id]), "claim show");
    assert_eq!(shown["data"]["claim"]["by"], "runner-1", "{shown}");
    assert_eq!(shown["data"]["claim"]["session"], "ses-runner");
    assert_eq!(shown["data"]["live"], true);

    std::thread::sleep(Duration::from_millis(1_200));
    let expired = applied(talita(&dir, &[], &["claim", "show", &id]), "claim show");
    assert_eq!(
        expired["data"]["claim"]["by"], "runner-1",
        "the expired claim is still recorded"
    );
    assert_eq!(expired["data"]["live"], false);

    applied(
        talita(
            &dir,
            &RUNNER,
            &["claim", "release", &id, "--reason", "stopping"],
        ),
        "claim release",
    );
    let gone = applied(talita(&dir, &[], &["claim", "show", &id]), "claim show");
    assert_eq!(gone["data"]["claim"], Value::Null);
    assert_eq!(gone["data"]["live"], false);
}

#[test]
fn a_non_holder_is_told_who_holds_the_claim() {
    let dir = project("who-holds");
    let id = ready(&dir, "Named holder");

    // Unclaimed: there is no holder to name, so the remedy is the take.
    let unclaimed = refused(
        talita(&dir, &RUNNER2, &["claim", "renew", &id]),
        "claim renew",
        "claim.not_holder",
        5,
    );
    assert_eq!(
        unclaimed["error"]["remedy"],
        format!("talita claim take {id}"),
        "{unclaimed}"
    );

    applied(talita(&dir, &RUNNER, &["claim", "take", &id]), "claim take");

    // Held: the refusal names the holder, and the remedy reads the claim, not
    // a take that the live lease would refuse.
    let renew = refused(
        talita(&dir, &RUNNER2, &["claim", "renew", &id]),
        "claim renew",
        "claim.not_holder",
        5,
    );
    assert!(
        renew["error"]["message"]
            .as_str()
            .unwrap()
            .contains("held by runner-1"),
        "{renew}"
    );
    assert_eq!(renew["error"]["remedy"], format!("talita claim show {id}"));

    let release = refused(
        talita(
            &dir,
            &RUNNER2,
            &["claim", "release", &id, "--reason", "not mine"],
        ),
        "claim release",
        "claim.not_holder",
        5,
    );
    assert!(
        release["error"]["message"]
            .as_str()
            .unwrap()
            .contains("held by runner-1"),
        "{release}"
    );
    assert!(read(&dir, &id).claim.is_some(), "a refusal keeps the claim");
}
