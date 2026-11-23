//! The configuration home: `~/.config/coding-crew/`, and the configuration
//! under it.
//!
//! hugo's policy lives on the tool's side, under the user's configuration
//! folder. The folder is the tool's own: when it is not there, hugo creates
//! it rather than failing. When `config.toml` is not there, hugo writes the
//! defaults — the project's own flow — and shows them.
//!
//! The file is TOML because a human edits it, and a comment is how a
//! configuration explains itself. The old `config.json` is not read: when one
//! sits beside the configuration, hugo names it rather than ignoring it, and
//! converts nothing.
//!
//! hugo never writes the file after the defaults. Its picks are the session's:
//! they live in the window and in the environment of the orchestrator's tab,
//! and end with the session.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use coding_crew_core::config::{CONFIG_FILE, Config, LEGACY_CONFIG_FILE};
pub use coding_crew_core::config::{config_path, home_at, user_home};

use crate::app::ReadError;

/// Create the configuration home under `home`, and return it.
///
/// The creation is the tool's, not the user's: an absent folder is made,
/// whole, and an existing one is left alone.
pub fn ensure_at(home: &Path) -> Result<PathBuf> {
    let dir = home_at(home);
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

/// The state of the configuration itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigState {
    /// There is no home to hold a configuration: the named empty state.
    Absent,
    /// The configuration, as read — or as written when it was absent.
    Config(Config),
    /// The file is there but cannot be read, or cannot be written. The
    /// defaults are what the tool would use; the sidebar shows both.
    Unreadable(ReadError),
}

/// What hugo found when it looked for its configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRead {
    /// The configuration and its state.
    pub state: ConfigState,
    /// An old `config.json` beside it, named so nothing is ignored silently.
    pub legacy: Option<String>,
    /// What the file says that the flow does not run as written: a warning
    /// line each. The configuration is used; the sidebar names the warnings.
    pub warnings: Vec<String>,
}

impl ConfigRead {
    /// A configuration and no old file beside it.
    pub fn of(state: ConfigState) -> Self {
        Self {
            state,
            legacy: None,
            warnings: Vec::new(),
        }
    }
}

/// Read the configuration under `home`, writing the defaults when the file is
/// absent.
///
/// The defaults are the project's own flow. The write is the tool's, not the
/// user's: only an absent file is written, so a configuration a human has
/// edited — even one that does not parse — is never overwritten. An old
/// `config.json` is never read and never converted; it is named instead.
pub fn load_at(home: &Path) -> ConfigRead {
    let dir = match ensure_at(home) {
        Ok(dir) => dir,
        Err(error) => {
            // The home is what could not be made: naming the file inside it
            // would name a file the tool never touched.
            return ConfigRead::of(ConfigState::Unreadable(unreadable(
                home_at(home).display().to_string(),
                error.to_string(),
            )));
        }
    };
    let legacy = legacy_at(&dir);
    let path = config_path(home);
    // A dangling link is a configuration the human left: it is named as
    // unreadable, and the defaults are not written through it.
    let state = if path.symlink_metadata().is_ok() {
        match Config::read(&path) {
            Ok(config) => ConfigState::Config(config),
            Err(error) => {
                ConfigState::Unreadable(unreadable(CONFIG_FILE.to_string(), error.to_string()))
            }
        }
    } else {
        let config = Config::default();
        match config.write(&path) {
            Ok(()) => ConfigState::Config(config),
            Err(error) => {
                ConfigState::Unreadable(unreadable(CONFIG_FILE.to_string(), error.to_string()))
            }
        }
    };
    // The warnings are read from the file the configuration was read from.
    let warnings = match &state {
        ConfigState::Config(_) => fs::read_to_string(&path)
            .map(|text| Config::warnings(&text))
            .unwrap_or_default(),
        ConfigState::Absent | ConfigState::Unreadable(_) => Vec::new(),
    };
    ConfigRead {
        state,
        legacy,
        warnings,
    }
}

