use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The root of the repository, from this test crate.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file name in a folder.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("reading the folder")
        .map(|entry| {
            entry
                .expect("a folder entry")
                .file_name()
                .into_string()
                .expect("a utf-8 file name")
        })
        .collect();
    names.sort();
    names
}

/// Removes the scratch folder when it is dropped, so a failed assertion
/// cannot leave it behind.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_generator_reproduces_the_committed_tree_byte_for_byte() {
    let root = repo_root();
    let source = root.join("mock-data/generate.py");
    let committed = root.join("mock-data/coding-lanes/work");
    let scratch =
        std::env::temp_dir().join(format!("coding-lanes-generate-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let _cleanup = Scratch(scratch.clone());
    fs::create_dir_all(&scratch).expect("a scratch folder");
    fs::copy(&source, scratch.join("generate.py")).expect("copying the generator");

    let output = Command::new("python3")
        .arg("generate.py")
        .current_dir(&scratch)
        .output()
        .expect("running the generator");
    assert!(
        output.status.success(),
        "the generator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = scratch.join("coding-lanes").join("work");
    let expected = names(&committed);
    assert_eq!(
        names(&generated),
        expected,
        "the generator writes a different set of files"
    );

    for name in expected {
        let committed_bytes = fs::read(committed.join(&name)).expect("reading a committed fixture");
        let generated_bytes = fs::read(generated.join(&name)).expect("reading a generated fixture");
        assert_eq!(
            generated_bytes, committed_bytes,
            "{name} differs from the committed fixture"
        );
    }
}
