//! talita's skeleton through the real binary, against scratch projects under
//! the temp dir: the envelope, the exit codes, identity, `session` and
//! `project`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const ENVS: [&str; 4] = [
    "TALITA_PROJECT",
    "TALITA_ACTOR",
    "TALITA_ROLE",
    "TALITA_SESSION",
];

/// A fresh, canonical scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    fs::canonicalize(&dir).expect("canonical scratch dir")
}

/// Run talita in `cwd` with only the given `TALITA_*` env; returns the
/// envelope and the exit code. stdout must hold one JSON object, stderr
/// nothing.
fn talita(cwd: &Path, env: &[(&str, &str)], args: &[&str]) -> (Value, i32) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_talita"));
    command.current_dir(cwd).args(args);
    for name in ENVS {
        command.env_remove(name);
    }
    command.envs(env.iter().copied());
    let output = command.output().expect("talita runs");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(
        output.stderr.is_empty(),
        "{args:?}: stderr {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        stdout.lines().count(),
        1,
        "{args:?}: one line, got {stdout:?}"
    );
    let envelope: Value = serde_json::from_str(&stdout).expect("JSON envelope");
    (envelope, output.status.code().expect("an exit code"))
}

/// Assert a refusal: `ok: false`, the code, its exit, and the command.
fn refused(result: (Value, i32), command: &str, code: &str, exit: i32) -> Value {
    let (envelope, status) = result;
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["command"], command, "{envelope}");
    assert_eq!(envelope["error"]["code"], code, "{envelope}");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .is_some_and(|m| !m.is_empty())
    );
    assert_eq!(status, exit, "{envelope}");
    envelope
}

fn applied(result: (Value, i32), command: &str) -> Value {
    let (envelope, status) = result;
    assert_eq!(envelope["ok"], true, "{envelope}");
    assert_eq!(envelope["command"], command);
    assert!(envelope["wrote"].is_array() && envelope["journal"].is_array());
    assert_eq!(status, 0);
    envelope
}

const AGENT: [&str; 6] = [
    "--as",
    "creator-1",
    "--role",
    "creator",
    "--session",
    "ses-1",
];

fn init(dir: &Path) {
    let mut args = vec!["project", "init"];
    args.extend(AGENT);
    applied(talita(dir, &[], &args), "project init");
}

#[test]
fn session_show_resolves_flag_over_env() {
    let dir = scratch("show-flag");
    let env = [
        ("TALITA_ACTOR", "env-actor"),
        ("TALITA_ROLE", "reviewer"),
        ("TALITA_SESSION", "ses-env"),
    ];
    let data = applied(
        talita(
            &dir,
            &env,
            &["session", "show", "--as", "flag-actor", "--role", "builder"],
        ),
        "session show",
    )["data"]
        .clone();
    assert_eq!(data["actor"]["value"], "flag-actor");
    assert_eq!(data["actor"]["source"], "flag");
    assert_eq!(data["role"]["value"], "runner", "builder reads as runner");
    assert_eq!(data["role"]["source"], "flag");
    assert_eq!(data["kind"], "agent");
    assert_eq!(data["session"]["value"], "ses-env");
    assert_eq!(data["session"]["source"], "env");

    let data = applied(talita(&dir, &[], &["session", "show"]), "session show")["data"].clone();
    assert!(data["actor"].is_null() && data["role"].is_null() && data["session"].is_null());

    let data = applied(
        talita(&dir, &[("TALITA_ROLE", "human")], &["session", "show"]),
        "session show",
    )["data"]
        .clone();
    assert_eq!(data["kind"], "human");
}

#[test]
fn session_new_mints_an_id_and_its_env_line() {
    let dir = scratch("session-new");
    let data = applied(
        talita(
            &dir,
            &[],
            &["session", "new", "--as", "builder-1", "--role", "builder"],
        ),
        "session new",
    )["data"]
        .clone();
    let session = data["session"].as_str().unwrap();
    let random = session
        .strip_prefix("ses-runner-builder-1-")
        .expect(session);
    assert_eq!(random.len(), 12);
    assert!(
        random.chars().all(|c| matches!(c, 'a'..='z' | '2'..='7')),
        "{random}"
    );
    assert_eq!(
        data["env"],
        format!("TALITA_ACTOR=builder-1 TALITA_ROLE=runner TALITA_SESSION={session}")
    );
    let again = applied(
        talita(
            &dir,
            &[],
            &["session", "new", "--as", "builder-1", "--role", "builder"],
        ),
        "session new",
    );
    assert_ne!(
        again["data"]["session"], session,
        "each call mints a fresh id"
    );
    assert!(
        fs::read_dir(&dir).unwrap().next().is_none(),
        "writes nothing"
    );
}