/// Read the configuration under a home, or the named empty state when the
/// machine has no home.
///
/// A machine without a home has nowhere for a configuration to live, and that
/// is not a reason to fail: the sidebar says there is no configuration yet.
pub fn load(home: Option<&Path>) -> ConfigRead {
    match home {
        Some(home) => load_at(home),
        None => ConfigRead::of(ConfigState::Absent),
    }
}

/// The environment variable a session's crew travels in. It holds the crew,
/// not an actor's session (ADR 0015): `s` sets it on the orchestrator's tab, so
/// the `hugo --skill` that tab runs prints the crew the session runs rather
/// than the file's.
pub const CREW_ENV: &str = "HUGO_CREW";

/// The crew a session runs, as the text the environment variable holds: the
/// configuration's four parts, as JSON. The file's own reader expects the
/// file's shape, so the parts are written and read one by one.
pub fn encode_crew(config: &Config) -> String {
    serde_json::json!({
        "agents": config.agents,
        "round_limit": config.round_limit,
        "order": config.order,
        "raul": config.raul,
    })
    .to_string()
}

/// The crew a session runs, read from the environment variable's text. Text
/// that is not a configuration is no crew.
pub fn decode_crew(text: &str) -> Option<Config> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let part = |key: &str| value.get(key).cloned();
    Some(Config {
        agents: serde_json::from_value(part("agents")?).ok()?,
        round_limit: serde_json::from_value(part("round_limit")?).ok()?,
        order: serde_json::from_value(part("order")?).ok()?,
        raul: serde_json::from_value(part("raul")?).ok()?,
    })
}

/// The configuration a skill is printed under: the session's crew when this
/// process runs in a session's tab, else the file as [`load`] reads it.
pub fn crew_or_load(home: Option<&Path>) -> ConfigRead {
    match std::env::var(CREW_ENV)
        .ok()
        .and_then(|text| decode_crew(&text))
    {
        Some(config) => ConfigRead::of(ConfigState::Config(config)),
        None => load(home),
    }
}

/// The name of the old configuration beside the new one, when it is there.
///
/// The old file is named by its existence, not by being a regular file: a
/// `config.json` that is a directory is still a file the human left behind.
fn legacy_at(dir: &Path) -> Option<String> {
    dir.join(LEGACY_CONFIG_FILE)
        .exists()
        .then(|| LEGACY_CONFIG_FILE.to_string())
}

