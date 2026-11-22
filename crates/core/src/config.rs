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
use toml_edit::{DocumentMut, Item, Table, TableLike, TomlError};

use crate::actor::Role;
use crate::lifecycle::Status;

/// The configuration's file name under the tool's home.
pub const CONFIG_FILE: &str = "config.toml";
/// The folder under the user's home that holds every tool's configuration.
pub const CONFIG_DIR: &str = ".config";
/// The folder's name under `~/.config/`.
pub const APP_DIR: &str = "coding-crew";
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
    /// raul's own settings. A file without the table has the defaults.
    #[serde(default)]
    pub raul: Raul,
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

    /// Write the whole configuration, atomically, as the store writes a task.
    ///
    /// This replaces the file's text with the defaults' shape: a file a human
    /// has edited is changed through [`ConfigFile::save`], which keeps it.
    pub fn write(&self, path: &Path) -> Result<(), ConfigError> {
        Self::write_text(path, &self.to_toml())
    }

    /// Write TOML text, atomically, as the store writes a task.
    ///
    /// A symbolic link is written through: the file it names is replaced, and
    /// the link stays where the human put it. The new file keeps the mode of
    /// the one it replaces.
    ///
    /// The temporary name is unique to the write: two hugo instances on one
    /// home must not rename each other's file.
    pub fn write_text(path: &Path, text: &str) -> Result<(), ConfigError> {
        let path = resolve_links(path)?;
        let tmp = temp_path(&path);
        let result = write_beside(&tmp, &path, text);
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        Ok(result?)
    }

    /// Set the orchestrator's model. `false` when there is no orchestrator.
    pub fn set_orchestrator_model(&mut self, model: &str) -> bool {
        match self
            .agents
            .iter_mut()
            .find(|agent| agent.role == Role::Orchestrator)
        {
            Some(agent) => {
                agent.model = model.to_string();
                true
            }
            None => false,
        }
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
            out.push_str(&format!("gates = {}\n\n", gates_text(&step.gates)));
        }
        out.push_str(
            "# raul's own settings: theme is dark, nord, catppuccin, dracula or\n\
             # terminal; icons is nerd or unicode. raul reads them at startup.\n\
             [raul]\n",
        );
        out.push_str(&format!("theme = {}\n", quote(&self.raul.theme)));
        out.push_str(&format!("icons = {}\n\n", quote(&self.raul.icons)));
        out.push_str(
            "# The order the Tasks tab opens in: the key it sorts by, and whether it\n\
             # runs backwards.\n\
             [raul.tasks]\n",
        );
        out.push_str(&format!("sort = {}\n", quote(&self.raul.tasks.sort)));
        out.push_str(&format!("reverse = {}\n", self.raul.tasks.reverse));
        while out.ends_with("\n\n") {
            out.pop();
        }
        out
    }
}

/// A configuration file as it is on disk.
///
/// The file is kept as a document, so a save writes only the values that
/// changed: the comments, the order, the spacing and the keys this code does
/// not know all stay as the human left them.
#[derive(Debug)]
pub struct ConfigFile {
    path: PathBuf,
    document: DocumentMut,
    /// The configuration the document held when it was read or last saved.
    loaded: Config,
    /// The hash of the text read, checked before a save.
    hash: u64,
}

impl ConfigFile {
    /// Read the configuration at `path`, writing the defaults first when there
    /// is no file. A dangling link is a file the human left: it is an error,
    /// and the defaults are not written through it.
    pub fn open(path: &Path) -> Result<Self, ConfigError> {
        if path.symlink_metadata().is_err() {
            Config::default().write(path)?;
        }
        Self::read(path)
    }

