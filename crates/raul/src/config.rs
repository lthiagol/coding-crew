//! raul's configuration: the `[raul]` table of `config.toml`.
//!
//! The file is the flow's too, under `~/.config/coding-crew/`, and hugo reads
//! it at launch. raul reads its own table at startup, and writes the defaults
//! when the file is absent, as hugo does. A file raul cannot use does not stop
//! it: raul starts on the defaults, and the one line that names the file and
//! the error is shown. A typo never draws the wrong palette without a word.
//!
//! The precedence is a flag, then the environment, then the file, then the
//! defaults. A flag or the environment that names nothing raul draws is
//! refused. The file's values are checked whether or not a flag beats them.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use coding_crew_core::config::{ConfigFile, Raul, config_path, home_at};
use coding_crew_theme::{IconSet, Preset, Theme};

use crate::tasks::{Key, Order};

/// The file a value is named by, in a message.
const FILE: &str = "config.toml";

/// The look and the Tasks order a window starts with.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub theme: Theme,
    pub icons: IconSet,
    pub order: Order,
    /// The one line that names what the file holds and raul could not use,
    /// and what it starts with instead. `None` when the file is usable.
    pub problem: Option<String>,
}

/// raul's table as read at startup, and the reason it could not be used.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub raul: Raul,
    /// The file could not be read, created or parsed: the table is the
    /// defaults, and this is the line that says why.
    pub problem: Option<String>,
}

/// raul's table under `home`, read at startup. The defaults are written first
/// when the file is absent. No home means there is nowhere to read from: the
/// defaults. A file that cannot be read is not written over: the defaults are
/// used, and the problem is kept with them.
pub fn load(home: Option<&Path>) -> Loaded {
    let Some(home) = home else {
        return Loaded {
            raul: Raul::default(),
            problem: None,
        };
    };
    match read(home) {
        Ok(raul) => Loaded {
            raul,
            problem: None,
        },
        Err(error) => Loaded {
            raul: Raul::default(),
            problem: Some(format!("{error:#}")),
        },
    }
}

/// raul's table in the file under `home`, after the file is created when it
/// is absent.
fn read(home: &Path) -> Result<Raul> {
    let dir = home_at(home);
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = config_path(home);
    let file = ConfigFile::open(&path).map_err(|error| anyhow!("{}: {error}", path.display()))?;
    Ok(file.config().raul.clone())
}

/// The look and the Tasks order in force.
///
/// The theme is the flag's, then the file's. The icon set is the flag's, then
/// the environment's (`env`, the variable's value), then the file's. A value
/// in the file that raul cannot use is named in `problem` and replaced by its
/// default, so the rest of the file still applies. The file's values are
/// checked even when a flag beats them.
pub fn resolve(
    theme: Option<Preset>,
    icons: Option<&str>,
    env: Option<&str>,
    loaded: &Loaded,
) -> Result<Look> {
    let raul = &loaded.raul;
    let mut problems: Vec<String> = loaded.problem.iter().cloned().collect();
    let file_theme = match preset_in_file(&raul.theme) {
        Ok(preset) => preset,
        Err(error) => {
            problems.push(error.to_string());
            Preset::Dark
        }
    };
    let file_icons = match icon_set_in_file(&raul.icons) {
        Ok(_) => Some(raul.icons.as_str()),
        Err(error) => {
            problems.push(error.to_string());
            None
        }
    };
    let key = match key_in_file(&raul.tasks.sort) {
        Ok(key) => key,
        Err(error) => {
            problems.push(error.to_string());
            Order::default().key
        }
    };
    let icons = IconSet::choose(icons, env, file_icons).map_err(|error| anyhow!("{error}"))?;
    Ok(Look {
        theme: Theme::preset(theme.unwrap_or(file_theme)),
        icons,
        order: Order {
            key,
            reverse: raul.tasks.reverse,
        },
        problem: (!problems.is_empty()).then(|| problems.join("; ")),
    })
}

fn preset_in_file(name: &str) -> Result<Preset> {
    Preset::parse(name).ok_or_else(|| {
        let names: Vec<&str> = Preset::ALL.iter().map(|preset| preset.name()).collect();
        anyhow!(
            "{FILE} [raul] theme: unknown theme {name}: the themes are {}",
            names.join(", ")
        )
    })
}

