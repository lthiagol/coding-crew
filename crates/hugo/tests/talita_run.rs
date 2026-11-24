//! hugo's main pane over a runtime record talita wrote (O2b).
//!
//! talita's verbs drive a scratch project in-process, through talita's
//! library, so the record is the one talita writes, not the mock's. Then
//! `hugo --render --plain` reads that record and draws the pane.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The identity one talita call acts under: the flags the call names.
struct Who {
    actor: &'static str,
    role: &'static str,
    session: &'static str,
}

const CREATOR: Who = Who {
    actor: "creator-1",
    role: "creator",
    session: "ses-creator",
};
const ORCHESTRATOR: Who = Who {
    actor: "orchestrator",
    role: "orchestrator",
    session: "ses-orchestrator",
};
const BUILDER: Who = Who {
    actor: "b-tsk01-0",
    role: "runner",
    session: "ses-builder",
};
const REVIEWER: Who = Who {
    actor: "rev-1",
    role: "reviewer",
    session: "ses-reviewer",
};

/// A fresh scratch folder for one test, and its home.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hugo-talita-run-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch folder");
    fs::canonicalize(&dir).expect("the scratch folder")
}

/// One talita call through the library: the flags name the caller, `--project`
/// names the folder, and the call must exit 0.
fn talita(project: &Path, who: &Who, verb: &[&str]) {
    let mut args: Vec<OsString> = [
        "talita",
        "--project",
        project.to_str().expect("a UTF-8 path"),
        "--as",
        who.actor,
        "--role",
        who.role,
        "--session",
        who.session,
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.extend(verb.iter().map(OsString::from));
    let code = coding_crew_talita::run(args);
    assert_eq!(code, 0, "talita {verb:?} exited {code}");
}

/// `draft` → `groomed` → `approved`, as talita's own runtime tests do.
fn approved_task(project: &Path) -> String {
    talita(
        project,
        &CREATOR,
        &["task", "create", "--title", "Measured"],
    );
    let id = "TSK01";
    talita(
        project,
        &CREATOR,
        &["task", "describe", id, "--outcome", "it works"],
    );
    talita(
        project,
        &CREATOR,
        &["scope", "exclude", id, "--item", "not this"],
    );
    talita(
        project,
        &CREATOR,
        &[
            "criterion",
            "add",
            id,
            "--description",
            "it works",
            "--verification",
            "echo it works",
        ],
    );
    talita(project, &CREATOR, &["task", "groom", id]);
    talita(
        project,
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
    );
    talita(project, &CREATOR, &["task", "approve", id]);
    id.to_string()
}

/// A run started over the task, with the orchestrator, a builder and a
/// reviewer joined.
fn run_with_a_crew(project: &Path, id: &str) {
    talita(project, &ORCHESTRATOR, &["run", "start", "--task", id]);
    talita(
        project,
        &ORCHESTRATOR,
        &[
            "crew",
            "join",
            "orchestrator",
            "--role",
            "orchestrator",
            "--model",
            "claude-opus-5-5",
        ],
    );
    talita(
        project,
        &ORCHESTRATOR,
        &[
            "crew",
            "join",
            BUILDER.actor,
            "--role",
            "builder",
            "--model",
            "deepseek-v4.1-flash",
        ],
    );
    talita(
        project,
        &ORCHESTRATOR,
        &[
            "crew",
            "join",
            REVIEWER.actor,
            "--role",
            "reviewer",
            "--model",
            "glm-5.3",
        ],
    );
}

/// A run with one builder, one reviewer and the orchestrator, whose task has
/// been built, reviewed once with two findings, and sent back for changes.
fn run_with_a_review_round(project: &Path) {
    let id = approved_task(project);
    run_with_a_crew(project, &id);
    built(project, &id);
    reviewed(
        project,
        &id,
        "changes-needed",
        serde_json::json!([
            {"ref": "D1", "severity": "high", "description": "the lock is not released"},
            {"ref": "D2", "severity": "low", "description": "the name is vague"},
        ]),
    );
    talita(project, &REVIEWER, &["task", "request-changes", &id]);
}

/// A run whose task is built and reviewed clean, passed, and whose crew has
/// left: the record a run stop writes once the queue is done.
fn run_done(project: &Path) {
    let id = approved_task(project);
    run_with_a_crew(project, &id);
    built(project, &id);
    reviewed(project, &id, "ok", serde_json::json!([]));
    talita(project, &REVIEWER, &["task", "pass-review", &id]);
    talita(project, &ORCHESTRATOR, &["crew", "leave", BUILDER.actor]);
    talita(project, &ORCHESTRATOR, &["crew", "leave", REVIEWER.actor]);
}

/// The build: claim, start, the step and its evidence, a self round, finish.
fn built(project: &Path, id: &str) {
    talita(project, &BUILDER, &["claim", "take", id, "--lease", "2h"]);
    talita(project, &BUILDER, &["task", "start", id]);
    talita(project, &BUILDER, &["step", "begin", id, "STP-01"]);
    talita(project, &BUILDER, &["step", "done", id, "STP-01"]);
    talita(project, &BUILDER, &["evidence", "run", id, "ACCEPT-01"]);
    talita(
        project,
        &BUILDER,
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
    );
    talita(project, &BUILDER, &["task", "finish", id]);
}

/// The review: the reviewer is assigned and files an external round with the
/// verdict and findings given.
fn reviewed(project: &Path, id: &str, verdict: &str, findings: serde_json::Value) {
    talita(
        project,
        &ORCHESTRATOR,
        &["crew", "assign", REVIEWER.actor, id],
    );
    let round = project.join("round.json");
    let input = serde_json::json!({
        "kind": "external",
        "focus": "spec-compliance",
        "verdict": verdict,
        "notes": "the review",
        "findings": findings,
    });
    fs::write(&round, serde_json::to_vec(&input).expect("json")).expect("the round file");
    talita(
        project,
        &REVIEWER,
        &[
            "round",
            "file",
            id,
            "--input",
            round.to_str().expect("a path"),
        ],
    );
}

/// The main pane, drawn plain by hugo in `project`, at a width the rows fit.
fn render(project: &Path, home: &Path) -> String {
    let output: Output = Command::new(env!("CARGO_BIN_EXE_hugo"))
        .current_dir(project)
        .env("HOME", home)
        .env_remove("CODING_CREW_ICONS")
        .args(["--render", "--plain", "--width", "120", "--height", "60"])
        .output()
        .expect("running hugo");
    assert!(output.status.success(), "hugo failed: {output:?}");
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}

/// The first line of the render that holds every needle: one pane row, whole.
fn row<'a>(text: &'a str, needles: &[&str]) -> &'a str {
    text.lines()
        .find(|line| needles.iter().all(|needle| line.contains(needle)))
        .unwrap_or_else(|| panic!("no row holds {needles:?}: {text}"))
}

