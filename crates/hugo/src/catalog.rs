//! The model catalogue, as the window reads it: the core's, with the test
//! helper that stands a tool in for a harness.

#[cfg(test)]
pub(crate) use coding_crew_core::catalog::READ_BOUND;
pub use coding_crew_core::catalog::{Catalog, is_model_id};

/// A scratch folder holding an executable tool that answers with the script's
/// own words. The caller removes the folder.
#[cfg(test)]
pub(crate) fn scratch_tool(name: &str, script: &str) -> std::path::PathBuf {
    scratch_tool_under(&std::env::temp_dir(), name, script)
}

/// The same tool, under `base`: a test can hold it under a path long enough to
/// fill the choice's line.
#[cfg(test)]
pub(crate) fn scratch_tool_under(
    base: &std::path::Path,
    name: &str,
    script: &str,
) -> std::path::PathBuf {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let dir = base.join(format!("hugo-catalog-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    let path = dir.join("a-tool");
    std::fs::write(&path, script).expect("the tool");
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("an executable tool");
    path
}
