//! The Settings tab: the configuration file, as a file.
//!
//! raul draws `~/.config/coding-crew/config.toml` the way the file reads: its
//! comments dim, its headers strong, one `key = value` per line in the file's
//! own order. Every value is editable. A choice opens a picker with enter, the
//! same list hugo's choices use, and the picker previews each choice as the
//! cursor moves, esc restoring the value it had; a word or a number is typed
//! inline with enter. A change previews at once — the theme, the icons and the
//! Tasks order repaint the window — and is written only by `w`.
//!
//! The file is what is on disk; the panel holds the draft being edited and what
//! the file held when it was last read or saved. The cursor stops on the value
//! lines: a comment, a header or a blank line is never a row. A file that cannot
//! be used is shown read-only, with the defaults under it.

use std::fs;
use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::announce;
use crate::tasks::{Key, Order};
use crate::view::{self, BAR, Lines, View};
use coding_crew_core::FlowRole;
use coding_crew_core::catalog::{
    Catalog, Levels, TYPE_A_LEVEL, TYPE_AN_ID, effort_word, is_typed_level, keeps_effort,
    stored_effort, switch_harness,
};
use coding_crew_core::config::{
    Agent, Config, ConfigError, ConfigFile, config_path, gates_from_text, gates_text, home_at,
    quote,
};
use coding_crew_core::harness::{self, Harness};
use coding_crew_theme::chrome;
use coding_crew_theme::icons::ENV as ICONS_ENV;
use coding_crew_theme::picker::{self, Row};
use coding_crew_theme::text::{cut_to, text_width};
use coding_crew_theme::{Icon, IconSet, Preset, Theme};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The words a role is written with, in the order the picker lists them: the
/// crew's roles first, then the order's creator.
const ROLES: [&str; 4] = ["orchestrator", "builder", "reviewer", "creator"];
/// The words a boolean is written with.
const BOOLEANS: [&str; 2] = ["true", "false"];
/// The values of raul's own table: the ones a file may leave out, when the
/// defaults stand in for them.
const RAUL_VALUES: [Field; 4] = [Field::Theme, Field::Icons, Field::Sort, Field::Reverse];
/// The group a file's top-level keys are named by in the announcement.
const TOP: &str = "top";
/// The group the lines of a read-only file are named by.
const LINES: &str = "lines";

/// The values the command line and the environment name. They win over the
/// file, and the tab says so on the line they override.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    /// `--theme`, when the flag names a palette.
    pub theme: Option<Preset>,
    /// `--icons`, by its name.
    pub icons: Option<String>,
    /// `CODING_CREW_ICONS`, when it names a set.
    pub env_icons: Option<String>,
}

/// A value of the file the tab edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Round,
    /// An agent's values, by the role its table names.
    AgentHarness(FlowRole),
    AgentModel(FlowRole),
    AgentEffort(FlowRole),
    StepName(usize),
    StepRole(usize),
    StepGates(usize),
    Theme,
    Icons,
    Sort,
    Reverse,
}

/// How a value is edited, which decides whether it is picked from a list or
/// typed, and how it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// One of a list of words, picked from a list and written as a string.
    Choice,
    /// `true` or `false`, picked from a list.
    Bool,
    /// A whole number, typed inline.
    Number,
    /// A word, typed inline and written as a string.
    Text,
    /// A list of gates, typed inline as TOML.
    Gates,
}

impl Field {
    /// The section the value sits in, and its table's place among an array's.
    fn place(self) -> (&'static str, Option<usize>) {
        match self {
            Field::Round => ("", None),
            Field::AgentHarness(_) | Field::AgentModel(_) | Field::AgentEffort(_) => {
                ("agents", None)
            }
            Field::StepName(index) | Field::StepRole(index) | Field::StepGates(index) => {
                ("order", Some(index))
            }
            Field::Theme | Field::Icons => ("raul", None),
            Field::Sort | Field::Reverse => ("raul.tasks", None),
        }
    }

    /// The key the file names the value by.
    fn key(self) -> &'static str {
        match self {
            Field::Round => "round_limit",
            Field::StepRole(_) => "role",
            Field::AgentHarness(_) => "harness",
            Field::AgentModel(_) => "model",
            Field::AgentEffort(_) => "effort",
            Field::StepName(_) => "step",
            Field::StepGates(_) => "gates",
            Field::Theme => "theme",
            Field::Icons => "icons",
            Field::Sort => "sort",
            Field::Reverse => "reverse",
        }
    }

    /// The value's id: its TOML path, which the cursor and the window name it
    /// by. `agents.0.model` is the model of the first agent.
    fn id(self) -> String {
        if let Some(role) = self.role() {
            return format!("agents.{}.{}", role.as_str(), self.key());
        }
        let (section, index) = self.place();
        match (section, index) {
            ("", _) => self.key().to_string(),
            (section, Some(index)) => format!("{section}.{index}.{}", self.key()),
            (section, None) => format!("{section}.{}", self.key()),
        }
    }

    /// The role of an agent's value: the table the value sits in.
    fn role(self) -> Option<FlowRole> {
        match self {
            Field::AgentHarness(role) | Field::AgentModel(role) | Field::AgentEffort(role) => {
                Some(role)
            }
            _ => None,
        }
    }

    fn kind(self) -> Kind {
        match self {
            Field::Round => Kind::Number,
            Field::Reverse => Kind::Bool,
            Field::Theme
            | Field::Icons
            | Field::Sort
            | Field::AgentHarness(_)
            | Field::AgentEffort(_)
            | Field::StepRole(_) => Kind::Choice,
            Field::AgentModel(_) | Field::StepName(_) => Kind::Text,
            Field::StepGates(_) => Kind::Gates,
        }
    }

    /// The words a choice or a boolean takes, in the order the picker lists
    /// them. A model is the harness's models with `type an id…` last; an effort
    /// is the levels its model takes, or, for a model the catalogue does not
    /// list, the harness's levels, `none` and `type a level…`.
    fn options(self, config: &Config, levels: &Levels) -> Vec<String> {
        let words = |words: &[&str]| words.iter().map(|word| word.to_string()).collect();
        match self {
            Field::Theme => Preset::ALL
                .iter()
                .map(|preset| preset.name().to_string())
                .collect(),
            Field::Icons => IconSet::ALL
                .iter()
                .map(|set| set.name().to_string())
                .collect(),
            Field::Sort => Key::ALL.iter().map(|key| key.name().to_string()).collect(),
            Field::StepRole(_) => words(&ROLES),
            Field::AgentHarness(_) => words(&harness::names()),
            Field::AgentModel(role) => {
                let listed = agent_of(config, role)
                    .and_then(|agent| levels.catalog(&agent.harness))
                    .map(|catalog| catalog.ids().into_iter().map(str::to_string).collect());
                let mut models: Vec<String> = listed.unwrap_or_default();
                models.push(TYPE_AN_ID.to_string());
                models
            }
            Field::AgentEffort(role) => agent_of(config, role)
                .and_then(|agent| {
                    let catalog = levels.catalog(&agent.harness)?;
                    Some(match catalog.efforts_of(&agent.model) {
                        Some(levels) => levels.to_vec(),
                        None if agent.model.is_empty() => Vec::new(),
                        None => catalog.unlisted_choices(),
                    })
                })
                .unwrap_or_default(),
            Field::Reverse => words(&BOOLEANS),
            _ => Vec::new(),
        }
    }

    /// The value as the text it is edited and compared in: a word without its
    /// quotes, a number, `true`, or a list of gates as TOML.
    fn text(self, config: &Config) -> String {
        let step = |index: usize| config.order.get(index);
        match self {
            Field::Round => config.round_limit.to_string(),
            Field::AgentHarness(role) => {
                agent_of(config, role).map_or_else(String::new, |a| a.harness.clone())
            }
            Field::AgentModel(role) => {
                agent_of(config, role).map_or_else(String::new, |a| a.model.clone())
            }
            Field::AgentEffort(role) => agent_of(config, role)
                .and_then(|a| {
                    a.effort
                        .as_deref()
                        .map(|effort| effort_word(effort).to_string())
                })
                .unwrap_or_default(),
            Field::StepName(index) => step(index).map_or_else(String::new, |s| s.step.clone()),
            Field::StepRole(index) => {
                step(index).map_or_else(String::new, |s| s.role.as_str().to_string())
            }
            Field::StepGates(index) => {
                step(index).map_or_else(String::new, |s| gates_text(&s.gates))
            }
            Field::Theme => config.raul.theme.clone(),
            Field::Icons => config.raul.icons.clone(),
            Field::Sort => config.raul.tasks.sort.clone(),
            Field::Reverse => config.raul.tasks.reverse.to_string(),
        }
    }

    /// The value as the file writes it, from its text.
    fn literal(self, text: &str) -> String {
        match self.kind() {
            Kind::Choice | Kind::Text => quote(text),
            Kind::Bool | Kind::Number | Kind::Gates => text.to_string(),
        }
    }

    /// Check a text and write it into `config`. The error is the reason the
    /// text is not a value the file can hold, in words. A model's effort is
    /// kept only when the model takes it, as `levels` say.
    fn apply(self, config: &mut Config, text: &str, levels: &Levels) -> Result<(), String> {
        let text = text.trim();
        match self {
            Field::Round => {
                let value: u32 = text
                    .parse()
                    .map_err(|_| "round_limit is a whole number".to_string())?;
                if value == 0 {
                    return Err("round_limit must be at least 1".to_string());
                }
                config.round_limit = value;
            }
            Field::AgentHarness(role) => {
                let name = choice(text, &self.options(config, levels))?;
                if let Some(agent) = agent_mut(config, role) {
                    // A harness whose models are read switches as hugo's does:
                    // the model and the effort stay only where it lists them.
                    match (harness::find(&name), levels.catalog(&name)) {
                        (Some(target), Some(catalog)) => switch_harness(agent, target, catalog),
                        _ => agent.harness = name,
                    }
                }
            }
            Field::AgentModel(role) => {
                let model = model_id(text)?;
                if let Some(agent) = agent_mut(config, role) {
                    let model_levels = levels
                        .catalog(&agent.harness)
                        .and_then(|catalog| catalog.efforts_of(&model))
                        .map(<[String]>::to_vec);
                    if !keeps_effort(agent.effort.as_deref(), model_levels.as_deref()) {
                        agent.effort = None;
                    }
                    agent.model = model;
                }
            }
            Field::AgentEffort(role) => {
                // The levels are the model's, when they have been read. A model
                // the catalogue does not list takes any one word as a level, and
                // `none` on it is no effort at all.
                let listed = agent_of(config, role)
                    .and_then(|agent| levels.of(agent))
                    .is_some();
                let effort = if listed {
                    choice(text, &self.options(config, levels))?
                } else {
                    let level = filled(text, "effort")?;
                    if !is_typed_level(&level) {
                        return Err("a level is one word, with no `#`".to_string());
                    }
                    level
                };
                if let Some(agent) = agent_mut(config, role) {
                    agent.effort = Some(stored_effort(&effort, listed));
                }
            }
            Field::StepName(index) => {
                let name = filled(text, "step")?;
                if let Some(step) = config.order.get_mut(index) {
                    step.step = name;
                }
            }
            Field::StepRole(index) => {
                let name = choice(text, &self.options(config, levels))?;
                if let (Some(step), Some(role)) = (config.order.get_mut(index), role_of(&name)) {
                    step.role = role;
                }
            }
            Field::StepGates(index) => {
                let gates = gates_from_text(text)
                    .map_err(|error| format!("not a list of gates: {error}"))?;
                if let Some(step) = config.order.get_mut(index) {
                    step.gates = gates;
                }
            }
            Field::Theme => config.raul.theme = choice(text, &self.options(config, levels))?,
            Field::Icons => config.raul.icons = choice(text, &self.options(config, levels))?,
            Field::Sort => config.raul.tasks.sort = choice(text, &self.options(config, levels))?,
            Field::Reverse => {
                config.raul.tasks.reverse = choice(text, &self.options(config, levels))? == "true";
            }
        }
        Ok(())
    }
}

