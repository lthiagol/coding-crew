//! Whether `hugo` on PATH is the running hugo. Agents run `hugo --skill <role>`,
//! so `s` starts the orchestrator only when the name resolves to this binary.
//! A refusal names the folder holding the running hugo, to add to PATH.

use std::ffi::OsStr;
use std::path::Path;

/// The name agents run.
const NAME: &str = "hugo";

/// The refusal when no `hugo` is on PATH.
pub const MISSING: &str = "hugo is not on PATH";

/// The refusal when another `hugo` comes first on PATH.
pub const SHADOWED: &str = "another hugo comes first on PATH";

/// Check the PATH the window runs under against the running binary.
pub fn check_current() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("hugo's own path is not known: {error}"))?;
    check(&std::env::var_os("PATH").unwrap_or_default(), &exe)
}

/// `Ok` when the first executable `hugo` on `path` is `exe`, by file or by
/// symlink. Otherwise the refusal names the folder holding `exe`.
pub fn check(path: &OsStr, exe: &Path) -> Result<(), String> {
    let folder = exe.parent().unwrap_or(exe).display();
    let found = std::env::split_paths(path)
        .map(|dir| dir.join(NAME))
        .find(|candidate| is_executable(candidate));
    match found {
        None => Err(format!("{MISSING}: add {folder} to PATH")),
        Some(found) if same_file(&found, exe) => Ok(()),
        Some(found) => Err(format!(
            "{SHADOWED} ({}): put {folder} ahead of it",
            found.display()
        )),
    }
}

/// Whether two paths name one file, once symlinks are followed.
fn same_file(found: &Path, exe: &Path) -> bool {
    let exe = started_from(exe);
    match (found.canonicalize(), exe.canonicalize()) {
        (Ok(found), Ok(exe)) => found == exe,
        _ => found == exe,
    }
}

/// The path a running binary was started from. Linux names a binary replaced
/// on disk `<path> (deleted)`; `<path>` then holds the replacement.
fn started_from(exe: &Path) -> &Path {
    exe.to_str()
        .and_then(|text| text.strip_suffix(" (deleted)"))
        .map_or(exe, Path::new)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A fresh folder for a test, empty.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hugo-on-path-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    /// A `hugo` file in `dir`, executable as a binary is.
    fn binary(dir: &Path) -> PathBuf {
        fs::create_dir_all(dir).expect("a folder");
        let file = dir.join(NAME);
        fs::write(&file, "").expect("a file");
        set_mode(&file, 0o755);
        file
    }

    #[cfg(unix)]
    fn set_mode(file: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(file, fs::Permissions::from_mode(mode)).expect("a mode");
    }

    #[cfg(not(unix))]
    fn set_mode(_file: &Path, _mode: u32) {}

    fn path_of(dirs: &[&Path]) -> std::ffi::OsString {
        std::env::join_paths(dirs).expect("a PATH")
    }

    #[test]
    fn refuses_when_no_hugo_is_on_path_and_names_the_running_hugos_folder() {
        let root = scratch("missing");
        let bin = root.join("bin");
        let exe = binary(&bin);
        let empty = root.join("empty");
        fs::create_dir_all(&empty).expect("a folder");
        let why = check(&path_of(&[&empty]), &exe).expect_err("no hugo on PATH");
        assert!(why.starts_with(MISSING), "{why}");
        assert!(
            why.contains(&format!("add {} to PATH", bin.display())),
            "{why}"
        );
    }

    #[test]
    fn refuses_when_another_hugo_comes_first_and_names_the_running_hugos_folder() {
        let root = scratch("shadowed");
        let bin = root.join("bin");
        let exe = binary(&bin);
        let other = binary(&root.join("other"));
        let why = check(&path_of(&[other.parent().unwrap(), &bin]), &exe)
            .expect_err("another hugo first");
        assert!(why.starts_with(SHADOWED), "{why}");
        assert!(why.contains(&other.display().to_string()), "{why}");
        assert!(
            why.contains(&format!("put {} ahead", bin.display())),
            "{why}"
        );
    }

    #[test]
    fn starts_when_the_path_holds_this_hugo() {
        let root = scratch("this");
        let bin = root.join("bin");
        let exe = binary(&bin);
        assert_eq!(check(&path_of(&[&bin]), &exe), Ok(()));
    }

    #[test]
    fn starts_when_the_path_reaches_this_hugo_through_a_symlink() {
        let root = scratch("symlink");
        let bin = root.join("bin");
        let exe = binary(&bin);
        let links = root.join("links");
        fs::create_dir_all(&links).expect("a folder");
        std::os::unix::fs::symlink(&exe, links.join(NAME)).expect("a symlink");
        assert_eq!(check(&path_of(&[&links]), &exe), Ok(()));
    }

    #[test]
    fn starts_when_this_hugo_was_replaced_on_disk_while_running() {
        let root = scratch("replaced");
        let bin = root.join("bin");
        binary(&bin);
        // Linux reports the old file of a running binary with this suffix.
        let exe = bin.join(format!("{NAME} (deleted)"));
        assert_eq!(check(&path_of(&[&bin]), &exe), Ok(()));
    }

    #[test]
    fn a_hugo_that_is_not_executable_does_not_shadow_this_one() {
        let root = scratch("inert");
        let bin = root.join("bin");
        let exe = binary(&bin);
        let inert = root.join("inert");
        let file = binary(&inert);
        set_mode(&file, 0o644);
        assert_eq!(check(&path_of(&[&inert, &bin]), &exe), Ok(()));
    }
}
