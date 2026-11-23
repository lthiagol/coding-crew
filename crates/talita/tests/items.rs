//! Items (T13) through the real binary, against scratch projects under the
//! temp dir: `item create|show|list|shape|dismiss|promote`.

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

const RUNNER: [(&str, &str); 3] = [
    ("TALITA_ACTOR", "runner-1"),
    ("TALITA_ROLE", "runner"),
    ("TALITA_SESSION", "ses-runner"),
];

const HUMAN: [(&str, &str); 2] = [("TALITA_ACTOR", "luiz"), ("TALITA_ROLE", "human")];

/// A fresh scratch project for one test.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("talita-items-{name}-{}", std::process::id()));
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

fn create(dir: &Path, title: &str, kind: &str) -> String {
    applied(
        talita(
            dir,
            &CREATOR,
            &["item", "create", "--kind", kind, "--title", title],
        ),
        "item create",
    )["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn an_idea_is_shaped_and_promoted_into_a_draft_task() {
    let dir = project("promote");
    let c = &CREATOR;
    let body_file = dir.join("body.md");
    fs::write(&body_file, "The config is hand-edited.\n").unwrap();
    let body = format!("@{}", body_file.display());
    let created = applied(
        talita(
            &dir,
            c,
            &[
                "item",
                "create",
                "--kind",
                "idea",
                "--title",
                "Parse the config",
                "--body",
                &body,
                "--scope",
                "the parser",
                "the writer",
                "--tag",
                "config",
            ],
        ),
        "item create",
    );
    assert_eq!(created["data"]["id"], "ITM01");
    assert_eq!(created["data"]["title"], "Parse the config");
    assert_eq!(created["data"]["kind"], "idea");
    assert_eq!(created["data"]["status"], "open");
    assert_eq!(created["data"]["slug"], "parse-the-config");
    assert_eq!(created["wrote"][0], "work/ITM01-parse-the-config.json");

    let shown = applied(talita(&dir, &[], &["item", "show", "ITM01"]), "item show");
    assert_eq!(
        shown["data"]["item"]["body"],
        "The config is hand-edited.\n"
    );
    assert_eq!(
        shown["data"]["item"]["scope"],
        json!(["the parser", "the writer"])
    );
    assert_eq!(shown["data"]["item"]["tags"], json!(["config"]));
    assert_eq!(
        shown["data"]["derived"]["promotable"], false,
        "an idea carries no criteria"
    );

    let shaped = applied(
        talita(
            &dir,
            c,
            &[
                "item", "shape", "ITM01", "--kind", "feature", "--scope", "one", "two",
            ],
        ),
        "item shape",
    );
    assert_eq!(shaped["data"]["id"], "ITM01", "the id does not change");
    assert_eq!(shaped["data"]["kind"], "feature");
    assert_eq!(shaped["data"]["scope"], json!(["one", "two"]));
    assert_eq!(
        file_of(&dir, "ITM01").file_name().unwrap(),
        "ITM01-parse-the-config.json"
    );

    let shown = applied(talita(&dir, &[], &["item", "show", "ITM01"]), "item show");
    assert_eq!(shown["data"]["item"]["status"], "open");
    assert_eq!(shown["data"]["derived"]["promotable"], true);

    let promoted = applied(
        talita(&dir, c, &["item", "promote", "ITM01"]),
        "item promote",
    );
    assert_eq!(promoted["data"]["item"]["id"], "ITM01");
    assert_eq!(promoted["data"]["item"]["status"], "promoted");
    assert_eq!(promoted["data"]["item"]["promoted_to"], "TSK01");
    assert_eq!(promoted["data"]["task"]["id"], "TSK01");
    assert_eq!(promoted["data"]["task"]["status"], "draft");
    assert_eq!(promoted["data"]["task"]["kind"], "feature");
    assert_eq!(promoted["data"]["task"]["promoted_from"], "ITM01");
    assert_eq!(
        promoted["wrote"],
        json!([
            "work/ITM01-parse-the-config.json",
            "work/TSK01-parse-the-config.json"
        ]),
        "the item is written first"
    );

    let shown = applied(talita(&dir, &[], &["task", "show", "TSK01"]), "task show");
    let task = &shown["data"]["task"];
    assert_eq!(task["promoted_from"], "ITM01");
    assert_eq!(task["status"], "draft");
    assert_eq!(task["kind"], "feature");
    assert_eq!(task["intent"]["outcome"], "Parse the config");
    assert_eq!(task["intent"]["problem"], "The config is hand-edited.\n");
    assert_eq!(task["scope"]["in_scope"], json!(["one", "two"]));
    let criteria = task["acceptance_criteria"].as_array().unwrap();
    assert_eq!(criteria.len(), 1);
    assert_eq!(criteria[0]["id"], "ACCEPT-01");
    assert_eq!(criteria[0]["description"], "Parse the config");
    assert_eq!(criteria[0]["verification"], "");
    let notes = task["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["id"], "NOTE-01");
    assert_eq!(notes[0]["body"], "from ITM01");
    assert_eq!(notes[0]["author"], "creator-1");
}

#[test]
fn each_item_refusal_carries_its_code_and_exit() {
    let dir = project("refusals");
    let c = &CREATOR;
    let idea = create(&dir, "An idea", "idea");
    let feature = create(&dir, "A feature", "feature");
    let dismissed = create(&dir, "Dismissed", "idea");
    applied(
        talita(
            &dir,
            c,
            &["item", "dismiss", &dismissed, "--reason", "not now"],
        ),
        "item dismiss",
    );

    // An idea may not be promoted; the remedy names `shape`.
    let e = refused(
        talita(&dir, c, &["item", "promote", &idea]),
        "item promote",
        "item.is_idea",
        4,
    );
    assert_eq!(
        e["error"]["remedy"],
        format!("talita item shape {idea} --kind feature")
    );

    // A settled item refuses every move.
    refused(
        talita(&dir, c, &["item", "shape", &dismissed, "--kind", "feature"]),
        "item shape",
        "item.not_open",
        4,
    );
    refused(
        talita(
            &dir,
            c,
            &["item", "dismiss", &dismissed, "--reason", "again"],
        ),
        "item dismiss",
        "item.not_open",
        4,
    );
    refused(
        talita(&dir, c, &["item", "promote", &dismissed]),
        "item promote",
        "item.not_open",
        4,
    );
    applied(
        talita(&dir, c, &["item", "promote", &feature]),
        "item promote",
    );
    refused(
        talita(&dir, c, &["item", "dismiss", &feature, "--reason", "late"]),
        "item dismiss",
        "item.not_open",
        4,
    );

    // A small task is shaped already.
    let shaped = create(&dir, "Shaped", "feature");
    refused(
        talita(&dir, c, &["item", "shape", &shaped, "--kind", "bug"]),
        "item shape",
        "item.already_shaped",
        4,
    );

    // An id no file holds.
    refused(
        talita(&dir, &[], &["item", "show", "ITM09"]),
        "item show",
        "item.not_found",
        3,
    );
    refused(
        talita(&dir, c, &["item", "shape", "ITM09", "--kind", "feature"]),
        "item shape",
        "item.not_found",
        3,
    );
    refused(
        talita(&dir, c, &["item", "dismiss", "ITM09", "--reason", "r"]),
        "item dismiss",
        "item.not_found",
        3,
    );
    refused(
        talita(&dir, c, &["item", "promote", "ITM09"]),
        "item promote",
        "item.not_found",
        3,
    );

    // Every write is the creator's.
    let writes: [(&[&str], &str); 4] = [
        (
            &["item", "create", "--kind", "idea", "--title", "t"],
            "item create",
        ),
        (&["item", "shape", &shaped, "--kind", "bug"], "item shape"),
        (
            &["item", "dismiss", &shaped, "--reason", "r"],
            "item dismiss",
        ),
        (&["item", "promote", &shaped], "item promote"),
    ];
    for (args, command) in writes {
        refused(talita(&dir, &RUNNER, args), command, "role.wrong", 5);
        refused(talita(&dir, &HUMAN, args), command, "actor.not_agent", 5);
    }

    // Input.
    refused(
        talita(
            &dir,
            c,
            &["item", "create", "--kind", "epic", "--title", "t"],
        ),
        "item create",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, c, &["item", "shape", &idea, "--kind", "idea"]),
        "item shape",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, c, &["item", "dismiss", &idea, "--reason", ""]),
        "item dismiss",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, &[], &["item", "list", "--status", "closed"]),
        "item list",
        "input.invalid",
        2,
    );
    refused(
        talita(&dir, &[], &["item", "list", "--kind", "epic"]),
        "item list",
        "input.invalid",
        2,
    );
}