/// The text, when it is not empty.
fn filled(text: &str, name: &str) -> Result<String, String> {
    if text.is_empty() {
        Err(format!("{name} is empty"))
    } else {
        Ok(text.to_string())
    }
}

/// The option the text names, or the reason it names none.
fn choice(text: &str, options: &[String]) -> Result<String, String> {
    options
        .iter()
        .find(|option| option.as_str() == text)
        .cloned()
        .ok_or_else(|| format!("{text} is not one of {}", options.join(" · ")))
}

/// A model id: one word, as the harness names it. Its effort is the agent's
/// own setting, so a `#` is not part of it. The catalogue is hugo's, so only
/// the shape is checked here.
fn model_id(text: &str) -> Result<String, String> {
    let shaped = !text.is_empty() && !text.chars().any(|c| c.is_whitespace() || c == '#');
    if shaped {
        Ok(text.to_string())
    } else {
        Err("a model is one word; its effort is set beside it".to_string())
    }
}

/// The role a word names. `runner` is the task model's word for the builder,
/// and a file that still says it is read as the builder.
fn role_of(name: &str) -> Option<FlowRole> {
    match name {
        "creator" => Some(FlowRole::Creator),
        "builder" => Some(FlowRole::Builder),
        "reviewer" => Some(FlowRole::Reviewer),
        "orchestrator" => Some(FlowRole::Orchestrator),
        _ => None,
    }
}

/// The agent whose table is `role`, when the configuration holds one.
fn agent_of(config: &Config, role: FlowRole) -> Option<&Agent> {
    config.agents.iter().find(|agent| agent.role == role)
}

/// The agent whose table is `role`, to be changed.
fn agent_mut(config: &mut Config, role: FlowRole) -> Option<&mut Agent> {
    config.agents.iter_mut().find(|agent| agent.role == role)
}

/// The Tasks order a configuration names. An unknown key is the default, as
/// the window's own look names it.
fn order_of(config: &Config) -> Order {
    Order {
        key: Key::from_name(&config.raul.tasks.sort).unwrap_or_default(),
        reverse: config.raul.tasks.reverse,
    }
}

/// A value being typed in: its field, and the text typed so far.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    field: Field,
    text: String,
}

/// A choice open in the picker: the value it sets, the words it lists, the
/// cursor on one of them, and the draft as it was before the pick opened, which
/// esc restores.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pick {
    field: Field,
    options: Vec<String>,
    cursor: usize,
    before: Config,
}

/// The Settings tab's state: the file as it reads, the draft being edited, and
/// what the pane says about them.
#[derive(Debug)]
pub struct Panel {
    /// The home folder the file is under, when there is one.
    home: Option<PathBuf>,
    /// The file's path, when there is one.
    path: Option<PathBuf>,
    /// The path as the pane names it: `~/.config/coding-crew/config.toml`.
    label: String,
    /// The file, open for a save. `None` when it cannot be used: the tab is
    /// read-only then.
    file: Option<ConfigFile>,
    /// Why the file cannot be used, in one line. The defaults are in use.
    problem: Option<String>,
    /// The file's lines, as they are on disk.
    text: Vec<String>,
    /// What the file holds, as it was read or last saved.
    saved: Config,
    /// The configuration being edited. The look is previewed from it.
    draft: Config,
    /// The Tasks order the window last took from the draft.
    applied: Order,
    /// The value being typed in, when one is.
    edit: Option<Edit>,
    /// The choice open in the picker, when one is.
    pick: Option<Pick>,
    /// Why the last save was refused, in a line.
    notice: Option<String>,
    /// The file changed on disk since it was read: a reload is offered.
    stale: bool,
    /// Quit was asked with unsaved changes: the footer asks.
    asking: bool,
    /// The values the command line and the environment name.
    overrides: Overrides,
    /// The levels each harness's models take, as the reads so far have said.
    levels: Levels,
    /// How a harness's models are read. The window reads them with the
    /// catalogue's bounded read; a test stands its own in.
    reader: fn(&Harness) -> Result<Catalog, String>,
}

impl Panel {
    /// The panel of the file under `home`. The defaults are written first when
    /// the file is absent, as hugo writes them. With no home there is no file:
    /// the defaults are shown, read-only.
    pub fn open(home: Option<&Path>, overrides: Overrides) -> Self {
        let defaults = Config::default();
        let mut panel = Panel {
            home: home.map(Path::to_path_buf),
            path: None,
            label: "config.toml".to_string(),
            file: None,
            problem: None,
            text: Vec::new(),
            saved: defaults.clone(),
            draft: defaults,
            applied: Order::default(),
            edit: None,
            pick: None,
            notice: None,
            stale: false,
            asking: false,
            overrides,
            levels: Levels::new(),
            reader: Catalog::read,
        };
        let Some(home) = home else {
            panel.problem =
                Some("no home folder holds config.toml: the defaults are in use".to_string());
            return panel;
        };
        let path = config_path(home);
        panel.label = label(home, &path);
        match open_file(home, &path) {
            Ok(file) => {
                panel.saved = file.config().clone();
                panel.draft = panel.saved.clone();
                panel.file = Some(file);
            }
            Err(problem) => panel.problem = Some(problem),
        }
        panel.text = read_lines(&path);
        panel.path = Some(path);
        panel.applied = order_of(&panel.draft);
        panel
    }

    /// The pane's title: the path, and the state of the file.
    pub fn title(&self) -> String {
        let state = if self.read_only() {
            " · read-only"
        } else if self.unsaved() {
            " · ● modified"
        } else {
            ""
        };
        format!("{}{state}", self.label)
    }

    /// Whether the draft differs from what the file holds.
    pub fn unsaved(&self) -> bool {
        self.draft != self.saved
    }

    /// Whether the file cannot be used, so nothing is written.
    pub fn read_only(&self) -> bool {
        self.file.is_none()
    }

    /// Whether a value is being typed in.
    pub fn editing(&self) -> bool {
        self.edit.is_some()
    }

    /// Whether a choice is open in the picker.
    pub fn picking(&self) -> bool {
        self.pick.is_some()
    }

    /// The index of the choice the picker's cursor is on, when one is open.
    pub fn pick_cursor(&self) -> Option<usize> {
        self.pick.as_ref().map(|pick| pick.cursor)
    }

    /// The text being typed in, for a key to add to or erase from.
    pub fn typing(&mut self) -> Option<&mut String> {
        self.edit.as_mut().map(|edit| &mut edit.text)
    }

    /// Whether quit is waiting for an answer.
    pub fn asking(&self) -> bool {
        self.asking
    }

    /// Ask whether to quit: the footer offers to save, to quit or to stay.
    pub fn ask(&mut self) {
        self.asking = true;
    }

    /// Stay, after a question about quitting.
    pub fn stay(&mut self) {
        self.asking = false;
    }

    /// The line the last refused save or change says, when there is one.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Whether the file changed on disk since it was read.
    pub fn stale(&self) -> bool {
        self.stale
    }

    /// The theme the window paints with: the flag's, then the draft's.
    pub fn theme(&self) -> Preset {
        self.overrides
            .theme
            .unwrap_or_else(|| Preset::parse(&self.draft.raul.theme).unwrap_or(Preset::Dark))
    }

    /// The icon set the window draws: the flag's, the environment's, then the
    /// draft's.
    pub fn icons(&self) -> IconSet {
        self.overrides
            .icons
            .iter()
            .chain(self.overrides.env_icons.iter())
            .find_map(|name| IconSet::from_name(name))
            .or_else(|| IconSet::from_name(&self.draft.raul.icons))
            .unwrap_or(IconSet::Nerd)
    }

    /// The Tasks order the draft holds, when it is not the one the window last
    /// took. The window takes it, and the draft is then the window's.
    pub fn order_changed(&mut self) -> Option<Order> {
        let order = order_of(&self.draft);
        (order != self.applied).then(|| {
            self.applied = order;
            order
        })
    }

    /// The cursor's ids, in the order the pane shows them: one per value line.
    /// A read-only file's ids are its lines.
    pub fn ids(&self) -> Vec<String> {
        self.parts()
            .iter()
            .filter_map(|part| part.stop.clone())
            .collect()
    }

