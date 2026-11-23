//! `hugo --skill` through the real binary.
//!
//! The skill is text for an agent: it prints and exits, with the crew the
//! configuration in force names above the role's own words.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use coding_crew_core::Config;

const ROLES: [&str; 3] = ["orchestrator", "builder", "reviewer"];

/// The text a role's file holds, as the binary embeds it.
fn skill_text(role: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("skills")
        .join(format!("{role}.md"));
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// A scratch home of this test's own, fresh on every call.
fn fresh_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hugo-skill-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch home");
    dir
}

/// The configuration file under a home, its folder made.
fn config_file(home: &Path) -> PathBuf {
    let dir = home.join(".config").join("coding-crew");
    fs::create_dir_all(&dir).expect("the configuration home");
    dir.join("config.toml")
}

fn run(home: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hugo"));
    // Run from a folder that is no project, so the runtime is never read. The
    // crew variables are cleared: the file's crew is what these tests pin.
    command
        .current_dir(std::env::temp_dir())
        .env_remove("CODING_CREW_ICONS")
        .env_remove("HUGO_CREW")
        .env_remove("HUGO_SESSION")
        .args(args);
    match home {
        Some(home) => command.env("HOME", home),
        None => command.env_remove("HOME"),
    };
    command.output().expect("running hugo")
}

/// Run the binary with one environment variable set, the crew variables
/// otherwise cleared, so the case is the only crew in force.
fn run_with_crew(home: &Path, name: &str, value: &str, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hugo"));
    command
        .current_dir(std::env::temp_dir())
        .env_remove("CODING_CREW_ICONS")
        .env_remove("HUGO_CREW")
        .env_remove("HUGO_SESSION")
        .env("HOME", home)
        .env(name, value)
        .args(args);
    command.output().expect("running hugo")
}

fn stdout_of(output: &Output) -> String {
    assert!(output.status.success(), "hugo failed: {output:?}");
    String::from_utf8(output.stdout.clone()).expect("the skill is UTF-8")
}

#[test]
fn no_role_lists_the_three_roles_and_exits_zero() {
    let home = fresh_home("list");
    let output = run(Some(&home), &["--skill"]);
    let text = stdout_of(&output);
    for role in ROLES {
        assert!(
            text.lines().any(|line| line.starts_with(role)),
            "{role} is not listed:\n{text}"
        );
    }
    assert!(!text.contains("# Crew"), "the list carries a crew:\n{text}");
}

#[test]
fn each_role_prints_the_crew_and_then_its_text() {
    let home = fresh_home("roles");
    for role in ROLES {
        let joined_form = format!("--skill={role}");
        let spaced = stdout_of(&run(Some(&home), &["--skill", role]));
        let joined = stdout_of(&run(Some(&home), &[joined_form.as_str()]));
        assert_eq!(spaced, joined, "{role}: the two forms print differently");
        assert!(spaced.starts_with("# Crew\n"), "{role}:\n{spaced}");
        assert!(
            spaced.contains("Review-round limit: 5."),
            "{role}:\n{spaced}"
        );
        assert!(
            spaced.ends_with(&skill_text(role)),
            "{role}: the text is not the file's:\n{spaced}"
        );
    }
}

#[test]
fn the_crew_header_reflects_a_configuration_that_is_not_the_default() {
    let home = fresh_home("configured");
    let mut config = Config {
        round_limit: 2,
        ..Config::default()
    };
    config.agents[2].harness = "opencode".to_string();
    config.agents[2].model = "opencode-go/glm-5.3".to_string();
    config.agents[2].effort = Some("low".to_string());
    config.write(&config_file(&home)).expect("a configuration");

    let text = stdout_of(&run(Some(&home), &["--skill", "reviewer"]));
    assert!(text.contains("Review-round limit: 2."), "{text}");
    assert!(
        text.contains("- reviewer: harness opencode, model opencode-go/glm-5.3, effort low\n"),
        "{text}"
    );
    assert!(
        text.contains("  start args: --standalone --auto\n"),
        "{text}"
    );
    assert!(
        text.contains("  start env: 'OPENCODE_CONFIG_CONTENT={"),
        "{text}"
    );
    assert!(!text.contains("Review-round limit: 5."), "{text}");
}

#[test]
fn an_unreadable_configuration_prints_the_defaults_and_says_so_in_one_line() {
    let home = fresh_home("unreadable");
    let config = config_file(&home);
    fs::write(&config, "round_limit = \n").expect("a broken configuration");

    let text = stdout_of(&run(Some(&home), &["--skill", "builder"]));
    assert!(
        text.contains("config.toml is unreadable: the defaults are in force.\n"),
        "{text}"
    );
    assert!(text.contains("Review-round limit: 5."), "{text}");
    assert_eq!(
        fs::read_to_string(&config).expect("the configuration"),
        "round_limit = \n",
        "hugo overwrote a configuration it could not read"
    );
}

#[test]
fn a_machine_with_no_home_prints_the_defaults_and_says_so() {
    let text = stdout_of(&run(None, &["--skill", "orchestrator"]));
    assert!(
        text.contains("no configuration yet: the defaults are in force.\n"),
        "{text}"
    );
    assert!(text.contains("Review-round limit: 5."), "{text}");
}

#[test]
fn the_crew_is_read_from_hugo_crew_and_hugo_session_is_ignored() {
    // The orchestrator's tab carries the session's crew in HUGO_CREW (ADR
    // 0015). The old name is not read: a tab that still sets it gets the
    // file's crew, not the stale one.
    let home = fresh_home("crew-env");
    let mut crew = Config::default();
    crew.agents.truncate(1);
    crew.round_limit = 2;
    let text = serde_json::json!({
        "agents": crew.agents,
        "round_limit": crew.round_limit,
        "order": crew.order,
        "raul": crew.raul,
    })
    .to_string();

    let from_crew = stdout_of(&run_with_crew(
        &home,
        "HUGO_CREW",
        &text,
        &["--skill", "builder"],
    ));
    assert!(
        from_crew.contains("Review-round limit: 2."),
        "HUGO_CREW is not read:\n{from_crew}"
    );
    assert!(
        from_crew.contains("- builder: none; the orchestrator builds it"),
        "the crew's roles are not the tab's:\n{from_crew}"
    );

    let from_session = stdout_of(&run_with_crew(
        &home,
        "HUGO_SESSION",
        &text,
        &["--skill", "builder"],
    ));
    assert!(
        from_session.contains("Review-round limit: 5."),
        "HUGO_SESSION is still read:\n{from_session}"
    );
    assert!(
        from_session.contains("- builder: harness claude"),
        "the stale variable replaced the file's crew:\n{from_session}"
    );
}

#[test]
fn an_unknown_role_fails_and_names_the_three() {
    let home = fresh_home("unknown");
    for args in [vec!["--skill", "runner"], vec!["--skill=runner"]] {
        let output = run(Some(&home), &args);
        assert!(!output.status.success(), "{args:?} succeeded");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("runner"), "{stderr}");
        for role in ROLES {
            assert!(stderr.contains(role), "{stderr}");
        }
    }
}

#[test]
fn a_skill_is_not_a_render() {
    let home = fresh_home("render");
    let output = run(Some(&home), &["--skill", "builder", "--render", "--plain"]);
    assert!(!output.status.success(), "the skill rendered");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not render"), "{stderr}");
    assert!(output.stdout.is_empty(), "the skill printed a screen");
}
