//! The flow's configuration: who acts, in what, and in which order.
//!
//! The configuration is **provisional**: the flow has not written one yet, and
//! the format may change when it does. It lives on the tool's side, at
//! `~/.config/coding-crew/config.toml`, because it is policy rather than work.
//! It is TOML rather than JSON because it is the one file a human edits, and
//! TOML carries comments: the defaults hugo writes explain themselves.
//! What it holds is settled in `docs/tasks/task-model.md` § 11.3: the agents
//! and their models, the harness they run in, the review-round limit and the
//! order — the project's own flow.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::actor::Role;
use crate::lifecycle::Status;

/// The configuration's file name under the tool's home.
pub const CONFIG_FILE: &str = "config.toml";
/// The file the configuration used before it was TOML. It is no longer read;
/// hugo names it when it finds one, so a human knows why it is ignored.
pub const LEGACY_CONFIG_FILE: &str = "config.json";

/// The flow's configuration, as the file holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// One entry per agent of the automation.
    pub agents: Vec<Agent>,
    /// The tool the agents run in, and the way it is driven.
    pub harness: Harness,
    /// The build-and-fix cycles a task gets.
    pub round_limit: u32,
    /// The order the flow follows: the steps and their gates.
    pub order: Vec<OrderStep>,
}

impl Config {
    /// Read a configuration from a file.
    pub fn read(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path)?;
        Self::from_toml(&text)
    }

    /// Read a configuration from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|error| ConfigError::Parse {
            at: error
                .span()
                .map(|span| location(text, span.start))
                .unwrap_or_default(),
            message: error.message().to_string(),
        })
    }

    /// Write a configuration, atomically, as the store writes a task.
    pub fn write(&self, path: &Path) -> Result<(), ConfigError> {
        let text = self.to_toml();
        let tmp = path.with_extension("toml.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The configuration as TOML, with the comments the file carries.
    ///
    /// Comments are the reason this format was chosen: each part says what it
    /// is, so a human who opens the file can read it without the model doc.
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "# coding-crew's flow configuration.\n\
             #\n\
             # This is the one file a human edits, so it is TOML and can carry\n\
             # comments. The format is provisional: it may change when the flow\n\
             # writes one itself. hugo reads this file at startup, writes the\n\
             # defaults only when it is absent, and never writes over a file it\n\
             # cannot read. The machine-written records - the journal and the\n\
             # runtime record - stay JSON.\n\
             \n",
        );
        out.push_str("# The build-and-fix cycles a task gets.\n");
        out.push_str(&format!("round_limit = {}\n\n", self.round_limit));
        if self.agents.is_empty() {
            out.push_str("# One entry per agent of the automation: none is configured.\n");
            out.push_str("agents = []\n\n");
        }
        // An empty order is a root key, not an array of tables: it has to be
        // written before the first table, where root keys still live.
        if self.order.is_empty() {
            out.push_str("# The order the flow follows: no step is configured.\n");
            out.push_str("order = []\n\n");
        }
        for (index, agent) in self.agents.iter().enumerate() {
            if index == 0 {
                out.push_str(
                    "# One entry per agent of the automation: the name its role config\n\
                     # gives it, the role it fills, and the model it runs.\n",
                );
            }
            out.push_str("[[agents]]\n");
            out.push_str(&format!("name = {}\n", quote(&agent.name)));
            out.push_str(&format!("role = {}\n", quote(agent.role.as_str())));
            out.push_str(&format!("model = {}\n\n", quote(&agent.model)));
        }
        out.push_str(
            "# The tool the agents run in, and the way it is driven.\n\
             [harness]\n",
        );
        out.push_str(&format!("tool = {}\n", quote(&self.harness.tool)));
        out.push_str(&format!("driven = {}\n\n", quote(&self.harness.driven)));
        for (index, step) in self.order.iter().enumerate() {
            if index == 0 {
                out.push_str(
                    "# The order the flow follows: the steps in the order they run, each\n\
                     # with the gates it passes, from one status to another.\n",
                );
            }
            out.push_str("[[order]]\n");
            out.push_str(&format!("step = {}\n", quote(&step.step)));
            out.push_str(&format!("role = {}\n", quote(step.role.as_str())));
            let gates: Vec<String> = step
                .gates
                .iter()
                .map(|gate| {
                    format!(
                        "{{ from = {}, to = {} }}",
                        quote(gate.from.as_str()),
                        quote(gate.to.as_str())
                    )
                })
                .collect();
            out.push_str(&format!("gates = [{}]\n\n", gates.join(", ")));
        }
        while out.ends_with("\n\n") {
            out.pop();
        }
        out
    }
}

