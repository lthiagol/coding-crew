//! The configuration home: `~/.config/coding-crew/`, and the configuration
//! under it.
//!
//! hugo's policy lives on the tool's side, under the user's configuration
//! folder. The folder is the tool's own: when it is not there, hugo creates
//! it rather than failing. When `config.json` is not there, hugo writes the
//! defaults — the project's own flow — and shows them.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use coding_crew_core::config::{CONFIG_FILE, Config};

use crate::app::ReadError;

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

/// The configuration file under a user's home folder.
pub fn config_path(home: &Path) -> PathBuf {
    home_at(home).join(CONFIG_FILE)
}

/// What hugo found when it looked for its configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigRead {
    /// There is no home to hold a configuration: the named empty state.
    Absent,
    /// The configuration, as read — or as written when it was absent.
    Config(Config),
    /// The file is there but cannot be read, or cannot be written.
    Unreadable(ReadError),
}

/// Read the configuration under `home`, writing the defaults when the file is
/// absent.
///
/// The defaults are the project's own flow. The write is the tool's, not the
/// user's: only an absent file is written, so a configuration a human has
/// edited — even one that does not parse — is never overwritten.
pub fn load_at(home: &Path) -> ConfigRead {
    if let Err(error) = ensure_at(home) {
        return unreadable(error.to_string());
    }
    let path = config_path(home);
    if path.exists() {
        return match Config::read(&path) {
            Ok(config) => ConfigRead::Config(config),
            Err(error) => unreadable(error.to_string()),
        };
    }
    let config = Config::default();
    match config.write(&path) {
        Ok(()) => ConfigRead::Config(config),
        Err(error) => unreadable(error.to_string()),
    }
}

/// Read the running user's configuration.
///
/// A machine without a home has nowhere for a configuration to live, and that
/// is not a reason to fail: the sidebar says there is no configuration yet.
pub fn load_home() -> ConfigRead {
    match user_home() {
        Some(home) => load_at(&home),
        None => ConfigRead::Absent,
    }
}

fn unreadable(error: String) -> ConfigRead {
    ConfigRead::Unreadable(ReadError {
        file: CONFIG_FILE.to_string(),
        error,
    })
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

    #[test]
    fn a_configuration_that_is_absent_is_written_with_the_defaults() {
        let home = scratch("written");
        match load_at(&home) {
            ConfigRead::Config(config) => assert_eq!(config, Config::default()),
            other => panic!("{other:?}"),
        }
        let path = config_path(&home);
        assert!(path.is_file(), "{} is not a file", path.display());
        assert_eq!(
            Config::read(&path).expect("the configuration"),
            Config::default()
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_configuration_that_is_there_is_read_and_left_alone() {
        let home = scratch("kept");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let path = config_path(&home);
        let kept = Config {
            round_limit: 2,
            ..Config::default()
        };
        kept.write(&path).expect("the configuration");
        let before = fs::read_to_string(&path).expect("the file");
        match load_at(&home) {
            ConfigRead::Config(config) => assert_eq!(config, kept),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            before,
            "the load wrote over a configuration that was there"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_configuration_that_cannot_be_read_is_named_and_left_alone() {
        let home = scratch("broken");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let path = config_path(&home);
        fs::write(&path, "{ not a configuration").expect("a broken configuration");
        match load_at(&home) {
            ConfigRead::Unreadable(error) => {
                assert_eq!(error.file, CONFIG_FILE);
                assert!(error.error.contains("parse"), "{}", error.error);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            "{ not a configuration",
            "the load wrote over a configuration it could not read"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_home_that_cannot_be_made_is_an_unreadable_configuration() {
        // A file where the configuration folder must go is not an empty
        // state: the sidebar names the file and the error.
        let home = scratch("blocked-home");
        fs::write(home.join(CONFIG_DIR), "not a folder").expect("a blocking file");
        match load_at(&home) {
            ConfigRead::Unreadable(error) => {
                assert_eq!(error.file, CONFIG_FILE);
                assert!(!error.error.is_empty());
            }
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(&home);
    }
}
