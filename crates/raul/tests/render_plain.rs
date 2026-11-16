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

fn snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/overview.txt")
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
    let output = overview();
    assert!(output.status.success(), "raul failed: {output:?}");
    let text = String::from_utf8(output.stdout).expect("the plain render is UTF-8");
    let snapshot = fs::read_to_string(snapshot_path()).expect("the Overview snapshot");
    assert_eq!(
        text, snapshot,
        "the Overview render differs from tests/snapshots/overview.txt"
    );
    assert!(!text.contains('\u{1b}'), "the render carries ANSI escapes");
    assert!(text.ends_with('\n'), "the render has no final newline");
    for (number, line) in text.lines().enumerate() {
        assert_eq!(
            line.trim_end_matches(' '),
            line,
            "row {number} ends in spaces that carry no information"
        );
    }
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
