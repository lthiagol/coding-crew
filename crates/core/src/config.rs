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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
    ///
    /// A document that is not TOML and a document whose value has the wrong
    /// type are different failures: the first is a syntax error, the second
    /// is a shape error. The document is parsed once to tell them apart.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        match toml::from_str(text) {
            Ok(config) => Ok(config),
            Err(error) => {
                let at = error
                    .span()
                    .map(|span| location(text, span.start))
                    .unwrap_or_default();
                let message = error.message().to_string();
                if toml::from_str::<toml::Value>(text).is_err() {
                    Err(ConfigError::Syntax { at, message })
                } else {
                    Err(ConfigError::Shape { at, message })
                }
            }
        }
    }

    /// Write a configuration, atomically, as the store writes a task.
    pub fn write(&self, path: &Path) -> Result<(), ConfigError> {
        Self::write_text(path, &self.to_toml())
    }

    /// Write TOML text, atomically, as the store writes a task.
    ///
    /// The temporary name is unique to the write: two hugo instances on one
    /// home must not rename each other's file.
    pub fn write_text(path: &Path, text: &str) -> Result<(), ConfigError> {
        let tmp = temp_path(path);
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The configuration's text with the orchestrator's model replaced.
    ///
    /// The configuration is the one file a human edits, so a pick is written
    /// into the text they wrote: the orchestrator's table keeps every other
    /// line, comments included. `None` when the text holds no orchestrator
    /// table or no `model` line in it.
    pub fn with_orchestrator_model(text: &str, model: &str) -> Option<String> {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        // Each `[[agents]]` table runs from its header to the next table's
        // header, or to the end of the file. The model line is searched for
        // inside the orchestrator's own table only, never past it.
        let mut tables: Vec<(usize, usize)> = Vec::new();
        let mut open: Option<usize> = None;
        for (index, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with('[') {
                if let Some(start) = open.take() {
                    tables.push((start, index));
                }
                open = line.trim_start().starts_with("[[agents]]").then_some(index);
            }
        }
        if let Some(start) = open {
            tables.push((start, lines.len()));
        }
        let (start, end) = tables
            .into_iter()
            .find(|(start, end)| is_orchestrator(&lines[*start..*end]))?;
        let at = model_line(&lines[start..end]).map(|line| start + line)?;
        let line = lines[at];
        let indent = &line[..line.len() - line.trim_start().len()];
        let comment = comment_of(line);
        let ending = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let mut out = String::with_capacity(text.len() + model.len());
        for (index, line) in lines.iter().enumerate() {
            if index == at {
                out.push_str(indent);
                out.push_str(&format!("model = {}{comment}{ending}", quote(model)));
            } else {
                out.push_str(line);
            }
        }
        Some(out)
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
             # defaults only when it is absent, saves the orchestrator's picked\n\
             # model in place, and never writes over a file it cannot read. The\n\
             # machine-written records - the journal and the runtime record -\n\
             # stay JSON.\n\
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

/// A temporary name beside `path`, unique to the write.
fn temp_path(path: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".to_string());
    let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(".{name}.{}.{nonce}.tmp", std::process::id()))
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

/// Whether an agent table's lines name the orchestrator.
fn is_orchestrator(lines: &[&str]) -> bool {
    lines.iter().any(|line| {
        pair(line).is_some_and(|(key, value)| {
            key == "role" && unquote(value) == Some(Role::Orchestrator.as_str())
        })
    })
}

/// A TOML string's contents, whether it is basic or literal.
///
/// A hand-edited file may use single quotes (`role = 'orchestrator'`); the
/// value is the same value. A value that is not one quoted string is `None`.
fn unquote(value: &str) -> Option<&str> {
    let first = value.chars().next()?;
    if first != '"' && first != '\'' {
        return None;
    }
    let last = value.chars().last()?;
    if value.len() < 2 || last != first {
        return None;
    }
    value.get(1..value.len() - 1)
}

/// The line of an agent table that sets the model, when it has one.
fn model_line(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .position(|line| pair(line).is_some_and(|(key, _)| key == "model"))
}

/// A line's key and value, the comment beside them taken away.
///
/// A `#` inside a string is not a comment: a model's variant is written
/// `provider/model#high`.
fn pair(line: &str) -> Option<(&str, &str)> {
    let body = line.trim_end_matches(['\r', '\n']);
    let body = match comment_start(body) {
        Some(at) => &body[..at],
        None => body,
    };
    let (key, value) = body.split_once('=')?;
    // A quoted key is the same key: `"model" = ...` is valid TOML.
    let key = key.trim();
    Some((unquote(key).unwrap_or(key), value.trim()))
}

/// The comment beside a line, with the space before it, when it has one.
fn comment_of(line: &str) -> &str {
    let body = line.trim_end_matches(['\r', '\n']);
    match comment_start(body) {
        Some(at) => &body[body[..at].trim_end().len()..],
        None => "",
    }
}

/// Where a line's comment starts, when it has one.
///
/// A `#` inside a string is not a comment. Both TOML strings are read: the
/// basic string (`"..."`), where a backslash escapes the next character, and
/// the literal string (`'...'`), where nothing is escaped.
fn comment_start(body: &str) -> Option<usize> {
    #[derive(Clone, Copy)]
    enum Quote {
        None,
        Basic,
        Literal,
    }
    let mut quote = Quote::None;
    let mut escaped = false;
    for (index, character) in body.char_indices() {
        match quote {
            Quote::Basic => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    quote = Quote::None;
                }
            }
            Quote::Literal => {
                if character == '\'' {
                    quote = Quote::None;
                }
            }
            Quote::None => match character {
                '"' => quote = Quote::Basic,
                '\'' => quote = Quote::Literal,
                '#' => return Some(index),
                _ => {}
            },
        }
    }
    None
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
    /// The document is not TOML at all.
    #[error("syntax error{at}: {message}")]
    Syntax { at: String, message: String },
    /// The document is TOML, but not this configuration: a value has the
    /// wrong type, or a field is missing.
    #[error("invalid configuration{at}: {message}")]
    Shape { at: String, message: String },
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
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
        assert!(error.to_string().contains("missing field"), "{error}");
    }

    #[test]
    fn a_syntax_error_is_a_syntax_error() {
        let error = Config::from_toml("round_limit = \n").expect_err("a broken value");
        assert!(matches!(&error, ConfigError::Syntax { .. }), "{error:?}");
        assert!(error.to_string().contains("syntax"), "{error}");
    }

    #[test]
    fn a_wrong_type_is_a_shape_error_not_a_syntax_error() {
        let error = Config::from_toml(
            "round_limit = \"five\"\nagents = []\norder = []\n[harness]\ntool = \"x\"\ndriven = \"y\"\n",
        )
        .expect_err("a string is not a round limit");
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
        assert!(!error.to_string().contains("syntax"), "{error}");
        assert!(error.to_string().contains("invalid"), "{error}");
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
        assert!(says.contains("syntax"), "{says}");
        assert!(says.contains("line 1"), "{says}");
        assert!(says.contains("column"), "{says}");
    }

    #[test]
    fn an_old_json_configuration_is_not_a_toml_one() {
        // The old file is not read: its JSON text is not TOML.
        let json = serde_json::to_string(&Config::default()).expect("the old shape");
        let error = Config::from_toml(&json).expect_err("JSON is not the configuration");
        assert!(error.to_string().contains("syntax"), "{error}");
    }

    #[test]
    fn the_written_defaults_say_that_a_pick_is_saved_in_place() {
        let text = Config::default().to_toml();
        assert!(text.contains("saves the orchestrator's picked"), "{text}");
        assert!(text.contains("model in place"), "{text}");
    }

    #[test]
    fn a_pick_replaces_only_the_orchestrators_model_line() {
        let text = Config::default().to_toml();
        let picked = Config::with_orchestrator_model(&text, "opencode-go/glm-5.3")
            .expect("the orchestrator's model line");
        let config = Config::from_toml(&picked).expect("the edited text still parses");
        let orchestrator = config
            .agents
            .iter()
            .find(|agent| agent.role == Role::Orchestrator)
            .expect("the orchestrator");
        assert_eq!(orchestrator.model, "opencode-go/glm-5.3");
        // The other agents are their own: the flow starts those, so a pick
        // does not touch them.
        let builder = config
            .agents
            .iter()
            .find(|agent| agent.role == Role::Runner)
            .expect("the builder");
        assert_eq!(builder.model, "opencode-go/deepseek-v4.1-flash#high");
        let reviewer = config
            .agents
            .iter()
            .find(|agent| agent.role == Role::Reviewer)
            .expect("the reviewer");
        assert_eq!(reviewer.model, "opencode-go/glm-5.3#high");
        // Every other line is byte-identical: the file is the human's.
        let before: Vec<&str> = text.lines().collect();
        let after: Vec<&str> = picked.lines().collect();
        assert_eq!(before.len(), after.len(), "{picked}");
        let changed: Vec<usize> = (0..before.len())
            .filter(|index| before[*index] != after[*index])
            .collect();
        assert_eq!(changed.len(), 1, "{picked}");
        assert!(after[changed[0]].contains("glm-5.3"), "{picked}");
    }

    #[test]
    fn a_pick_keeps_a_comment_beside_the_line_it_replaces() {
        // The comments are the human's: the pick changes the value, not the
        // notes beside it, and not the rest of the file.
        let text = "\
# the flow's configuration
round_limit = 5

# One entry per agent.
[[agents]]
# the builder
name = \"builder\"
role = \"runner\"
model = \"opencode-go/deepseek-v4.1-flash#high\"

[[agents]]
name = \"orchestrator\"
role = \"orchestrator\"
model = \"opencode-go/glm-5.3#high\" # the orchestrator's lens

# the harness
[harness]
tool = \"opencode\"
driven = \"--standalone\"

# the order
[[order]]
step = \"groom\"
role = \"creator\"
gates = [{ from = \"groomed\", to = \"approved\" }]
";
        assert!(text.contains("the orchestrator's lens"), "{text}");
        let picked = Config::with_orchestrator_model(text, "opencode-go/kimi-k3")
            .expect("the orchestrator's model line");
        assert!(
            picked.contains("model = \"opencode-go/kimi-k3\" # the orchestrator's lens"),
            "{picked}"
        );
        for kept in [
            "# the flow's configuration",
            "# the builder",
            "# the harness",
            "# the order",
            "model = \"opencode-go/deepseek-v4.1-flash#high\"",
        ] {
            assert!(picked.contains(kept), "{kept}: {picked}");
        }
        assert_eq!(
            Config::from_toml(&picked)
                .expect("the edited text parses")
                .agents
                .iter()
                .find(|agent| agent.role == Role::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("opencode-go/kimi-k3")
        );
    }

    #[test]
    fn a_single_quoted_toml_string_is_read_as_toml() {
        // A human may write the literal string form. The role is the same
        // role, and a `#` inside the string is not a comment.
        let text = "\
round_limit = 5
[[agents]]
name = 'builder'
role = 'runner'
model = 'opencode-go/deepseek-v4.1-flash#high'
[[agents]]
name = 'orchestrator'
role = 'orchestrator'
model = 'opencode-go/glm-5.3#high' # the orchestrator's lens
[harness]
tool = 'opencode'
driven = '--standalone'
[[order]]
step = 'groom'
role = 'creator'
gates = [{ from = 'groomed', to = 'approved' }]
";
        let config = Config::from_toml(text).expect("single-quoted TOML");
        assert_eq!(
            config
                .agents
                .iter()
                .find(|agent| agent.role == Role::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("opencode-go/glm-5.3#high")
        );
        assert_eq!(config.harness.tool, "opencode");
        let picked = Config::with_orchestrator_model(text, "opencode-go/kimi-k3#low")
            .expect("the orchestrator's model line");
        assert!(
            picked.contains("model = \"opencode-go/kimi-k3#low\" # the orchestrator's lens"),
            "{picked}"
        );
    }

    #[test]
    fn a_hash_inside_a_literal_string_is_not_a_comment() {
        assert_eq!(comment_start("name = 'a#b'"), None);
        assert_eq!(comment_start("name = 'a#b' # note"), Some(13));
        assert_eq!(pair("name = 'a#b'"), Some(("name", "'a#b'")));
    }

    #[test]
    fn two_writers_of_one_configuration_do_not_collide() {
        // Two instances on a fresh home wrote to one fixed temporary name:
        // the loser's rename failed. Each write has its own name now.
        let dir =
            std::env::temp_dir().join(format!("coding-crew-config-race-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        let path = dir.join(CONFIG_FILE);
        let mut handles = Vec::new();
        for n in 0..8u32 {
            let path = path.clone();
            handles.push(std::thread::spawn(move || {
                let config = Config {
                    round_limit: n,
                    ..Config::default()
                };
                config.write(&path).expect("a concurrent write");
            }));
        }
        for handle in handles {
            handle.join().expect("a writer");
        }
        // One of the writes is the file, whole and parsable.
        let saved = Config::read(&path).expect("the configuration");
        assert!((0..8).contains(&saved.round_limit));
        // The temporary files are gone: each writer renamed its own.
        let leftovers: Vec<String> = fs::read_dir(&dir)
            .expect("the folder")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pick_reads_a_hash_inside_a_model_as_part_of_the_value() {
        // `#high` is the model's variant, not a comment: the line it is on
        // is still the model line.
        let text = Config::default().to_toml();
        let picked = Config::with_orchestrator_model(&text, "opencode-go/glm-5.3#low")
            .expect("the orchestrator's model line");
        assert!(
            picked.contains("model = \"opencode-go/glm-5.3#low\""),
            "{picked}"
        );
    }

    #[test]
    fn text_without_an_orchestrator_has_no_pick_to_make() {
        let text = Config::default().to_toml();
        let without = text
            .lines()
            .filter(|line| !line.contains("orchestrator"))
            .collect::<Vec<&str>>()
            .join("\n");
        // Drop the now-empty agent table: the text no longer parses as the
        // same configuration, but the function only reads the model lines.
        assert_eq!(
            Config::with_orchestrator_model(&without, "opencode-go/kimi-k3"),
            None
        );
        assert_eq!(
            Config::with_orchestrator_model("round_limit = 5\n", "opencode-go/kimi-k3"),
            None
        );
    }

    #[test]
    fn a_pick_reads_a_quoted_model_key_as_the_model_key() {
        // `"model" = ...` is valid TOML. The orchestrator is first, so the
        // reviewer's table follows it: the pick must change the orchestrator's
        // line and no other.
        let text = "\
[[agents]]
name = \"orchestrator\"
role = \"orchestrator\"
\"model\" = \"opencode-go/glm-5.3#high\"
[[agents]]
name = \"reviewer\"
role = \"reviewer\"
model = \"opencode-go/glm-5.3#high\"
";
        let picked = Config::with_orchestrator_model(text, "opencode-go/kimi-k3")
            .expect("the orchestrator's model line");
        assert!(
            picked.contains("model = \"opencode-go/kimi-k3\""),
            "{picked}"
        );
        assert!(
            picked.ends_with(
                "[[agents]]\nname = \"reviewer\"\nrole = \"reviewer\"\nmodel = \"opencode-go/glm-5.3#high\"\n"
            ),
            "{picked}"
        );
    }

    #[test]
    fn an_orchestrator_with_no_model_line_is_not_given_another_agents() {
        // The orchestrator has no model line, and the reviewer after it does.
        // The search stays in the orchestrator's table: no pick is made.
        let text = "\
[[agents]]
name = \"orchestrator\"
role = \"orchestrator\"
[[agents]]
name = \"reviewer\"
role = \"reviewer\"
model = \"opencode-go/glm-5.3#high\"
";
        assert_eq!(
            Config::with_orchestrator_model(text, "opencode-go/kimi-k3"),
            None
        );
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