#[test]
fn session_new_refusals() {
    let dir = scratch("session-new-refused");
    let envelope = refused(
        talita(&dir, &[], &["session", "new", "--role", "creator"]),
        "session new",
        "identity.missing",
        5,
    );
    assert!(envelope["error"]["remedy"].is_string());
    refused(
        talita(
            &dir,
            &[],
            &["session", "new", "--as", "x", "--role", "boss"],
        ),
        "session new",
        "input.invalid",
        2,
    );
    refused(
        talita(
            &dir,
            &[],
            &["session", "new", "--as", "a b", "--role", "creator"],
        ),
        "session new",
        "input.invalid",
        2,
    );
}

#[test]
fn usage_errors_print_the_envelope() {
    let dir = scratch("usage");
    refused(
        talita(&dir, &[], &["task", "frobnicate"]),
        "task frobnicate",
        "usage.invalid",
        2,
    );
    refused(
        talita(&dir, &[], &["session", "nope"]),
        "session nope",
        "usage.invalid",
        2,
    );
    refused(
        talita(&dir, &[], &["--as", "x", "project"]),
        "project",
        "usage.invalid",
        2,
    );
    refused(talita(&dir, &[], &[]), "", "usage.invalid", 2);
    // The old flat verbs are gone.
    for verb in ["create", "list", "show"] {
        refused(talita(&dir, &[], &[verb]), verb, "usage.invalid", 2);
    }
}

#[test]
fn project_init_creates_the_work_folder_with_no_identity() {
    let dir = scratch("init");
    // No --as, --role or --session: init writes no task and no journal, so
    // there is nothing to attribute (ADR 0015).
    let envelope = applied(talita(&dir, &[], &["project", "init"]), "project init");
    assert_eq!(envelope["wrote"], serde_json::json!(["work/"]));
    assert!(
        envelope["journal"].as_array().unwrap().is_empty(),
        "{envelope}"
    );
    assert_eq!(envelope["data"]["project"], dir.to_str().unwrap());
    assert!(dir.join("coding-crew/work").is_dir());
    assert_eq!(
        fs::read_dir(dir.join("coding-crew/work")).unwrap().count(),
        0
    );

    refused(
        talita(&dir, &[], &["project", "init"]),
        "project init",
        "project.exists",
        4,
    );

    let other = scratch("init-path");
    let target = other.join("sub");
    fs::create_dir(&target).unwrap();
    let path = target.to_str().unwrap();
    let envelope = applied(
        talita(&other, &[], &["project", "init", "--path", path]),
        "project init",
    );
    assert_eq!(envelope["data"]["project"], target.to_str().unwrap());
    assert!(target.join("coding-crew/work").is_dir());
    assert!(!other.join("coding-crew").exists());
}

#[test]
fn project_init_refusals() {
    let dir = scratch("init-refused");
    // A role word that is not one is still input, even though init needs none.
    refused(
        talita(
            &dir,
            &[("TALITA_ROLE", "chief")],
            &["project", "init", "--as", "x"],
        ),
        "project init",
        "input.invalid",
        2,
    );
    assert!(!dir.join("coding-crew").exists());

    // A file where the directory should be: an I/O failure is `internal`.
    let blocker = dir.join("blocker");
    fs::write(&blocker, "").unwrap();
    refused(
        talita(
            &dir,
            &[],
            &["project", "init", "--path", blocker.to_str().unwrap()],
        ),
        "project init",
        "internal",
        1,
    );
}

#[test]
fn project_is_found_by_flag_env_or_walk() {
    let dir = scratch("locate");
    init(&dir);
    let nested = dir.join("a/b");
    fs::create_dir_all(&nested).unwrap();
    let data = applied(talita(&nested, &[], &["project", "show"]), "project show")["data"].clone();
    assert_eq!(data["project"], dir.to_str().unwrap());

    let elsewhere = scratch("locate-elsewhere");
    let root = dir.to_str().unwrap();
    let data = applied(
        talita(
            &elsewhere,
            &[("TALITA_PROJECT", root)],
            &["project", "show"],
        ),
        "project show",
    )["data"]
        .clone();
    assert_eq!(data["project"], root);

    // The flag beats the env.
    let envelope = refused(
        talita(
            &elsewhere,
            &[("TALITA_PROJECT", root)],
            &["project", "show", "--project", elsewhere.to_str().unwrap()],
        ),
        "project show",
        "project.not_found",
        3,
    );
    assert!(
        envelope["error"]["remedy"]
            .as_str()
            .unwrap()
            .starts_with("talita project init")
    );
}

