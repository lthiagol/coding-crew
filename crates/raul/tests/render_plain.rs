//! `raul --render --plain` through the real binary.
//!
//! The command runs from the repository root with the work folder as a
//! relative path, so the Overview's title line — which names the folder it
//! reads — is the same on every machine.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

/// The work folder, relative to the repository root.
const WORK: &str = "mock-data/coding-lanes/work";

/// A fresh project folder holding a `work` folder with the duplicate-id
/// files, written in `order`. The project folder is the working directory of
/// a render, so every case names the same relative work folder.
fn duplicate_id_project(tag: &str, order: &[usize]) -> PathBuf {
    let project = std::env::temp_dir().join(format!("raul-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project);
    let dir = project.join("work");
    fs::create_dir_all(&dir).expect("a work folder");
    for index in order {
        let (name, contents) = support::DUPLICATE_ID_FILES[*index];
        fs::write(dir.join(name), contents).expect("writing a duplicate-id file");
    }
    project
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn run(args: &[&str]) -> Output {
    run_in(&repo_root(), args)
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_raul"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("running raul")
}

/// Every ordering of the indices `0..count`.
fn permutations(count: usize) -> Vec<Vec<usize>> {
    fn walk(next: usize, current: &mut Vec<usize>, orders: &mut Vec<Vec<usize>>) {
        if next == current.len() {
            orders.push(current.clone());
            return;
        }
        for index in next..current.len() {
            current.swap(next, index);
            walk(next + 1, current, orders);
            current.swap(next, index);
        }
    }
    let mut orders = Vec::new();
    let mut current: Vec<usize> = (0..count).collect();
    walk(0, &mut current, &mut orders);
    orders
}

/// The Overview at the default size, as the snapshot records it.
fn overview() -> Output {
    run(&[
        "--render", "--plain", "--width", "100", "--height", "40", WORK,
    ])
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

/// The terminal columns a line takes.
fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

/// The terminal columns one character takes, for the characters a render can
/// carry: box drawing and ASCII are one column, and a wide grapheme is two.
fn char_width(ch: char) -> usize {
    match ch {
        '\u{1100}'..='\u{115F}'
        | '\u{2E80}'..='\u{303E}'
        | '\u{3041}'..='\u{33FF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{A000}'..='\u{A4CF}'
        | '\u{AC00}'..='\u{D7A3}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FE30}'..='\u{FE6F}'
        | '\u{FF00}'..='\u{FF60}'
        | '\u{FFE0}'..='\u{FFE6}'
        | '\u{1F300}'..='\u{1FAFF}'
        | '\u{20000}'..='\u{3FFFD}' => 2,
        _ => 1,
    }
}

/// A work folder holding one approved task whose title is a wide grapheme.
fn wide_task_work() -> PathBuf {
    let source = repo_root()
        .join(WORK)
        .join("TSK29-lorem-ipsum-dolor-sit-amet.json");
    let task = fs::read_to_string(&source)
        .expect("the task the wide title is built from")
        .replace("\"TSK29\"", "\"TSK01\"")
        .replace("\"Lorem ipsum dolor sit amet\"", "\"界\"")
        .replace(
            "\"depends_on\": [\n    \"TSK22\"\n  ]",
            "\"depends_on\": []",
        );
    let dir = std::env::temp_dir().join(format!("raul-wide-grapheme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a work folder");
    fs::write(dir.join("TSK01-wide.json"), task).expect("writing the wide task");
    dir
}

#[test]
fn the_overview_render_matches_the_snapshot() {
    assert_overview_at(100, "overview.txt");
}

/// The detail pane of a real mock-data task, as the snapshot records it.
fn detail_snapshot() -> Output {
    run(&[
        "--render",
        "--plain",
        "--detail",
        DETAIL_TASK,
        "--width",
        "100",
        "--height",
        "40",
        WORK,
    ])
}

fn detail_snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/detail.txt")
}

#[test]
fn the_detail_render_matches_the_snapshot() {
    // The snapshot is a real mock-data task, not a synthetic one: the
    // document the command prints is asserted byte for byte against the
    // committed file, so any change to the projection, the engine or the
    // pane shows up here first.
    let output = detail_snapshot();
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let snapshot = fs::read_to_string(detail_snapshot_path()).expect("the detail snapshot");
    assert_eq!(
        text, snapshot,
        "the detail render differs from tests/snapshots/detail.txt"
    );
    assert!(!text.contains('\u{1b}'), "the render carries ANSI escapes");
    assert!(text.ends_with('\n'), "the render has no final newline");
}

/// The Overview at a canonical width, asserted against its snapshot. The
/// files are what `scripts/pty-sweep.py` writes at that width: one plain
/// render, byte for byte.
fn assert_overview_at(width: u16, name: &str) {
    let width_arg = width.to_string();
    let output = run(&[
        "--render", "--plain", "--width", &width_arg, "--height", "40", WORK,
    ]);
    assert!(
        output.status.success(),
        "raul failed at width {width}: {output:?}"
    );
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let path = snapshot_path(name);
    let snapshot =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(
        text, snapshot,
        "the Overview at width {width} differs from {name}"
    );
    assert!(!text.contains('\u{1b}'), "the render carries ANSI escapes");
    assert!(text.ends_with('\n'), "the render has no final newline");
    for (number, line) in text.lines().enumerate() {
        assert_eq!(
            line.trim_end_matches(' '),
            line,
            "row {number} ends in spaces that carry no information"
        );
        let measured = display_width(line);
        assert!(
            measured <= width as usize,
            "row {number} is {measured} columns wide, not at most {width}: {line:?}"
        );
    }
}

#[test]
fn the_overview_matches_the_snapshot_at_every_canonical_width() {
    // Widths the sweep and the reviewers read: the narrow design width, the
    // two-column layout, and the default. A rendering regression that the
    // data unit tests miss changes one of these files.
    for (width, name) in [
        (40, "overview-40.txt"),
        (60, "overview-60.txt"),
        (100, "overview.txt"),
    ] {
        assert_overview_at(width, name);
    }
}

/// A tab, named on the command line, rendered at a width.
fn render_tab(tab: &str, width: u16) -> String {
    render_tab_with("dark", tab, width)
}

/// A tab, rendered at a width in a named preset.
fn render_tab_with(preset: &str, tab: &str, width: u16) -> String {
    let width_arg = width.to_string();
    let output = run(&[
        "--render", "--plain", "--theme", preset, "--tab", tab, "--width", &width_arg, "--height",
        "40", WORK,
    ]);
    assert!(
        output.status.success(),
        "raul failed for {preset}/{tab} at width {width}: {output:?}"
    );
    String::from_utf8(output.stdout).expect("the plain render is UTF-8")
}

#[test]
fn the_settings_tab_is_reachable_in_render_mode_at_every_canonical_width() {
    for width in [40, 60, 100] {
        let text = render_tab("settings", width);
        assert_eq!(
            text.lines().count(),
            40,
            "the render did not take --height at width {width}"
        );
        // The tab bar names it, so the screen is the Settings tab.
        assert!(text.contains("Settings"), "width {width}: {text}");
        // It lists nothing yet, and says what it will hold.
        assert!(text.contains("nothing to set yet"), "width {width}: {text}");
        for words in ["keybinds", "sort keys", "theme"] {
            assert!(text.contains(words), "width {width}: {words} is missing");
        }
        assert!(!text.contains("not built yet"), "width {width}: {text}");
        // The footer names no key that does nothing on this tab.
        let footer = text.lines().last().expect("a footer row").trim_end();
        assert!(!footer.contains("enter"), "width {width}: {footer}");
        assert!(!footer.contains("open"), "width {width}: {footer}");
        for (number, line) in text.lines().enumerate() {
            let measured = display_width(line);
            assert!(
                measured <= width as usize,
                "row {number} is {measured} columns wide, not at most {width}: {line:?}"
            );
        }
    }
}

#[test]
fn the_row_tabs_fit_their_width_at_every_canonical_width() {
    // The selection bar is paint only: the plain render carries the same
    // words, and no line grows past the width it is drawn in.
    for tab in ["needs-you", "path", "backlog"] {
        for width in [40u16, 60, 100] {
            let text = render_tab(tab, width);
            assert_eq!(
                text.lines().count(),
                40,
                "{tab} at {width}: the render did not take --height"
            );
            for (number, line) in text.lines().enumerate() {
                assert_eq!(
                    line.trim_end_matches(' '),
                    line,
                    "{tab} at {width}: row {number} ends in spaces that carry no information"
                );
                let measured = display_width(line);
                assert!(
                    measured <= width as usize,
                    "{tab} at {width}: row {number} is {measured} columns wide: {line:?}"
                );
            }
        }
    }
}

/// Every tab the window has, by its command-line name.
const TAB_NAMES: [&str; 6] = [
    "overview",
    "needs-you",
    "path",
    "backlog",
    "autopilot",
    "settings",
];

#[test]
fn every_tab_renders_alike_in_nord_and_terminal_at_every_canonical_width() {
    // The plain render carries no colour, so the words are the whole
    // contract a preset keeps on screen. Nord is a dark preset: it changes
    // the paint, and the same screen says the same thing in every preset.
    // Terminal is the one that names no colour; it says the same words too.
    for tab in TAB_NAMES {
        for width in [40u16, 60, 100] {
            let dark = render_tab_with("dark", tab, width);
            for preset in ["nord", "terminal"] {
                let other = render_tab_with(preset, tab, width);
                assert_eq!(
                    other, dark,
                    "{preset} changes the words of {tab} at width {width}"
                );
                assert_eq!(
                    other.lines().count(),
                    40,
                    "{preset}/{tab} at {width}: the render did not take --height"
                );
            }
        }
    }
}

#[test]
fn the_tab_flag_needs_the_render_mode() {
    let output = run(&["--tab", "settings", WORK]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--tab without the render mode exited 0: {output:?}"
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tab"), "{stderr}");
    assert!(stderr.contains("--render"), "{stderr}");
}

#[test]
fn an_unknown_tab_is_refused_before_rendering() {
    let output = run(&["--render", "--plain", "--tab", "settingss", WORK]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unknown tab did not exit 1: {output:?}"
    );
    assert!(output.stdout.is_empty(), "a screen was printed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("settingss"), "{stderr}");
    assert!(stderr.contains("settings"), "{stderr}");
}

#[test]
fn two_runs_print_the_same_bytes() {
    let first = overview();
    let second = run(&["--plain", "--height=40", "--render", "--width=100", WORK]);
    assert!(first.status.success(), "the first run failed: {first:?}");
    assert!(second.status.success(), "the second run failed: {second:?}");
    assert_eq!(first.stdout, second.stdout, "two runs printed differently");
}

#[test]
fn the_default_size_is_100_by_40() {
    let explicit = overview();
    let default = run(&["--render", "--plain", WORK]);
    assert!(
        explicit.status.success(),
        "the sized run failed: {explicit:?}"
    );
    assert!(
        default.status.success(),
        "the default run failed: {default:?}"
    );
    assert_eq!(
        default.stdout, explicit.stdout,
        "the default size is not 100 by 40"
    );
}

#[test]
fn the_render_takes_the_requested_dimensions() {
    let width = 73;
    let height = 23;
    let output = run(&[
        "--render", "--plain", "--width", "73", "--height", "23", WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), height, "the render did not take --height");
    for (number, row) in rows.iter().enumerate() {
        let measured = display_width(row);
        assert!(
            measured <= width,
            "row {number} is {measured} columns wide, not at most {width}: {row:?}"
        );
    }
}

#[test]
fn a_wide_grapheme_costs_two_columns_not_three() {
    let dir = wide_task_work();
    let work = dir.to_str().expect("the work folder is UTF-8");
    let output = run(&[
        "--render", "--plain", "--width", "100", "--height", "40", work,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 40, "the render did not take --height");
    for (number, row) in rows.iter().enumerate() {
        let measured = display_width(row);
        assert!(
            measured <= 100,
            "row {number} is {measured} columns wide, not at most 100: {row:?}"
        );
    }
    let row = rows
        .iter()
        .find(|row| row.contains('界'))
        .expect("the wide title is not on screen");
    assert!(
        row.contains("TSK01"),
        "the wide title is not named: {row:?}"
    );
    assert_eq!(
        display_width(row),
        100,
        "the row of the wide title is not aligned: {row:?}"
    );
    assert!(
        row.ends_with('│'),
        "the right border is not on the row of the wide title: {row:?}"
    );
}

#[test]
fn unreadable_files_are_rendered_in_name_order() {
    // The malformed files are created in a different order from their names,
    // so the render can only be stable if the load orders them.
    let dir = support::malformed_work("render-unreadable-order");
    let work = dir.to_str().expect("the work folder is UTF-8");
    let output = run(&[
        "--render", "--plain", "--width", "100", "--height", "40", work,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let mut previous = 0;
    for name in support::NAME_ORDER {
        let at = text
            .find(name)
            .unwrap_or_else(|| panic!("{name} is not named: {text}"));
        assert!(at >= previous, "{name} is out of order: {text}");
        previous = at;
    }
}

#[test]
fn duplicate_ids_render_the_same_in_every_creation_order() {
    // Two folders with the same files, created in different orders, must
    // render the same bytes. Each file name is unique, so it breaks the id
    // tie the same way in every order; nothing here assumes the order the
    // filesystem hands back.
    let orders = permutations(support::DUPLICATE_ID_FILES.len());
    let mut first: Option<Vec<u8>> = None;
    for (index, order) in orders.iter().enumerate() {
        let project = duplicate_id_project(&format!("render-duplicate-order-{index}"), order);
        let output = run_in(
            &project,
            &[
                "--render", "--plain", "--width", "100", "--height", "40", "work",
            ],
        );
        assert!(
            output.status.success(),
            "raul failed for creation order {order:?}: {output:?}"
        );
        let text = String::from_utf8(output.stdout.clone()).expect("the plain render is UTF-8");
        // The Overview names only the first claimable task, so the duplicate
        // that wins the tie is the one on screen. Asserting the winner is
        // Alpha catches a load that hands Beta back first; a check for either
        // title would not.
        assert!(
            text.contains("Alpha task"),
            "Alpha task is not the rendered duplicate for creation order {order:?}: {text}"
        );
        assert!(
            !text.contains("Beta task"),
            "Beta task renders before its file name for creation order {order:?}: {text}"
        );
        match &first {
            None => first = Some(output.stdout),
            Some(expected) => assert_eq!(
                expected, &output.stdout,
                "creation order {order:?} rendered different bytes"
            ),
        }
        let _ = fs::remove_dir_all(&project);
    }
    assert!(first.is_some(), "there is no creation order to check");
}

#[test]
fn either_flag_alone_is_refused() {
    for flag in ["--render", "--plain"] {
        let output = run(&[flag, WORK]);
        assert!(!output.status.success(), "{flag} alone exited 0");
        assert!(output.stdout.is_empty(), "{flag} alone printed a screen");
    }
}

/// The task id whose title the detail tests show.
const DETAIL_TASK: &str = "TSK13";

/// The title of the detail task.
fn detail_title() -> String {
    let source = repo_root().join(WORK).join(Path::new(
        "TSK13-praesentium-voluptatum-deleniti-atque-corrupti.json",
    ));
    let json = fs::read_to_string(&source).expect("the detail task fixture");
    let task: serde_json::Value = serde_json::from_str(&json).expect("the fixture parses");
    task["title"].as_str().expect("a title").to_string()
}

#[test]
fn the_detail_renders_the_record_close_up() {
    let output = run(&[
        "--render",
        "--plain",
        "--detail",
        DETAIL_TASK,
        "--width",
        "100",
        "--height",
        "40",
        WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 40, "the render did not take --height");
    // The header: the id and the title, both on screen.
    assert!(text.contains(DETAIL_TASK), "{text}");
    assert!(text.contains(&detail_title()), "{text}");
    // The body is the document: the first sections are on display.
    assert!(text.contains("Header"), "{text}");
    assert!(text.contains("Intent"), "{text}");
    // The footer says what the pane does.
    assert!(text.contains("esc") && text.contains("close"), "{text}");
    // The pane is full screen: the window under it is not on display.
    assert!(!text.contains("Autopilot"), "{text}");
    assert!(!text.contains("Overview"), "{text}");
}

#[test]
fn the_detail_is_bounded_by_the_dimensions() {
    let title = detail_title();
    let output = run(&[
        "--render",
        "--plain",
        "--detail",
        DETAIL_TASK,
        "--width",
        "30",
        "--height",
        "12",
        WORK,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 12, "the render did not take --height");
    for (number, row) in rows.iter().enumerate() {
        let measured = display_width(row);
        assert!(
            measured <= 30,
            "row {number} is {measured} columns wide, not at most 30: {row:?}"
        );
    }
    // The title is cut, and the cut is marked: a part of it is never
    // shown as if it were the whole one.
    assert!(
        rows.iter().any(|row| row.contains('…')),
        "no cut is marked: {text}"
    );
    assert!(
        !text.contains(&title),
        "the whole title is shown where it does not fit: {text}"
    );
    assert!(text.contains(DETAIL_TASK), "{text}");
}

/// A work folder holding one task whose title is `count` joined `é`
/// graphemes: two scalars each, one column each.
fn combining_title_work() -> PathBuf {
    let title = "e\u{0301}".repeat(30);
    let source = repo_root()
        .join(WORK)
        .join("TSK29-lorem-ipsum-dolor-sit-amet.json");
    let task = fs::read_to_string(&source)
        .expect("the task the title is built from")
        .replace("\"TSK29\"", "\"TSK01\"")
        .replace("\"Lorem ipsum dolor sit amet\"", &format!("\"{title}\""))
        .replace(
            "\"depends_on\": [\n    \"TSK22\"\n  ]",
            "\"depends_on\": []",
        );
    let dir = std::env::temp_dir().join(format!("raul-combining-title-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a work folder");
    fs::write(dir.join("TSK01-combining.json"), task).expect("writing the task");
    dir
}

#[test]
fn a_combining_title_is_cut_on_grapheme_boundaries() {
    // The header holds more graphemes than the pane has columns for: the
    // cut keeps whole graphemes. A scalar cut would leave a dangling
    // combining mark behind, and the check can fail on one.
    let dir = combining_title_work();
    let work = dir.to_str().expect("the work folder is UTF-8");
    let output = run(&[
        "--render", "--plain", "--detail", "TSK01", "--width", "30", "--height", "12", work,
    ]);
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    for (number, row) in text.lines().enumerate() {
        for grapheme in ratatui::text::Span::raw(row.to_string())
            .styled_graphemes(ratatui::style::Style::default())
        {
            assert_ne!(
                grapheme.symbol, "\u{0301}",
                "row {number} cut a grapheme: {row:?}"
            );
        }
    }
    assert!(text.contains('…'), "no cut is marked: {text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_detail_id_is_refused_before_rendering() {
    let output = run(&[
        "--render", "--plain", "--detail", "TSK99", "--width", "100", "--height", "40", WORK,
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "an unknown id did not exit 1: {output:?}"
    );
    assert!(output.stdout.is_empty(), "a screen was printed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TSK99"), "{stderr}");
}

#[test]
fn the_detail_flag_needs_the_render_mode() {
    let output = run(&["--detail", DETAIL_TASK, WORK]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "--detail without the render mode exited 0: {output:?}"
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--detail"), "{stderr}");
}

#[test]
fn a_dimension_over_1000_is_refused_before_rendering() {
    for (flag, value) in [
        ("--width", "1001"),
        ("--height", "1001"),
        ("--width", "65535"),
        ("--height", "65535"),
    ] {
        let output = run(&["--render", "--plain", flag, value, WORK]);
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
        assert!(stderr.contains("1000"), "{flag} {value}: {stderr}");
        assert!(
            !stderr.contains("panicked") && !stderr.contains("memory allocation"),
            "{flag} {value} aborted: {stderr}"
        );
    }
}
