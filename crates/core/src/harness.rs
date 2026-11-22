//! The harnesses an agent runs in: the tools coding-crew starts agents in, and
//! what each one takes.
//!
//! This table is the only list of harnesses. The configuration names one by
//! its `name`; hugo and raul list its models and starts its agents through it.
//! An entry says how the tool is found, how herdr names its kind, how its
//! models are listed — and so which effort levels each model takes, since the
//! levels belong to the model — and how a model and an effort become the
//! arguments and environment that start an agent in it. The listing itself is
//! read by [`crate::catalog`].

use std::env;

use serde_json::json;

/// One harness: a tool an agent runs in.
#[derive(Debug, Clone, Copy)]
pub struct Harness {
    /// The name the configuration holds: `claude`, `opencode`, `cursor-agent`.
    pub name: &'static str,
    /// The program: found on `PATH`, and run to list its models.
    pub binary: &'static str,
    /// The agent kind herdr starts the tool as.
    pub kind: &'static str,
    /// Where the tool's models are listed, and the levels each one takes.
    pub models: Models,
    /// The arguments and environment that start an agent: the agent's name,
    /// its model and its effort, when it has one.
    start: fn(&str, &str, Option<&str>) -> Invocation,
}

/// Where a harness's models are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Models {
    /// The models the table holds, each with the levels it takes.
    Curated(&'static [CuratedModel]),
    /// The output of the harness's program with these arguments, read by
    /// `listing`.
    Command {
        args: &'static [&'static str],
        listing: Listing,
    },
}

/// A model the table holds, and the effort levels it takes, lowest first. An
/// empty list is a model that takes no level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CuratedModel {
    pub id: &'static str,
    pub efforts: &'static [&'static str],
}

/// How a harness's listing reads: the format its program prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// One model id per line, with an optional ` - label`. No levels.
    Lines,
    /// `cursor-agent --list-models`: one id per line. An id ending in a level
    /// (`claude-opus-5-5-high`) is that model at that level, so the model is
    /// the id without it.
    Cursor,
    /// `opencode api --standalone model.list`: JSON, one entry per model, with
    /// its `variants` as the levels.
    Opencode,
}

/// What starts an agent in a harness: the arguments after the program, and the
/// environment the tool reads its model from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Invocation {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// The supported harnesses. Nothing else is a harness.
pub const HARNESSES: [Harness; 3] = [CLAUDE, OPENCODE, CURSOR];

/// Claude's levels, for the models that take all five.
const FIVE: &[&str] = &["low", "medium", "high", "xhigh", "max"];
/// Claude's levels, for the models that take three.
const THREE: &[&str] = &["low", "medium", "high"];

const CLAUDE: Harness = Harness {
    name: "claude",
    binary: "claude",
    kind: "claude",
    models: Models::Curated(&[
        CuratedModel {
            id: "opus",
            efforts: FIVE,
        },
        CuratedModel {
            id: "sonnet",
            efforts: FIVE,
        },
        CuratedModel {
            id: "haiku",
            efforts: THREE,
        },
        CuratedModel {
            id: "fable",
            efforts: FIVE,
        },
        CuratedModel {
            id: "claude-opus-5-5",
            efforts: FIVE,
        },
        CuratedModel {
            id: "claude-sonnet-5-5",
            efforts: FIVE,
        },
        CuratedModel {
            id: "claude-haiku-5-5",
            efforts: THREE,
        },
        CuratedModel {
            id: "claude-fable-5-1",
            efforts: FIVE,
        },
    ]),
    start: claude_start,
};

const OPENCODE: Harness = Harness {
    name: "opencode",
    binary: "opencode",
    kind: "opencode",
    models: Models::Command {
        // The server's model list: `opencode models` names the same models,
        // but not their variants, which are the levels.
        args: &["api", "--standalone", "model.list"],
        listing: Listing::Opencode,
    },
    start: opencode_start,
};

const CURSOR: Harness = Harness {
    name: "cursor-agent",
    binary: "cursor-agent",
    kind: "cursor",
    models: Models::Command {
        args: &["--list-models"],
        listing: Listing::Cursor,
    },
    start: cursor_start,
};

/// claude takes the model and the effort as flags, and runs unattended in
/// its auto permission mode.
fn claude_start(_agent: &str, model: &str, effort: Option<&str>) -> Invocation {
    let mut args = words(&["--model", model]);
    if let Some(effort) = effort {
        args.extend(words(&["--effort", effort]));
    }
    args.extend(words(&["--permission-mode", "auto"]));
    Invocation {
        args,
        env: Vec::new(),
    }
}