    /// Read the configuration at `path`.
    pub fn read(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path)?;
        let loaded = Config::from_toml(&text)?;
        let document: DocumentMut = text.parse().map_err(|error: TomlError| {
            let at = error
                .span()
                .map(|span| location(&text, span.start))
                .unwrap_or_default();
            ConfigError::Syntax {
                at,
                message: error.message().to_string(),
            }
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            document,
            loaded,
            hash: content_hash(&text),
        })
    }

    /// The configuration as the file holds it.
    pub fn config(&self) -> &Config {
        &self.loaded
    }

    /// Whether the file assigns `key` in the table `table`, in any form TOML
    /// writes it: a header, a dotted key or an inline table. The table is
    /// dotted, as `raul.tasks`; the root is the empty name. An array of tables
    /// is not a table here, so no key is found in one.
    pub fn assigns(&self, table: &str, key: &str) -> bool {
        let mut node: &dyn TableLike = self.document.as_table();
        for name in table.split('.').filter(|name| !name.is_empty()) {
            match node.get(name).and_then(Item::as_table_like) {
                Some(next) => node = next,
                None => return false,
            }
        }
        node.contains_key(key)
    }

    /// Save `config` into the file. Only the values that differ from the
    /// configuration as loaded are written; every other part of the file is
    /// kept. A list whose length changes is written whole. A file that changed
    /// on disk since it was read is not written: the save is refused, and the
    /// file stays as it is on disk.
    pub fn save(&mut self, config: &Config) -> Result<(), ConfigError> {
        match fs::read_to_string(&self.path) {
            Ok(text) if content_hash(&text) == self.hash => {}
            Ok(_) => return Err(self.changed()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(self.changed());
            }
            Err(error) => return Err(error.into()),
        }
        let before = document_of(&self.loaded)?;
        let after = document_of(config)?;
        // The edits go into a copy: a refused save must not leave them in the
        // document, where the next save would write them.
        let mut document = self.document.clone();
        apply(document.as_table_mut(), before.as_table(), after.as_table());
        let text = document.to_string();
        // A value the text does not hold would be lost on the next read. The
        // text is checked before it is written, so the file keeps what it had.
        let saved = Config::from_toml(&text)?;
        if saved != *config {
            return Err(ConfigError::Lost {
                file: self.path.display().to_string(),
            });
        }
        Config::write_text(&self.path, &text)?;
        self.document = document;
        self.loaded = saved;
        self.hash = content_hash(&text);
        Ok(())
    }

    fn changed(&self) -> ConfigError {
        ConfigError::Changed {
            file: self.path.display().to_string(),
        }
    }
}

/// The configuration as a document, the way the writer writes it.
fn document_of(config: &Config) -> Result<DocumentMut, ConfigError> {
    config
        .to_toml()
        .parse()
        .map_err(|error: TomlError| ConfigError::Shape {
            at: String::new(),
            message: error.message().to_string(),
        })
}

/// Write into `document` the values that differ between `before` and `after`,
/// and nothing else. A value that did not change keeps its place; a key the
/// document does not have yet is added. A table the human wrote inline is
/// written into in place, so it stays inline.
fn apply(document: &mut dyn TableLike, before: &Table, after: &Table) {
    for (key, now) in after.iter() {
        let was = before.get(key);
        if was.is_some_and(|was| same(was, now)) {
            continue;
        }
        match (was, now) {
            (Some(Item::Table(was)), Item::Table(now)) => {
                let entry = document.entry(key).or_insert(Item::Table(Table::new()));
                if let Some(table) = entry.as_table_like_mut() {
                    apply(table, was, now);
                }
            }
            (Some(Item::ArrayOfTables(was)), Item::ArrayOfTables(now))
                if was.len() == now.len() =>
            {
                let entry = document
                    .entry(key)
                    .or_insert(Item::ArrayOfTables(Default::default()));
                if let Some(tables) = entry.as_array_of_tables_mut() {
                    for (index, (was, now)) in was.iter().zip(now.iter()).enumerate() {
                        if let Some(table) = tables.get_mut(index) {
                            apply(table, was, now);
                        }
                    }
                }
            }
            _ => set(document, key, now),
        }
    }
}

