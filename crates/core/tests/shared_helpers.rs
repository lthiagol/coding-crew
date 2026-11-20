//! The shared helpers have one implementation, in `crates/core`.
//!
//! The plain render's plumbing — `plain_text`, `setup` and `restore` — and the
//! age words live once, and raul and hugo read them. A private copy in either
//! TUI is a second rule for the same fact, and the two drift: the age units
//! already said a hold under a minute differently before they moved. This
//! guard scans the source, so even an uncompiled copy is caught.

use std::fs;
use std::path::{Path, PathBuf};

mod common;

use common::{repo_root, working_files};

/// The helpers, by the definition a copy would carry.
const HELPERS: &[(&str, &str)] = &[
    ("plain_text", "fn plain_text("),
    ("setup", "fn setup("),
    ("restore", "fn restore("),
    ("age_text", "fn age_text("),
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

/// The `(file, helper)` pairs that define a copy of a shared helper.
fn copies(files: &[PathBuf]) -> Vec<String> {
    let mut copies = Vec::new();
    for path in files {
        let source = fs::read_to_string(path).expect("a source file");
        for (name, definition) in HELPERS {
            if source.contains(definition) {
                copies.push(format!("{}: {name}", path.display()));
            }
        }
    }
    copies
}

/// The definition text found in a file, when it defines the helper.
fn defines(source: &str, definition: &str) -> bool {
    source.contains(definition)
}

#[test]
fn no_tui_holds_a_second_copy_of_a_shared_helper() {
    let root = repo_root();
    let mut files = sources(&root, Path::new("crates/raul/src"));
    files.extend(sources(&root, Path::new("crates/hugo/src")));
    // The scan is not vacuous: the two trees hold the tools' sources.
    assert!(
        files.len() > 2,
        "the scan found no sources to read: {files:?}"
    );
    let copies = copies(&files);
    assert!(
        copies.is_empty(),
        "a private copy of a shared helper is back: {copies:?}"
    );
}

#[test]
fn each_helper_is_defined_once_in_core() {
    // The one implementation is in `crates/core`, one definition per helper:
    // a helper that left the TUI but never landed in core would build nowhere,
    // and a helper defined twice in core is two implementations again.
    let files = sources(&repo_root(), Path::new("crates/core/src"));
    assert!(!files.is_empty(), "the core scan found nothing to read");
    for (name, definition) in HELPERS {
        let found: Vec<String> = files
            .iter()
            .filter(|path| {
                let source = fs::read_to_string(path).expect("a source file");
                defines(&source, definition)
            })
            .map(|path| path.display().to_string())
            .collect();
        assert_eq!(found.len(), 1, "{name} is defined {found:?}");
    }
}

#[test]
fn both_tools_call_the_shared_helpers() {
    // The copies are gone and the tools still draw: raul's and hugo's frames
    // call the shared plumbing, and both tabs call the shared age words.
    let root = repo_root();
    let called: &[(&str, &[&str])] = &[
        (
            "crates/raul/src/main.rs",
            &["plain_text(", "setup()", "restore("],
        ),
        (
            "crates/hugo/src/main.rs",
            &["plain_text(", "setup()", "restore("],
        ),
        ("crates/raul/src/needs.rs", &["age_text("]),
        ("crates/hugo/src/app.rs", &["age_text("]),
    ];
    for (file, calls) in called {
        let source =
            fs::read_to_string(root.join(file)).unwrap_or_else(|error| panic!("{file}: {error}"));
        for call in *calls {
            assert!(source.contains(call), "{file} does not call {call}");
        }
    }
}