    /// Start the value at `id`. A choice opens the picker, with the cursor on
    /// the choice in force; a model opens the picker too, and its last entry
    /// types an id. A word or a number is typed in. Nothing starts for a value
    /// that is none of these.
    pub fn begin(&mut self, id: &str) {
        let Some(field) = self.field_at(id) else {
            return;
        };
        if matches!(field, Field::AgentModel(_))
            || matches!(field.kind(), Kind::Choice | Kind::Bool)
        {
            self.open_pick(field);
            return;
        }
        self.edit = Some(Edit {
            field,
            text: field.text(&self.draft),
        });
    }

    /// Open the picker on a field's choices. The models and levels it lists
    /// are read first, when they are not known yet; a read that fails says why
    /// and leaves `type an id…` as the model's only entry.
    fn open_pick(&mut self, field: Field) {
        let mut read_error = None;
        match field {
            Field::AgentModel(role) | Field::AgentEffort(role) => {
                let harness = agent_of(&self.draft, role).map(|agent| agent.harness.clone());
                if let Some(harness) = harness
                    && let Err(error) = self.load(&harness)
                {
                    read_error = Some(error);
                }
            }
            _ => {}
        }
        if let Some(error) = &read_error
            && matches!(field, Field::AgentModel(_))
        {
            self.notice = Some(format!("the model catalog cannot be read: {error}"));
        }
        let options = field.options(&self.draft, &self.levels);
        if options.is_empty() {
            self.notice = Some(match field {
                Field::AgentEffort(role) => match agent_of(&self.draft, role) {
                    Some(agent) if agent.model.is_empty() => "pick a model first".to_string(),
                    Some(agent) => match (&read_error, self.levels.of(agent)) {
                        (Some(error), _) => format!("the model catalog cannot be read: {error}"),
                        (None, Some(_)) => format!("{} takes no effort level", agent.model),
                        (None, None) => format!("{} is not listed", agent.model),
                    },
                    None => "no effort to pick".to_string(),
                },
                _ => "nothing to pick".to_string(),
            });
            return;
        }
        let current = field.text(&self.draft);
        let cursor = options
            .iter()
            .position(|option| *option == current)
            .unwrap_or(0);
        self.pick = Some(Pick {
            field,
            options,
            cursor,
            before: self.draft.clone(),
        });
    }

    /// Read a harness's models, unless they are known: the levels of its models
    /// are then known for the rest of the session.
    fn load(&mut self, name: &str) -> Result<(), String> {
        if self.levels.catalog(name).is_some() {
            return Ok(());
        }
        let target = harness::find(name).ok_or_else(|| format!("{name} is not a harness"))?;
        let catalog = (self.reader)(target)?;
        self.levels.insert(name, catalog);
        Ok(())
    }

    /// Move the pick's cursor by `step`, clamped to its list, and preview the
    /// choice under it in the draft.
    pub fn step_pick(&mut self, step: isize) {
        let Some(pick) = self.pick.as_mut() else {
            return;
        };
        let last = pick.options.len().saturating_sub(1) as isize;
        pick.cursor = (pick.cursor as isize + step).clamp(0, last) as usize;
        let option = &pick.options[pick.cursor];
        // `type an id…` and `type a level…` are not values: they preview nothing.
        if option == TYPE_AN_ID || option == TYPE_A_LEVEL {
            return;
        }
        let mut draft = pick.before.clone();
        if pick.field.apply(&mut draft, option, &self.levels).is_ok() {
            self.draft = draft;
        }
        self.notice = None;
    }

    /// Keep the choice under the pick's cursor, and close the pick. A pick that
    /// opened on a value outside its list has no preview yet, so the cursor's
    /// choice is applied here, as `step_pick` would apply it.
    pub fn keep_pick(&mut self) {
        let Some(pick) = self.pick.take() else {
            return;
        };
        let option = pick.options[pick.cursor].clone();
        if option == TYPE_AN_ID || option == TYPE_A_LEVEL {
            // The model is typed in, from the model in force; a level is typed
            // in from nothing.
            self.draft = pick.before.clone();
            let text = if option == TYPE_A_LEVEL {
                String::new()
            } else {
                pick.field.text(&self.draft)
            };
            self.edit = Some(Edit {
                field: pick.field,
                text,
            });
            return;
        }
        if let Field::AgentHarness(_) = pick.field
            && let Err(error) = self.load(&option)
        {
            self.draft = pick.before;
            self.notice = Some(format!("the model catalog cannot be read: {error}"));
            return;
        }
        let mut draft = pick.before.clone();
        if pick.field.apply(&mut draft, &option, &self.levels).is_ok() {
            self.draft = draft;
        }
    }

    /// Drop the pick: the draft is what it was before the pick opened.
    pub fn drop_pick(&mut self) {
        if let Some(pick) = self.pick.take() {
            self.draft = pick.before;
        }
    }

