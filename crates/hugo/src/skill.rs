//! The skills `hugo --skill` prints: the text an agent reads to take one role
//! of the flow.
//!
//! The texts are markdown embedded in the binary. Each opens with the crew the
//! configuration in force names — the harness, each agent's model, the
//! review-round limit — because the texts refer to "the reviewer model above".
//! The configuration is read the way the window reads it, so the crew printed
//! is the crew hugo shows.

use anyhow::Result;

use coding_crew_core::Config;

use crate::catalog::model_args;
use crate::config::{ConfigRead, ConfigState};

/// The roles a skill is printed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillName {
    Orchestrator,
    Runner,
    Reviewer,
}

impl SkillName {
    /// Every role, in the order the list prints them.
    pub const ALL: [SkillName; 3] = [
        SkillName::Orchestrator,
        SkillName::Runner,
        SkillName::Reviewer,
    ];

    /// The role's name on the command line.
    pub fn name(self) -> &'static str {
        match self {
            SkillName::Orchestrator => "orchestrator",
            SkillName::Runner => "runner",
            SkillName::Reviewer => "reviewer",
        }
    }

    /// The role in one line, for the list.
    fn describe(self) -> &'static str {
        match self {
            SkillName::Orchestrator => "runs the build, review and fix loop",
            SkillName::Runner => "builds one job, and fixes what a review found",
            SkillName::Reviewer => "reviews the change and reports its actionable defects",
        }
    }

    /// The skill's text, as the file holds it.
    fn text(self) -> &'static str {
        match self {
            SkillName::Orchestrator => include_str!("../skills/orchestrator.md"),
            SkillName::Runner => include_str!("../skills/runner.md"),
            SkillName::Reviewer => include_str!("../skills/reviewer.md"),
        }
    }
}

/// What the command line asks a skill for: the list of roles, or one role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRequest {
    List,
    Role(SkillName),
}

/// The role a name on the command line stands for.
pub fn role_of(name: &str) -> Result<SkillName> {
    SkillName::ALL
        .into_iter()
        .find(|role| role.name() == name)
        .ok_or_else(|| {
            let names: Vec<&str> = SkillName::ALL.iter().map(|role| role.name()).collect();
            anyhow::anyhow!(
                "unknown skill role {name}: the roles are {}",
                names.join(", ")
            )
        })
}

/// The roles and what each one does, one line each.
pub fn list() -> String {
    let mut out = String::new();
    for role in SkillName::ALL {
        out.push_str(&format!("{:<13}{}\n", role.name(), role.describe()));
    }
    out.push_str("\nhugo --skill <role> prints that role's skill.\n");
    out
}

/// One role's skill, opened by the crew the configuration in force names.
///
/// A configuration that cannot be read, or a machine with no home, is not a
/// reason to fail: the defaults are in force, and one line says so.
pub fn for_role(role: SkillName, read: ConfigRead) -> String {
    let (config, note) = match read.state {
        ConfigState::Config(config) => (config, None),
        ConfigState::Absent => (
            Config::default(),
            Some("no configuration yet: the defaults are in force.".to_string()),
        ),
        ConfigState::Unreadable(error) => (
            Config::default(),
            Some(format!(
                "{} is unreadable: the defaults are in force.",
                error.file
            )),
        ),
    };
    format!("{}{}", crew(&config, note.as_deref()), role.text())
}

/// The crew section: the harness, each agent's role and model with the
/// arguments that start it, and the review-round limit.
fn crew(config: &Config, note: Option<&str>) -> String {
    let mut out = String::from("# Crew\n\n");
    if let Some(note) = note {
        out.push_str(note);
        out.push_str("\n\n");
    }
    out.push_str(&format!(
        "Harness: {}, driven as {}.\n",
        config.harness.tool, config.harness.driven
    ));
    out.push_str(&format!("Review-round limit: {}.\n\n", config.round_limit));
    for agent in &config.agents {
        out.push_str(&format!(
            "- {} ({}): {}\n",
            agent.name,
            agent.role.as_str(),
            agent.model
        ));
        let args = model_args(&config.harness.tool, &agent.model);
        out.push_str(&format!("  model args: {}\n", args.join(" ")));
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ReadError;

    #[test]
    fn the_list_names_the_three_roles_with_a_line_each() {
        let text = list();
        for role in ["orchestrator", "runner", "reviewer"] {
            let line = text
                .lines()
                .find(|line| line.starts_with(role))
                .unwrap_or_else(|| panic!("{role} is not listed:\n{text}"));
            assert!(line.len() > role.len() + 1, "{role} has no description");
        }
    }

    #[test]
    fn a_role_is_named_by_its_name_and_no_other() {
        for role in SkillName::ALL {
            assert_eq!(role_of(role.name()).expect("a role"), role);
        }
        let error = role_of("builder").expect_err("builder is an agent's name, not a role");
        for role in ["orchestrator", "runner", "reviewer"] {
            assert!(error.to_string().contains(role), "{error}");
        }
    }

    #[test]
    fn each_role_opens_with_the_crew_and_then_its_own_text() {
        for role in SkillName::ALL {
            let text = for_role(role, ConfigRead::of(ConfigState::Config(Config::default())));
            assert!(text.starts_with("# Crew\n"), "{text}");
            assert!(text.ends_with(role.text()), "{}", role.name());
            assert!(text.contains("Review-round limit: 5."), "{text}");
        }
    }

    #[test]
    fn the_crew_names_each_agent_with_its_model_and_start_args() {
        let text = crew(&Config::default(), None);
        assert!(text.contains("Harness: opencode, driven as --standalone · role config."));
        assert!(text.contains("- builder (runner): opencode-go/deepseek-v4.1-flash#high\n"));
        assert!(text.contains("- reviewer (reviewer): opencode-go/glm-5.3#high\n"));
        assert!(text.contains("- orchestrator (orchestrator): opencode-go/glm-5.3#high\n"));
        assert!(text.contains("  model args: --model opencode-go/glm-5.3#high\n"));
    }

    #[test]
    fn an_unreadable_configuration_is_the_defaults_and_one_line_says_so() {
        let read = ConfigRead::of(ConfigState::Unreadable(ReadError {
            file: "config.toml".to_string(),
            error: "syntax error".to_string(),
        }));
        let text = for_role(SkillName::Runner, read);
        assert!(
            text.contains("config.toml is unreadable: the defaults are in force.\n"),
            "{text}"
        );
        assert!(text.contains("- builder (runner): "), "{text}");
    }

    #[test]
    fn a_crew_with_no_agents_still_prints_the_header() {
        let mut config = Config::default();
        config.agents.clear();
        config.round_limit = 1;
        let text = crew(&config, None);
        assert!(!text.contains("- "), "{text}");
        assert!(text.contains("Review-round limit: 1."), "{text}");
    }
}
