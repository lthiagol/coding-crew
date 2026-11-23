//! The flow's configuration: who acts, in what, and in which order.
//!
//! The configuration is **provisional**: the flow has not written one yet, and
//! the format may change when it does. It lives on the tool's side, at
//! `~/.config/coding-crew/config.toml`, because it is policy rather than work.
//! It is TOML rather than JSON because it is the one file a human edits, and
//! TOML carries comments: the defaults hugo writes explain themselves.
//! What it holds is settled in `docs/tasks/task-model.md` § 11.3: the agents,
//! the harness, model and effort each runs, the review-round limit and the
//! order — the project's own flow.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, RawString, Table, TableLike, TomlError};

use crate::actor::FlowRole;
use crate::harness;
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
#[serde(try_from = "Stored")]
pub struct Config {
    /// The crew: one agent per role present, in the order orchestrator,
    /// builder, reviewer. The orchestrator is always there.
    pub agents: Vec<Agent>,
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

    /// What a configuration file says that the flow does not run as written,
    /// one line each, naming the table or the crew. The file still loads: a
    /// warning is not an error. Empty when the file is clean or does not read.
    pub fn warnings(text: &str) -> Vec<String> {
        let Ok(config) = Config::from_toml(text) else {
            return Vec::new();
        };
        match text.parse::<DocumentMut>() {
            Ok(document) => warnings_of(&document, &config),
            Err(_) => Vec::new(),
        }
    }

    /// Set the orchestrator's model. `false` when there is no orchestrator.
    pub fn set_orchestrator_model(&mut self, model: &str) -> bool {
        match self
            .agents
            .iter_mut()
            .find(|agent| agent.role == FlowRole::Orchestrator)
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
             # writes one itself. hugo reads this file at startup and writes the\n\
             # defaults only when it is absent; its picks last the session and are\n\
             # never written here, and it never writes over a file it cannot read.\n\
             # raul's Settings edits these values. The machine-written records - the\n\
             # journal and the runtime record - stay JSON.\n\
             \n",
        );
        out.push_str("# The build-and-fix cycles a task gets.\n");
        out.push_str(&format!("round_limit = {}\n\n", self.round_limit));
        // An empty order is a root key, not an array of tables: it has to be
        // written before the first table, where root keys still live.
        if self.order.is_empty() {
            out.push_str("# The order the flow follows: no step is configured.\n");
            out.push_str("order = []\n\n");
        }
        let mut agents = self.agents.clone();
        agents.sort_by_key(|agent| rank(agent.role));
        for (index, agent) in agents.iter().enumerate() {
            if index == 0 {
                out.push_str(
                    "# The crew: one table per role. The orchestrator is always present;\n\
                     # a builder and a reviewer are optional. With no reviewer the\n\
                     # orchestrator reviews, and with no builder it builds too. Each table\n\
                     # names the harness it runs in, the model and the effort. The\n\
                     # harnesses are claude, opencode and cursor-agent.\n",
                );
            }
            out.push_str(&format!("[agents.{}]\n", agent.role.as_str()));
            out.push_str(&format!("harness = {}\n", quote(&agent.harness)));
            out.push_str(&format!("model = {}\n", quote(&agent.model)));
            if let Some(effort) = &agent.effort {
                out.push_str(&format!("effort = {}\n", quote(effort)));
            }
            out.push('\n');
        }
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