/// opencode's interactive mode takes no model flag: the model is read from the
/// configuration in `OPENCODE_CONFIG_CONTENT`, where the effort is the model's
/// `#variant`. The agent is the one the configuration names as the default.
fn opencode_start(agent: &str, model: &str, effort: Option<&str>) -> Invocation {
    let model = match effort {
        Some(effort) => format!("{model}#{effort}"),
        None => model.to_string(),
    };
    let config = json!({
        "default_agent": agent,
        "agent": { (agent): { "mode": "primary", "model": model } },
    });
    Invocation {
        args: words(&["--standalone", "--auto"]),
        env: vec![("OPENCODE_CONFIG_CONTENT".to_string(), config.to_string())],
    }
}

/// cursor-agent lists a parameterised model as its id ending in the level, so
/// the effort is joined back on with a dash: `claude-opus-5-5` at `high` is
/// `claude-opus-5-5-high`. With no effort it is the bare id. It runs unattended
/// with its force flag.
fn cursor_start(_agent: &str, model: &str, effort: Option<&str>) -> Invocation {
    let model = match effort {
        Some(effort) => format!("{model}-{effort}"),
        None => model.to_string(),
    };
    let mut args = words(&["--model"]);
    args.push(model);
    args.extend(words(&["--force"]));
    Invocation {
        args,
        env: Vec::new(),
    }
}

fn words(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// The harness a configuration names, when the table has it.
pub fn find(name: &str) -> Option<&'static Harness> {
    HARNESSES.iter().find(|harness| harness.name == name)
}

/// The names of the harnesses, in the table's order.
pub fn names() -> Vec<&'static str> {
    HARNESSES.iter().map(|harness| harness.name).collect()
}

impl Harness {
    /// The arguments and environment that start `agent` on `model` at
    /// `effort`. An agent with no effort starts on the tool's own default, and
    /// so does one whose effort is empty: the `none` an unlisted model is given.
    pub fn invocation(&self, agent: &str, model: &str, effort: Option<&str>) -> Invocation {
        (self.start)(agent, model, effort.filter(|effort| !effort.is_empty()))
    }