#[test]
fn item_list_filters_by_status_and_kind_and_orders_ids_numerically() {
    let dir = project("list");
    for n in 1..=10 {
        let title = format!("Idea {n}");
        let kind = if n == 10 { "feature" } else { "idea" };
        applied(
            talita(
                &dir,
                &CREATOR,
                &["item", "create", "--kind", kind, "--title", &title],
            ),
            "item create",
        );
    }
    applied(
        talita(
            &dir,
            &CREATOR,
            &["item", "dismiss", "ITM02", "--reason", "not now"],
        ),
        "item dismiss",
    );
    applied(
        talita(&dir, &CREATOR, &["item", "promote", "ITM10"]),
        "item promote",
    );

    let all = applied(talita(&dir, &[], &["item", "list"]), "item list");
    let rows = all["data"].as_array().unwrap();
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0]["id"], "ITM01");
    assert_eq!(rows[8]["id"], "ITM09");
    assert_eq!(rows[9]["id"], "ITM10", "ITM10 sorts after ITM09");
    assert_eq!(rows[9]["promoted_to"], "TSK01");

    let open = applied(
        talita(&dir, &[], &["item", "list", "--status", "open"]),
        "item list",
    );
    assert_eq!(open["data"].as_array().unwrap().len(), 8);
    let dismissed = applied(
        talita(&dir, &[], &["item", "list", "--status", "dismissed"]),
        "item list",
    );
    assert_eq!(dismissed["data"].as_array().unwrap().len(), 1);
    assert_eq!(dismissed["data"][0]["id"], "ITM02");
    let promoted = applied(
        talita(&dir, &[], &["item", "list", "--status", "promoted"]),
        "item list",
    );
    assert_eq!(promoted["data"].as_array().unwrap().len(), 1);
    assert_eq!(promoted["data"][0]["id"], "ITM10");

    let features = applied(
        talita(&dir, &[], &["item", "list", "--kind", "feature"]),
        "item list",
    );
    assert_eq!(features["data"].as_array().unwrap().len(), 1);
    assert_eq!(features["data"][0]["id"], "ITM10");

    let shown = applied(talita(&dir, &[], &["item", "show", "ITM02"]), "item show");
    assert_eq!(shown["data"]["item"]["id"], "ITM02");
    assert_eq!(shown["data"]["item"]["status"], "dismissed");
    assert_eq!(shown["data"]["item"]["dismissed_reason"], "not now");
}