/// Whether two items hold the same text. A table's own text leaves out its
/// sub-tables, so tables are compared key by key.
fn same(was: &Item, now: &Item) -> bool {
    match (was.as_table_like(), now.as_table_like()) {
        (Some(was), Some(now)) => {
            was.len() == now.len()
                && was
                    .iter()
                    .all(|(key, item)| now.get(key).is_some_and(|other| same(item, other)))
        }
        _ => was.to_string() == now.to_string(),
    }
}

/// Set one key. A value keeps the decoration it had — its spacing and the
/// comment beside it — and takes the new value. Any other item is replaced.
fn set(table: &mut dyn TableLike, key: &str, now: &Item) {
    if let Some(old) = table.get_mut(key).and_then(Item::as_value_mut)
        && let Item::Value(new) = now
    {
        let mut new = new.clone();
        *new.decor_mut() = old.decor().clone();
        *old = new;
        return;
    }
    table.insert(key, now.clone());
}

/// A hash of a file's text. FNV-1a, so every build of the tools computes the
/// same hash for the same text.
fn content_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The configuration home under a user's home folder.
pub fn home_at(home: &Path) -> PathBuf {
    home.join(CONFIG_DIR).join(APP_DIR)
}

/// The configuration file under a user's home folder.
pub fn config_path(home: &Path) -> PathBuf {
    home_at(home).join(CONFIG_FILE)
}

/// The running user's home folder, when there is one.
pub fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
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

/// The file a path names, with its symbolic links followed.
///
/// A path that is not a link, or is not there, names itself: a new
/// configuration is written where it was asked for.
fn resolve_links(path: &Path) -> std::io::Result<PathBuf> {
    // A chain of links is followed to its end; a loop is an error, not a hang.
    const LIMIT: usize = 32;
    let mut path = path.to_path_buf();
    for _ in 0..LIMIT {
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target = fs::read_link(&path)?;
                path = match path.parent() {
                    Some(parent) => parent.join(target),
                    None => target,
                };
            }
            _ => return Ok(path),
        }
    }
    Err(std::io::Error::other("too many symbolic links"))
}

/// Write `text` to `tmp`, give it the mode of `path` when there is one, and
/// rename it over `path`.
fn write_beside(tmp: &Path, path: &Path, text: &str) -> std::io::Result<()> {
    {
        let mut file = fs::File::create(tmp)?;
        if let Ok(meta) = fs::metadata(path) {
            file.set_permissions(meta.permissions())?;
        }
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)
}

/// The gates as the file writes them: a TOML list of inline tables.
pub fn gates_text(gates: &[Gate]) -> String {
    let gates: Vec<String> = gates
        .iter()
        .map(|gate| {
            format!(
                "{{ from = {}, to = {} }}",
                quote(gate.from.as_str()),
                quote(gate.to.as_str())
            )
        })
        .collect();
    format!("[{}]", gates.join(", "))
}

/// The gates a TOML list of inline tables holds, as the file writes them.
///
/// The error is the parser's message: the text is not a list of gates.
pub fn gates_from_text(text: &str) -> Result<Vec<Gate>, String> {
    #[derive(Deserialize)]
    struct Holder {
        gates: Vec<Gate>,
    }
    toml::from_str::<Holder>(&format!("gates = {text}"))
        .map(|holder| holder.gates)
        .map_err(|error| error.message().to_string())
}

/// A TOML string, escaped.
pub fn quote(value: &str) -> String {
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
    /// the budget it works to, the order `docs/roles/roles.md` records, and
    /// raul's own defaults.
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
            raul: Raul::default(),
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

/// raul's own settings: the palette it paints with, the icons it draws, and the
/// order the Tasks tab opens in.
///
/// The names are the words the file holds; raul checks them against its own
/// presets and icon sets, which the core does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Raul {
    /// The palette: `dark`, `nord`, `catppuccin`, `dracula` or `terminal`.
    pub theme: String,
    /// The icon set: `nerd` or `unicode`.
    pub icons: String,
    pub tasks: RaulTasks,
}