    /// The levels a model the table holds takes, when the table holds it. A
    /// model read from a command is not in the table, so its levels are the
    /// catalogue's to say.
    pub fn curated_efforts(&self, model: &str) -> Option<&'static [&'static str]> {
        match self.models {
            Models::Curated(models) => models
                .iter()
                .find(|known| known.id == model)
                .map(|known| known.efforts),
            Models::Command { .. } => None,
        }
    }

    /// Whether the program is on `PATH`, so the tool can be run at all.
    pub fn on_path(&self) -> bool {
        env::var_os("PATH")
            .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join(self.binary).is_file()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_three_supported_harnesses_and_no_others() {
        assert_eq!(names(), ["claude", "opencode", "cursor-agent"]);
        assert!(find("aider").is_none());
        assert!(find("claude").is_some());
    }

    #[test]
    fn every_harness_has_a_program_a_kind_and_models() {
        for harness in HARNESSES {
            assert!(!harness.binary.is_empty(), "{}", harness.name);
            assert!(!harness.kind.is_empty(), "{}", harness.name);
            match harness.models {
                Models::Curated(models) => assert!(!models.is_empty(), "{}", harness.name),
                Models::Command { args, .. } => assert!(!args.is_empty(), "{}", harness.name),
            }
        }
    }

    #[test]
    fn every_harness_is_started_as_its_own_herdr_kind() {
        let kinds: Vec<&str> = HARNESSES.iter().map(|harness| harness.kind).collect();
        assert_eq!(kinds, ["claude", "opencode", "cursor"]);
    }

    #[test]
    fn claude_takes_five_levels_on_opus_sonnet_and_fable_and_three_on_haiku() {
        let claude = find("claude").expect("claude");
        for model in [
            "opus",
            "sonnet",
            "fable",
            "claude-opus-5-5",
            "claude-fable-5-1",
        ] {
            assert_eq!(
                claude.curated_efforts(model),
                Some(FIVE),
                "{model} takes five levels"
            );
        }
        for model in ["haiku", "claude-haiku-5-5"] {
            assert_eq!(claude.curated_efforts(model), Some(THREE), "{model}");
        }
        assert_eq!(claude.curated_efforts("claude-nothing-5"), None);
    }

    #[test]
    fn claude_lists_its_aliases_and_full_ids() {
        let Models::Curated(models) = find("claude").expect("claude").models else {
            panic!("claude's models are curated");
        };
        for model in [
            "opus",
            "sonnet",
            "haiku",
            "fable",
            "claude-opus-5-5",
            "claude-sonnet-5-5",
            "claude-haiku-5-5",
            "claude-fable-5-1",
        ] {
            assert!(
                models.iter().any(|known| known.id == model),
                "{model} is not listed"
            );
        }
    }

    #[test]
    fn opencode_and_cursor_list_their_models_from_their_own_command() {
        assert_eq!(
            find("opencode").expect("opencode").models,
            Models::Command {
                args: &["api", "--standalone", "model.list"],
                listing: Listing::Opencode,
            }
        );
        assert_eq!(
            find("cursor-agent").expect("cursor-agent").models,
            Models::Command {
                args: &["--list-models"],
                listing: Listing::Cursor,
            }
        );
    }

    #[test]
    fn a_command_harness_has_no_curated_levels() {
        // Their levels are read from the listing, not held in the table.
        assert_eq!(
            find("opencode")
                .expect("opencode")
                .curated_efforts("opencode-go/glm-5.3"),
            None
        );
    }

    #[test]
    fn claude_starts_with_its_model_effort_and_auto_permissions() {
        let claude = find("claude").expect("claude");
        assert_eq!(
            claude.invocation("builder", "claude-haiku-5-5", Some("high")),
            Invocation {
                args: words(&[
                    "--model",
                    "claude-haiku-5-5",
                    "--effort",
                    "high",
                    "--permission-mode",
                    "auto",
                ]),
                env: Vec::new(),
            }
        );
    }

    #[test]
    fn claude_without_an_effort_takes_its_own_default() {
        let claude = find("claude").expect("claude");
        assert_eq!(
            claude.invocation("builder", "opus", None).args,
            ["--model", "opus", "--permission-mode", "auto"]
        );
    }

    #[test]
    fn an_empty_effort_starts_with_no_effort_flag() {
        // The `none` an unlisted model is given is stored empty: it starts as
        // an agent with no effort does, on every harness.
        for harness in HARNESSES {
            assert_eq!(
                harness.invocation("builder", "some-model", Some("")),
                harness.invocation("builder", "some-model", None),
                "{}",
                harness.name
            );
        }
    }

    #[test]
    fn opencode_reads_its_model_and_effort_from_the_configuration_in_its_environment() {
        let opencode = find("opencode").expect("opencode");
        let started = opencode.invocation("builder", "opencode-go/glm-5.3", Some("high"));
        assert_eq!(started.args, ["--standalone", "--auto"]);
        assert_eq!(started.env.len(), 1);
        let (key, value) = &started.env[0];
        assert_eq!(key, "OPENCODE_CONFIG_CONTENT");
        let config: serde_json::Value = serde_json::from_str(value).expect("the config is JSON");
        assert_eq!(
            config,
            json!({
                "default_agent": "builder",
                "agent": {
                    "builder": {
                        "mode": "primary",
                        "model": "opencode-go/glm-5.3#high",
                    }
                }
            })
        );
    }

    #[test]
    fn opencode_without_an_effort_names_the_model_alone() {
        let opencode = find("opencode").expect("opencode");
        let started = opencode.invocation("reviewer", "opencode-go/glm-5.3", None);
        let config: serde_json::Value =
            serde_json::from_str(&started.env[0].1).expect("the config is JSON");
        assert_eq!(config["agent"]["reviewer"]["model"], "opencode-go/glm-5.3");
    }

    #[test]
    fn cursor_starts_a_model_at_the_id_its_listing_names_for_the_level() {
        // The listed id is the model and its level joined by a dash: that is
        // the id cursor-agent accepts. A bracketed `[effort=…]` is refused.
        let cursor = find("cursor-agent").expect("cursor-agent");
        assert_eq!(
            cursor.invocation("builder", "claude-opus-5-5", Some("high")),
            Invocation {
                args: words(&["--model", "claude-opus-5-5-high", "--force"]),
                env: Vec::new(),
            }
        );
        assert_eq!(
            cursor
                .invocation("builder", "gpt-5.3-codex", Some("low"))
                .args,
            ["--model", "gpt-5.3-codex-low", "--force"]
        );
        assert_eq!(
            cursor.invocation("builder", "auto", None).args,
            ["--model", "auto", "--force"]
        );
        assert_eq!(
            cursor.invocation("builder", "gpt-5.3-codex", None).args,
            ["--model", "gpt-5.3-codex", "--force"]
        );
    }

    #[test]
    fn a_program_that_is_not_on_the_path_is_not_found() {
        let missing = Harness {
            name: "missing",
            binary: "coding-crew-no-such-harness-binary",
            kind: "missing",
            models: Models::Command {
                args: &[],
                listing: Listing::Lines,
            },
            start: claude_start,
        };
        assert!(!missing.on_path());
    }
}