fn unreadable(file: String, error: String) -> ReadError {
    ReadError { file, error }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use coding_crew_core::config::{CONFIG_DIR, LEGACY_CONFIG_FILE};

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
        assert_eq!(
            config_path(home),
            home_at(home).join("config.toml"),
            "the configuration is not the TOML file"
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
            ConfigRead {
                state: ConfigState::Config(config),
                legacy: None,
                ..
            } => assert_eq!(config, Config::default()),
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
            ConfigRead {
                state: ConfigState::Config(config),
                ..
            } => assert_eq!(config, kept),
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
        fs::write(&path, "round_limit = \n").expect("a broken configuration");
        match load_at(&home) {
            ConfigRead {
                state: ConfigState::Unreadable(error),
                ..
            } => {
                assert_eq!(error.file, CONFIG_FILE);
                assert!(error.error.contains("syntax"), "{}", error.error);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            "round_limit = \n",
            "the load wrote over a configuration it could not read"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_odd_table_loads_and_its_warning_is_kept_for_the_sidebar() {
        // A table the crew has no role for is read past, and the read says so.
        let home = scratch("odd-table");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let path = config_path(&home);
        let text = format!(
            "{}\n[agents.unused]\nharness = \"claude\"\nmodel = \"claude-haiku-5-5\"\n",
            Config::default().to_toml()
        );
        fs::write(&path, &text).expect("the configuration");
        let read = load_at(&home);
        assert!(
            matches!(&read.state, ConfigState::Config(config) if *config == Config::default()),
            "{:?}",
            read.state
        );
        assert_eq!(read.warnings.len(), 1, "{:?}", read.warnings);
        assert!(
            read.warnings[0].contains("[agents.unused]"),
            "{:?}",
            read.warnings
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_link_is_named_and_the_defaults_are_not_written_through_it() {
        let home = scratch("dangling");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let path = config_path(&home);
        let missing = home.join("gone").join("hugo.toml");
        std::os::unix::fs::symlink(&missing, &path).expect("a dangling link");
        match load_at(&home) {
            ConfigRead {
                state: ConfigState::Unreadable(error),
                ..
            } => assert_eq!(error.file, CONFIG_FILE),
            other => panic!("{other:?}"),
        }
        assert!(
            path.symlink_metadata()
                .expect("the link")
                .file_type()
                .is_symlink(),
            "the link was replaced"
        );
        assert!(
            !missing.exists(),
            "the defaults were written through the link"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_old_json_that_is_a_directory_is_still_named() {
        // The old file is named by its existence, not by being a regular
        // file: a human who left a directory there is still told.
        let home = scratch("legacy-dir");
        fs::create_dir_all(home_at(&home)).expect("the home");
        fs::create_dir_all(home_at(&home).join(LEGACY_CONFIG_FILE)).expect("a directory");
        match load_at(&home) {
            ConfigRead {
                legacy: Some(file), ..
            } => assert_eq!(file, LEGACY_CONFIG_FILE),
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_old_json_configuration_is_named_not_read() {
        let home = scratch("legacy");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let old = home_at(&home).join(LEGACY_CONFIG_FILE);
        let old_text = r#"{"round_limit": 9}"#;
        fs::write(&old, old_text).expect("an old configuration");

        match load_at(&home) {
            ConfigRead {
                state: ConfigState::Config(config),
                legacy: Some(file),
                ..
            } => {
                assert_eq!(file, LEGACY_CONFIG_FILE);
                // The old file is not read: the configuration is the
                // defaults, written beside it, not the nine rounds it holds.
                assert_eq!(config, Config::default());
            }
            other => panic!("{other:?}"),
        }
        assert!(old.is_file(), "the old file was removed instead of named");
        assert_eq!(
            fs::read_to_string(&old).expect("the old file"),
            old_text,
            "the old file was converted"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_crews_environment_variable_is_hugo_crew_and_not_a_session() {
        // The variable holds the crew, not an actor's session (ADR 0015), so
        // its name says crew.
        assert_eq!(CREW_ENV, "HUGO_CREW");
        assert_ne!(CREW_ENV, "HUGO_SESSION");
    }

    #[test]
    fn a_session_crew_travels_in_the_environment_and_reads_back_whole() {
        // The orchestrator's tab carries the session's crew as text; the skill
        // it prints reads the same crew back, with the roles it has.
        let mut crew = Config::default();
        crew.agents.truncate(2);
        crew.round_limit = 3;
        let text = encode_crew(&crew);
        assert!(!text.contains('\n'), "the value spans lines: {text}");
        assert_eq!(decode_crew(&text), Some(crew));
        assert_eq!(decode_crew("round_limit = 3"), None, "TOML is no crew");
    }

    #[test]
    fn a_home_that_cannot_be_made_is_an_unreadable_configuration() {
        // A file where the configuration folder must go is not an empty
        // state: the sidebar names the home and the error.
        let home = scratch("blocked-home");
        fs::write(home.join(CONFIG_DIR), "not a folder").expect("a blocking file");
        match load_at(&home) {
            ConfigRead {
                state: ConfigState::Unreadable(error),
                ..
            } => {
                assert_eq!(error.file, home_at(&home).display().to_string());
                assert!(!error.error.is_empty());
            }
            other => panic!("{other:?}"),
        }
        let _ = fs::remove_dir_all(&home);
    }
}