    /// The warnings the file carries, as [`Config::warnings`] says them.
    pub fn warnings(&self) -> Vec<String> {
        warnings_of(&self.document, &self.loaded)
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
        // A file in the shape before this one holds its agents as an array of
        // tables, which cannot hold the crew's tables. Its agents are written
        // in the table shape, as the loaded configuration reads back, and the
        // save then edits only the values that changed.
        let agents_changed = match (before.get("agents"), after.get("agents")) {
            (Some(was), Some(now)) => !same(was, now),
            _ => true,
        };
        if agents_changed {
            table_shape(&mut document, &before, &after);
        }
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

/// The tables the crew may hold: one per role, the orchestrator always.
const CREW_TABLES: [&str; 3] = ["orchestrator", "builder", "reviewer"];

/// The warnings of a file that reads as `config`: a table the crew has no role
/// for is named and not read, and a crew the flow does not run is named as it
/// is. Three crews run: the orchestrator alone, the orchestrator and the
/// builder, or all three.
fn warnings_of(document: &DocumentMut, config: &Config) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(agents) = document.get("agents").and_then(Item::as_table_like) {
        for (name, _) in agents.iter() {
            if !CREW_TABLES.contains(&name) {
                warnings.push(format!(
                    "[agents.{name}] is not read: the crew's tables are {}",
                    CREW_TABLES.join(", ")
                ));
            }
        }
    }
    let roles: Vec<FlowRole> = config.agents.iter().map(|agent| agent.role).collect();
    let runs = matches!(
        roles.as_slice(),
        [FlowRole::Orchestrator]
            | [FlowRole::Orchestrator, FlowRole::Builder]
            | [
                FlowRole::Orchestrator,
                FlowRole::Builder,
                FlowRole::Reviewer
            ]
    );
    if !runs {
        let crew: Vec<&str> = roles.iter().map(|role| role.as_str()).collect();
        warnings.push(format!(
            "the crew is {}, which the flow does not run: it runs the orchestrator alone, the orchestrator and the builder, or all three",
            crew.join(" and ")
        ));
    }
    warnings
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

/// Move a file's agents from the array shape into the table shape, once a save
/// changes one of them. Each `[[agents]]` entry becomes the table of its role:
/// its values keep their places and comments, and its `name` and `role` lines
/// go, since the table's name is the role. A `runner` entry is the builder.
///
/// The table then takes what the loaded configuration says of each changed
/// agent: a harness or an effort the file left out, and a model without its
/// `#effort`.
fn table_shape(document: &mut DocumentMut, before: &DocumentMut, after: &DocumentMut) {
    let Some(entries) = document.get("agents").and_then(Item::as_array_of_tables) else {
        return;
    };
    let mut crew = Table::new();
    crew.set_implicit(true);
    for entry in entries.iter() {
        let Some(role) = entry.get("role").and_then(Item::as_str) else {
            continue;
        };
        let role = if role == "runner" { "builder" } else { role };
        let mut table = entry.clone();
        drop_identity(&mut table);
        crew.insert(role, Item::Table(table));
    }
    for (key, item) in crew.iter_mut() {
        let role = key.get();
        let now = after.get("agents").and_then(|agents| agents.get(role));
        let was = before.get("agents").and_then(|agents| agents.get(role));
        if let (Some(now), Some(table)) = (now, item.as_table_mut())
            && !was.is_some_and(|was| same(was, now))
        {
            name_the_shape(table, now.as_table_like().expect("an agent is a table"));
        }
    }
    document.insert("agents", Item::Table(crew));
}

/// Remove an entry's `name` and `role` lines: the table's name is the role.
/// A comment written above either line is the human's, so it moves onto the
/// next line the entry keeps, or onto the table's own decor when none is left.
fn drop_identity(table: &mut Table) {
    const IDENTITY: [&str; 2] = ["name", "role"];
    let keys: Vec<String> = table
        .iter()
        .map(|(key, _)| key.to_string())
        .filter(|key| IDENTITY.contains(&key.as_str()))
        .collect();
    let mut carried = String::new();
    for key in keys {
        if let Some(prefix) = table
            .key(&key)
            .and_then(|key| key.leaf_decor().prefix())
            .and_then(RawString::as_str)
        {
            carried.push_str(prefix);
        }
        table.remove(&key);
    }
    if carried.is_empty() {
        return;
    }
    let next = table.iter().next().map(|(key, _)| key.to_string());
    match next {
        Some(next) => {
            if let Some(mut key) = table.key_mut(&next) {
                let decor = key.leaf_decor_mut();
                let old = decor.prefix().and_then(RawString::as_str).unwrap_or("");
                let prefix = format!("{carried}{old}");
                decor.set_prefix(prefix);
            }
        }
        None => {
            let decor = table.decor_mut();
            let old = decor.prefix().and_then(RawString::as_str).unwrap_or("");
            let prefix = format!("{old}{carried}");
            decor.set_prefix(prefix);
        }
    }
}

/// Give a changed agent the keys the file may not name yet. A file in the array
/// shape may leave out its harness, and its effort may be a model's `#variant`;
/// once the agent is saved it names both itself, so the file reads back as it
/// was saved. The model is written without its `#variant` too, or an explicit
/// effort would leave the `#variant` in the model.
fn name_the_shape(table: &mut Table, now: &dyn TableLike) {
    for key in ["harness", "effort"] {
        if !table.contains_key(key)
            && let Some(item) = now.get(key)
        {
            table.insert(key, item.clone());
        }
    }
    if let Some(model) = now.get("model")
        && table.get("model").and_then(Item::as_str) != model.as_str()
    {
        set(table, "model", model);
    }
}

/// Write into `document` the values that differ between `before` and `after`,
/// and nothing else. A value that did not change keeps its place; a key the
/// document does not have yet is added. A table the human wrote inline is
/// written into in place, so it stays inline.
fn apply(document: &mut dyn TableLike, before: &Table, after: &Table) {
    for (key, _) in before.iter() {
        if !after.contains_key(key) {
            document.remove(key);
        }
    }
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
    /// The project's own flow: the crew it runs, the models and efforts each
    /// runs at, the budget it works to, the order `docs/roles/roles.md`
    /// records, and raul's own defaults.
    fn default() -> Self {
        Self {
            agents: vec![
                Agent {
                    role: FlowRole::Orchestrator,
                    harness: "claude".to_string(),
                    model: "claude-opus-5-5".to_string(),
                    effort: Some("medium".to_string()),
                },
                Agent {
                    role: FlowRole::Builder,
                    harness: "claude".to_string(),
                    model: "claude-haiku-5-5".to_string(),
                    effort: Some("high".to_string()),
                },
                Agent {
                    role: FlowRole::Reviewer,
                    harness: "claude".to_string(),
                    model: "claude-opus-5-5".to_string(),
                    effort: Some("medium".to_string()),
                },
            ],
            round_limit: 5,
            order: vec![
                OrderStep {
                    step: "groom".to_string(),
                    role: FlowRole::Creator,
                    gates: vec![gate(Status::Groomed, Status::Approved)],
                },
                OrderStep {
                    step: "build".to_string(),
                    role: FlowRole::Builder,
                    gates: vec![gate(Status::Approved, Status::Executing)],
                },
                OrderStep {
                    step: "self-review".to_string(),
                    role: FlowRole::Builder,
                    gates: vec![gate(Status::Executing, Status::CodeReview)],
                },
                OrderStep {
                    step: "review".to_string(),
                    role: FlowRole::Reviewer,
                    gates: vec![
                        gate(Status::CodeReview, Status::Done),
                        gate(Status::CodeReview, Status::Remediating),
                        gate(Status::CodeReview, Status::NeedsHuman),
                    ],
                },
                OrderStep {
                    step: "remediate".to_string(),
                    role: FlowRole::Builder,
                    gates: vec![gate(Status::Remediating, Status::CodeReview)],
                },
                OrderStep {
                    step: "re-review".to_string(),
                    role: FlowRole::Reviewer,
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

/// One agent of the crew: the role it fills, the harness it runs in, and the
/// model and effort it runs at. Its role is the name of its table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    pub role: FlowRole,
    /// The harness it runs in: a name of the harness table.
    pub harness: String,
    /// The model it runs, as the harness names it.
    pub model: String,
    /// The effort it runs at: one of the harness's levels. `None` is not yet
    /// picked, a gap that hugo refuses to start over; the file leaves the key
    /// out.
    pub effort: Option<String>,
}

/// The order of the crew's roles, as the screens list them.
fn rank(role: FlowRole) -> u8 {
    match role {
        FlowRole::Orchestrator => 0,
        FlowRole::Builder => 1,
        FlowRole::Reviewer => 2,
        FlowRole::Creator => 3,
    }
}

/// The configuration as a file holds it.
///
/// Two shapes read: the one this code writes, one table per role under
/// `[agents]`, and the one before it, an `[[agents]]` array whose entries name
/// their role. That one also had a global `[harness]` table for all of them,
/// and a model that carried its effort as `#effort`.
#[derive(Deserialize)]
struct Stored {
    agents: StoredAgents,
    /// The old shape's harness: an agent that names none runs in it.
    harness: Option<StoredHarness>,
    round_limit: u32,
    order: Vec<OrderStep>,
    #[serde(default)]
    raul: Raul,
}

/// The list comes first: a struct also reads a sequence, field by field.
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredAgents {
    List(Vec<StoredListAgent>),
    Tables(StoredTables),
}

/// The crew's tables: one for each role it may hold.
#[derive(Deserialize)]
struct StoredTables {
    orchestrator: Option<StoredAgent>,
    builder: Option<StoredAgent>,
    reviewer: Option<StoredAgent>,
}

/// An agent's table. Its role is the table's name, so it is not repeated here.
#[derive(Deserialize)]
struct StoredAgent {
    harness: Option<String>,
    model: String,
    effort: Option<String>,
}

/// An agent of the old `[[agents]]` array: its name and role were its own.
#[derive(Deserialize)]
struct StoredListAgent {
    #[allow(dead_code)]
    name: String,
    role: FlowRole,
    harness: Option<String>,
    model: String,
    effort: Option<String>,
}

/// The old shape's global harness. Only the tool is read: the way it was
/// driven is no longer a setting.
#[derive(Deserialize)]
struct StoredHarness {
    tool: String,
}

impl TryFrom<Stored> for Config {
    type Error = String;

    fn try_from(stored: Stored) -> Result<Self, String> {
        let legacy = stored.harness.as_ref().map(|harness| harness.tool.clone());
        let entries: Vec<(FlowRole, StoredAgent)> = match stored.agents {
            StoredAgents::Tables(tables) => [
                (FlowRole::Orchestrator, tables.orchestrator),
                (FlowRole::Builder, tables.builder),
                (FlowRole::Reviewer, tables.reviewer),
            ]
            .into_iter()
            .filter_map(|(role, agent)| agent.map(|agent| (role, agent)))
            .collect(),
            StoredAgents::List(list) => {
                let mut entries: Vec<(FlowRole, StoredAgent)> = Vec::new();
                for agent in list {
                    if agent.role == FlowRole::Creator {
                        return Err(format!(
                            "{} is not an agent's role: the crew holds the orchestrator, the builder and the reviewer",
                            agent.role.as_str()
                        ));
                    }
                    entries.push((
                        agent.role,
                        StoredAgent {
                            harness: agent.harness,
                            model: agent.model,
                            effort: agent.effort,
                        },
                    ));
                }
                entries
            }
        };
        let mut agents = Vec::new();
        for (role, agent) in entries {
            if agents.iter().any(|done: &Agent| done.role == role) {
                return Err(format!(
                    "the {} is in the crew twice: the file holds one table per role",
                    role.as_str()
                ));
            }
            agents.push(agent_of(role, agent, legacy.as_deref())?);
        }
        if !agents
            .iter()
            .any(|agent| agent.role == FlowRole::Orchestrator)
        {
            return Err(
                "the crew has no orchestrator: [agents.orchestrator] is missing, and the orchestrator is always present"
                    .to_string(),
            );
        }
        agents.sort_by_key(|agent| rank(agent.role));
        Ok(Self {
            agents,
            round_limit: stored.round_limit,
            order: stored.order,
            raul: stored.raul,
        })
    }
}

/// An agent as the file states it, checked against the harness table. An
/// agent that names no harness takes the old global one; a model's `#effort`
/// is split off into the effort when the file names none.
fn agent_of(role: FlowRole, stored: StoredAgent, legacy: Option<&str>) -> Result<Agent, String> {
    let name = role.as_str();
    let harness = stored
        .harness
        .or_else(|| legacy.map(str::to_string))
        .ok_or_else(|| format!("{name} names no harness"))?;
    let known = harness::find(&harness).ok_or_else(|| {
        format!(
            "{name} runs in {harness}, which is not a harness: the harnesses are {}",
            harness::names().join(", ")
        )
    })?;
    let (model, effort) = match (stored.effort, stored.model.split_once('#')) {
        (None, Some((model, effort))) => (model.to_string(), Some(effort.to_string())),
        (effort, _) => (stored.model, effort),
    };
    // A model the table holds names its levels; any other model's levels are
    // its listing's to say, and the file cannot be checked against them here.
    if let (Some(effort), Some(levels)) = (&effort, known.curated_efforts(&model))
        && !levels.contains(&effort.as_str())
    {
        return Err(format!(
            "{name} runs {model} at {effort}, which is not one of its efforts: {}",
            levels.join(", ")
        ));
    }
    Ok(Agent {
        role,
        harness,
        model,
        effort,
    })
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
    pub role: FlowRole,
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
        // One agent per role the automation runs; the creator is the human's
        // grooming session and holds no agent.
        let roles: Vec<FlowRole> = config.agents.iter().map(|agent| agent.role).collect();
        assert_eq!(
            roles,
            vec![
                FlowRole::Orchestrator,
                FlowRole::Builder,
                FlowRole::Reviewer
            ]
        );
        for agent in &config.agents {
            assert_eq!(agent.harness, "claude", "{}", agent.role.as_str());
            assert!(agent.effort.is_some(), "{}", agent.role.as_str());
        }
    }

    #[test]
    fn the_defaults_run_each_role_on_its_model_and_effort() {
        let config = Config::default();
        let runs: Vec<(&str, &str, Option<&str>)> = config
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.role.as_str(),
                    agent.model.as_str(),
                    agent.effort.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            runs,
            [
                ("orchestrator", "claude-opus-5-5", Some("medium")),
                ("builder", "claude-haiku-5-5", Some("high")),
                ("reviewer", "claude-opus-5-5", Some("medium")),
            ]
        );
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
            "[agents.orchestrator]",
            "[agents.builder]",
            "[agents.reviewer]",
            "harness = \"claude\"",
            "effort = \"high\"",
            "[[order]]",
            "[raul]",
            "[raul.tasks]",
        ] {
            assert!(text.contains(part), "{part} is not in the shape:\n{text}");
        }
        // The harness is an agent's, not the flow's: no global table is written.
        assert!(!text.contains("[harness]"), "{text}");
        // The comments are not a second copy of the shape: they sit above it.
        assert_eq!(
            Config::from_toml(&text).expect("the written shape parses"),
            Config::default()
        );
    }

    #[test]
    fn an_escaped_string_round_trips() {
        let mut config = Config::default();
        config.agents[0].model = "a \"quoted\" \\ model\tand\u{7f}".to_string();
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
        let error = Config::from_toml("round_limit = \"five\"\nagents = []\norder = []\n")
            .expect_err("a string is not a round limit");
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
        assert!(!error.to_string().contains("syntax"), "{error}");
        assert!(error.to_string().contains("invalid"), "{error}");
    }

    #[test]
    fn an_empty_order_round_trips() {
        // The writer writes its own type, an empty order included: an empty
        // list is a root key, written where root keys still live.
        let config = Config {
            order: Vec::new(),
            ..Config::default()
        };
        let text = config.to_toml();
        assert!(text.contains("order = []"), "{text}");
        assert_eq!(
            Config::from_toml(&text).expect("the written shape parses"),
            config
        );
    }

    #[test]
    fn a_crew_without_an_orchestrator_is_unreadable() {
        // The orchestrator is always in the crew: a file that names only the
        // builder and the reviewer is named, not filled in.
        let text =
            Config::default()
                .to_toml()
                .replacen("[agents.orchestrator]", "[agents.unused]", 1);
        let error = Config::from_toml(&text).expect_err("no orchestrator");
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
        assert!(error.to_string().contains("orchestrator"), "{error}");
    }

    #[test]
    fn a_crew_of_one_is_the_orchestrator_alone() {
        // One table is a crew: the orchestrator builds and reviews.
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role == FlowRole::Orchestrator);
        let text = config.to_toml();
        assert!(!text.contains("[agents.builder]"), "{text}");
        assert!(!text.contains("[agents.reviewer]"), "{text}");
        assert_eq!(Config::from_toml(&text).expect("a crew of one"), config);
    }

    #[test]
    fn the_defaults_and_the_crews_the_flow_runs_have_no_warning() {
        assert!(Config::warnings(&Config::default().to_toml()).is_empty());
        for keep in [
            vec![FlowRole::Orchestrator],
            vec![FlowRole::Orchestrator, FlowRole::Builder],
        ] {
            let mut config = Config::default();
            config.agents.retain(|agent| keep.contains(&agent.role));
            assert!(Config::warnings(&config.to_toml()).is_empty(), "{keep:?}");
        }
    }

    #[test]
    fn an_unknown_agent_table_loads_and_is_named() {
        // The table is not read, so the crew is the same; the file says so.
        let text = format!(
            "{}\n[agents.unused]\nharness = \"claude\"\nmodel = \"claude-haiku-5-5\"\n",
            Config::default().to_toml()
        );
        assert_eq!(
            Config::from_toml(&text).expect("an unknown table loads"),
            Config::default()
        );
        let warnings = Config::warnings(&text);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("[agents.unused]"), "{warnings:?}");
        assert!(
            warnings[0].contains("orchestrator, builder, reviewer"),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_crew_the_flow_does_not_run_loads_and_names_the_crew() {
        // The orchestrator and the reviewer: two agents, not a crew the flow runs.
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role != FlowRole::Builder);
        let text = config.to_toml();
        assert_eq!(Config::from_toml(&text).expect("it loads"), config);
        let warnings = Config::warnings(&text);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].starts_with("the crew is orchestrator and reviewer"),
            "{warnings:?}"
        );
    }

