//! `hugo --render --plain` through the real binary.
//!
//! The contract is raul's: the screen prints as plain text, deterministic,
//! so every screen can be pinned byte for byte.

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
    run_in_home(&home(), args)
}

fn run_in_home(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hugo"))
        .current_dir(repo_root())
        .env("HOME", dir)
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
    // the default: the three screens the shell has.
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
    assert!(wide.contains("no configuration yet"), "{wide}");
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
            narrow.contains("nothing is running"),
            "{width}: the flow went with the sidebar: {narrow}"
        );
        assert!(
            !narrow.contains("no configuration yet"),
            "{width}: the sidebar is still drawn: {narrow}"
        );
    }
}

#[test]
fn the_empty_states_are_named() {
    let text = screen(100, 40);
    assert!(
        text.contains("no configuration yet"),
        "the sidebar has no named empty state: {text}"
    );
    assert!(
        text.contains("nothing is running"),
        "the main pane has no named empty state: {text}"
    );
}

#[test]
fn the_window_has_a_title_line_a_footer_and_no_tabs() {
    let text = screen(100, 40);
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows[0], " hugo", "the title line names the tool");
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
fn the_footer_names_the_quit_key_and_no_key_that_does_nothing() {
    let text = screen(100, 40);
    let footer = text.lines().last().expect("a footer").trim();
    assert_eq!(footer, "q  quit", "{footer}");
    for dead in ["help", "open", "move", "tab", "esc"] {
        assert!(!footer.contains(dead), "{footer}: {dead}");
    }
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
    let output = run_in_home(&home, &["--render", "--plain"]);
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
    // a reason to fail: the flow still shows.
    let output = Command::new(env!("CARGO_BIN_EXE_hugo"))
        .current_dir(repo_root())
        .env_remove("HOME")
        .args(["--render", "--plain", "--width", "60", "--height", "12"])
        .output()
        .expect("running hugo");
    assert!(output.status.success(), "hugo failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    assert!(text.contains("nothing is running"), "{text}");
}