    /// Keep the value being typed in, when the file can hold it. A value it
    /// cannot hold stays open, with its reason on its line.
    pub fn keep(&mut self) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let mut draft = self.draft.clone();
        if edit
            .field
            .apply(&mut draft, &edit.text, &self.levels)
            .is_ok()
        {
            self.draft = draft;
            self.edit = None;
        }
    }

    /// Drop the value being typed in: the draft keeps what it held.
    pub fn drop_edit(&mut self) {
        self.edit = None;
    }

    /// Drop the unsaved changes: the draft is what the file holds.
    pub fn undo(&mut self) {
        self.draft = self.saved.clone();
        self.edit = None;
        self.notice = None;
        self.asking = false;
    }

    /// Write the draft into the file. Returns whether it was written.
    ///
    /// A save is refused when a value is not one the file can hold, when there
    /// is no file, and when the file changed on disk since it was read.
    pub fn save(&mut self) -> bool {
        if let Some(reason) = self.first_reason() {
            self.notice = Some(format!("not saved: {reason}"));
            return false;
        }
        let Some(file) = self.file.as_mut() else {
            self.notice = Some("not saved: the file cannot be read".to_string());
            return false;
        };
        match file.save(&self.draft) {
            Ok(()) => {
                self.saved = self.draft.clone();
                self.notice = None;
                self.stale = false;
                self.edit = None;
                if let Some(path) = &self.path {
                    self.text = read_lines(path);
                }
                true
            }
            Err(ConfigError::Changed { .. }) => {
                self.notice = Some("the file changed on disk".to_string());
                self.stale = true;
                false
            }
            Err(error) => {
                self.notice = Some(format!("not saved: {error}"));
                false
            }
        }
    }

    /// Read the file again. The draft and the unsaved changes become the file's.
    pub fn reload(&mut self) {
        let applied = self.applied;
        *self = Panel::open(self.home.as_deref(), self.overrides.clone());
        self.applied = applied;
    }

    /// Why a text is not a value the file can hold, when it is not.
    fn reason(&self, field: Field, text: &str) -> Option<String> {
        field
            .apply(&mut self.draft.clone(), text, &self.levels)
            .err()
    }

    /// The first value of the draft the file cannot hold, and why.
    fn first_reason(&self) -> Option<String> {
        self.parts().iter().find_map(|part| match &part.shape {
            Shape::Value { field, .. } => self.reason(*field, &field.text(&self.draft)),
            Shape::Text(..) => None,
        })
    }

    /// The value at a cursor id.
    fn field_at(&self, id: &str) -> Option<Field> {
        self.parts().iter().find_map(|part| match &part.shape {
            Shape::Value { field, .. } if part.stop.as_deref() == Some(id) => Some(*field),
            _ => None,
        })
    }

    /// The lines of the pane, as the file has them, or as the read-only view
    /// has them.
    fn parts(&self) -> Vec<Part> {
        match &self.problem {
            Some(problem) => self.read_only_parts(problem),
            None => self.file_parts(),
        }
    }

    /// The read-only view: the problem, the file as it is, and the defaults in
    /// use. Every line is a cursor stop, so the window scrolls them.
    fn read_only_parts(&self, problem: &str) -> Vec<Part> {
        let mut lines = vec![(problem.to_string(), Paint::Error)];
        if self.path.is_some() {
            lines.push(("the file, as it is:".to_string(), Paint::Strong));
            lines.extend(self.text.iter().map(|line| (line.clone(), Paint::Normal)));
            lines.push((String::new(), Paint::Dim));
        }
        lines.push(("the defaults in use:".to_string(), Paint::Strong));
        let defaults = Config::default().to_toml();
        lines.extend(defaults.lines().map(|line| (line.to_string(), Paint::Dim)));
        lines
            .into_iter()
            .enumerate()
            .map(|(index, (text, paint))| Part {
                shape: Shape::Text(text, paint),
                stop: Some(format!("line:{index}")),
                section: LINES.to_string(),
            })
            .collect()
    }

    /// The file's lines. A line that assigns a value the tab edits is a value
    /// and a cursor stop; every other line is text. The values the file leaves
    /// out follow the lines: see `absent_parts`.
    fn file_parts(&self) -> Vec<Part> {
        let mut parts = Vec::new();
        let mut place = Place::default();
        let mut agents = 0;
        let mut steps = 0;
        let crew = array_roles(&self.text);
        for raw in &self.text {
            let trimmed = raw.trim();
            let shape = if trimmed.starts_with('[') {
                place = header(trimmed, &mut agents, &mut steps, &crew);
                Shape::Text(raw.clone(), Paint::Strong)
            } else if let Some((field, value)) = assignment(trimmed, &place) {
                let (value, comment) = split_comment(value.trim_start());
                Shape::Value {
                    field,
                    value: value.to_string(),
                    comment: comment.map(str::to_string),
                }
            } else if trimmed.starts_with('#') {
                Shape::Text(raw.clone(), Paint::Dim)
            } else {
                Shape::Text(raw.clone(), Paint::Normal)
            };
            let stop = match &shape {
                Shape::Value { field, .. } => Some(field.id()),
                Shape::Text(..) => None,
            };
            let section = if place.name.is_empty() {
                TOP.to_string()
            } else if place.name.starts_with("agents.") {
                // An agent's table is one group: the agents, whatever their roles.
                "agents".to_string()
            } else {
                place.name.clone()
            };
            parts.push(Part {
                shape,
                stop,
                section,
            });
        }
        parts.extend(self.absent_parts());
        parts
    }

    /// The `[raul]` values the file leaves out, drawn after its lines as the
    /// defaults in use. Each is a cursor stop: a change to one is saved into
    /// the file, which adds the key.
    fn absent_parts(&self) -> Vec<Part> {
        let absent: Vec<Field> = RAUL_VALUES
            .into_iter()
            .filter(|field| self.absent(*field))
            .collect();
        if absent.is_empty() {
            return Vec::new();
        }
        let defaults = Config::default();
        let mut parts = vec![
            Part {
                shape: Shape::Text(String::new(), Paint::Dim),
                stop: None,
                section: TOP.to_string(),
            },
            Part {
                shape: Shape::Text(
                    "# not in the file: the defaults in use".to_string(),
                    Paint::Dim,
                ),
                stop: None,
                section: TOP.to_string(),
            },
        ];
        let mut section = "";
        for field in absent {
            let (name, _) = field.place();
            if name != section {
                section = name;
                parts.push(Part {
                    shape: Shape::Text(format!("[{name}]"), Paint::Dim),
                    stop: None,
                    section: name.to_string(),
                });
            }
            parts.push(Part {
                shape: Shape::Value {
                    field,
                    value: field.literal(&field.text(&defaults)),
                    comment: None,
                },
                stop: Some(field.id()),
                section: name.to_string(),
            });
        }
        parts
    }

    /// Whether the file leaves one of raul's values out: it assigns the key in
    /// no form. Only those values have defaults drawn in place of a line.
    fn absent(&self, field: Field) -> bool {
        let (section, _) = field.place();
        RAUL_VALUES.contains(&field)
            && self
                .file
                .as_ref()
                .is_some_and(|file| !file.assigns(section, field.key()))
    }

    /// The blocks the pane draws: each cursor stop with the lines above it that
    /// belong to it, its own line, and the reason it is not kept.
    fn blocks(
        &self,
        width: usize,
        theme: &Theme,
        icons: IconSet,
        selected: Option<&str>,
    ) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut pending: Vec<Line<'static>> = Vec::new();
        for part in self.parts() {
            let mine = part.stop.is_some() && part.stop.as_deref() == selected;
            let (line, reason) = self.paint(&part, width, theme);
            let main = if part.stop.is_some() {
                view::barred(line, width, mine, icons, theme)
            } else {
                indented(line)
            };
            let mut lines = vec![main];
            lines.extend(reason.map(indented));
            match part.stop {
                Some(stop) => {
                    let mut all = std::mem::take(&mut pending);
                    all.extend(lines);
                    blocks.push(Block {
                        lines: all,
                        stop: Some(stop),
                        section: part.section,
                    });
                }
                None => pending.extend(lines),
            }
        }
        // Lines after the last value belong to it: the pane has no row for them.
        if !pending.is_empty() {
            match blocks.last_mut() {
                Some(last) => last.lines.extend(pending),
                None => blocks.push(Block {
                    lines: pending,
                    stop: None,
                    section: String::new(),
                }),
            }
        }
        blocks
    }

    /// One line of the pane, and the reason line under it when there is one.
    fn paint(
        &self,
        part: &Part,
        width: usize,
        theme: &Theme,
    ) -> (Line<'static>, Option<Line<'static>>) {
        let room = width.saturating_sub(BAR);
        match &part.shape {
            Shape::Text(text, paint) => (
                Line::from(Span::styled(cut_to(text, room), paint_of(*paint, theme))),
                None,
            ),
            Shape::Value {
                field,
                value,
                comment,
            } => self.value_line(*field, value, comment.as_deref(), room, theme),
        }
    }

    /// A value's line: `key = value`, then its comment, and the note of the flag
    /// that overrides it. A value the file leaves out is dim, with the note that
    /// it is a default. The reason line is the value's text, when the file
    /// cannot hold it.
    fn value_line(
        &self,
        field: Field,
        shown: &str,
        comment: Option<&str>,
        room: usize,
        theme: &Theme,
    ) -> (Line<'static>, Option<Line<'static>>) {
        let dim = theme.text.dim;
        let strong = theme.text.strong;
        let current = field.text(&self.draft);
        let typing = self.edit.as_ref().filter(|edit| edit.field == field);
        let absent = self.absent(field);
        let mut spans = vec![
            Span::styled(field.key(), if absent { dim } else { theme.text.normal }),
            Span::styled(" = ", dim),
        ];
        let checked = match typing {
            Some(edit) => {
                let quoted = field.kind() == Kind::Text;
                if quoted {
                    spans.push(Span::styled("\"", strong));
                }
                spans.push(Span::styled(edit.text.clone(), strong));
                spans.push(Span::styled(
                    " ",
                    Style::default().add_modifier(Modifier::REVERSED),
                ));
                if quoted {
                    spans.push(Span::styled("\"", strong));
                }
                edit.text.clone()
            }
            None => {
                // A value the user has not changed is drawn as the file wrote
                // it; a changed one is drawn as it would be written. An effort
                // on a model that takes none is `—`, as hugo draws it.
                let none = current.is_empty() && self.takes_no_effort(field);
                let written = if none {
                    "—".to_string()
                } else if field.text(&self.saved) == current {
                    shown.to_string()
                } else {
                    self.written(field, &current)
                };
                let painted = if absent || none { dim } else { strong };
                spans.push(Span::styled(written, painted));
                current.clone()
            }
        };
        let mut tails: Vec<Vec<Span<'static>>> = Vec::new();
        if let Some(comment) = comment {
            tails.push(vec![Span::styled(format!("  {comment}"), dim)]);
        }
        if absent {
            let note = if current == field.text(&Config::default()) {
                "(default, not in the file)"
            } else {
                "(not in the file yet)"
            };
            tails.push(vec![Span::styled(format!("  {note}"), dim)]);
        }
        if let Some(note) = self.overridden(field, &current) {
            tails.push(vec![Span::styled(format!("  {note}"), dim)]);
        }
        let line = fit(spans, tails, room, theme);
        let reason = self.reason(field, &checked).map(|reason| {
            Line::from(Span::styled(
                cut_to(&format!("  {reason}"), room),
                theme.semantic.error,
            ))
        });
        (line, reason)
    }

    /// The text a changed value is written as. An effort is written as it is
    /// stored: `none` on a model the catalogue does not list is an empty one.
    fn written(&self, field: Field, current: &str) -> String {
        match field {
            Field::AgentEffort(role) => {
                let listed = agent_of(&self.draft, role)
                    .is_some_and(|agent| self.levels.of(agent).is_some());
                field.literal(&stored_effort(current, listed))
            }
            _ => field.literal(current),
        }
    }

    /// Whether a field is an agent's effort on a model that takes no level, so
    /// the effort needs none.
    fn takes_no_effort(&self, field: Field) -> bool {
        match field {
            Field::AgentEffort(role) => {
                agent_of(&self.draft, role).is_some_and(|agent| self.levels.takes_none(agent))
            }
            _ => false,
        }
    }

    /// The flag or the environment that overrides a value on its line, when
    /// one does: it names the flag, and the value the file holds is not used.
    fn overridden(&self, field: Field, current: &str) -> Option<String> {
        match field {
            Field::Theme => self
                .overrides
                .theme
                .filter(|preset| preset.name() != current)
                .map(|preset| format!("(overridden by --theme {})", preset.name())),
            Field::Icons => match (&self.overrides.icons, &self.overrides.env_icons) {
                (Some(flag), _) => {
                    (flag != current).then(|| format!("(overridden by --icons {flag})"))
                }
                (None, Some(env)) => {
                    (env != current).then(|| format!("(overridden by {ICONS_ENV}={env})"))
                }
                (None, None) => None,
            },
            _ => None,
        }
    }
}

/// One line of the file, as the pane reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Part {
    shape: Shape,
    /// The cursor's id, for a line the cursor stops on.
    stop: Option<String>,
    /// The group the announcement names the line by, when it is hidden.
    section: String,
}

/// What a line of the file is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shape {
    /// A line of text, painted as given.
    Text(String, Paint),
    /// A value the tab edits: its field, the value as the file writes it, and
    /// the comment the line ends with.
    Value {
        field: Field,
        value: String,
        comment: Option<String>,
    },
}

/// How a line of text is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paint {
    Normal,
    Dim,
    Strong,
    Error,
}

/// The section a line sits in: a table's name, and its place among an array's
/// tables.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Place {
    name: String,
    index: Option<usize>,
}

/// A cursor stop and the lines drawn for it.
#[derive(Debug, Clone)]
struct Block {
    lines: Vec<Line<'static>>,
    stop: Option<String>,
    section: String,
}

/// The lines a window draws, the stops it shows, whether it may stop there
/// (every hidden row is said, or none is hidden), and whether it said them.
struct Window {
    lines: Lines,
    shown: Vec<usize>,
    whole: bool,
    announced: bool,
}

