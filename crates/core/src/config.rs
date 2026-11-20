//! The flow's configuration: who acts, in what, and in which order.
//!
//! The configuration is **provisional**: the flow has not written one yet, and
//! the format may change when it does. It lives on the tool's side, at
//! `~/.config/coding-crew/config.json`, because it is policy rather than work.
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
pub const CONFIG_FILE: &str = "config.json";

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
        Ok(serde_json::from_str(&text)?)
    }

    /// Write a configuration, atomically, as the store writes a task.
    pub fn write(&self, path: &Path) -> Result<(), ConfigError> {
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }
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
    #[error("failed to parse the configuration: {0}")]
    Parse(#[from] serde_json::Error),
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
    fn a_partial_configuration_is_unreadable_rather_than_filled_in() {
        // A file that does not say the order does not get the default order
        // silently: the tool says it cannot read the file.
        let error = serde_json::from_str::<Config>(r#"{"round_limit": 5}"#)
            .expect_err("a partial configuration is not a configuration");
        assert!(error.to_string().contains("missing field"), "{error}");
    }

    #[test]
    fn a_role_and_a_status_keep_their_stored_words() {
        let json = serde_json::to_string(&Config::default()).expect("the configuration");
        for word in [
            "\"runner\"",
            "\"reviewer\"",
            "\"orchestrator\"",
            "\"creator\"",
            "\"code-review\"",
            "\"needs-human\"",
        ] {
            assert!(json.contains(word), "{word}: {json}");
        }
    }
}
