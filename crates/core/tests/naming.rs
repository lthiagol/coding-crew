//! The project's name, pinned across the working tree.
//!
//! A rename is only done when no file still carries the old name. The test
//! walks the working tree, tracked or not and ignored or not, so a half-done
//! rename fails the gate while it is still in flight.

use std::fs;

mod common;

use common::{repo_root, working_files};

/// The parts of the project's former name.
const OLD_PARTS: [&str; 2] = ["coding", "lanes"];

/// The project's former name, in both spellings, built from its parts so that
/// this test does not itself contain the strings it forbids.
fn old_names() -> [String; 2] {
    [OLD_PARTS.join("-"), OLD_PARTS.join("_")]
}

/// The GitHub remote keeps its name by decision, so the URL that records it is
/// not a use of the old project name.
fn allowed_remote() -> String {
    format!("https://github.com/lthiagol/{}.git", OLD_PARTS.join("-"))
}

#[test]
fn no_file_in_the_working_tree_still_says_the_old_project_name() {
    let root = repo_root();
    let olds = old_names();
    let allowed = allowed_remote();

    let mut offenders: Vec<String> = Vec::new();
    for path in working_files(&root) {
        let name = path
            .strip_prefix(&root)
            .expect("a path under the repository")
            .display()
            .to_string();
        let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let text = String::from_utf8_lossy(&bytes).replace(&allowed, "");
        for (number, line) in text.lines().enumerate() {
            if olds.iter().any(|old| line.contains(old.as_str())) {
                offenders.push(format!("{name}:{}: {}", number + 1, line.trim()));
                break;
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "the old project name is still in the working tree:\n{}",
        offenders.join("\n")
    );
}