/// The file the panel reads, open at `path` under `home`: the file is written
/// when it is absent, and an error names the file.
fn open_file(home: &Path, path: &Path) -> Result<ConfigFile, String> {
    let dir = home_at(home);
    fs::create_dir_all(&dir).map_err(|error| format!("creating {}: {error}", dir.display()))?;
    ConfigFile::open(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// The path as the pane names it: from the home folder, with `~` for it.
fn label(home: &Path, path: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The lines of a file, or none when it cannot be read.
fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// The roles of an old file's `[[agents]]` entries, in the order they are
/// written: each entry's `role` line, with `runner` read as the builder. An
/// entry with no role line, or a role that is not an agent's, has none.
fn array_roles(lines: &[String]) -> Vec<Option<FlowRole>> {
    let mut roles = Vec::new();
    let mut inside = false;
    for raw in lines {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') {
            inside = trimmed.starts_with("[[agents]]");
            if inside {
                roles.push(None);
            }
            continue;
        }
        if inside
            && let Some(last) = roles.last_mut()
            && let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == "role"
        {
            let word = split_comment(value.trim_start())
                .0
                .trim()
                .trim_matches(['"', '\'']);
            *last = match word {
                "runner" => Some(FlowRole::Builder),
                other => role_of(other).filter(|role| *role != FlowRole::Creator),
            };
        }
    }
    roles
}

/// The place a `[section]` or `[[array]]` header opens. An agent's table is
/// named by its role, as `agents.builder`; an entry of an old array is named
/// by the role its `role` line gives it.
fn header(
    trimmed: &str,
    agents: &mut usize,
    steps: &mut usize,
    crew: &[Option<FlowRole>],
) -> Place {
    let array = trimmed.starts_with("[[");
    let name = trimmed
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    let index = match (array, name.as_str()) {
        (true, "agents") => {
            *agents += 1;
            Some(*agents - 1)
        }
        (true, "order") => {
            *steps += 1;
            Some(*steps - 1)
        }
        _ => None,
    };
    let name = match (array, name.as_str(), index) {
        (true, "agents", Some(index)) => match crew.get(index).copied().flatten() {
            Some(role) => format!("agents.{}", role.as_str()),
            None => name,
        },
        _ => name,
    };
    Place { name, index }
}

/// The value a line assigns, when it assigns one the tab edits, with the text
/// after the `=`.
fn assignment<'a>(trimmed: &'a str, place: &Place) -> Option<(Field, &'a str)> {
    if trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    let index = place.index;
    let agent = place
        .name
        .strip_prefix("agents.")
        .and_then(role_of)
        .filter(|role| *role != FlowRole::Creator);
    let field = match (place.name.as_str(), key.trim()) {
        ("", "round_limit") => Field::Round,
        ("raul", "theme") => Field::Theme,
        ("raul", "icons") => Field::Icons,
        ("raul.tasks", "sort") => Field::Sort,
        ("raul.tasks", "reverse") => Field::Reverse,
        (_, "harness") if agent.is_some() => Field::AgentHarness(agent?),
        (_, "model") if agent.is_some() => Field::AgentModel(agent?),
        (_, "effort") if agent.is_some() => Field::AgentEffort(agent?),
        ("order", "step") => Field::StepName(index?),
        ("order", "role") => Field::StepRole(index?),
        ("order", "gates") => Field::StepGates(index?),
        _ => return None,
    };
    Some((field, value))
}

/// A value's text and the comment after it, split at the first `#` that is
/// outside a string.
fn split_comment(value: &str) -> (&str, Option<&str>) {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        match (quote, character) {
            (Some('"'), '\\') if !escaped => {
                escaped = true;
                continue;
            }
            (Some(open), close) if open == close && !escaped => quote = None,
            (None, '"' | '\'') => quote = Some(character),
            (None, '#') => return (value[..index].trim_end(), Some(&value[index..])),
            _ => {}
        }
        escaped = false;
    }
    (value.trim_end(), None)
}

/// The paint of a line of text.
fn paint_of(paint: Paint, theme: &Theme) -> Style {
    match paint {
        Paint::Normal => theme.text.normal,
        Paint::Dim => theme.text.dim,
        Paint::Strong => theme.text.strong,
        Paint::Error => theme.semantic.error,
    }
}

/// A line made of its spans and the tails that fit beside them, in the order
/// given. A line too wide for the room is cut, and its tails are dropped.
fn fit(
    mut spans: Vec<Span<'static>>,
    tails: Vec<Vec<Span<'static>>>,
    room: usize,
    theme: &Theme,
) -> Line<'static> {
    let width = |spans: &[Span<'static>]| -> usize {
        spans.iter().map(|span| text_width(&span.content)).sum()
    };
    if width(&spans) > room {
        let plain: String = spans.iter().map(|span| span.content.as_ref()).collect();
        return Line::from(Span::styled(cut_to(&plain, room), theme.text.normal));
    }
    for tail in tails {
        if width(&spans) + width(&tail) <= room {
            spans.extend(tail);
        }
    }
    Line::from(spans)
}

/// A line with the selection bar's column left empty.
fn indented(line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    spans.extend(line.spans);
    Line::from(spans)
}

/// The lines of the blocks from the stop `first` that fit in `capacity`, and
/// the stops they show. A block taller than the pane shows its last lines, so
/// its own line stays on screen.
fn layout(blocks: &[Block], first: usize, capacity: usize) -> (Lines, Vec<usize>) {
    let start = blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.stop.is_some())
        .nth(first)
        .map_or(0, |(index, _)| index);
    let mut lines = Lines::default();
    let mut shown = Vec::new();
    let mut stop = first;
    for block in &blocks[start..] {
        // A block with a stop is the row the stop names: its lines belong to
        // that row. A block without one is a heading, and belongs to none.
        let owner = block.stop.as_ref().map(|_| stop);
        if lines.len() + block.lines.len() > capacity {
            if lines.is_empty() && block.stop.is_some() && capacity > 0 {
                let skip = block.lines.len() - capacity;
                lines.extend(block.lines[skip..].to_vec(), owner);
                shown.push(stop);
            }
            break;
        }
        lines.extend(block.lines.clone(), owner);
        if block.stop.is_some() {
            shown.push(stop);
            stop += 1;
        }
    }
    (lines, shown)
}

/// The groups of the stops a window does not show, each named by its section
/// with the count of its stops that are out of view.
fn hidden_groups(
    blocks: &[Block],
    first: usize,
    shown: usize,
    theme: &Theme,
) -> Vec<announce::Group> {
    let mut groups: Vec<announce::Group> = Vec::new();
    for (stop, block) in blocks
        .iter()
        .filter(|block| block.stop.is_some())
        .enumerate()
    {
        let hidden = stop < first || stop >= first + shown;
        if !hidden {
            continue;
        }
        match groups.iter_mut().find(|group| group.label == block.section) {
            Some(group) => group.count += 1,
            None => groups.push(announce::Group {
                label: block.section.clone(),
                style: theme.text.strong,
                count: 1,
            }),
        }
    }
    groups
}

/// The window over the blocks, from the stop `first`, in `height` lines. When
/// stops are hidden, the window keeps a line for the announcement and says
/// what it hides.
fn window(blocks: &[Block], first: usize, height: usize, width: usize, theme: &Theme) -> Window {
    let stops = blocks.iter().filter(|block| block.stop.is_some()).count();
    let (lines, shown) = layout(blocks, first, height);
    if stops == shown.len() || height < 2 {
        return Window {
            whole: stops == shown.len(),
            lines,
            shown,
            announced: false,
        };
    }
    let (mut lines, shown) = layout(blocks, first, height - 1);
    let room = height.saturating_sub(lines.len());
    let groups = hidden_groups(blocks, first, shown.len(), theme);
    let (said, cut) = announce::lines(&groups, width.saturating_sub(BAR), room, theme);
    let announced = !said.is_empty();
    lines.extend(said.into_iter().map(indented).collect(), None);
    Window {
        lines,
        shown,
        whole: !cut,
        announced,
    }
}

/// Draw the tab: the file's lines through the window the keys move. Returns the
/// count of hidden rows the pane does not say, for the footer to carry.
pub fn render_view(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    icons: IconSet,
    selected: Option<&str>,
    view: &mut View,
    panel: &Panel,
) -> Option<usize> {
    let block = chrome::pane(&panel.title(), Icon::TabSettings, icons, false, theme);
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let blocks = panel.blocks(width, theme, icons, selected);
    let ids: Vec<String> = blocks
        .iter()
        .filter_map(|block| block.stop.clone())
        .collect();
    let (rows, announced) = view.draw(&ids, selected, |first| {
        let window = window(&blocks, first, height, width, theme);
        (window.lines, window.announced, window.shown, window.whole)
    });
    frame.render_widget(Paragraph::new(rows).block(block), area);
    let hidden = ids.len().saturating_sub(view.page());
    (hidden > 0 && !announced).then_some(hidden)
}

