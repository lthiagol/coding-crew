//! `hugo --render --plain` through the real binary.
//!
//! The contract is raul's: the screen prints as plain text, deterministic,
//! so every screen can be pinned byte for byte. The screens are rendered in
//! the mock project, so the runtime record is the one the fixtures hold.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use coding_crew_theme::text::text_width;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// The mock project: its `coding-crew/` folder holds the runtime record the
/// screens show, and the store's own discovery finds it from here.
fn mock_project() -> PathBuf {
    repo_root().join("mock-data")
}

/// A scratch home every render shares: hugo writes its configuration home,
/// and the test's own home folder must not be where it lands.
fn home() -> PathBuf {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("hugo-render-home-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a scratch home");
        dir
    })
    .clone()
}

fn run(args: &[&str]) -> Output {
    run_in(&mock_project(), &home(), args)
}

fn run_in(dir: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hugo"))
        .current_dir(dir)
        .env("HOME", home)
        // A developer's icon set must not change these renders.
        .env_remove("CODING_CREW_ICONS")
        .args(args)
        .output()
        .expect("running hugo")
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

fn screen(width: u16, height: u16) -> String {
    let output = run(&[
        "--render",
        "--plain",
        "--width",
        &width.to_string(),
        "--height",
        &height.to_string(),
    ]);
    assert!(output.status.success(), "hugo failed: {output:?}");
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}

/// The screen at a canonical width, asserted against its snapshot.
fn assert_screen_at(width: u16, name: &str) {
    let text = screen(width, 40);
    let path = snapshot_path(name);
    let snapshot =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(text, snapshot, "the screen at {width} differs from {name}");
    assert!(!text.contains('\u{1b}'), "the render carries ANSI escapes");
    assert!(text.ends_with('\n'), "the render has no final newline");
    for (number, line) in text.lines().enumerate() {
        assert_eq!(
            line.trim_end_matches(' '),
            line,
            "row {number} ends in spaces that carry no information"
        );
        let measured = text_width(line);
        assert!(
            measured <= usize::from(width),
            "row {number} is {measured} columns wide, not at most {width}: {line:?}"
        );
    }
}

#[test]
fn the_screen_matches_the_snapshot_at_every_canonical_width() {
    // The narrow design width, the width where the sidebar is dropped, and
    // the default: the three screens the flow has.
    for (width, name) in [
        (40, "screen-40.txt"),
        (60, "screen-60.txt"),
        (100, "screen-100.txt"),
    ] {
        assert_screen_at(width, name);
    }
}

#[test]
fn two_runs_print_the_same_bytes() {
    let first = run(&["--render", "--plain", "--width", "100", "--height", "40"]);
    let second = run(&["--plain", "--height=40", "--render", "--width=100"]);
    assert!(first.status.success(), "the first run failed: {first:?}");
    assert!(second.status.success(), "the second run failed: {second:?}");
    assert_eq!(first.stdout, second.stdout, "two runs printed differently");
}

#[test]
fn the_default_size_is_100_by_40() {
    let default = run(&["--render", "--plain"]);
    let explicit = run(&["--render", "--plain", "--width", "100", "--height", "40"]);
    assert!(
        default.status.success(),
        "the default run failed: {default:?}"
    );
    assert!(
        explicit.status.success(),
        "the sized run failed: {explicit:?}"
    );
    assert_eq!(
        default.stdout, explicit.stdout,
        "the default size is not 100 by 40"
    );
}

#[test]
fn the_render_takes_the_requested_dimensions() {
    let (width, height) = (73, 23);
    let text = screen(width, height);
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(
        rows.len(),
        usize::from(height),
        "the render did not take --height"
    );
    for (number, row) in rows.iter().enumerate() {
        assert!(
            text_width(row) <= usize::from(width),
            "row {number} is wider than {width}: {row:?}"
        );
    }
}

#[test]
fn the_sidebar_is_shown_wide_and_announced_where_it_is_dropped() {
    // The two panes, side by side: the configuration on the left and the
    // flow on the right.
    let wide = screen(100, 40);
    assert!(wide.contains("configuration"), "{wide}");
    assert!(wide.contains("flow"), "{wide}");
    for group in ["AGENTS", "ROUNDS", "ORDER"] {
        assert!(
            wide.contains(group),
            "{group} is not on the sidebar: {wide}"
        );
    }
    assert!(
        !wide.contains("sidebar not shown"),
        "a shown sidebar is announced as gone: {wide}"
    );
    // Narrow: the sidebar is dropped whole, the flow keeps the window, and
    // the drop is said out loud.
    for width in [40u16, 60, 79] {
        let narrow = screen(width, 40);
        assert!(
            narrow.contains("sidebar not shown"),
            "{width}: the drop is silent: {narrow}"
        );
        assert!(
            narrow.contains("configuration"),
            "{width}: the line does not name what is not shown: {narrow}"
        );
        assert!(
            !narrow.contains("AGENTS"),
            "{width}: the sidebar is still drawn: {narrow}"
        );
    }
}

#[test]
fn the_crew_leads_the_main_pane_the_shell_has_content() {
    // The runtime record is read from the mock project's coding-crew folder,
    // discovered from the current directory like the store is.
    let text = screen(100, 40);
    assert!(text.contains("the automation's queue"), "{text}");
    let runtime = coding_crew_core::Runtime::read(&mock_project().join("coding-crew/runtime.json"))
        .expect("the runtime record");
    let mut facts = vec![
        "crew".to_string(),
        "orchestrator".to_string(),
        "runner".to_string(),
        "reviewer".to_string(),
        "one per role · the reviewer reviews".to_string(),
        "external rounds".to_string(),
        format!("{} of {}", runtime.budget.spent, runtime.budget.total),
        format!("{} open findings", runtime.findings),
        "running since".to_string(),
    ];
    for member in &runtime.crew {
        if let Some(id) = &member.holds {
            facts.push(id.clone());
        }
    }
    for fact in facts {
        assert!(
            text.contains(fact.as_str()),
            "{fact} is not on screen: {text}"
        );
    }
    assert!(!text.contains("nothing is running"), "{text}");
    // The crew is the first block under the flow's title.
    let rows: Vec<&str> = text.lines().collect();
    let crew = rows
        .iter()
        .position(|row| row.contains("crew"))
        .expect("crew");
    let queue = rows
        .iter()
        .position(|row| row.contains("queue"))
        .expect("queue");
    assert!(crew < queue, "{crew} {queue}");
}

#[test]
fn a_project_without_a_record_shows_the_named_empty_state() {
    // A folder with no coding-crew folder above it is not a project: there is
    // nothing to read and the pane says so.
    let home = home();
    let outside = outside_any_project("no-project");
    let text = screen_in(&outside, &home);
    assert!(text.contains("nothing is running"), "{text}");
    // The configuration is the tool's, not the project's: a project without a
    // record still shows the configuration the tool wrote.
    assert!(text.contains("build-and-fix rounds"), "{text}");
    let _ = fs::remove_dir_all(&outside);

    // The same through a project whose coding-crew folder holds no record.
    let dir = std::env::temp_dir().join(format!("hugo-no-record-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("coding-crew/work")).expect("a project");
    let text = screen_in(&dir, &home);
    assert!(text.contains("nothing is running"), "{text}");
    assert!(text.contains("build-and-fix rounds"), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_record_that_cannot_be_read_is_named_not_swallowed_as_no_run() {
    // The files a writer bug leaves behind: malformed JSON, and a record
    // without its required stamp. Both are unreadable, never "no run".
    let home = home();
    for (name, record) in [
        ("broken", "{ not json"),
        ("no-stamp", r#"{"started_at": "2026-11-19T09:00:00Z"}"#),
    ] {
        let dir = std::env::temp_dir().join(format!("hugo-{name}-record-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("coding-crew/work")).expect("a project");
        fs::write(dir.join("coding-crew/runtime.json"), record).expect("a broken record");
        let output = run_in(
            &dir,
            &home,
            &["--render", "--plain", "--width", "100", "--height", "12"],
        );
        assert!(output.status.success(), "{name}: hugo failed: {output:?}");
        let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
        assert!(text.contains("cannot be read"), "{name}: {text}");
        assert!(text.contains("runtime.json"), "{name}: {text}");
        assert!(
            !text.contains("nothing is running"),
            "{name}: an unreadable record reads as no run: {text}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_window_has_a_title_line_a_footer_and_no_tabs() {
    let text = screen(100, 40);
    let rows: Vec<&str> = text.lines().collect();
    // The icon, then the name; the project is the mock folder, which the
    // render names from the working directory it runs in. The pill on the
    // right says the automation runs.
    assert!(
        rows[0].starts_with(" 󰚩 hugo · mock-data") && rows[0].ends_with("󰑮 running"),
        "the title line names the tool and says whether it runs: {:?}",
        rows[0]
    );
    assert!(rows.last().expect("a footer").contains("quit"), "{text}");
    // The shell is one sidebar and one main pane: no tab bar, no tab key.
    assert!(!text.contains("Overview"), "{text}");
    assert!(!text.contains("Needs you"), "{text}");
    assert!(!text.contains("Backlog"), "{text}");
    assert!(!text.contains("Settings"), "{text}");
    assert!(!text.contains("1-5"), "{text}");
    assert!(!text.contains("switch tab"), "{text}");
}

#[test]
fn the_footer_names_the_keys_that_work_and_no_key_that_does_nothing() {
    // The render starts with the cursor on the builder, where enter opens
    // nothing: the footer names quit, reload and help.
    let text = screen(100, 40);
    let footer = text.lines().last().expect("a footer").trim();
    assert_eq!(
        footer, "q  quit   r  reload   ↑↓ j/k  move   ?  help",
        "{footer}"
    );
    for dead in ["open", "tab", "esc", "choose", "pick"] {
        assert!(!footer.contains(dead), "{footer}: {dead}");
    }
    // The sidebar is shown, so the move keys work; a window that drops it
    // does not name them.
    let narrow = screen(60, 40);
    let narrow_footer = narrow.lines().last().expect("a footer").trim();
    assert!(!narrow_footer.contains("move"), "{narrow_footer}");
    // The same keys are in the help.
    assert!(text.contains("reload"), "{text}");
}

#[test]
fn a_theme_changes_the_paint_and_not_the_words() {
    // The theme crate is shared, so the same screen says the same thing in
    // every preset; the paint is what moves.
    let dark = screen(100, 40);
    for preset in ["nord", "catppuccin", "dracula", "terminal"] {
        let text = screen_with_theme(preset, 100, 40);
        assert_eq!(text, dark, "{preset} changes the words of the shell");
    }
}

fn screen_with_theme(preset: &str, width: u16, height: u16) -> String {
    let output = run(&[
        "--render",
        "--plain",
        "--theme",
        preset,
        "--width",
        &width.to_string(),
        "--height",
        &height.to_string(),
    ]);
    assert!(output.status.success(), "{preset}: hugo failed: {output:?}");
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}

#[test]
fn an_unknown_theme_is_refused_before_rendering() {
    let output = run(&["--render", "--plain", "--theme", "light"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unknown theme did not exit 1: {output:?}"
    );
    assert!(output.stdout.is_empty(), "a screen was printed: {output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("light"));
}

#[test]
fn either_flag_alone_is_refused() {
    for flag in ["--render", "--plain"] {
        let output = run(&[flag]);
        assert!(!output.status.success(), "{flag} alone exited 0");
        assert!(output.stdout.is_empty(), "{flag} alone printed a screen");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("go together"),
            "{flag}: {output:?}"
        );
    }
}

#[test]
fn a_dimension_is_bounded_before_rendering() {
    for (flag, value) in [
        ("--width", "1001"),
        ("--height", "1001"),
        ("--width", "65535"),
        ("--height", "65535"),
        ("--width", "0"),
        ("--height", "wide"),
    ] {
        let output = run(&["--render", "--plain", flag, value]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{flag} {value} did not exit 1: {output:?}"
        );
        assert!(
            output.stdout.is_empty(),
            "{flag} {value} printed a screen before refusing"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(flag), "{flag} {value}: {stderr}");
    }
}

#[test]
fn with_a_temporary_home_running_hugo_leaves_the_configuration_folder_there() {
    // The tool creates its home rather than failing: the home does not exist
    // at all before the run, and the run makes it.
    let home = std::env::temp_dir().join(format!("hugo-home-created-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let config = home.join(".config").join("coding-crew");
    assert!(!config.exists(), "the test home is not fresh");
    let output = run_in(&mock_project(), &home, &["--render", "--plain"]);
    assert!(output.status.success(), "hugo failed: {output:?}");
    assert!(
        config.is_dir(),
        "hugo did not create {}: {output:?}",
        config.display()
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn the_shell_runs_without_a_home_folder() {
    // A machine with no home has nowhere for hugo to write, and that is not
    // a reason to fail: the flow still shows, and the sidebar says there is
    // no configuration.
    let output = Command::new(env!("CARGO_BIN_EXE_hugo"))
        .current_dir(outside_any_project("no-home"))
        .env_remove("HOME")
        .args(["--render", "--plain", "--width", "100", "--height", "20"])
        .output()
        .expect("running hugo");
    assert!(output.status.success(), "hugo failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    assert!(text.contains("nothing is running"), "{text}");
    assert!(text.contains("no configuration yet"), "{text}");
}

#[test]
fn an_absent_configuration_is_written_with_the_defaults() {
    // The tool writes the defaults into the home it creates, and the sidebar
    // shows them. The agents are the project's own flow.
    let home = fresh_home("defaults");
    let config = home.join(".config").join("coding-crew").join("config.toml");
    assert!(!config.exists(), "the test home is not fresh");
    let text = screen_in(&mock_project(), &home);
    for fact in [
        "builder · runner",
        "claude-haiku-5-5",
        "claude-opus-5-5",
        "5 build-and-fix rounds",
        "1 groom · creator",
    ] {
        assert!(text.contains(fact), "{fact} is not on screen: {text}");
    }
    assert!(config.is_file(), "hugo did not write {}", config.display());
    // The format a human opens is TOML, and it explains itself.
    let written = fs::read_to_string(&config).expect("the configuration");
    assert!(written.contains("[[agents]]"), "{written}");
    assert!(
        written.lines().any(|line| line.starts_with('#')),
        "the written defaults carry no comment: {written}"
    );
    assert_eq!(
        coding_crew_core::Config::read(&config).expect("the configuration parses"),
        coding_crew_core::Config::default()
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn a_configuration_that_is_there_is_read_and_never_overwritten() {
    // A human's configuration is the one the pane shows, and the tool writes
    // nothing over it — not even when it differs from the defaults.
    let home = fresh_home("kept");
    let config = home.join(".config").join("coding-crew").join("config.toml");
    fs::create_dir_all(config.parent().expect("the home")).expect("the configuration home");
    let mut mine = coding_crew_core::Config {
        round_limit: 2,
        ..coding_crew_core::Config::default()
    };
    mine.agents[0].harness = "cursor-agent".to_string();
    mine.agents[0].model = "auto".to_string();
    mine.write(&config).expect("a configuration");
    let before = fs::read_to_string(&config).expect("the configuration");

    let text = screen_in(&mock_project(), &home);
    assert!(text.contains("2 build-and-fix rounds"), "{text}");
    assert!(text.contains("harness cursor-agent"), "{text}");
    assert!(!text.contains("5 build-and-fix rounds"), "{text}");
    assert_eq!(
        fs::read_to_string(&config).expect("the configuration"),
        before,
        "hugo overwrote a configuration that was there"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn a_configuration_that_cannot_be_read_is_named_and_left_alone() {
    // Absent and unreadable are different states: a file that does not parse
    // is named with its error, the defaults are shown beside it, and the tool
    // does not replace it.
    let home = fresh_home("broken");
    let config = home.join(".config").join("coding-crew").join("config.toml");
    fs::create_dir_all(config.parent().expect("the home")).expect("the configuration home");
    fs::write(&config, "round_limit = \n").expect("a broken configuration");

    let text = screen_in(&mock_project(), &home);
    assert!(text.contains("configuration is unreadable"), "{text}");
    assert!(text.contains("config.toml"), "{text}");
    assert!(!text.contains("no configuration yet"), "{text}");
    assert!(
        text.contains("using the defaults:"),
        "the defaults are not shown beside the error: {text}"
    );
    assert!(text.contains("5 build-and-fix rounds"), "{text}");
    assert_eq!(
        fs::read_to_string(&config).expect("the configuration"),
        "round_limit = \n",
        "hugo overwrote a configuration it could not read"
    );
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn an_old_json_configuration_is_named_and_not_read() {
    // The old file is not converted and not silently ignored: hugo names it
    // and uses the defaults, which it writes beside it.
    let home = fresh_home("legacy");
    let dir = home.join(".config").join("coding-crew");
    fs::create_dir_all(&dir).expect("the configuration home");
    let old = dir.join("config.json");
    let old_text = r#"{"round_limit": 9}"#;
    fs::write(&old, old_text).expect("an old configuration");

    let text = screen_in(&mock_project(), &home);
    assert!(text.contains("config.json"), "{text}");
    assert!(text.contains("not read"), "{text}");
    assert!(text.contains("5 build-and-fix rounds"), "{text}");
    assert!(!text.contains("9 build-and-fix rounds"), "{text}");
    assert!(
        dir.join("config.toml").is_file(),
        "no defaults were written"
    );
    assert_eq!(
        fs::read_to_string(&old).expect("the old file"),
        old_text,
        "hugo converted the old configuration"
    );
    let _ = fs::remove_dir_all(&home);
}

/// A scratch folder under the system's temporary folder, which has no
/// coding-crew folder above it: a run from here is not inside any project.
fn outside_any_project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hugo-{name}-outside-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// A scratch home of this test's own: the configuration is written into it.
fn fresh_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hugo-{name}-home-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch home");
    dir
}

/// The 100-by-40 screen, in a project and a home of the caller's choosing.
fn screen_in(dir: &Path, home: &Path) -> String {
    let output = run_in(
        dir,
        home,
        &["--render", "--plain", "--width", "100", "--height", "40"],
    );
    assert!(output.status.success(), "hugo failed: {output:?}");
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}
