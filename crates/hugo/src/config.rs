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

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use coding_crew_core::config::{CONFIG_FILE, Config, ConfigError, ConfigFile, LEGACY_CONFIG_FILE};
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
}

impl ConfigRead {
    /// A configuration and no old file beside it.
    pub fn of(state: ConfigState) -> Self {
        Self {
            state,
            legacy: None,
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
    ConfigRead { state, legacy }
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

/// Save the orchestrator's model into the configuration under `home`.
///
/// The pick goes into the file the human wrote: the file is read again, the
/// orchestrator's model is set, and only that value is written back, so every
/// comment and every other setting stays. When the file is gone, the
/// configuration in hand is written whole, with the pick. Returns the
/// configuration as saved, so the sidebar shows what the file holds.
pub fn save_pick_at(home: &Path, config: &Config, model: &str) -> Result<Config, ReadError> {
    let _ = ensure_at(home)
        .map_err(|error| unreadable(home_at(home).display().to_string(), error.to_string()))?;
    let path = config_path(home);
    let mut file = match ConfigFile::read(&path) {
        Ok(file) => file,
        Err(ConfigError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut whole = config.clone();
            if !whole.set_orchestrator_model(model) {
                return Err(no_orchestrator());
            }
            whole
                .write(&path)
                .map_err(|error| unreadable(CONFIG_FILE.to_string(), error.to_string()))?;
            return Ok(whole);
        }
        Err(error) => return Err(unreadable(CONFIG_FILE.to_string(), error.to_string())),
    };
    let mut picked = file.config().clone();
    if !picked.set_orchestrator_model(model) {
        return Err(no_orchestrator());
    }
    file.save(&picked)
        .map_err(|error| unreadable(CONFIG_FILE.to_string(), error.to_string()))?;
    Ok(file.config().clone())
}

fn no_orchestrator() -> ReadError {
    unreadable(
        CONFIG_FILE.to_string(),
        "the configuration has no orchestrator".to_string(),
    )
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
    fn a_pick_is_saved_into_the_text_the_human_wrote() {
        let home = scratch("pick");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let path = config_path(&home);
        let text = "\
# Mine: three cycles is enough.
round_limit = 3

# One entry per agent.
[[agents]]
name = \"builder\"
role = \"runner\"
model = \"opencode-go/deepseek-v4.1-flash#high\"

[[agents]]
name = \"orchestrator\"
role = \"orchestrator\"
model = \"opencode-go/glm-5.3#high\"

[harness]
tool = \"opencode\"
driven = \"--standalone\"

[[order]]
step = \"groom\"
role = \"creator\"
gates = [{ from = \"groomed\", to = \"approved\" }]
";
        let config = Config::from_toml(text).expect("the configuration");
        fs::write(&path, text).expect("a human's configuration");

        let saved = save_pick_at(&home, &config, "opencode-go/kimi-k3").expect("the pick");
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == coding_crew_core::Role::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("opencode-go/kimi-k3")
        );
        assert_eq!(saved.round_limit, 3, "the pick lost another setting");
        let after = fs::read_to_string(&path).expect("the file");
        assert!(after.contains("Mine: three cycles"), "{after}");
        assert!(after.contains("model = \"opencode-go/kimi-k3\""), "{after}");
        // The builder is the flow's: the pick did not touch it.
        assert!(
            after.contains("model = \"opencode-go/deepseek-v4.1-flash#high\""),
            "{after}"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pick_with_no_file_writes_the_configuration_whole() {
        let home = scratch("pick-absent");
        let mut config = Config::default();
        config.agents.iter_mut().for_each(|agent| {
            if agent.role == coding_crew_core::Role::Orchestrator {
                agent.model = "opencode-go/kimi-k3".to_string();
            }
        });
        let saved = save_pick_at(&home, &config, "opencode-go/kimi-k3").expect("the pick");
        let after = fs::read_to_string(config_path(&home)).expect("the file");
        assert!(after.contains("model = \"opencode-go/kimi-k3\""), "{after}");
        assert_eq!(saved, config);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pick_that_cannot_be_written_names_the_home() {
        let home = scratch("pick-blocked");
        fs::write(home.join(CONFIG_DIR), "not a folder").expect("a blocking file");
        let error = save_pick_at(&home, &Config::default(), "opencode-go/kimi-k3")
            .expect_err("the home cannot be made");
        assert_eq!(error.file, home_at(&home).display().to_string());
        assert!(!error.error.is_empty(), "{error:?}");
        let _ = fs::remove_dir_all(&home);
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
