//! `raul --version` through the real binary: it prints the workspace version
//! and exits, as the root manifest names it.

use std::path::Path;
use std::process::Command;

/// The workspace version: the `version` under `[workspace.package]` in the
/// root manifest.
fn workspace_version() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let text = std::fs::read_to_string(&manifest).expect("the root manifest");
    let mut in_package = false;
    for line in text.lines() {
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package
            && let Some(value) = line
                .strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        {
            return value.to_string();
        }
    }
    panic!("the root manifest names no workspace version");
}

#[test]
fn version_prints_the_workspace_version() {
    let want = format!("raul {}\n", workspace_version());
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_raul"))
            .arg(flag)
            .output()
            .expect("raul runs");
        assert!(output.status.success(), "{flag}: {:?}", output.status);
        assert_eq!(String::from_utf8_lossy(&output.stdout), want, "{flag}");
    }
}