/// Draw the pick open over the window, when one is: the choices it lists, with
/// the cursor on the one the draft holds.
pub fn render_pick(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    icons: IconSet,
    panel: &Panel,
) -> picker::Picked {
    let Some(pick) = panel.pick.as_ref() else {
        return picker::Picked::default();
    };
    let rows: Vec<Row> = pick
        .options
        .iter()
        .map(|option| Row {
            text: option.clone(),
            style: theme.text.normal,
        })
        .collect();
    picker::draw(
        frame,
        picker::popup(area, rows.len()),
        &pick.field.id(),
        Icon::TabSettings,
        icons,
        theme,
        &[],
        &rows,
        pick.cursor,
    )
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;
    use coding_crew_core::harness::CuratedModel;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// A scratch home folder for one test.
    fn scratch(name: &str) -> PathBuf {
        let home =
            std::env::temp_dir().join(format!("raul-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).expect("a scratch home");
        home
    }

    /// A panel over a fresh file of the defaults.
    /// The levels a harness's listing gives in these tests: the builder's model,
    /// and two more, one of which takes no level.
    fn listing(_harness: &Harness) -> Result<Catalog, String> {
        Ok(Catalog::from_curated(&[
            CuratedModel {
                id: "claude-haiku-5-5",
                efforts: &["low", "medium", "high"],
            },
            CuratedModel {
                id: "opencode-go/glm-5.3",
                efforts: &["low", "medium", "high"],
            },
            CuratedModel {
                id: "opencode-go/kimi-k3",
                efforts: &[],
            },
        ]))
    }

    /// Open the builder's model picker and pick `type an id…`, which opens the
    /// typed input on the model in force.
    fn type_a_model(panel: &mut Panel) {
        panel.begin("agents.builder.model");
        panel.step_pick(99);
        panel.keep_pick();
    }

    fn panel(name: &str) -> (Panel, PathBuf) {
        let home = scratch(name);
        let panel = Panel::open(Some(&home), Overrides::default());
        (panel, config_path(&home))
    }

    /// A whole configuration whose round limit is seven: someone else's edit.
    fn seven() -> String {
        Config {
            round_limit: 7,
            ..Config::default()
        }
        .to_toml()
    }

    /// A whole configuration whose round limit is nine: someone else's edit.
    fn nine() -> String {
        Config {
            round_limit: 9,
            ..Config::default()
        }
        .to_toml()
    }

    /// The lines of the pane at a size, with the cursor on `selected`.
    fn draw(panel: &Panel, width: u16, height: u16, selected: Option<&str>) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let mut view = View::default();
        let theme = theme();
        terminal
            .draw(|frame| {
                render_view(
                    frame,
                    frame.area(),
                    &theme,
                    IconSet::Nerd,
                    selected,
                    &mut view,
                    panel,
                );
            })
            .expect("drawing the settings tab");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    /// Pick the choice `step` places from the one in force: enter, the cursor,
    /// then enter.
    fn choose(panel: &mut Panel, id: &str, step: isize) {
        panel.begin(id);
        panel.step_pick(step);
        panel.keep_pick();
    }

    /// The screen the open pick draws, as rows.
    fn draw_pick(panel: &Panel, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let theme = theme();
        terminal
            .draw(|frame| {
                render_pick(frame, frame.area(), &theme, IconSet::Nerd, panel);
            })
            .expect("drawing the pick");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    /// The text of the rows inside the border.
    fn inner(rows: &[String]) -> String {
        rows.iter()
            .skip(1)
            .take(rows.len().saturating_sub(2))
            .cloned()
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// The row that holds a text.
    fn row_with<'a>(rows: &'a [String], needle: &str) -> &'a str {
        rows.iter()
            .find(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("no row holds {needle}: {}", rows.join("\n")))
    }

    /// The colour of the first cell of `needle`, on the row that holds it.
    fn colour_of(theme: &Theme, needle: &str) -> Option<Color> {
        let mut terminal = Terminal::new(TestBackend::new(100, 200)).expect("a test terminal");
        let mut view = View::default();
        let (panel, _) = panel("colour");
        terminal
            .draw(|frame| {
                render_view(
                    frame,
                    frame.area(),
                    theme,
                    IconSet::Nerd,
                    None,
                    &mut view,
                    &panel,
                );
            })
            .expect("drawing the settings tab");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        for y in 0..area.height {
            let row: String = (0..area.width)
                .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
                .collect();
            if let Some(byte) = row.find(needle) {
                let column = row[..byte].chars().count() as u16;
                return buffer.cell((column, y)).map(|cell| cell.fg);
            }
        }
        None
    }

    #[test]
    fn the_file_is_drawn_as_its_own_lines() {
        let (panel, _) = panel("lines");
        let rows = draw(&panel, 100, 200, None);
        let text = inner(&rows);
        assert!(
            text.contains("# coding-crew's flow configuration."),
            "{text}"
        );
        assert!(text.contains("[raul.tasks]"), "{text}");
        assert!(text.contains("round_limit = 5"), "{text}");
        assert!(text.contains("theme = \"dark\""), "{text}");
        assert!(text.contains("sort = \"id\""), "{text}");
        assert!(text.contains("reverse = false"), "{text}");
        // The title names the path, and the file is not modified.
        let title = rows[0].as_str();
        assert!(
            title.contains("~/.config/coding-crew/config.toml"),
            "{title}"
        );
        assert!(!title.contains("modified"), "{title}");
    }

    #[test]
    fn the_comments_are_dim_and_the_headers_strong() {
        let theme = theme();
        assert_eq!(
            colour_of(&theme, "# coding-crew's"),
            theme.text.dim.fg,
            "a comment is not dim"
        );
        assert_eq!(
            colour_of(&theme, "[raul.tasks]"),
            theme.text.strong.fg,
            "a header is not strong"
        );
    }

    #[test]
    fn only_the_value_lines_are_cursor_stops() {
        let (panel, _) = panel("stops");
        let ids = panel.ids();
        assert_eq!(
            ids,
            [
                "round_limit",
                "agents.orchestrator.harness",
                "agents.orchestrator.model",
                "agents.orchestrator.effort",
                "agents.builder.harness",
                "agents.builder.model",
                "agents.builder.effort",
                "agents.reviewer.harness",
                "agents.reviewer.model",
                "agents.reviewer.effort",
                "order.0.step",
                "order.0.role",
                "order.0.gates",
                "order.1.step",
                "order.1.role",
                "order.1.gates",
                "order.2.step",
                "order.2.role",
                "order.2.gates",
                "order.3.step",
                "order.3.role",
                "order.3.gates",
                "order.4.step",
                "order.4.role",
                "order.4.gates",
                "order.5.step",
                "order.5.role",
                "order.5.gates",
                "raul.theme",
                "raul.icons",
                "raul.tasks.sort",
                "raul.tasks.reverse",
            ]
        );
    }

    #[test]
    fn a_value_written_inline_or_dotted_is_text_not_a_stop() {
        // Valid TOML, and core saves into it; the tab shows the line as text for
        // now. Making each value a stop is a follow-up in TODO.md.
        let base = Config::default()
            .to_toml()
            .replace("[raul.tasks]\nsort = \"id\"\nreverse = false\n", "");
        for (number, form) in [
            "tasks = { sort = \"id\", reverse = false }\n",
            "tasks.sort = \"id\"\ntasks.reverse = false\n",
        ]
        .into_iter()
        .enumerate()
        {
            let text = base.replace("icons = \"nerd\"\n", &format!("icons = \"nerd\"\n{form}"));
            let home = scratch(&format!("inline-stops-{number}"));
            fs::create_dir_all(home_at(&home)).expect("the folder");
            fs::write(config_path(&home), &text).expect("the file");
            let panel = Panel::open(Some(&home), Overrides::default());
            assert!(!panel.read_only(), "the file cannot be read");
            let ids = panel.ids();
            for id in ["raul.tasks.sort", "raul.tasks.reverse"] {
                assert!(!ids.iter().any(|stop| stop == id), "{id} is a stop");
            }
            assert!(ids.iter().any(|stop| stop == "raul.theme"));
            let rows = draw(&panel, 100, 200, None);
            let shown = inner(&rows);
            assert!(
                shown.contains(form.lines().next().expect("a line")),
                "{shown}"
            );
            // Both are set in the file, so neither is a default drawn in its place.
            assert!(!shown.contains("(default, not in the file)"), "{shown}");
            let _ = fs::remove_dir_all(&home);
        }
    }

    #[test]
    fn the_cursor_bar_marks_the_value_it_is_on() {
        let (panel, _) = panel("bar");
        let rows = draw(&panel, 100, 60, Some("round_limit"));
        let bar = Icon::CursorBar.glyph(IconSet::Nerd);
        let marked: Vec<&String> = rows.iter().filter(|row| row.contains(bar)).collect();
        assert_eq!(marked.len(), 1, "{}", rows.join("\n"));
        assert!(marked[0].contains("round_limit = 5"), "{}", marked[0]);
    }

    #[test]
    fn a_choice_opens_a_picker_over_its_choices() {
        let (mut panel, _) = panel("picker-open");
        panel.begin("raul.theme");
        assert!(panel.picking(), "enter did not open the picker");
        assert!(!panel.editing(), "a choice was typed inline");
        let rows = draw_pick(&panel, 100, 40);
        let text = rows.join("\n");
        assert!(text.contains("raul.theme"), "{text}");
        for preset in ["dark", "nord", "catppuccin", "dracula", "terminal"] {
            assert!(
                rows.iter().any(|row| row.contains(preset)),
                "{preset}: {text}"
            );
        }
        // The cursor is on the choice in force, and the picker is the pane.
        assert_eq!(panel.pick.as_ref().map(|pick| pick.cursor), Some(0));
        panel.keep_pick();
        assert!(!panel.picking());
    }

    #[test]
    fn a_pick_previews_each_choice_and_keep_keeps_it() {
        let (mut panel, path) = panel("picker-preview");
        panel.begin("raul.theme");
        panel.step_pick(1);
        assert_eq!(panel.theme(), Preset::Nord, "the cursor did not preview");
        assert!(panel.unsaved());
        panel.keep_pick();
        assert_eq!(panel.theme(), Preset::Nord, "keep dropped the choice");
        // Nothing is written until save.
        assert!(
            fs::read_to_string(&path)
                .expect("the file")
                .contains("theme = \"dark\""),
            "a pick was written before save"
        );
        assert!(panel.title().contains("● modified"), "{}", panel.title());
    }

    #[test]
    fn a_pick_stops_at_either_end_of_its_list() {
        let (mut panel, _) = panel("picker-ends");
        panel.begin("raul.theme");
        panel.step_pick(-1);
        assert_eq!(panel.theme(), Preset::Dark, "the top moved past its end");
        panel.step_pick(99);
        assert_eq!(panel.theme(), Preset::Terminal, "the list ran past its end");
        panel.step_pick(-99);
        assert_eq!(panel.theme(), Preset::Dark);
        panel.keep_pick();
    }

    #[test]
    fn esc_restores_the_choice_the_pick_opened_on() {
        let (mut panel, _) = panel("picker-restore");
        panel.begin("raul.theme");
        panel.step_pick(2);
        assert_eq!(panel.theme(), Preset::Catppuccin);
        panel.drop_pick();
        assert!(!panel.picking());
        assert_eq!(panel.theme(), Preset::Dark, "esc kept the preview");
        assert!(!panel.unsaved(), "esc left a change in the draft");
    }

    #[test]
    fn esc_keeps_a_change_made_before_the_pick_opened() {
        let (mut panel, _) = panel("picker-before");
        panel.begin("raul.tasks.reverse");
        panel.step_pick(-1);
        panel.keep_pick();
        panel.begin("raul.theme");
        panel.step_pick(1);
        panel.drop_pick();
        assert!(
            panel.unsaved(),
            "esc dropped a change the pick did not make"
        );
        assert!(panel.draft.raul.tasks.reverse);
    }

    #[test]
    fn a_pick_starts_on_the_value_in_force() {
        let (mut panel, _) = panel("picker-cursor");
        panel.begin("raul.tasks.reverse");
        assert_eq!(
            panel.pick.as_ref().map(|pick| pick.cursor),
            Some(1),
            "false"
        );
        let rows = draw_pick(&panel, 100, 40);
        let text = rows.join("\n");
        assert!(text.contains("true") && text.contains("false"), "{text}");
        panel.drop_pick();
    }

    #[test]
    fn a_number_or_a_word_is_typed_not_picked() {
        let (mut panel, _) = panel("not-choice");
        panel.begin("round_limit");
        assert!(panel.editing());
        assert!(!panel.picking());
        panel.drop_edit();
        // A model is picked from its harness's list, and `type an id…` types it.
        panel.begin("agents.builder.model");
        assert!(panel.picking());
        assert!(!panel.editing());
        panel.drop_pick();
        assert!(!panel.unsaved());
    }

    #[test]
    fn a_word_is_typed_inline_and_kept_or_dropped() {
        let (mut panel, _) = panel("typed");
        type_a_model(&mut panel);
        assert!(panel.editing());
        panel.typing().expect("the text").push_str("-x");
        let rows = draw(&panel, 100, 60, Some("agents.builder.model"));
        assert!(row_with(&rows, "claude-haiku-5-5-x").contains("-x"));
        panel.keep();
        assert!(!panel.editing());
        assert_eq!(panel.draft.agents[1].model, "claude-haiku-5-5-x");

        assert!(
            !panel.ids().iter().any(|id| id.ends_with(".name")),
            "an agent's name is its table's, not a value"
        );
    }

    #[test]
    fn a_harness_is_picked_from_the_table_and_its_efforts_follow_it() {
        let (mut panel, _) = panel("harness");
        panel.reader = listing;
        // The builder runs haiku at high: opencode is next in the table, and
        // lists haiku at low, medium and high, so the model and effort stay.
        panel.begin("agents.builder.harness");
        panel.step_pick(1);
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].harness, "opencode");
        assert_eq!(panel.draft.agents[1].model, "claude-haiku-5-5");
        assert_eq!(panel.draft.agents[1].effort.as_deref(), Some("high"));
        // The effort is picked from opencode's levels; the cursor starts on high.
        panel.begin("agents.builder.effort");
        let rows = draw_pick(&panel, 100, 40);
        assert!(
            rows.iter().any(|row| row.contains("medium")),
            "{}",
            rows.join("\n")
        );
        panel.step_pick(-1);
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].effort.as_deref(), Some("medium"));
        // The harness list ends at the last harness and does not wrap.
        panel.begin("agents.builder.harness");
        panel.step_pick(9);
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].harness, "cursor-agent");
    }

    #[test]
    fn enter_on_a_pick_opened_on_an_unset_effort_sets_the_first_level() {
        let (mut panel, _) = panel("unset-effort");
        // No effort in force: the pick opens on the harness's first level, and
        // enter keeps that level rather than the old, unset value.
        panel.draft.agents[1].effort = None;
        panel.begin("agents.builder.effort");
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].effort.as_deref(), Some("low"));
        assert!(!panel.picking());
    }

    #[test]
    fn the_effort_list_is_the_levels_of_the_model_in_force() {
        let (mut panel, _) = panel("levels-list");
        // haiku takes three levels, so the list has no xhigh or max.
        panel.begin("agents.builder.effort");
        assert!(panel.picking());
        assert_eq!(
            panel.pick.as_ref().map(|pick| pick.options.clone()),
            Some(vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string()
            ])
        );
        panel.drop_pick();
        // A model that takes no level has no list: the pane says why.
        panel.draft.agents[1].model = "opencode-go/kimi-k3".to_string();
        panel.draft.agents[1].harness = "opencode".to_string();
        panel.reader = listing;
        panel.begin("agents.builder.effort");
        assert!(!panel.picking());
        assert_eq!(
            panel.notice(),
            Some("opencode-go/kimi-k3 takes no effort level")
        );
    }

    /// A catalogue that cannot be read: its tool is not there.
    fn unreadable(_harness: &Harness) -> Result<Catalog, String> {
        Err("opencode api --standalone model.list: not found".to_string())
    }

    /// The effort list of a model the catalogue does not list: the harness's
    /// levels, then `none`, then `type a level…`.
    fn unlisted_effort(name: &str) -> (Panel, PathBuf) {
        let (mut panel, path) = panel(name);
        panel.reader = listing;
        panel.draft.agents[1].harness = "opencode".to_string();
        panel.draft.agents[1].model = "opencode-go/new-model".to_string();
        panel.draft.agents[1].effort = None;
        panel.begin("agents.builder.effort");
        (panel, path)
    }

    #[test]
    fn an_effort_of_a_model_the_list_does_not_name_is_picked_from_the_harness_levels() {
        let (panel, _) = unlisted_effort("effort-unlisted");
        assert!(panel.picking());
        assert_eq!(
            panel.pick.as_ref().map(|pick| pick.options.clone()),
            Some(vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
                "none".to_string(),
                TYPE_A_LEVEL.to_string(),
            ])
        );
    }

    #[test]
    fn none_on_a_model_the_list_does_not_name_saves_an_empty_effort() {
        let (mut panel, path) = unlisted_effort("effort-none");
        panel.step_pick(3);
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].effort.as_deref(), Some(""));
        assert!(panel.save(), "{:?}", panel.notice());
        let text = fs::read_to_string(&path).expect("the file");
        assert!(text.contains("effort = \"\""), "{text}");
    }

    #[test]
    fn type_a_level_takes_one_word_for_a_model_the_list_does_not_name() {
        let (mut panel, _) = unlisted_effort("effort-typed");
        // `type a level…` opens the inline input, and previews nothing.
        panel.step_pick(4);
        panel.keep_pick();
        assert!(panel.editing());
        panel.typing().expect("the text").push_str("a b");
        panel.keep();
        assert!(panel.editing(), "a level of two words was kept");
        assert_eq!(panel.draft.agents[1].effort, None);
        panel.typing().expect("the text").replace_range(.., "ultra");
        panel.keep();
        assert!(!panel.editing());
        assert_eq!(panel.draft.agents[1].effort.as_deref(), Some("ultra"));
    }

    #[test]
    fn an_effort_whose_list_cannot_be_read_says_why() {
        let (mut panel, _) = panel("effort-unreadable");
        panel.reader = unreadable;
        panel.draft.agents[1].harness = "opencode".to_string();
        panel.draft.agents[1].model = "opencode-go/glm-5.3".to_string();
        panel.begin("agents.builder.effort");
        assert!(!panel.picking());
        assert_eq!(
            panel.notice(),
            Some(
                "the model catalog cannot be read: opencode api --standalone model.list: not found"
            )
        );
    }

    #[test]
    fn an_effort_on_a_model_that_takes_none_is_a_dash() {
        let (mut panel, _) = panel("effort-dash");
        panel.reader = listing;
        panel.draft.agents[1].harness = "opencode".to_string();
        panel.draft.agents[1].model = "opencode-go/kimi-k3".to_string();
        panel.draft.agents[1].effort = None;
        panel.load("opencode").expect("the listing");
        let rows = draw(&panel, 100, 60, None);
        assert!(
            row_with(&rows, "effort = —").contains('—'),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn a_model_pick_clears_an_effort_the_new_model_does_not_take() {
        let (mut panel, _) = panel("model-levels");
        panel.reader = listing;
        panel.draft.agents[1].harness = "opencode".to_string();
        panel.draft.agents[1].model = "opencode-go/glm-5.3".to_string();
        panel.draft.agents[1].effort = Some("high".to_string());
        // glm takes high: a pick of kimi, which takes none, clears it.
        panel.begin("agents.builder.model");
        panel.step_pick(1);
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].model, "opencode-go/kimi-k3");
        assert_eq!(panel.draft.agents[1].effort, None);
    }

    #[test]
    fn an_effort_the_model_does_not_take_is_refused_on_save() {
        let (mut panel, path) = panel("refused-effort");
        panel.reader = listing;
        // cursor-agent lists haiku at three levels: a draft that names xhigh
        // for it is one the file cannot hold.
        panel.draft.agents[1].harness = "cursor-agent".to_string();
        panel.begin("agents.builder.model");
        panel.keep_pick();
        assert_eq!(panel.draft.agents[1].model, "claude-haiku-5-5");
        panel.draft.agents[1].effort = Some("xhigh".to_string());
        assert!(
            !panel.save(),
            "an effort the harness does not take was saved"
        );
        assert!(
            panel
                .notice()
                .is_some_and(|notice| notice.contains("xhigh"))
        );
        assert!(
            fs::read_to_string(&path)
                .expect("the file")
                .contains("effort = \"high\""),
            "the file changed"
        );
    }

    #[test]
    fn a_choice_is_not_typed_inline() {
        let (mut panel, _) = panel("no-type");
        panel.begin("raul.icons");
        assert!(!panel.editing());
        assert!(panel.picking());
        panel.drop_pick();
    }

    #[test]
    fn round_limit_must_be_a_whole_number_of_at_least_one() {
        let (mut panel, _) = panel("round");
        panel.begin("round_limit");
        panel.typing().expect("the text").replace_range(.., "0");
        let rows = draw(&panel, 100, 60, Some("round_limit"));
        assert!(
            rows.iter()
                .any(|row| row.contains("round_limit must be at least 1")),
            "{}",
            rows.join("\n")
        );
        panel.keep();
        assert!(panel.editing(), "a value the file cannot hold was kept");
        assert_eq!(panel.draft.round_limit, 5);

        panel.typing().expect("the text").replace_range(.., "x");
        panel.keep();
        assert!(panel.editing());
        assert!(
            draw(&panel, 100, 60, Some("round_limit"))
                .iter()
                .any(|row| row.contains("round_limit is a whole number"))
        );

        panel.typing().expect("the text").replace_range(.., "12");
        panel.keep();
        assert!(!panel.editing());
        assert_eq!(panel.draft.round_limit, 12);
    }

    #[test]
    fn a_model_is_one_word_and_its_effort_is_set_beside_it() {
        assert!(model_id("claude-haiku-5-5").is_ok());
        assert!(model_id("opencode-go/glm-5.3").is_ok());
        for bad in ["", "a b", "glm#high"] {
            assert_eq!(
                model_id(bad),
                Err("a model is one word; its effort is set beside it".to_string()),
                "{bad}"
            );
        }
        let (mut panel, _) = panel("model");
        type_a_model(&mut panel);
        panel.typing().expect("the text").replace_range(.., "a b");
        panel.keep();
        assert!(panel.editing());
        assert_eq!(panel.draft.agents[1].model, "claude-haiku-5-5");
    }

    #[test]
    fn a_reason_is_drawn_under_the_value_it_names() {
        let (mut panel, _) = panel("reason");
        type_a_model(&mut panel);
        panel.typing().expect("the text").replace_range(.., "");
        let rows = draw(&panel, 100, 60, Some("agents.builder.model"));
        let at = rows
            .iter()
            .position(|row| row.contains("▌model"))
            .expect("the value");
        assert!(
            rows[at + 1].contains("a model is one word"),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn a_gate_list_is_typed_as_toml_and_checked() {
        let (mut panel, _) = panel("gates");
        panel.begin("order.0.gates");
        panel
            .typing()
            .expect("the text")
            .replace_range(.., "[{ from = \"nowhere\", to = \"done\" }]");
        panel.keep();
        assert!(panel.editing(), "an unknown status was kept");
        panel.typing().expect("the text").replace_range(.., "[]");
        panel.keep();
        assert!(!panel.editing());
        assert!(panel.draft.order[0].gates.is_empty());
    }

    #[test]
    fn a_flag_that_overrides_a_value_says_so_on_its_line() {
        let home = scratch("override");
        let overrides = Overrides {
            theme: Some(Preset::Nord),
            icons: None,
            env_icons: Some("unicode".to_string()),
        };
        let panel = Panel::open(Some(&home), overrides);
        let rows = draw(&panel, 100, 200, None);
        assert!(
            row_with(&rows, "theme = \"dark\"").contains("(overridden by --theme nord)"),
            "{}",
            rows.join("\n")
        );
        assert!(
            row_with(&rows, "icons = \"nerd\"")
                .contains("(overridden by CODING_CREW_ICONS=unicode)"),
        );
        // The window paints with the flag, not the file.
        assert_eq!(panel.theme(), Preset::Nord);
        assert_eq!(panel.icons(), IconSet::Unicode);
    }

    #[test]
    fn a_flag_equal_to_the_file_overrides_nothing() {
        let home = scratch("agree");
        let overrides = Overrides {
            theme: Some(Preset::Dark),
            ..Overrides::default()
        };
        let panel = Panel::open(Some(&home), overrides);
        let rows = draw(&panel, 100, 60, None);
        assert!(!inner(&rows).contains("overridden"), "{}", rows.join("\n"));
    }

    #[test]
    fn the_preview_follows_the_draft_and_the_flag_wins_in_it() {
        let (mut panel, _) = panel("preview");
        choose(&mut panel, "raul.icons", 1);
        assert_eq!(panel.icons(), IconSet::Unicode);
        choose(&mut panel, "raul.theme", 1);
        assert_eq!(panel.theme(), Preset::Nord);
        choose(&mut panel, "raul.tasks.sort", 1);
        assert_eq!(
            panel.order_changed(),
            Some(Order {
                key: Key::Created,
                reverse: false
            })
        );
        assert_eq!(panel.order_changed(), None, "the order is taken once");
    }

    #[test]
    fn save_writes_the_change_and_keeps_the_file() {
        let home = scratch("save");
        let path = config_path(&home);
        let mut panel = Panel::open(Some(&home), Overrides::default());
        let before = fs::read_to_string(&path).expect("the file").replace(
            "# raul's own settings: theme is dark",
            "# raul's own settings, kept: theme is dark",
        );
        fs::write(&path, &before).expect("the edited file");
        panel.reload();
        choose(&mut panel, "raul.theme", 1);
        assert!(panel.save());
        assert!(!panel.unsaved());
        let after = fs::read_to_string(&path).expect("the file");
        assert!(after.contains("theme = \"nord\""), "{after}");
        assert!(
            after.contains("# raul's own settings, kept"),
            "the comment is lost: {after}"
        );
        assert!(panel.title().ends_with("config.toml"), "{}", panel.title());
    }

    #[test]
    fn a_save_refused_for_a_changed_file_is_named_and_offers_a_reload() {
        let (mut panel, path) = panel("stale");
        choose(&mut panel, "raul.theme", 1);
        fs::write(&path, nine()).expect("someone else's edit");
        assert!(!panel.save());
        assert!(panel.stale());
        assert_eq!(panel.notice(), Some("the file changed on disk"));
        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            nine(),
            "a refused save wrote over the file"
        );
        // The reload takes the file, and the draft is the file's.
        panel.reload();
        assert!(!panel.stale());
        assert!(!panel.unsaved());
        assert_eq!(panel.draft.round_limit, 9);
    }

    #[test]
    fn undo_drops_the_unsaved_changes() {
        let (mut panel, _) = panel("undo");
        choose(&mut panel, "raul.theme", 1);
        assert!(panel.unsaved());
        panel.undo();
        assert!(!panel.unsaved());
        assert_eq!(panel.theme(), Preset::Dark);
    }

    #[test]
    fn reload_takes_the_file_and_drops_the_draft() {
        let (mut panel, path) = panel("reload");
        fs::write(&path, seven()).expect("someone else's edit");
        choose(&mut panel, "raul.theme", 1);
        panel.reload();
        assert!(!panel.unsaved());
        assert_eq!(panel.draft.round_limit, 7);
        assert!(!panel.read_only(), "a reloaded file is read");
    }

    #[test]
    fn quit_asks_when_there_are_unsaved_changes() {
        let (mut panel, _) = panel("asks");
        assert!(!panel.asking());
        choose(&mut panel, "raul.theme", 1);
        panel.ask();
        assert!(panel.asking());
        panel.stay();
        assert!(!panel.asking());
    }

    #[test]
    fn an_unreadable_file_is_named_read_only_with_the_defaults_beside_it() {
        let home = scratch("broken");
        fs::create_dir_all(home_at(&home)).expect("the home");
        let broken = "round_limit = \n";
        fs::write(config_path(&home), broken).expect("a broken file");
        let mut panel = Panel::open(Some(&home), Overrides::default());
        assert!(panel.read_only());
        assert!(panel.title().contains("read-only"), "{}", panel.title());
        let rows = draw(&panel, 100, 60, None);
        let text = inner(&rows);
        assert!(text.contains("syntax error"), "{text}");
        assert!(
            text.contains("round_limit = "),
            "the file is not shown: {text}"
        );
        assert!(text.contains("the defaults in use:"), "{text}");
        assert!(
            text.contains("round_limit = 5"),
            "the defaults are not beside it: {text}"
        );
        // Nothing is changed, and nothing is written.
        choose(&mut panel, "raul.theme", 1);
        assert!(!panel.unsaved());
        assert!(!panel.save());
        assert_eq!(
            fs::read_to_string(config_path(&home)).expect("the file"),
            broken,
            "the broken file was written over"
        );
        // A read-only pane's lines are the cursor's stops, so it scrolls.
        assert!(panel.ids().iter().all(|id| id.starts_with("line:")));
    }

    #[test]
    fn a_pane_with_no_home_shows_the_defaults_read_only() {
        let panel = Panel::open(None, Overrides::default());
        assert!(panel.read_only());
        let rows = draw(&panel, 100, 60, None);
        assert!(
            inner(&rows).contains("no home folder"),
            "{}",
            rows.join("\n")
        );
    }

    /// A panel over a file written before the `[raul]` table existed: the
    /// defaults' text up to its raul part.
    fn before_raul(name: &str) -> (Panel, PathBuf) {
        let home = scratch(name);
        let text = Config::default().to_toml();
        let old = &text[..text.find("# raul's own settings").expect("the table")];
        fs::create_dir_all(home_at(&home)).expect("the folder");
        fs::write(config_path(&home), old).expect("an old file");
        let panel = Panel::open(Some(&home), Overrides::default());
        (panel, config_path(&home))
    }

    #[test]
    fn a_file_without_raul_offers_the_defaults_it_leaves_out() {
        let (panel, _) = before_raul("no-raul");
        let rows = draw(&panel, 100, 200, None);
        let text = inner(&rows);
        for line in [
            "[raul]",
            "theme = \"dark\"  (default, not in the file)",
            "icons = \"nerd\"  (default, not in the file)",
            "[raul.tasks]",
            "sort = \"id\"  (default, not in the file)",
            "reverse = false  (default, not in the file)",
        ] {
            assert!(text.contains(line), "{line} is missing: {text}");
        }
        let ids = panel.ids();
        for id in [
            "raul.theme",
            "raul.icons",
            "raul.tasks.sort",
            "raul.tasks.reverse",
        ] {
            assert!(ids.iter().any(|stop| stop == id), "{id} is not a stop");
        }
    }

    #[test]
    fn a_change_to_a_default_the_file_leaves_out_adds_its_key_on_save() {
        let (mut panel, path) = before_raul("no-raul-save");
        let old = fs::read_to_string(&path).expect("the file");
        choose(&mut panel, "raul.theme", 1);
        assert!(panel.unsaved(), "the default cannot be changed");
        assert!(panel.save());
        let saved = fs::read_to_string(&path).expect("the file");
        assert!(
            saved.starts_with(&old),
            "the file's own lines changed: {saved}"
        );
        assert!(saved.contains("theme = \"nord\""), "{saved}");
        assert_eq!(
            Config::from_toml(&saved)
                .expect("the saved file")
                .raul
                .theme,
            "nord"
        );
        // The key the file now holds is a line of its own, not a default.
        let rows = draw(&panel, 100, 200, None);
        assert!(
            !inner(&rows).contains("theme = \"nord\"  (default"),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn every_line_fits_the_pane_at_every_size() {
        let (panel, _) = panel("fits");
        for width in 10..=50u16 {
            for height in [3u16, 5, 8, 12, 30] {
                let rows = draw(&panel, width, height, Some("raul.tasks.reverse"));
                for row in &rows {
                    assert!(
                        text_width(row) <= usize::from(width),
                        "{width}x{height}: {row:?} is wider than the pane"
                    );
                }
            }
        }
    }

    #[test]
    fn a_short_pane_says_what_it_hides_by_section() {
        let (panel, _) = panel("hidden");
        let rows = draw(&panel, 100, 12, Some("order.3.role"));
        let text = inner(&rows);
        assert!(text.contains("not shown:"), "{text}");
        assert!(text.contains("hidden"), "{text}");
        assert!(text.contains("order"), "{text}");
        // The cursor's line is drawn: the pane scrolled to it.
        assert!(text.contains("role = \"reviewer\""), "{text}");
        assert!(!text.contains("round_limit = 5"), "{text}");
    }

    #[test]
    fn the_pinned_render_at_40_and_100_columns() {
        let (panel, _) = panel("pinned");
        let narrow = inner(&draw(&panel, 40, 200, Some("round_limit")));
        assert!(narrow.contains("round_limit = 5"), "{narrow}");
        assert!(narrow.contains("theme = \"dark\""), "{narrow}");
        let wide = inner(&draw(&panel, 100, 200, Some("round_limit")));
        assert!(
            wide.contains("# The build-and-fix cycles a task gets."),
            "{wide}"
        );
        assert!(wide.contains("round_limit = 5"), "{wide}");
    }
}