#[test]
fn the_pane_shows_the_crew_round_budget_and_findings_talita_wrote() {
    let project = scratch("pane");
    let home = scratch("pane-home");
    talita(&project, &CREATOR, &["project", "init"]);
    run_with_a_review_round(&project);
    let record = project.join("coding-crew").join("runtime.json");
    assert!(record.is_file(), "talita wrote no runtime record");

    let text = render(&project, &home);
    // The crew: one row per agent, with its model, its hold and its state.
    let orchestrator = row(&text, &["orchestrator", "claude-opus-5-5", "idle"]);
    assert!(orchestrator.contains("—"), "{orchestrator}");
    let builder = row(&text, &["builder", "deepseek-v4.1-flash", "TSK01", "idle"]);
    assert!(builder.contains("under a minute"), "{builder}");
    let reviewer = row(&text, &["reviewer", "glm-5.3", "TSK01"]);
    assert!(reviewer.contains("under a minute"), "{reviewer}");
    assert!(
        text.contains("one per role · the reviewer reviews"),
        "{text}"
    );
    // The round: the task in flight, its earlier round's findings, the
    // round in flight.
    let round = row(&text, &["TSK01 · external"]);
    assert!(round.contains("round 1"), "{round}");
    assert!(round.contains("2 findings"), "{round}");
    assert!(round.contains("round 2"), "{round}");
    // The budget: two of the task's five rounds.
    assert!(row(&text, &["rounds", "2 of 5"]).contains("2 of 5"));
    // The findings: two open, by severity.
    row(&text, &["2 open findings · 1 high · 0 medium · 1 low"]);

    let _ = fs::remove_dir_all(&project);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn a_stopped_run_is_not_drawn_with_a_crew_or_budget_it_no_longer_has() {
    let project = scratch("stopped");
    let home = scratch("stopped-home");
    talita(&project, &CREATOR, &["project", "init"]);
    run_done(&project);
    talita(
        &project,
        &ORCHESTRATOR,
        &["run", "stop", "--why", "queue done"],
    );

    let text = render(&project, &home);
    // The run is over: the stamp says so, and the crew's size is not claimed.
    row(&text, &["not running: queue done"]);
    assert!(!text.contains("the orchestrator reviews"), "{text}");
    assert!(!text.contains("the reviewer reviews"), "{text}");
    // No task is in flight, so the budget shows the configured limit, not
    // the record's empty 0 of 0.
    assert!(!text.contains("0 of 0"), "{text}");
    assert!(row(&text, &["rounds", "of 5"]).contains("0 of 5"));

    let _ = fs::remove_dir_all(&project);
    let _ = fs::remove_dir_all(&home);
}
