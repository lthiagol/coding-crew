//! The configuration defaults to the project's own flow.
//!
//! The order the configuration ships with is the one `docs/roles/roles.md`
//! records, so the two cannot drift: editing the table alone fails the gate, and
//! so does editing the defaults alone. The flow's role configs name their agents
//! only; the models are the configuration's, pinned by its own unit tests.

use std::path::PathBuf;

use coding_crew_core::config::{Config, Gate};
use coding_crew_core::harness::{self, Models};
use coding_crew_core::{FlowRole, Status};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A status the doc names, by its stored word.
fn status_of(word: &str) -> Status {
    serde_json::from_str(&format!("\"{word}\""))
        .unwrap_or_else(|_| panic!("the doc names an unknown status: {word}"))
}

/// A role the doc names, by its stored word.
fn role_of(word: &str) -> FlowRole {
    serde_json::from_str(&format!("\"{word}\""))
        .unwrap_or_else(|_| panic!("the doc names an unknown role: {word}"))
}

fn decks(word: &str) -> String {
    word.trim().trim_matches('`').trim().to_string()
}

/// The order table of `docs/roles/roles.md`: the numbered rows, in the order
/// they are read.
fn doc_order() -> Vec<(u32, String, FlowRole, Vec<Gate>)> {
    let doc = include_str!("../../../docs/roles/roles.md");
    let mut rows = Vec::new();
    for line in doc.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').skip(1).collect();
        let Some(number) = cells
            .first()
            .and_then(|cell| cell.trim().parse::<u32>().ok())
        else {
            continue;
        };
        assert!(cells.len() >= 4, "an order row is short: {line}");
        let step = decks(cells[1]);
        let role = role_of(&decks(cells[2]));
        let gates: Vec<Gate> = cells[3]
            .split(',')
            .map(decks)
            .filter(|gate| !gate.is_empty())
            .map(|gate| {
                let (from, to) = gate
                    .split_once('→')
                    .unwrap_or_else(|| panic!("a gate is not a transition: {gate}"));
                Gate {
                    from: status_of(from.trim()),
                    to: status_of(to.trim()),
                }
            })
            .collect();
        rows.push((number, step, role, gates));
    }
    rows
}

#[test]
fn the_default_order_is_the_one_roles_md_records() {
    let rows = doc_order();
    assert!(!rows.is_empty(), "roles.md records no order");
    let defaults = Config::default().order;
    assert_eq!(
        rows.len(),
        defaults.len(),
        "the doc and the defaults differ"
    );
    for (index, ((number, step, role, gates), default)) in rows.iter().zip(&defaults).enumerate() {
        assert_eq!(*number, index as u32 + 1, "the doc's numbering: {step}");
        assert_eq!(*step, default.step, "step {}", number);
        assert_eq!(*role, default.role, "the role of {step}");
        assert_eq!(*gates, default.gates, "the gates of {step}");
    }
}

#[test]
fn the_flow_files_name_their_agents_and_no_model() {
    // A role config names the agent it runs and nothing else: the model is the
    // configuration's, so a flow file cannot pin one that the defaults do not.
    let config = Config::default();
    for (config_file, agent_name) in [("builder.json", "builder"), ("reviewer.json", "reviewer")] {
        let text = std::fs::read_to_string(repo_root().join("flow").join(config_file))
            .unwrap_or_else(|error| panic!("flow/{config_file}: {error}"));
        let flow: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("flow/{config_file}: {error}"));
        let agent = &flow["agents"][agent_name];
        assert!(
            agent.is_object(),
            "flow/{config_file} names no {agent_name}"
        );
        assert!(
            agent.get("model").is_none(),
            "flow/{config_file} pins a model: the configuration names it"
        );
        assert!(
            config
                .agents
                .iter()
                .any(|agent| agent.role.as_str() == agent_name),
            "the defaults name no {agent_name}"
        );
    }
}

#[test]
fn the_round_limit_is_the_models_budget() {
    // The provenance is the model's own round budget, not a phrase the
    // configuration unit wrote about itself.
    let config = Config::default();
    let doc = include_str!("../../../docs/tasks/task-model.md");
    assert!(
        doc.contains("`review.max_rounds` is `5` and counts **external** rounds"),
        "the model doc does not name the round budget"
    );
    assert_eq!(config.round_limit, 5);
}

#[test]
fn the_model_doc_shows_the_configurations_toml_shape() {
    // The configuration is TOML, and the doc that defines its shape shows it:
    // the file, the format, and a shape a human can copy.
    let doc = include_str!("../../../docs/tasks/task-model.md");
    for fact in [
        "config.toml",
        "config.json",
        "TOML",
        "[agents.builder]",
        "harness = ",
        "[[order]]",
        "provisional",
    ] {
        assert!(doc.contains(fact), "the model doc does not name {fact:?}");
    }
}

#[test]
fn a_role_runs_the_same_model_in_the_configuration_and_in_the_crew() {
    // The sidebar names the configured model and the main pane names the one
    // the run's record says: one role, one model, or the screen contradicts
    // itself. The mock's crew is the record the pinned screens show.
    let runtime =
        coding_crew_core::Runtime::read(&repo_root().join("mock-data/coding-crew/runtime.json"))
            .expect("the mock's runtime record");
    let config = Config::default();
    for role in [
        FlowRole::Builder,
        FlowRole::Reviewer,
        FlowRole::Orchestrator,
    ] {
        let configured = config
            .agents
            .iter()
            .find(|agent| agent.role == role)
            .unwrap_or_else(|| panic!("the defaults name no {role:?}"));
        let crew = runtime
            .crew
            .iter()
            .find(|member| member.role == role)
            .unwrap_or_else(|| panic!("the crew names no {role:?}"));
        assert_eq!(
            configured.model, crew.model,
            "{role:?} runs two models: the configuration and the crew"
        );
    }
}

#[test]
fn every_agent_runs_a_model_its_harness_lists_at_an_effort_it_takes() {
    // The harness table is the only list of harnesses, and a model is one the
    // harness lists: a configuration that names neither cannot start.
    for agent in Config::default().agents {
        let harness = harness::find(&agent.harness).unwrap_or_else(|| {
            panic!(
                "{} runs in no harness: {}",
                agent.role.as_str(),
                agent.harness
            )
        });
        if let Models::Curated(models) = harness.models {
            assert!(
                models.iter().any(|known| known.id == agent.model),
                "{} runs {} on a model it does not list: {}",
                agent.role.as_str(),
                harness.name,
                agent.model
            );
        }
        let effort = agent
            .effort
            .unwrap_or_else(|| panic!("{} names no effort", agent.role.as_str()));
        // A command harness's levels are read from its listing, not here.
        if let Some(levels) = harness.curated_efforts(&agent.model) {
            assert!(
                levels.contains(&effort.as_str()),
                "{}: {effort}",
                agent.role.as_str()
            );
        }
    }
}