fn icon_set_in_file(name: &str) -> Result<IconSet> {
    IconSet::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = IconSet::ALL.iter().map(|set| set.name()).collect();
        anyhow!(
            "{FILE} [raul] icons: unknown icon set {name}: the sets are {}",
            names.join(", ")
        )
    })
}

fn key_in_file(name: &str) -> Result<Key> {
    Key::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = Key::ALL.iter().map(|key| key.name()).collect();
        anyhow!(
            "{FILE} [raul.tasks] sort: unknown key {name}: the keys are {}",
            names.join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use coding_crew_core::config::{CONFIG_FILE, Config, RaulTasks};

    use super::*;

    fn scratch_home(name: &str) -> std::path::PathBuf {
        let home = std::env::temp_dir().join(format!("raul-config-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).expect("a scratch home");
        home
    }

    fn with_raul(theme: &str, icons: &str, sort: &str, reverse: bool) -> Loaded {
        Loaded {
            raul: Raul {
                theme: theme.to_string(),
                icons: icons.to_string(),
                tasks: RaulTasks {
                    sort: sort.to_string(),
                    reverse,
                },
            },
            problem: None,
        }
    }

    #[test]
    fn the_defaults_are_the_look_without_a_flag_or_a_file() {
        let look =
            resolve(None, None, None, &with_raul("dark", "nerd", "id", false)).expect("the look");
        assert_eq!(
            look,
            Look {
                theme: Theme::preset(Preset::Dark),
                icons: IconSet::Nerd,
                order: Order::default(),
                problem: None,
            }
        );
    }

    #[test]
    fn the_flag_beats_the_file_for_the_theme() {
        let raul = with_raul("dracula", "nerd", "id", false);
        let look = resolve(Some(Preset::Nord), None, None, &raul).expect("the look");
        assert_eq!(look.theme, Theme::preset(Preset::Nord));
        let look = resolve(None, None, None, &raul).expect("the look");
        assert_eq!(look.theme, Theme::preset(Preset::Dracula));
    }

    #[test]
    fn the_flag_beats_the_environment_and_the_environment_beats_the_file() {
        let raul = with_raul("dark", "nerd", "id", false);
        let flag = resolve(None, Some("unicode"), Some("nerd"), &raul).expect("the look");
        assert_eq!(flag.icons, IconSet::Unicode);
        let env = resolve(None, None, Some("unicode"), &raul).expect("the look");
        assert_eq!(env.icons, IconSet::Unicode);
        let file = with_raul("dark", "unicode", "id", false);
        let file = resolve(None, None, None, &file).expect("the look");
        assert_eq!(file.icons, IconSet::Unicode);
        let empty = resolve(None, None, Some(""), &raul).expect("the look");
        assert_eq!(empty.icons, IconSet::Nerd, "an empty variable is unset");
    }

    #[test]
    fn the_file_sets_the_tasks_order() {
        let raul = with_raul("dark", "nerd", "created", true);
        let look = resolve(None, None, None, &raul).expect("the look");
        assert_eq!(
            look.order,
            Order {
                key: Key::Created,
                reverse: true,
            }
        );
    }

    #[test]
    fn a_value_raul_cannot_use_is_named_even_when_a_flag_beats_it() {
        // The flag still beats the file's theme, and the bad value is named
        // beside the look raul starts with.
        let theme = resolve(
            Some(Preset::Nord),
            None,
            None,
            &with_raul("light", "nerd", "id", false),
        )
        .expect("the look");
        assert_eq!(theme.theme, Theme::preset(Preset::Nord));
        let text = theme.problem.expect("the named value");
        assert!(
            text.contains(FILE) && text.contains("[raul] theme"),
            "{text}"
        );
        assert!(text.contains("light") && text.contains("nord"), "{text}");

        let icons = resolve(
            None,
            Some("unicode"),
            None,
            &with_raul("dark", "emoji", "id", false),
        )
        .expect("the look");
        assert_eq!(icons.icons, IconSet::Unicode);
        let text = icons.problem.expect("the named value");
        assert!(
            text.contains("[raul] icons") && text.contains("emoji"),
            "{text}"
        );

        let sort = resolve(
            None,
            None,
            None,
            &with_raul("dark", "nerd", "colour", false),
        )
        .expect("the look");
        assert_eq!(sort.order.key, Key::Id);
        let text = sort.problem.expect("the named value");
        assert!(
            text.contains("[raul.tasks] sort") && text.contains("colour"),
            "{text}"
        );
        assert!(text.contains("created"), "{text}");
    }

    #[test]
    fn a_bad_value_falls_back_alone_and_the_rest_of_the_file_applies() {
        let raul = with_raul("light", "nerd", "created", true);
        let look = resolve(None, None, None, &raul).expect("the look");
        assert_eq!(look.theme, Theme::preset(Preset::Dark));
        assert_eq!(
            look.order,
            Order {
                key: Key::Created,
                reverse: true,
            }
        );
        let text = look.problem.expect("the named value");
        assert!(text.contains("[raul] theme"), "{text}");
        assert!(!text.contains("sort"), "{text}");
    }

    #[test]
    fn a_problem_found_at_load_is_kept_on_the_look() {
        let loaded = Loaded {
            raul: Raul::default(),
            problem: Some("config.toml: syntax error".to_string()),
        };
        let look = resolve(None, None, None, &loaded).expect("the look");
        assert_eq!(look.theme, Theme::preset(Preset::Dark));
        assert_eq!(look.problem.as_deref(), Some("config.toml: syntax error"));
    }

    #[test]
    fn every_name_the_file_may_hold_is_one_raul_uses() {
        for preset in Preset::ALL {
            let raul = with_raul(preset.name(), "nerd", "id", false);
            let look = resolve(None, None, None, &raul).expect("the look");
            assert_eq!(look.problem, None, "{}", preset.name());
        }
        for key in Key::ALL {
            let raul = with_raul("dark", "nerd", key.name(), false);
            let look = resolve(None, None, None, &raul).expect("the key");
            assert_eq!(look.order.key, key);
            assert_eq!(look.problem, None, "{}", key.name());
        }
    }

    #[test]
    fn an_old_file_without_the_table_starts_with_the_defaults() {
        // The file predates `[raul]`: it loads with the defaults, and it is
        // not rewritten to add the table.
        let home = scratch_home("old");
        let text = Config::default().to_toml();
        let old = &text[..text.find("# raul's own settings").expect("the table")];
        fs::create_dir_all(home_at(&home)).expect("the home");
        fs::write(config_path(&home), old).expect("the old file");
        assert_eq!(
            load(Some(&home)),
            Loaded {
                raul: Raul::default(),
                problem: None,
            }
        );
        assert_eq!(
            fs::read_to_string(config_path(&home)).expect("the file"),
            old
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_absent_file_is_created_with_the_defaults() {
        let home = scratch_home("absent");
        assert_eq!(
            load(Some(&home)),
            Loaded {
                raul: Raul::default(),
                problem: None,
            }
        );
        assert_eq!(
            fs::read_to_string(config_path(&home)).expect("the file"),
            Config::default().to_toml()
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn no_home_is_the_defaults_and_nothing_is_written() {
        assert_eq!(
            load(None),
            Loaded {
                raul: Raul::default(),
                problem: None,
            }
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_named_with_its_path() {
        // raul starts on the defaults and names the file. The broken file is
        // left as it is: it is the human's to mend.
        let home = scratch_home("broken");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let broken = "round_limit = \n";
        fs::write(config_path(&home), broken).expect("a broken file");
        let loaded = load(Some(&home));
        assert_eq!(loaded.raul, Raul::default());
        let text = loaded.problem.expect("the named file");
        assert!(text.contains(CONFIG_FILE), "{text}");
        assert!(text.contains("syntax error"), "{text}");
        assert!(!text.contains('\n'), "the problem is one line: {text}");
        assert_eq!(
            fs::read_to_string(config_path(&home)).expect("the file"),
            broken,
            "the broken file is not written over"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_home_that_cannot_hold_the_folder_is_named_and_the_defaults_are_used() {
        // `.config` is a file, so the configuration folder cannot be made.
        let home = scratch_home("unwritable");
        fs::write(home.join(".config"), "").expect("a file where the folder goes");
        let loaded = load(Some(&home));
        assert_eq!(loaded.raul, Raul::default());
        let text = loaded.problem.expect("the named folder");
        assert!(text.starts_with("creating "), "{text}");
        assert!(text.contains("coding-crew"), "{text}");
        assert!(!text.contains('\n'), "the problem is one line: {text}");
        let _ = fs::remove_dir_all(&home);
    }
}
