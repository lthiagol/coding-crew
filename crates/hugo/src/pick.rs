//! The rules a pick follows: what a typed id must look like, what a switch of
//! harness or model does to the rest of an agent, and what the flow lacks
//! before it can start.
//!
//! The window reads the catalogue and asks these what a pick changes. Nothing
//! here reads a tool or a file, so each rule is tested on its own.

use coding_crew_core::config::Config;
use coding_crew_core::harness;

use crate::catalog;

pub use coding_crew_core::catalog::{
    Levels, TYPE_A_LEVEL, TYPE_AN_ID, effort_word, is_typed_level, keeps_effort, stored_effort,
    switch_harness,
};

/// Whether a typed model id has the shape of one. The effort is a setting of
/// its own, so a `#variant` is not part of an id.
pub fn is_typed_id(text: &str) -> bool {
    let id = text.trim();
    catalog::is_model_id(id) && !id.contains('#')
}

/// What the configuration lacks before the flow can start, one phrase per gap,
/// named by the agent it is missing from. Empty when every agent has a harness
/// the table holds, a model, and an effort its model takes — or a model that
/// takes none.
pub fn gaps(config: &Config, levels: &Levels) -> Vec<String> {
    let mut gaps = Vec::new();
    for agent in &config.agents {
        if harness::find(&agent.harness).is_none() {
            gaps.push(format!("{} has no harness", agent.role.as_str()));
        }
        if agent.model.is_empty() {
            gaps.push(format!("{} has no model", agent.role.as_str()));
        }
        if agent.effort.is_none() && !levels.takes_none(agent) {
            gaps.push(format!("{} has no effort", agent.role.as_str()));
        }
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use coding_crew_core::FlowRole;
    use coding_crew_core::catalog::NONE_ENTRY;
    use coding_crew_core::config::Agent;
    use coding_crew_core::harness::Harness;

    fn agent(harness: &str, model: &str, effort: Option<&str>) -> Agent {
        Agent {
            role: FlowRole::Builder,
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

    /// A listing in which `opencode-go/glm-5.3` takes low and high, and
    /// `opencode-go/kimi-k3` takes none.
    fn listing() -> Catalog {
        Catalog::from_curated(&[
            coding_crew_core::harness::CuratedModel {
                id: "opencode-go/glm-5.3",
                efforts: &["low", "high"],
            },
            coding_crew_core::harness::CuratedModel {
                id: "opencode-go/kimi-k3",
                efforts: &[],
            },
        ])
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
    fn a_switch_keeps_a_model_and_an_effort_the_new_harness_lists() {
        let mut builder = agent("claude", "opencode-go/glm-5.3", Some("high"));
        switch_harness(&mut builder, opencode(), &listing());
        assert_eq!(builder.harness, "opencode");
        assert_eq!(builder.model, "opencode-go/glm-5.3");
        assert_eq!(builder.effort.as_deref(), Some("high"));
    }

    #[test]
    fn a_switch_clears_a_model_the_new_harness_does_not_list() {
        // Nothing is carried over silently: the model is cleared, and the row
        // says the agent needs one.
        let mut builder = agent("claude", "claude-haiku-5-5", Some("high"));
        switch_harness(&mut builder, opencode(), &listing());
        assert_eq!(builder.harness, "opencode");
        assert_eq!(builder.model, "");
        assert_eq!(builder.effort, None);
    }

    #[test]
    fn a_switch_clears_an_effort_the_model_does_not_take() {
        // The model is kept, and takes low and high but not xhigh.
        let mut builder = agent("claude", "opencode-go/glm-5.3", Some("xhigh"));
        switch_harness(&mut builder, opencode(), &listing());
        assert_eq!(builder.model, "opencode-go/glm-5.3");
        assert_eq!(builder.effort, None);
    }

    #[test]
    fn a_switch_to_a_model_that_takes_no_level_clears_the_effort() {
        let mut builder = agent("claude", "opencode-go/kimi-k3", Some("low"));
        switch_harness(&mut builder, opencode(), &listing());
        assert_eq!(builder.model, "opencode-go/kimi-k3");
        assert_eq!(builder.effort, None);
    }

    #[test]
    fn a_switch_leaves_an_absent_effort_absent() {
        // An absent effort is a gap, not a level the switch fills.
        let mut builder = agent("claude", "opencode-go/glm-5.3", None);
        switch_harness(&mut builder, opencode(), &listing());
        assert_eq!(builder.effort, None);
    }

    #[test]
    fn a_switch_to_a_harness_that_lists_nothing_clears_the_model_and_the_effort() {
        let mut builder = agent("opencode", "opencode-go/glm-5.3", Some("low"));
        switch_harness(&mut builder, claude(), &Catalog::default());
        assert_eq!(builder.model, "");
        assert_eq!(builder.effort, None);
    }

    #[test]
    fn a_model_pick_keeps_an_effort_the_new_model_takes_and_clears_the_rest() {
        let low_and_high = ["low".to_string(), "high".to_string()];
        assert!(keeps_effort(Some("high"), Some(&low_and_high)));
        assert!(!keeps_effort(Some("xhigh"), Some(&low_and_high)));
        // A model that takes no level keeps nothing, and so does one whose
        // levels are not known.
        assert!(!keeps_effort(Some("high"), Some(&[])));
        assert!(!keeps_effort(Some("high"), None));
        assert!(!keeps_effort(None, Some(&low_and_high)));
    }

    #[test]
    fn the_curated_harness_lists_the_models_its_switch_keeps() {
        let levels = Levels::new();
        let catalog = levels.catalog("claude").expect("claude's table");
        let mut builder = agent("opencode", "claude-haiku-5-5", Some("high"));
        switch_harness(&mut builder, claude(), catalog);
        assert_eq!(builder.model, "claude-haiku-5-5");
        assert_eq!(builder.effort.as_deref(), Some("high"));
    }

    #[test]
    fn a_model_on_the_table_takes_the_levels_the_table_gives_it() {
        let levels = Levels::new();
        let haiku = agent("claude", "claude-haiku-5-5", Some("high"));
        assert_eq!(levels.of(&haiku).map(<[String]>::len), Some(3));
        assert!(!levels.takes_none(&haiku));
        // A harness nothing has read yet knows no levels.
        let cursor = agent("cursor-agent", "auto", None);
        assert_eq!(levels.of(&cursor), None);
        assert!(!levels.takes_none(&cursor));
    }

    #[test]
    fn the_default_configuration_has_nothing_missing() {
        assert!(gaps(&Config::default(), &Levels::new()).is_empty());
    }

    #[test]
    fn a_gap_names_the_agent_and_what_it_lacks() {
        let mut config = Config::default();
        config.agents[1].model.clear();
        config.agents[2].effort = None;
        assert_eq!(
            gaps(&config, &Levels::new()),
            ["builder has no model", "reviewer has no effort"]
        );
    }

    #[test]
    fn a_harness_the_table_does_not_hold_is_a_gap() {
        let mut config = Config::default();
        config.agents[1].harness = "aider".to_string();
        assert_eq!(gaps(&config, &Levels::new()), ["builder has no harness"]);
    }

    #[test]
    fn a_model_that_takes_no_level_needs_no_effort() {
        // The effort is absent, and the model's listing says it takes none:
        // the agent is not a gap.
        let mut levels = Levels::default();
        levels.insert("opencode", listing());
        let mut config = Config::default();
        config.agents[1].harness = "opencode".to_string();
        config.agents[1].model = "opencode-go/kimi-k3".to_string();
        config.agents[1].effort = None;
        assert!(gaps(&config, &levels).is_empty());
    }

    #[test]
    fn a_model_the_listing_names_bare_needs_no_effort() {
        // cursor-agent names gpt-5.3-codex bare as well as at its levels: with
        // no effort it runs on the bare id, so it is not a gap.
        let mut levels = Levels::default();
        levels.insert(
            "cursor-agent",
            coding_crew_core::harness::Listing::Cursor
                .parse("gpt-5.3-codex-low\ngpt-5.3-codex\n")
                .expect("a list"),
        );
        let mut config = Config::default();
        config.agents[1].harness = "cursor-agent".to_string();
        config.agents[1].model = "gpt-5.3-codex".to_string();
        config.agents[1].effort = None;
        assert!(gaps(&config, &levels).is_empty());
    }

    #[test]
    fn a_typed_model_is_started_once_its_effort_is_picked_from_its_list() {
        // An unlisted model has no levels of its own: picking `none` on it
        // stores an empty effort, which completes the role.
        let mut config = Config::default();
        config.agents[1].harness = "opencode".to_string();
        config.agents[1].model = "opencode-go/new-model".to_string();
        config.agents[1].effort = None;
        assert_eq!(gaps(&config, &Levels::default()), ["builder has no effort"]);
        config.agents[1].effort = Some(stored_effort(NONE_ENTRY, false));
        assert!(gaps(&config, &Levels::default()).is_empty());
        // A typed level completes it the same way.
        config.agents[1].effort = Some("high".to_string());
        assert!(gaps(&config, &Levels::default()).is_empty());
    }

    #[test]
    fn a_model_whose_levels_are_not_read_is_a_gap_until_they_are() {
        let mut config = Config::default();
        config.agents[1].harness = "opencode".to_string();
        config.agents[1].model = "opencode-go/glm-5.3".to_string();
        config.agents[1].effort = None;
        assert_eq!(gaps(&config, &Levels::default()), ["builder has no effort"]);
        let mut levels = Levels::default();
        levels.insert("opencode", listing());
        assert_eq!(gaps(&config, &levels), ["builder has no effort"]);
    }
}