#[test]
fn project_show_and_check_refuse_without_a_project() {
    let dir = scratch("no-project");
    for verb in ["show", "check"] {
        let command = format!("project {verb}");
        refused(
            talita(&dir, &[], &["project", verb]),
            &command,
            "project.not_found",
            3,
        );
    }
}

fn write(dir: &Path, name: &str, json: &str) {
    fs::write(dir.join("coding-crew/work").join(name), json).unwrap();
}

#[test]
fn project_show_counts_tasks_and_items() {
    let dir = scratch("show");
    init(&dir);
    let data = applied(talita(&dir, &[], &["project", "show"]), "project show")["data"].clone();
    assert_eq!(data["tasks"]["total"], 0);
    assert_eq!(data["journal"]["exists"], false);
    assert_eq!(data["runtime"]["exists"], false);
    assert_eq!(data["schema_version"], 1);

    write(
        &dir,
        "TSK01-a.json",
        r#"{"id":"TSK01","title":"a","status":"done","kind":"bug"}"#,
    );
    write(&dir, "TSK02-b.json", r#"{"id":"TSK02","title":"b"}"#);
    write(
        &dir,
        "ITM01-c.json",
        r#"{"id":"ITM01","title":"c","kind":"idea"}"#,
    );
    let data = applied(talita(&dir, &[], &["project", "show"]), "project show")["data"].clone();
    assert_eq!(data["tasks"]["total"], 2);
    assert_eq!(data["tasks"]["by_status"]["done"], 1);
    assert_eq!(data["tasks"]["by_status"]["draft"], 1);
    assert_eq!(data["tasks"]["by_kind"]["bug"], 1);
    assert_eq!(data["items"]["total"], 1);
    assert_eq!(data["items"]["by_kind"]["idea"], 1);
}

#[test]
fn project_check_reports_and_writes_nothing() {
    let dir = scratch("check");
    init(&dir);
    let data = applied(talita(&dir, &[], &["project", "check"]), "project check")["data"].clone();
    assert_eq!(data["clean"], true);

    write(
        &dir,
        "TSK01-a.json",
        r#"{"id":"TSK01","title":"a","children":["TSK01.1","TSK09"],"depends_on":["TSK07"]}"#,
    );
    write(
        &dir,
        "TSK01.1-b.json",
        r#"{"id":"TSK01.1","title":"b","parent":"TSK01"}"#,
    );
    write(
        &dir,
        "TSK02-c.json",
        r#"{"id":"TSK02","title":"c","parent":"TSK01",
        "acceptance_criteria":[{"id":"ACCEPT-01","status":"passed",
        "evidence":[{"command":"false","exit_code":1,"summary":"","at":"2026-11-23T10:00:00Z","by":"runner"}]}]}"#,
    );
    write(&dir, "TSK03-d.json", r#"{"id":"TSK03","title":"d"}"#);
    write(
        &dir,
        "TSK03-d-old.json",
        r#"{"id":"TSK03","title":"d old"}"#,
    );
    write(&dir, "TSK04-e.json", "{ not json");
    let before: Vec<_> = fs::read_dir(dir.join("coding-crew/work"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    let data = applied(talita(&dir, &[], &["project", "check"]), "project check")["data"].clone();
    assert_eq!(data["clean"], false);
    assert_eq!(data["unreadable"][0]["file"], "work/TSK04-e.json");
    assert_eq!(data["duplicate_ids"], serde_json::json!(["TSK03"]));
    assert_eq!(
        data["dangling_depends_on"],
        serde_json::json!([{"task":"TSK01","on":"TSK07"}])
    );
    let family = data["family_mismatches"].to_string();
    assert!(family.contains("child TSK09 does not exist"), "{family}");
    assert!(family.contains("parent TSK01 does not list it"), "{family}");
    assert_eq!(
        data["criterion_status_mismatches"],
        serde_json::json!([{"task":"TSK02","criterion":"ACCEPT-01","stored":"passed","derived":"failed"}])
    );
    let after: Vec<_> = fs::read_dir(dir.join("coding-crew/work"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(before.len(), after.len());
}
