//! The configuration defaults to the project's own flow.
//!
//! The order the configuration ships with is the one `docs/roles/roles.md`
//! records, so the two cannot drift: editing the table alone fails the gate, and
//! so does editing the defaults alone. The models are the ones the flow's role
//! configs name.

use std::path::PathBuf;

use coding_crew_core::config::{Config, Gate};
use coding_crew_core::{Role, Status};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A status the doc names, by its stored word.
fn status_of(word: &str) -> Status {
    serde_json::from_str(&format!("\"{word}\""))
        .unwrap_or_else(|_| panic!("the doc names an unknown status: {word}"))
}

/// A role the doc names, by its stored word.
fn role_of(word: &str) -> Role {
    serde_json::from_str(&format!("\"{word}\""))
        .unwrap_or_else(|_| panic!("the doc names an unknown role: {word}"))
}

fn decks(word: &str) -> String {
    word.trim().trim_matches('`').trim().to_string()
}

/// The order table of `docs/roles/roles.md`: the numbered rows, in the order
/// they are read.
fn doc_order() -> Vec<(u32, String, Role, Vec<Gate>)> {
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
fn the_default_models_are_the_ones_the_flows_role_configs_name() {
    let config = Config::default();
    for (config_file, agent_name) in [("builder.json", "builder"), ("reviewer.json", "reviewer")] {
        let text = std::fs::read_to_string(repo_root().join("flow").join(config_file))
            .unwrap_or_else(|error| panic!("flow/{config_file}: {error}"));
        let flow: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("flow/{config_file}: {error}"));
        let model = flow["agents"][agent_name]["model"]
            .as_str()
            .unwrap_or_else(|| panic!("flow/{config_file} names no model for {agent_name}"));
        let agent = config
            .agents
            .iter()
            .find(|agent| agent.name == agent_name)
            .unwrap_or_else(|| panic!("the defaults name no {agent_name}"));
        assert_eq!(
            agent.model, model,
            "the defaults and flow/{config_file} differ"
        );
    }
}

#[test]
fn the_round_limit_is_the_budget_the_model_doc_names() {
    let config = Config::default();
    let doc = include_str!("../../../docs/tasks/task-model.md");
    assert!(
        doc.contains("five build-and-fix cycles"),
        "the model doc does not name the round limit"
    );
    assert_eq!(config.round_limit, 5);
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
    for role in [Role::Runner, Role::Reviewer, Role::Orchestrator] {
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
fn every_agent_runs_a_model_with_a_provider_and_a_variant() {
    // The sidebar shows both halves; a model without either is one the pane
    // could not name whole.
    for agent in Config::default().agents {
        let (provider, model) = agent
            .model
            .split_once('/')
            .unwrap_or_else(|| panic!("{} names no provider: {}", agent.name, agent.model));
        assert!(!provider.is_empty(), "{}", agent.name);
        assert!(
            model.contains('#'),
            "{} names no variant: {}",
            agent.name,
            model
        );
    }
}
