//! The harnesses an agent runs in: the tools coding-crew starts agents in, and
//! what each one takes.
//!
//! This table is the only list of harnesses. The configuration names one by
//! its `name`; hugo lists its models and starts its agents through it, and
//! raul offers its names and efforts. An entry says how the tool is found,
//! how herdr names its kind, the efforts it takes, how its models are listed,
//! and how a model and an effort become the arguments and environment that
//! start an agent in it.

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
    /// The effort levels the tool takes, lowest first.
    pub efforts: &'static [&'static str],
    /// Where the tool's models are listed.
    pub models: Models,
    /// The arguments and environment that start an agent: the agent's name,
    /// its model and its effort, when it has one.
    start: fn(&str, &str, Option<&str>) -> Invocation,
}

/// Where a harness's models are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Models {
    /// The ids the table holds.
    Curated(&'static [&'static str]),
    /// The output of the harness's program with these arguments, one model per
    /// line.
    Command(&'static [&'static str]),
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

const CLAUDE: Harness = Harness {
    name: "claude",
    binary: "claude",
    kind: "claude",
    efforts: &["low", "medium", "high", "xhigh", "max"],
    models: Models::Curated(&[
        "opus",
        "sonnet",
        "haiku",
        "fable",
        "claude-opus-5-5",
        "claude-sonnet-5-5",
        "claude-haiku-5-5",
        "claude-fable-5-1",
    ]),
    start: claude_start,
};

const OPENCODE: Harness = Harness {
    name: "opencode",
    binary: "opencode",
    kind: "opencode",
    efforts: &["low", "medium", "high"],
    models: Models::Command(&["models"]),
    start: opencode_start,
};

const CURSOR: Harness = Harness {
    name: "cursor-agent",
    binary: "cursor-agent",
    kind: "cursor",
    efforts: &["low", "medium", "high"],
    models: Models::Command(&["--list-models"]),
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

/// cursor-agent takes the effort in the model's brackets, and runs unattended
/// with its force flag.
fn cursor_start(_agent: &str, model: &str, effort: Option<&str>) -> Invocation {
    let model = match effort {
        Some(effort) => format!("{model}[effort={effort}]"),
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
    /// `effort`. An agent with no effort starts on the tool's own default.
    pub fn invocation(&self, agent: &str, model: &str, effort: Option<&str>) -> Invocation {
        (self.start)(agent, model, effort)
    }

    /// Whether the harness takes an effort by this name.
    pub fn takes(&self, effort: &str) -> bool {
        self.efforts.contains(&effort)
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
    fn every_harness_has_a_program_a_kind_efforts_and_models() {
        for harness in HARNESSES {
            assert!(!harness.binary.is_empty(), "{}", harness.name);
            assert!(!harness.kind.is_empty(), "{}", harness.name);
            assert!(!harness.efforts.is_empty(), "{}", harness.name);
            match harness.models {
                Models::Curated(models) => assert!(!models.is_empty(), "{}", harness.name),
                Models::Command(args) => assert!(!args.is_empty(), "{}", harness.name),
            }
        }
    }

    #[test]
    fn every_harness_is_started_as_its_own_herdr_kind() {
        let kinds: Vec<&str> = HARNESSES.iter().map(|harness| harness.kind).collect();
        assert_eq!(kinds, ["claude", "opencode", "cursor"]);
    }

    #[test]
    fn the_efforts_are_the_levels_each_tool_takes() {
        assert_eq!(
            find("claude").expect("claude").efforts,
            ["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            find("cursor-agent").expect("cursor-agent").efforts,
            ["low", "medium", "high"]
        );
        assert!(find("claude").expect("claude").takes("xhigh"));
        assert!(!find("cursor-agent").expect("cursor-agent").takes("xhigh"));
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
            assert!(models.contains(&model), "{model} is not listed");
        }
    }

    #[test]
    fn opencode_and_cursor_list_their_models_from_their_own_command() {
        assert_eq!(
            find("opencode").expect("opencode").models,
            Models::Command(&["models"])
        );
        assert_eq!(
            find("cursor-agent").expect("cursor-agent").models,
            Models::Command(&["--list-models"])
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
    fn cursor_takes_the_effort_in_the_models_brackets() {
        let cursor = find("cursor-agent").expect("cursor-agent");
        assert_eq!(
            cursor.invocation("builder", "gpt-5.3-codex", Some("low")),
            Invocation {
                args: words(&["--model", "gpt-5.3-codex[effort=low]", "--force"]),
                env: Vec::new(),
            }
        );
        assert_eq!(
            cursor.invocation("builder", "auto", None).args,
            ["--model", "auto", "--force"]
        );
    }

    #[test]
    fn a_program_that_is_not_on_the_path_is_not_found() {
        let missing = Harness {
            name: "missing",
            binary: "coding-crew-no-such-harness-binary",
            kind: "missing",
            efforts: &["low"],
            models: Models::Command(&[]),
            start: claude_start,
        };
        assert!(!missing.on_path());
    }
}