/// A TOML string, escaped.
fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 || character == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

/// Where in a document a parse error sits, in words: ` at line 3, column 5`.
fn location(text: &str, index: usize) -> String {
    let before = text.get(..index).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map(|line| line.chars().count())
        .unwrap_or(0)
        + 1;
    format!(" at line {line}, column {column}")
}

impl Default for Config {
    /// The project's own flow: the pair it runs, the harness its panes run in,
    /// the budget it works to, and the order `docs/roles/roles.md` records.
    fn default() -> Self {
        Self {
            agents: vec![
                Agent {
                    name: "builder".to_string(),
                    role: Role::Runner,
                    model: "opencode-go/deepseek-v4.1-flash#high".to_string(),
                },
                Agent {
                    name: "reviewer".to_string(),
                    role: Role::Reviewer,
                    model: "opencode-go/glm-5.3#high".to_string(),
                },
                Agent {
                    name: "orchestrator".to_string(),
                    role: Role::Orchestrator,
                    model: "opencode-go/glm-5.3#high".to_string(),
                },
            ],
            harness: Harness {
                tool: "opencode".to_string(),
                driven: "--standalone · role config".to_string(),
            },
            round_limit: 5,
            order: vec![
                OrderStep {
                    step: "groom".to_string(),
                    role: Role::Creator,
                    gates: vec![gate(Status::Groomed, Status::Approved)],
                },
                OrderStep {
                    step: "build".to_string(),
                    role: Role::Runner,
                    gates: vec![gate(Status::Approved, Status::Executing)],
                },
                OrderStep {
                    step: "self-review".to_string(),
                    role: Role::Runner,
                    gates: vec![gate(Status::Executing, Status::CodeReview)],
                },
                OrderStep {
                    step: "review".to_string(),
                    role: Role::Reviewer,
                    gates: vec![
                        gate(Status::CodeReview, Status::Done),
                        gate(Status::CodeReview, Status::Remediating),
                        gate(Status::CodeReview, Status::NeedsHuman),
                    ],
                },
                OrderStep {
                    step: "remediate".to_string(),
                    role: Role::Runner,
                    gates: vec![gate(Status::Remediating, Status::CodeReview)],
                },
                OrderStep {
                    step: "re-review".to_string(),
                    role: Role::Reviewer,
                    gates: vec![gate(Status::CodeReview, Status::Done)],
                },
            ],
        }
    }
}

fn gate(from: Status, to: Status) -> Gate {
    Gate { from, to }
}

/// One agent of the automation: its name, the role it fills, and its model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The name its role config gives it — `builder`, `reviewer`.
    pub name: String,
    pub role: Role,
    /// The model it runs, provider and variant.
    pub model: String,
}

/// The tool the agents run in, and the way it is driven.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Harness {
    pub tool: String,
    /// The way the tool is driven, in a short phrase.
    pub driven: String,
}

/// One step of the order: its name, the role that does it, and the gates it
/// passes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderStep {
    pub step: String,
    pub role: Role,
    /// The transitions the step performs.
    pub gates: Vec<Gate>,
}

/// A transition of the lifecycle: the status it leaves and the one it reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    pub from: Status,
    pub to: Status,
}