impl Default for Raul {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            icons: "nerd".to_string(),
            tasks: RaulTasks::default(),
        }
    }
}

/// The Tasks tab's order: the key it sorts by, and whether it runs backwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RaulTasks {
    /// The key: `id`, `created`, `updated`, `status`, `priority` or `effort`.
    pub sort: String,
    pub reverse: bool,
}

impl Default for RaulTasks {
    fn default() -> Self {
        Self {
            sort: "id".to_string(),
            reverse: false,
        }
    }
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
    /// The file changed on disk since it was read: a save would overwrite it.
    #[error("the file changed on disk since it was read: {file}")]
    Changed { file: String },
    /// The text a save would write does not hold the configuration it was
    /// given: nothing is written.
    #[error("the save would not keep every value in {file}: nothing was written")]
    Lost { file: String },
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

    #[cfg(unix)]
    #[test]
    fn a_write_goes_through_a_symbolic_link() {
        // The link is the human's: the file it names changes, and the link
        // stays where it was.
        let dir =
            std::env::temp_dir().join(format!("coding-crew-config-link-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        let target = dir.join("dotfiles-hugo.toml");
        fs::write(&target, "round_limit = 1\n").expect("the target");
        let path = dir.join(CONFIG_FILE);
        std::os::unix::fs::symlink(&target, &path).expect("a link");
        Config::write_text(&path, "round_limit = 2\n").expect("the write");
        assert!(
            path.symlink_metadata()
                .expect("the link")
                .file_type()
                .is_symlink(),
            "the link was replaced by a file"
        );
        assert_eq!(
            fs::read_to_string(&target).expect("the target"),
            "round_limit = 2\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_write_keeps_the_mode_of_the_file_it_replaces() {
        use std::os::unix::fs::PermissionsExt;

        let dir =
            std::env::temp_dir().join(format!("coding-crew-config-mode-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        let path = dir.join(CONFIG_FILE);
        fs::write(&path, "round_limit = 1\n").expect("the file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("a private mode");
        Config::write_text(&path, "round_limit = 2\n").expect("the write");
        let mode = fs::metadata(&path).expect("the file").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the write reset the mode");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_write_leaves_no_temporary_file() {
        // A folder where the file must go makes the rename fail, after the
        // temporary file has been written beside it.
        let dir =
            std::env::temp_dir().join(format!("coding-crew-config-failed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        let path = dir.join(CONFIG_FILE);
        fs::create_dir_all(&path).expect("a folder where the file goes");
        assert!(Config::write_text(&path, "round_limit = 2\n").is_err());
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
        for part in [
            "round_limit",
            "[[agents]]",
            "[harness]",
            "[[order]]",
            "[raul]",
            "[raul.tasks]",
        ] {
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

    /// A scratch folder holding one configuration file. The folder is removed
    /// when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn path(&self) -> PathBuf {
            self.0.join(CONFIG_FILE)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A scratch folder whose configuration file holds `text`.
    fn scratch_file(name: &str, text: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("coding-crew-config-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        fs::write(dir.join(CONFIG_FILE), text).expect("the file");
        Scratch(dir)
    }

    /// Pick `model` for the orchestrator of the file holding `text`, and
    /// return the file's text after the save.
    fn pick(name: &str, text: &str, model: &str) -> String {
        let scratch = scratch_file(name, text);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let mut config = file.config().clone();
        assert!(config.set_orchestrator_model(model), "no orchestrator");
        file.save(&config).expect("the save");
        fs::read_to_string(&path).expect("the file")
    }

    /// A configuration written by hand: comments, a table out of the writer's
    /// order, a key this code does not know, and spacing the writer would not
    /// produce.
    const HAND_WRITTEN: &str = "\
# coding-crew's configuration, kept by hand.
extra_root = \"kept\"

# The build-and-fix cycles a task gets.
round_limit = 5 # three is enough here, five is what it is

# the harness
[harness]
driven = \"--standalone\"
tool = \"opencode\"

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
note = \"a key this code does not know\"

[raul]
theme = 'nord'
icons = \"nerd\"

[raul.tasks]
sort = \"id\"
reverse = false

# the order
[[order]]
step = \"groom\"
role = \"creator\"
gates = [{ from = \"groomed\", to = \"approved\" }]
";

    #[test]
    fn an_edit_changes_only_the_values_it_changes() {
        let scratch = scratch_file("edit", HAND_WRITTEN);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let mut config = file.config().clone();
        config.round_limit = 3;
        config.raul.theme = "dracula".to_string();
        config.raul.tasks.reverse = true;
        file.save(&config).expect("the save");
        let saved = fs::read_to_string(&path).expect("the file");
        let expected = HAND_WRITTEN
            .replacen("round_limit = 5 #", "round_limit = 3 #", 1)
            .replacen("theme = 'nord'", "theme = \"dracula\"", 1)
            .replacen("reverse = false", "reverse = true", 1);
        assert_eq!(saved, expected);
        assert_eq!(
            Config::from_toml(&saved).expect("the saved file parses"),
            config
        );
    }

    #[test]
    fn a_save_into_an_inline_table_keeps_the_inline_form() {
        // `harness` and `[raul] tasks` written inline are valid TOML. A save
        // changes a value in each, and the file keeps both inline.
        let text = HAND_WRITTEN
            .replacen(
                "[harness]\ndriven = \"--standalone\"\ntool = \"opencode\"\n",
                "harness = { tool = \"opencode\", driven = \"--standalone\" }\n",
                1,
            )
            .replacen("[raul.tasks]\nsort = \"id\"\nreverse = false\n", "", 1)
            .replacen(
                "icons = \"nerd\"\n",
                "icons = \"nerd\"\ntasks = { sort = \"id\", reverse = false }\n",
                1,
            );
        let scratch = scratch_file("inline", &text);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let mut config = file.config().clone();
        config.harness.driven = "--embedded".to_string();
        config.raul.tasks.reverse = true;
        file.save(&config).expect("the save");
        let saved = fs::read_to_string(&path).expect("the file");
        let expected = text
            .replacen("driven = \"--standalone\"", "driven = \"--embedded\"", 1)
            .replacen("reverse = false", "reverse = true", 1);
        assert_eq!(saved, expected);
        assert_eq!(
            Config::from_toml(&saved).expect("the saved file parses"),
            config
        );
    }

    #[test]
    fn a_save_of_an_unchanged_configuration_writes_the_same_text() {
        // Comments, order, spacing and the unknown keys all survive a save
        // that changes nothing.
        let scratch = scratch_file("unchanged", HAND_WRITTEN);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let config = file.config().clone();
        file.save(&config).expect("the save");
        assert_eq!(fs::read_to_string(&path).expect("the file"), HAND_WRITTEN);
    }

    #[test]
    fn an_old_file_without_raul_is_not_rewritten_to_add_its_keys() {
        // The file predates the `[raul]` table: it loads with the defaults,
        // and a save of another value does not add the table.
        let text = Config::default().to_toml();
        let old = &text[..text.find("# raul's own settings").expect("the table")];
        assert!(!old.contains("[raul]"), "{old}");
        let loaded = Config::from_toml(old).expect("an old configuration");
        assert_eq!(loaded.raul, Raul::default());
        let scratch = scratch_file("old", old);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let mut config = file.config().clone();
        file.save(&config).expect("the save");
        assert_eq!(fs::read_to_string(&path).expect("the file"), old);
        config.round_limit = 3;
        file.save(&config).expect("the save");
        let saved = fs::read_to_string(&path).expect("the file");
        assert!(!saved.contains("[raul]"), "{saved}");
        assert_eq!(saved, old.replacen("round_limit = 5", "round_limit = 3", 1));
    }

    #[test]
    fn a_key_is_found_under_a_header_a_dotted_key_or_an_inline_table() {
        let text = Config::default().to_toml();
        let scratch = scratch_file("assigns", &text);
        let file = ConfigFile::read(&scratch.path()).expect("the configuration");
        assert!(file.assigns("", "round_limit"));
        assert!(file.assigns("raul", "theme"));
        assert!(file.assigns("raul.tasks", "reverse"));
        assert!(!file.assigns("raul", "reverse"));
        assert!(!file.assigns("raul.tasks", "theme"));
        assert!(!file.assigns("nowhere", "theme"));

        // The tasks table written as an inline table, and as dotted keys.
        let head = &text[..text
            .find("# The order the Tasks tab")
            .expect("the tasks table")];
        for tasks in [
            "tasks = { sort = \"id\", reverse = false }\n",
            "tasks.sort = \"id\"\ntasks.reverse = false\n",
        ] {
            let scratch = scratch_file("assigns-tasks", &format!("{head}{tasks}"));
            let file = ConfigFile::read(&scratch.path()).expect("the configuration");
            assert!(file.assigns("raul.tasks", "sort"), "{tasks}");
            assert!(file.assigns("raul.tasks", "reverse"), "{tasks}");
            assert!(!file.assigns("raul.tasks", "theme"), "{tasks}");
        }
    }

    #[test]
    fn a_missing_file_is_created_with_the_defaults() {
        let scratch = scratch_file("created", "");
        let path = scratch.path();
        fs::remove_file(&path).expect("no file yet");
        let file = ConfigFile::open(&path).expect("the configuration");
        assert_eq!(file.config(), &Config::default());
        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            Config::default().to_toml()
        );
    }

    #[test]
    fn a_save_refuses_a_file_changed_on_disk() {
        let scratch = scratch_file("changed", HAND_WRITTEN);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let mut config = file.config().clone();
        config.round_limit = 3;
        let theirs = Config::default().to_toml();
        fs::write(&path, &theirs).expect("another writer");
        let error = file.save(&config).expect_err("the file changed");
        assert!(matches!(error, ConfigError::Changed { .. }), "{error:?}");
        assert!(error.to_string().contains("changed on disk"), "{error}");
        assert_eq!(fs::read_to_string(&path).expect("the file"), theirs);
    }

    #[test]
    fn a_save_refuses_a_file_removed_from_disk() {
        let scratch = scratch_file("removed", HAND_WRITTEN);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let config = file.config().clone();
        fs::remove_file(&path).expect("removed");
        let error = file.save(&config).expect_err("the file is gone");
        assert!(matches!(error, ConfigError::Changed { .. }), "{error:?}");
        assert!(!path.exists());
    }

    #[test]
    fn a_refused_save_leaves_no_edit_in_the_document() {
        // The document's tool is one the loaded configuration does not have,
        // so the text a save writes would not hold the configuration: the save
        // is refused. Its round-limit edit must not stay in the document.
        let scratch = scratch_file("refused", HAND_WRITTEN);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        file.document["harness"]["tool"] = toml_edit::value("elsewhere");
        let mut config = file.config().clone();
        config.round_limit = 3;
        let error = file.save(&config).expect_err("the text would not hold it");
        assert!(matches!(error, ConfigError::Lost { .. }), "{error:?}");
        assert_eq!(file.document["round_limit"].as_integer(), Some(5));
        assert_eq!(fs::read_to_string(&path).expect("the file"), HAND_WRITTEN);
    }

    #[test]
    fn a_wrong_type_in_raul_is_a_shape_error() {
        let text = Config::default()
            .to_toml()
            .replace("reverse = false", "reverse = \"yes\"");
        let error = Config::from_toml(&text).expect_err("a string is not a flag");
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
    }

    #[test]
    fn the_pick_replaces_only_the_orchestrators_model_line() {
        let text = Config::default().to_toml();
        let picked = pick("pick", &text, "opencode-go/glm-5.3");
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
        let picked = pick("comment", HAND_WRITTEN, "opencode-go/kimi-k3");
        assert!(
            picked.contains("model = \"opencode-go/kimi-k3\" # the orchestrator's lens"),
            "{picked}"
        );
        for kept in [
            "# coding-crew's configuration, kept by hand.",
            "# the builder",
            "# the harness",
            "# the order",
            "extra_root = \"kept\"",
            "note = \"a key this code does not know\"",
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
        let text = HAND_WRITTEN.replace(
            "model = \"opencode-go/glm-5.3#high\" # the orchestrator's lens",
            "model = 'opencode-go/glm-5.3#high' # the orchestrator's lens",
        );
        let config = Config::from_toml(&text).expect("single-quoted TOML");
        assert_eq!(
            config
                .agents
                .iter()
                .find(|agent| agent.role == Role::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("opencode-go/glm-5.3#high")
        );
        let picked = pick("literal", &text, "opencode-go/kimi-k3#low");
        assert!(
            picked.contains("model = \"opencode-go/kimi-k3#low\" # the orchestrator's lens"),
            "{picked}"
        );
    }

    #[test]
    fn a_hash_inside_a_model_is_part_of_the_value() {
        // `#high` is the model's variant, not a comment.
        let picked = pick(
            "variant",
            &Config::default().to_toml(),
            "opencode-go/glm-5.3#low",
        );
        assert!(
            picked.contains("model = \"opencode-go/glm-5.3#low\""),
            "{picked}"
        );
    }

    #[test]
    fn a_configuration_without_an_orchestrator_has_no_pick_to_make() {
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role != Role::Orchestrator);
        assert!(!config.set_orchestrator_model("opencode-go/kimi-k3"));
    }

    #[test]
    fn a_pick_reads_a_quoted_model_key_as_the_model_key() {
        // `"model" = ...` is valid TOML. The orchestrator is first, so the
        // reviewer's table follows it: the pick must change the orchestrator's
        // line and no other.
        let text = "\
round_limit = 5

[[agents]]
name = \"orchestrator\"
role = \"orchestrator\"
\"model\" = \"opencode-go/glm-5.3#high\"

[[agents]]
name = \"reviewer\"
role = \"reviewer\"
model = \"opencode-go/glm-5.3#high\"

[harness]
tool = \"opencode\"
driven = \"--standalone\"

[[order]]
step = \"groom\"
role = \"creator\"
gates = [{ from = \"groomed\", to = \"approved\" }]
";
        let picked = pick("quoted", text, "opencode-go/kimi-k3");
        assert!(
            picked.contains("\"model\" = \"opencode-go/kimi-k3\""),
            "{picked}"
        );
        assert!(
            picked.contains("model = \"opencode-go/glm-5.3#high\""),
            "{picked}"
        );
        assert_eq!(picked.matches("kimi-k3").count(), 1, "{picked}");
    }

    #[test]
    fn an_agent_without_a_model_is_unreadable() {
        // A pick is no longer searched for in the text: a model is what an
        // agent runs, and an agent that names none is not a configuration.
        let text = Config::default().to_toml().replacen(
            "role = \"orchestrator\"\nmodel = \"opencode-go/glm-5.3#high\"\n",
            "role = \"orchestrator\"\n",
            1,
        );
        let error = Config::from_toml(&text).expect_err("the orchestrator names no model");
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
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

    #[test]
    fn gates_are_written_and_read_back() {
        let gates = vec![
            gate(Status::Groomed, Status::Approved),
            gate(Status::CodeReview, Status::Done),
        ];
        let text = gates_text(&gates);
        assert_eq!(
            text,
            "[{ from = \"groomed\", to = \"approved\" }, { from = \"code-review\", to = \"done\" }]"
        );
        assert_eq!(gates_from_text(&text), Ok(gates));
        assert_eq!(gates_from_text("[]"), Ok(Vec::new()));
        assert!(gates_from_text("[{ from = \"nowhere\", to = \"done\" }]").is_err());
        assert!(gates_from_text("[").is_err());
    }
}
