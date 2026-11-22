//! The rules a pick follows: what a typed id must look like, what a switch of
//! harness does to the rest of an agent, and what the flow lacks before it can
//! start.
//!
//! The window reads the catalogue and asks these what a pick changes. Nothing
//! here reads a tool or a file, so each rule is tested on its own.

use coding_crew_core::config::{Agent, Config};
use coding_crew_core::harness::{self, Harness};

use crate::catalog::{self, Catalog};

/// The last entry of a model list: it opens the inline input for an id the
/// catalogue does not list.
pub const TYPE_AN_ID: &str = "type an id…";

/// Whether a typed model id has the shape of one. The effort is a setting of
/// its own, so a `#variant` is not part of an id.
pub fn is_typed_id(text: &str) -> bool {
    let id = text.trim();
    catalog::is_model_id(id) && !id.contains('#')
}

/// Switch an agent to another harness.
///
/// The model stays only when the new harness lists it, and is cleared
/// otherwise, so the agent says it needs a model rather than running one the
/// tool does not know. The effort stays only when the new harness takes it,
/// and otherwise becomes the harness's default level.
pub fn switch_harness(agent: &mut Agent, to: &Harness, catalog: &Catalog) {
    agent.harness = to.name.to_string();
    if !catalog.knows(&agent.model) {
        agent.model.clear();
    }
    let kept = agent
        .effort
        .as_deref()
        .is_some_and(|effort| to.takes(effort));
    if !kept {
        agent.effort = Some(to.default_effort.to_string());
    }
}

/// What the configuration lacks before the flow can start, one phrase per gap,
/// named by the agent it is missing from. Empty when every agent has a
/// harness the table holds, a model and an effort.
pub fn gaps(config: &Config) -> Vec<String> {
    let mut gaps = Vec::new();
    for agent in &config.agents {
        if harness::find(&agent.harness).is_none() {
            gaps.push(format!("{} has no harness", agent.name));
        }
        if agent.model.is_empty() {
            gaps.push(format!("{} has no model", agent.name));
        }
        if agent.effort.is_none() {
            gaps.push(format!("{} has no effort", agent.name));
        }
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_crew_core::Role;
    use coding_crew_core::harness::Models;

    fn agent(harness: &str, model: &str, effort: Option<&str>) -> Agent {
        Agent {
            name: "builder".to_string(),
            role: Role::Runner,
            harness: harness.to_string(),
            model: model.to_string(),
            effort: effort.map(str::to_string),
        }
    }

    fn claude() -> &'static Harness {
        harness::find("claude").expect("claude")
    }

    fn opencode() -> &'static Harness {
        harness::find("opencode").expect("opencode")
    }

    #[test]
    fn a_typed_id_is_a_model_id_with_no_variant() {
        assert!(is_typed_id("opencode-go/glm-5.3"));
        assert!(is_typed_id("  openrouter/anthropic/claude-sonnet-4  "));
        assert!(is_typed_id("ollama/llama3:8b"));
        assert!(is_typed_id("claude-opus-5-5"));
        assert!(!is_typed_id(""));
        assert!(!is_typed_id("   "));
        assert!(!is_typed_id("glm 5.3"));
        assert!(!is_typed_id("opencode-go/glm-5.3#high"));
        assert!(!is_typed_id("opencode-go/"));
    }

    #[test]
    fn a_switch_keeps_a_model_the_new_harness_lists() {
        let catalog = Catalog::from_lines("opencode-go/glm-5.3\n");
        let mut builder = agent("claude", "opencode-go/glm-5.3", Some("high"));
        switch_harness(&mut builder, opencode(), &catalog);
        assert_eq!(builder.harness, "opencode");
        assert_eq!(builder.model, "opencode-go/glm-5.3");
        assert_eq!(builder.effort.as_deref(), Some("high"));
    }

    #[test]
    fn a_switch_clears_a_model_the_new_harness_does_not_list() {
        // Nothing is carried over silently: the model is cleared, and the row
        // says the agent needs one.
        let catalog = Catalog::from_lines("opencode-go/glm-5.3\n");
        let mut builder = agent("claude", "claude-haiku-5-5", Some("high"));
        switch_harness(&mut builder, opencode(), &catalog);
        assert_eq!(builder.harness, "opencode");
        assert_eq!(builder.model, "");
    }

    #[test]
    fn a_switch_resets_an_effort_the_new_harness_does_not_take() {
        // claude takes xhigh; opencode does not, so the effort falls to
        // opencode's default level.
        let catalog = Catalog::from_lines("opencode-go/glm-5.3\n");
        let mut builder = agent("claude", "opencode-go/glm-5.3", Some("xhigh"));
        switch_harness(&mut builder, opencode(), &catalog);
        assert_eq!(builder.effort.as_deref(), Some("medium"));
    }

    #[test]
    fn a_switch_gives_an_agent_with_no_effort_the_default_level() {
        let catalog = Catalog::from_lines("opencode-go/glm-5.3\n");
        let mut builder = agent("claude", "opencode-go/glm-5.3", None);
        switch_harness(&mut builder, opencode(), &catalog);
        assert_eq!(builder.effort.as_deref(), Some("medium"));
    }

    #[test]
    fn a_switch_to_a_harness_that_lists_nothing_clears_the_model() {
        let catalog = Catalog::default();
        let mut builder = agent("opencode", "opencode-go/glm-5.3", Some("low"));
        switch_harness(&mut builder, claude(), &catalog);
        assert_eq!(builder.model, "");
        assert_eq!(builder.effort.as_deref(), Some("low"));
    }

    #[test]
    fn the_curated_harness_lists_the_models_its_switch_keeps() {
        let Models::Curated(models) = claude().models else {
            panic!("claude's models are curated");
        };
        let catalog = Catalog::from_models(models);
        let mut builder = agent("opencode", "claude-haiku-5-5", Some("high"));
        switch_harness(&mut builder, claude(), &catalog);
        assert_eq!(builder.model, "claude-haiku-5-5");
    }

    #[test]
    fn the_default_configuration_has_nothing_missing() {
        assert!(gaps(&Config::default()).is_empty());
    }

    #[test]
    fn a_gap_names_the_agent_and_what_it_lacks() {
        let mut config = Config::default();
        config.agents[0].model.clear();
        config.agents[1].effort = None;
        assert_eq!(
            gaps(&config),
            ["builder has no model", "reviewer has no effort"]
        );
    }

    #[test]
    fn a_harness_the_table_does_not_hold_is_a_gap() {
        let mut config = Config::default();
        config.agents[0].harness = "aider".to_string();
        assert_eq!(gaps(&config), ["builder has no harness"]);
    }
}