    #[test]
    fn an_unknown_harness_is_still_unreadable_and_has_no_warning() {
        // A harness the table does not hold is an error, not a warning.
        let text = Config::default().to_toml().replacen(
            "harness = \"claude\"\nmodel = \"claude-haiku-5-5\"",
            "harness = \"nope\"\nmodel = \"claude-haiku-5-5\"",
            1,
        );
        assert!(Config::from_toml(&text).is_err());
        assert!(Config::warnings(&text).is_empty());
    }

    #[test]
    fn a_file_names_its_warnings_as_it_is_read() {
        let text = format!(
            "{}\n[agents.unused]\nharness = \"claude\"\nmodel = \"claude-haiku-5-5\"\n",
            Config::default().to_toml()
        );
        let scratch = scratch_file("warnings", &text);
        let file = ConfigFile::read(&scratch.path()).expect("the configuration");
        assert_eq!(file.warnings(), Config::warnings(&text));
        assert_eq!(file.warnings().len(), 1);
    }

    #[test]
    fn a_crew_of_two_is_the_orchestrator_and_the_builder() {
        // Without a reviewer the orchestrator reviews the builder's work.
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role != FlowRole::Reviewer);
        let text = config.to_toml();
        assert!(text.contains("[agents.builder]"), "{text}");
        assert!(!text.contains("[agents.reviewer]"), "{text}");
        assert_eq!(Config::from_toml(&text).expect("a crew of two"), config);
    }

    #[test]
    fn the_crew_reads_in_the_order_the_screens_list_it() {
        // The tables may be written in any order; the crew is read in the order
        // the screens list it, so an index means the same role in every reader.
        let text = "\
round_limit = 5

[agents.reviewer]
harness = \"claude\"
model = \"claude-opus-5-5\"

[agents.builder]
harness = \"claude\"
model = \"claude-haiku-5-5\"

[agents.orchestrator]
harness = \"claude\"
model = \"claude-opus-5-5\"

[[order]]
step = \"groom\"
role = \"creator\"
gates = []
";
        let read = Config::from_toml(text).expect("the tables in another order");
        let roles: Vec<FlowRole> = read.agents.iter().map(|agent| agent.role).collect();
        assert_eq!(
            roles,
            vec![
                FlowRole::Orchestrator,
                FlowRole::Builder,
                FlowRole::Reviewer
            ]
        );
    }

    #[test]
    fn a_role_in_the_crew_twice_is_named() {
        // The array shape allows two entries of one role; the table shape
        // cannot, so the file is named rather than one of them dropped.
        let text = "\
round_limit = 5

[[agents]]
name = \"orchestrator\"
role = \"orchestrator\"
harness = \"claude\"
model = \"claude-opus-5-5\"

[[agents]]
name = \"a\"
role = \"runner\"
harness = \"claude\"
model = \"claude-haiku-5-5\"

[[agents]]
name = \"b\"
role = \"runner\"
harness = \"claude\"
model = \"claude-haiku-5-5\"

[[order]]
step = \"groom\"
role = \"creator\"
gates = []
";
        let error = Config::from_toml(text).expect_err("two builders");
        assert!(error.to_string().contains("builder"), "{error}");
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
    fn the_written_defaults_say_that_a_pick_lasts_the_session() {
        let text = Config::default().to_toml();
        assert!(text.contains("its picks last the session"), "{text}");
        assert!(text.contains("never written here"), "{text}");
        assert!(
            text.contains("raul's Settings edits these values"),
            "{text}"
        );
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
        // `[raul] tasks` written inline is valid TOML. A save changes a value
        // in it, and the file keeps it inline.
        let text = HAND_WRITTEN
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
        config.raul.tasks.reverse = true;
        file.save(&config).expect("the save");
        let saved = fs::read_to_string(&path).expect("the file");
        let expected = text.replacen("reverse = false", "reverse = true", 1);
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
        // The document's theme is one the loaded configuration does not have,
        // so the text a save writes would not hold the configuration: the save
        // is refused. Its round-limit edit must not stay in the document.
        let scratch = scratch_file("refused", HAND_WRITTEN);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        file.document["raul"]["theme"] = toml_edit::value("elsewhere");
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
        let picked = pick("pick", &text, "claude-sonnet-5-5");
        let config = Config::from_toml(&picked).expect("the edited text still parses");
        let orchestrator = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Orchestrator)
            .expect("the orchestrator");
        assert_eq!(orchestrator.model, "claude-sonnet-5-5");
        // The effort is the orchestrator's own: a pick keeps it.
        assert_eq!(orchestrator.effort.as_deref(), Some("medium"));
        // The other agents are their own: the flow starts those, so a pick
        // does not touch them.
        let builder = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Builder)
            .expect("the builder");
        assert_eq!(builder.model, "claude-haiku-5-5");
        let reviewer = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Reviewer)
            .expect("the reviewer");
        assert_eq!(reviewer.model, "claude-opus-5-5");
        // Every other line is byte-identical: the file is the human's.
        let before: Vec<&str> = text.lines().collect();
        let after: Vec<&str> = picked.lines().collect();
        assert_eq!(before.len(), after.len(), "{picked}");
        let changed: Vec<usize> = (0..before.len())
            .filter(|index| before[*index] != after[*index])
            .collect();
        assert_eq!(changed.len(), 1, "{picked}");
        assert!(after[changed[0]].contains("claude-sonnet-5-5"), "{picked}");
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
                .find(|agent| agent.role == FlowRole::Orchestrator)
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
                .find(|agent| agent.role == FlowRole::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("opencode-go/glm-5.3")
        );
        let picked = pick("literal", &text, "opencode-go/kimi-k3");
        assert!(
            picked.contains("model = \"opencode-go/kimi-k3\" # the orchestrator's lens"),
            "{picked}"
        );
    }

    #[test]
    fn a_configuration_without_an_orchestrator_has_no_pick_to_make() {
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role != FlowRole::Orchestrator);
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
            "[agents.orchestrator]\nharness = \"claude\"\nmodel = \"claude-opus-5-5\"\n",
            "[agents.orchestrator]\nharness = \"claude\"\n",
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
            "\"builder\"",
            "\"reviewer\"",
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

    #[test]
    fn an_old_file_reads_its_global_harness_and_splits_the_effort() {
        // The old shape: one harness for every agent, and `#effort` in the
        // model. The file reads as the agents it names, in the new shape.
        let config = Config::from_toml(HAND_WRITTEN).expect("the old shape");
        let runs: Vec<(&str, &str, &str, Option<&str>)> = config
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.role.as_str(),
                    agent.harness.as_str(),
                    agent.model.as_str(),
                    agent.effort.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            runs,
            [
                (
                    "orchestrator",
                    "opencode",
                    "opencode-go/glm-5.3",
                    Some("high")
                ),
                (
                    "builder",
                    "opencode",
                    "opencode-go/deepseek-v4.1-flash",
                    Some("high")
                ),
            ]
        );
    }

    #[test]
    fn an_agent_that_names_its_harness_is_not_the_old_global_one() {
        let text = HAND_WRITTEN.replacen(
            "model = \"opencode-go/glm-5.3#high\" # the orchestrator's lens",
            "harness = \"cursor-agent\"\nmodel = \"auto\"\neffort = \"low\"",
            1,
        );
        let config = Config::from_toml(&text).expect("a mixed configuration");
        let orchestrator = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Orchestrator)
            .expect("the orchestrator");
        assert_eq!(orchestrator.harness, "cursor-agent");
        assert_eq!(orchestrator.model, "auto");
        assert_eq!(orchestrator.effort.as_deref(), Some("low"));
    }

    #[test]
    fn an_old_file_saved_names_the_harness_and_effort_of_each_agent_it_changes() {
        // The pick changes the orchestrator's model. Its harness and effort
        // were the old file's: the save names them, so the file reads back as
        // it was saved, and the builder's lines are left as they were.
        let picked = pick("names", HAND_WRITTEN, "opencode-go/kimi-k3");
        let config = Config::from_toml(&picked).expect("the saved file");
        let orchestrator = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Orchestrator)
            .expect("the orchestrator");
        assert_eq!(orchestrator.harness, "opencode");
        assert_eq!(orchestrator.effort.as_deref(), Some("high"));
        assert!(picked.contains("harness = \"opencode\""), "{picked}");
        assert!(picked.contains("effort = \"high\""), "{picked}");
        assert!(
            picked.contains("model = \"opencode-go/deepseek-v4.1-flash#high\""),
            "{picked}"
        );
    }

    #[test]
    fn a_save_of_an_old_file_writes_its_crew_as_tables() {
        // The array cannot hold the tables, so a save that changes an agent
        // writes the crew as tables, and keeps the rest of the file as it was.
        let saved = change_builder("tables", HAND_WRITTEN, |agent| {
            agent.model = "opencode-go/kimi-k3".to_string();
        });
        assert!(saved.contains("[agents.builder]"), "{saved}");
        assert!(saved.contains("[agents.orchestrator]"), "{saved}");
        assert!(!saved.contains("[[agents]]"), "{saved}");
        assert!(saved.contains("extra_root = \"kept\""), "{saved}");
        assert!(saved.contains("[[order]]"), "{saved}");
    }

    /// Change the builder of a file holding `text`, save it, and check that the
    /// file reads back as the configuration saved. Returns the saved text.
    fn change_builder(name: &str, text: &str, change: impl FnOnce(&mut Agent)) -> String {
        let scratch = scratch_file(name, text);
        let path = scratch.path();
        let mut file = ConfigFile::read(&path).expect("the configuration");
        let mut config = file.config().clone();
        let builder = config
            .agents
            .iter_mut()
            .find(|agent| agent.role == FlowRole::Builder)
            .expect("a builder");
        change(builder);
        file.save(&config).expect("the save");
        let saved = fs::read_to_string(&path).expect("the file");
        assert_eq!(
            Config::from_toml(&saved).expect("the saved file parses"),
            config
        );
        saved
    }

    #[test]
    fn an_old_file_saved_with_only_the_effort_changed_reads_back() {
        // The builder's model still carries `#high` in the file. A save that
        // changes only its effort names the model without the `#`, so the
        // effort does not leave `#high` in the model.
        let saved = change_builder("effort-only", HAND_WRITTEN, |agent| {
            agent.effort = Some("low".to_string());
        });
        assert!(saved.contains("effort = \"low\""), "{saved}");
        assert!(saved.contains("harness = \"opencode\""), "{saved}");
        assert!(
            saved.contains("model = \"opencode-go/deepseek-v4.1-flash\""),
            "{saved}"
        );
        assert!(!saved.contains("deepseek-v4.1-flash#high"), "{saved}");
    }

    #[test]
    fn an_old_file_saved_with_only_the_harness_changed_reads_back() {
        let saved = change_builder("harness-only", HAND_WRITTEN, |agent| {
            agent.harness = "claude".to_string();
        });
        assert!(saved.contains("harness = \"claude\""), "{saved}");
        assert!(saved.contains("effort = \"high\""), "{saved}");
        assert!(
            saved.contains("model = \"opencode-go/deepseek-v4.1-flash\""),
            "{saved}"
        );
        assert!(!saved.contains("deepseek-v4.1-flash#high"), "{saved}");
    }

    #[test]
    fn an_agent_with_no_harness_and_no_global_one_is_unreadable() {
        let text = Config::default().to_toml().replacen(
            "[agents.builder]\nharness = \"claude\"\n",
            "[agents.builder]\n",
            1,
        );
        let error = Config::from_toml(&text).expect_err("the builder names no harness");
        assert!(matches!(&error, ConfigError::Shape { .. }), "{error:?}");
        assert!(
            error.to_string().contains("builder names no harness"),
            "{error}"
        );
    }

    #[test]
    fn a_harness_the_table_does_not_hold_is_unreadable_and_named() {
        let text = Config::default().to_toml().replacen(
            "harness = \"claude\"\nmodel = \"claude-haiku-5-5\"",
            "harness = \"aider\"\nmodel = \"claude-haiku-5-5\"",
            1,
        );
        let error = Config::from_toml(&text).expect_err("aider is not a harness");
        assert!(error.to_string().contains("aider"), "{error}");
        assert!(error.to_string().contains("cursor-agent"), "{error}");
    }

    #[test]
    fn an_effort_the_harness_does_not_take_is_unreadable_and_named() {
        let text = Config::default().to_toml().replacen(
            "harness = \"claude\"\nmodel = \"claude-haiku-5-5\"\neffort = \"high\"",
            "harness = \"claude\"\nmodel = \"claude-haiku-5-5\"\neffort = \"xhigh\"",
            1,
        );
        let error = Config::from_toml(&text).expect_err("haiku takes no xhigh");
        assert!(error.to_string().contains("xhigh"), "{error}");
        assert!(error.to_string().contains("low, medium, high"), "{error}");
    }

    #[test]
    fn an_effort_on_a_command_harness_is_not_checked_until_its_listing_is_read() {
        // A command harness's model is not in the table, so its levels are
        // not known from the file: the file reads, and the listing decides.
        let text = Config::default().to_toml().replacen(
            "harness = \"claude\"\nmodel = \"claude-haiku-5-5\"\neffort = \"high\"",
            "harness = \"cursor-agent\"\nmodel = \"claude-haiku-5-5\"\neffort = \"xhigh\"",
            1,
        );
        let config = Config::from_toml(&text).expect("the file reads");
        assert_eq!(config.agents[1].effort.as_deref(), Some("xhigh"));
    }

    #[test]
    fn an_agent_with_no_effort_reads_back_unpicked() {
        let text = Config::default()
            .to_toml()
            .replacen("effort = \"high\"\n", "", 1);
        let config = Config::from_toml(&text).expect("an agent without an effort");
        let builder = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Builder)
            .expect("a builder");
        assert_eq!(builder.effort, None);
        assert!(
            !config.to_toml().contains("effort = \"high\""),
            "{}",
            config.to_toml()
        );
    }
}
