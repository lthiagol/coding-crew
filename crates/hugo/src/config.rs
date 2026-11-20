//! The configuration home: `~/.config/coding-crew/`.
//!
//! hugo's policy lives on the tool's side, under the user's configuration
//! folder. The folder is the tool's own: when it is not there, hugo creates
//! it rather than failing. What it holds comes later.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The folder's name under `~/.config/`.
pub const APP_DIR: &str = "coding-crew";
/// The folder under the user's home that holds every tool's configuration.
pub const CONFIG_DIR: &str = ".config";

/// The configuration home under a user's home folder.
pub fn home_at(home: &Path) -> PathBuf {
    home.join(CONFIG_DIR).join(APP_DIR)
}

/// The running user's home folder, when there is one.
pub fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Create the configuration home under `home`, and return it.
///
/// The creation is the tool's, not the user's: an absent folder is made,
/// whole, and an existing one is left alone.
pub fn ensure_at(home: &Path) -> Result<PathBuf> {
    let dir = home_at(home);
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

/// Create the running user's configuration home, when a home is known.
///
/// A machine without a home folder is not an error: there is nowhere to put
/// the configuration, and the shell still shows the flow.
pub fn ensure_home() -> Result<Option<PathBuf>> {
    match user_home() {
        Some(home) => ensure_at(&home).map(Some),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A scratch home folder for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hugo-home-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch home");
        dir
    }

    #[test]
    fn the_home_is_the_config_folder_under_the_users_home() {
        let home = Path::new("/home/someone");
        assert_eq!(
            home_at(home),
            PathBuf::from("/home/someone")
                .join(".config")
                .join("coding-crew")
        );
    }

    #[test]
    fn an_absent_home_is_created() {
        let home = scratch("created");
        let dir = ensure_at(&home).expect("the home");
        assert_eq!(dir, home.join(".config").join("coding-crew"));
        assert!(dir.is_dir(), "the folder is not there");
        // The tool created the parent too, not only the leaf.
        assert!(home.join(".config").is_dir());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn creating_the_home_twice_leaves_it_alone() {
        let home = scratch("twice");
        let first = ensure_at(&home).expect("the first home");
        let marker = first.join("a-file");
        fs::write(&marker, "kept").expect("a file in the home");
        let second = ensure_at(&home).expect("the second home");
        assert_eq!(first, second);
        assert_eq!(
            fs::read_to_string(&marker).expect("the file is still there"),
            "kept"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_home_that_cannot_be_created_is_an_error() {
        // A file where the configuration folder must go is not silently
        // treated as an existing home.
        let home = scratch("blocked");
        let config = home.join(CONFIG_DIR);
        fs::write(&config, "not a folder").expect("a blocking file");
        let error = ensure_at(&home).expect_err("a file is not a folder");
        assert!(error.to_string().contains("coding-crew"), "{error}");
        let _ = fs::remove_dir_all(&home);
    }
}
