//! `hugo --skill` through the real binary.
//!
//! The skill is text for an agent: it prints and exits, with the crew the
//! configuration in force names above the role's own words.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use coding_crew_core::Config;

const ROLES: [&str; 3] = ["orchestrator", "runner", "reviewer"];

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
    // Run from a folder that is no project, so the runtime is never read.
    command
        .current_dir(std::env::temp_dir())
        .env_remove("CODING_CREW_ICONS")
        .args(args);
    match home {
        Some(home) => command.env("HOME", home),
        None => command.env_remove("HOME"),
    };
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
    let mut config = Config::default();
    config.harness.tool = "claude".to_string();
    config.round_limit = 2;
    config.agents[1].model = "anthropic/claude-opus#high".to_string();
    config.write(&config_file(&home)).expect("a configuration");

    let text = stdout_of(&run(Some(&home), &["--skill", "reviewer"]));
    assert!(text.contains("Harness: claude,"), "{text}");
    assert!(text.contains("Review-round limit: 2."), "{text}");
    assert!(
        text.contains("- reviewer (reviewer): anthropic/claude-opus#high\n"),
        "{text}"
    );
    assert!(
        text.contains("  model args: --model anthropic/claude-opus --effort high\n"),
        "{text}"
    );
    assert!(!text.contains("Review-round limit: 5."), "{text}");
}

#[test]
fn an_unreadable_configuration_prints_the_defaults_and_says_so_in_one_line() {
    let home = fresh_home("unreadable");
    let config = config_file(&home);
    fs::write(&config, "round_limit = \n").expect("a broken configuration");

    let text = stdout_of(&run(Some(&home), &["--skill", "runner"]));
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
fn an_unknown_role_fails_and_names_the_three() {
    let home = fresh_home("unknown");
    for args in [vec!["--skill", "builder"], vec!["--skill=builder"]] {
        let output = run(Some(&home), &args);
        assert!(!output.status.success(), "{args:?} succeeded");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("builder"), "{stderr}");
        for role in ROLES {
            assert!(stderr.contains(role), "{stderr}");
        }
    }
}

#[test]
fn a_skill_is_not_a_render() {
    let home = fresh_home("render");
    let output = run(Some(&home), &["--skill", "runner", "--render", "--plain"]);
    assert!(!output.status.success(), "the skill rendered");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not render"), "{stderr}");
    assert!(output.stdout.is_empty(), "the skill printed a screen");
}
