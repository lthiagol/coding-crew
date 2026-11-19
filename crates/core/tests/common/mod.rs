//! The working tree, shared by the repository guards.

use std::fs;
use std::path::{Path, PathBuf};

/// The root of the repository, from this test crate.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file in the working tree, except the git directory and the build
/// directory.
pub fn working_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        let entries =
            fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == ".git" || name == "target" {
                continue;
            }
            let path = entry.path();
            let kind = entry.file_type().expect("a file type");
            if kind.is_dir() {
                walk(&path, files);
            } else if kind.is_file() {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    files
}
