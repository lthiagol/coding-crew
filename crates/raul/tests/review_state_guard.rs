//! The review state has one implementation, in `crates/raul/src/review_state.rs`.
//!
//! The wording, the severity mapping and the open-finding rule live once, and
//! the Needs you and Path tabs call them. This guard scans the source of every
//! file under `crates/raul/src`, so a copy under another name — or an
//! uncompiled one — is caught all the same.

use std::fs;
use std::path::{Path, PathBuf};

#[path = "../../core/tests/common/mod.rs"]
mod common;

use common::{repo_root, working_files};

/// The one implementation's own source, relative to the repository.
const ONE: &str = "crates/raul/src/review_state.rs";

/// The definitions a copy would carry: the severity mapping and the words of
/// the state. A copy has to say one of them to be a copy, whatever it is
/// called.
const DEFINITIONS: &[(&str, &str)] = &[
    ("the severity mapping", "Severity::High => \"high\""),
    ("the severity mapping", "Severity::Medium => \"medium\""),
    ("the severity mapping", "Severity::Low => \"low\""),
    (
        "the severity order",
        "Severity::High, Severity::Medium, Severity::Low",
    ),
    ("the findings wording", "\" open {}\""),
    ("the rounds wording", "\"/{} rounds\""),
];

/// Every `.rs` file under `dir`, from the working tree's own walk.
fn sources(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let dir = root.join(dir);
    working_files(root)
        .into_iter()
        .filter(|path| path.starts_with(&dir))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("rs"))
        .collect()
}

/// The `(file, definition)` pairs a source carries.
fn copies_in(name: &str, source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (fact, definition) in DEFINITIONS {
        if source.contains(definition) {
            found.push(format!("{name}: {fact}"));
        }
    }
    found
}

/// The copies the sources hold, outside the one implementation.
fn copies(root: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut copies = Vec::new();
    for path in files {
        let relative = path.strip_prefix(root).expect("a path under the root");
        if relative == Path::new(ONE) {
            continue;
        }
        let source = fs::read_to_string(path).expect("a source file");
        copies.extend(copies_in(&relative.display().to_string(), &source));
    }
    copies
}

#[test]
fn the_two_tabs_do_not_carry_their_own_copy_of_the_state() {
    let root = repo_root();
    let files = sources(&root, Path::new("crates/raul/src"));
    // The scan covers the whole tree, not a hand-picked list: every source
    // file under the directory is read.
    assert!(
        files.len() > 4,
        "the scan found no sources to read: {files:?}"
    );
    assert!(
        files
            .iter()
            .any(|path| path.strip_prefix(&root).ok() == Some(Path::new(ONE))),
        "the scan does not read the one implementation: {files:?}"
    );
    let copies = copies(&root, &files);
    assert!(
        copies.is_empty(),
        "a second copy of the review state: {copies:?}"
    );
}

#[test]
fn both_tabs_call_the_one_implementation() {
    let root = repo_root();
    for name in ["needs.rs", "path.rs"] {
        let source = fs::read_to_string(root.join("crates/raul/src").join(name))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        for call in [
            "review_state::state(",
            "review_state::fit(",
            "review_state::paint(",
        ] {
            assert!(source.contains(call), "{name} does not call {call}");
        }
    }
}

#[test]
fn a_planted_copy_under_another_name_is_caught() {
    // The copy is a source string, compiled by nothing: the pin must catch it
    // without the linker, the way a copy that never reaches a build is caught.
    let mapped = r#"
        fn my_words(severity: Severity) -> &'static str {
            match severity {
                Severity::High => "high",
                Severity::Medium => "medium",
                Severity::Low => "low",
            }
        }
    "#;
    assert!(
        !copies_in("planted.rs", mapped).is_empty(),
        "a copy under another name passed"
    );
    let worded = r#"
        fn my_state(open: usize, rounds: usize) -> String {
            format!("{open} open {}", if open == 1 { "finding" } else { "findings" })
        }
        fn my_rounds(rounds: usize, max: usize) -> String {
            format!("/{} rounds", max)
        }
    "#;
    assert!(
        !copies_in("planted.rs", worded).is_empty(),
        "a copy of the wording passed"
    );
}