impl Gate {
    /// The transition in words: `groomed → approved`.
    pub fn says(&self) -> String {
        format!("{} → {}", self.from.as_str(), self.to.as_str())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read the configuration: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error{at}: {message}")]
    Parse { at: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_projects_flow() {
        let config = Config::default();
        assert_eq!(config.round_limit, 5);
        assert_eq!(config.harness.tool, "opencode");
        assert!(!config.harness.driven.is_empty());
        // One agent per role the automation runs; the creator is the human's
        // grooming session and holds no agent.
        let roles: Vec<Role> = config.agents.iter().map(|agent| agent.role).collect();
        assert_eq!(
            roles,
            vec![Role::Runner, Role::Reviewer, Role::Orchestrator]
        );
        for agent in &config.agents {
            assert!(agent.model.contains('/'), "{}: {}", agent.name, agent.model);
        }
    }

    #[test]
    fn a_gate_says_the_transition_it_closes() {
        let gate = Gate {
            from: Status::CodeReview,
            to: Status::NeedsHuman,
        };
        assert_eq!(gate.says(), "code-review → needs-human");
    }

    #[test]
    fn a_configuration_round_trips_through_its_file() {
        let dir = std::env::temp_dir().join(format!("coding-crew-config-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        let path = dir.join(CONFIG_FILE);
        let config = Config::default();
        config.write(&path).expect("the configuration");
        let text = fs::read_to_string(&path).expect("the file");
        assert!(text.ends_with('\n'), "the file has no final newline");
        assert_eq!(Config::read(&path).expect("the record"), config);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_written_defaults_are_toml_and_explain_themselves() {
        let text = Config::default().to_toml();
        // A comment beside every part, and a header that says what the file
        // is: that is why the format is TOML.
        let comments: Vec<&str> = text.lines().filter(|line| line.starts_with('#')).collect();
        assert!(
            comments.len() >= 4,
            "the defaults carry no comments:\n{text}"
        );
        assert!(text.contains("provisional"), "{text}");
        for part in ["round_limit", "[[agents]]", "[harness]", "[[order]]"] {
            assert!(text.contains(part), "{part} is not in the shape:\n{text}");
        }
        // The comments are not a second copy of the shape: they sit above it.
        assert_eq!(
            Config::from_toml(&text).expect("the written shape parses"),
            Config::default()
        );
    }

    #[test]
    fn an_escaped_string_round_trips() {
        let config = Config {
            harness: Harness {
                tool: "a \"quoted\" \\ tool".to_string(),
                driven: "one\ttab\u{7f}".to_string(),
            },
            ..Config::default()
        };
        assert_eq!(
            Config::from_toml(&config.to_toml()).expect("the written shape parses"),
            config
        );
    }

    #[test]
    fn the_file_names_are_the_toml_one_and_the_old_json_one() {
        assert_eq!(CONFIG_FILE, "config.toml");
        assert_eq!(LEGACY_CONFIG_FILE, "config.json");
    }

    #[test]
    fn a_partial_configuration_is_unreadable_rather_than_filled_in() {
        // A file that does not say the order does not get the default order
        // silently: the tool says it cannot read the file.
        let error = Config::from_toml("round_limit = 5\n")
            .expect_err("a partial configuration is not a configuration");
        assert!(error.to_string().contains("parse"), "{error}");
        assert!(error.to_string().contains("missing field"), "{error}");
    }

    #[test]
    fn an_empty_crew_and_order_round_trip() {
        // The writer writes its own type, empty vectors included: an empty
        // list is a root key, written where root keys still live.
        let config = Config {
            agents: Vec::new(),
            order: Vec::new(),
            ..Config::default()
        };
        let text = config.to_toml();
        assert!(text.contains("agents = []"), "{text}");
        assert!(text.contains("order = []"), "{text}");
        assert_eq!(
            Config::from_toml(&text).expect("the written shape parses"),
            config
        );
    }

    #[test]
    fn a_hand_written_configuration_keeps_its_comments_beside_it() {
        // The point of TOML: a human adds a comment and a different value.
        let mine = Config::default().to_toml().replace(
            "# The build-and-fix cycles a task gets.\nround_limit = 5",
            "# Three rounds is enough for this project.\nround_limit = 3",
        );
        assert!(mine.contains("Three rounds"), "{mine}");
        let config = Config::from_toml(&mine).expect("a commented configuration");
        assert_eq!(config.round_limit, 3);
    }

    #[test]
    fn a_parse_error_names_where_it_is() {
        let error = Config::from_toml("round_limit = \n").expect_err("a broken value");
        let says = error.to_string();
        assert!(says.contains("parse"), "{says}");
        assert!(says.contains("line 1"), "{says}");
        assert!(says.contains("column"), "{says}");
    }

    #[test]
    fn an_old_json_configuration_is_not_a_toml_one() {
        // The old file is not read: its JSON text is not TOML.
        let json = serde_json::to_string(&Config::default()).expect("the old shape");
        let error = Config::from_toml(&json).expect_err("JSON is not the configuration");
        assert!(error.to_string().contains("parse"), "{error}");
    }

    #[test]
    fn a_role_and_a_status_keep_their_stored_words() {
        let toml = Config::default().to_toml();
        for word in [
            "\"runner\"",
            "\"reviewer\"",
            "\"orchestrator\"",
            "\"creator\"",
            "\"code-review\"",
            "\"needs-human\"",
        ] {
            assert!(toml.contains(word), "{word}: {toml}");
        }
    }
}
