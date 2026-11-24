//! The window: one sidebar, one main pane, a title line and a footer.
//!
//! The sidebar is the configuration and the main pane is what is running.
//! There is no tab and no tab bar. The main pane leads with the crew — the
//! reading the human picked — then the queue, the round, the budget and the
//! findings, with the stamp at its foot. `tab` moves the keys to the queue,
//! where a task is marked for the next start.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use coding_crew_core::config::{Agent, Config};
use coding_crew_core::harness::{self, HARNESSES, Harness};
use coding_crew_core::runtime::{AgentState, CrewMember, QueueItem, Runtime};
use coding_crew_core::{FlowRole, Status, age_text, parse_rfc3339};
use coding_crew_theme::picker::Picked;
use coding_crew_theme::text::{cut_to, head_to, tail_to, text_width};
use coding_crew_theme::{
    Icon, IconSet, Slot, Theme, chrome, phase_style, picker, role_style, selected_line,
};

use crate::catalog::Catalog;
use crate::config::{self, ConfigRead, ConfigState};
use crate::launch::{self, LaunchRequest, Launcher, Start};
use crate::on_path;
use crate::pick;
use crate::sidebar::{self, Field, role_icon};

/// The rows one notch of the wheel moves.
const WHEEL_ROWS: isize = 3;
/// The columns the sidebar takes when it is shown, its border included.
pub const SIDEBAR_WIDTH: u16 = 32;
/// The narrowest the main pane is drawn beside the sidebar.
pub const MAIN_MIN_WIDTH: u16 = 48;
/// The narrowest window that holds both panes: the sidebar and a main pane
/// that is not squeezed. Below it the sidebar is dropped, whole.
pub const SIDEBAR_MIN_WIDTH: u16 = SIDEBAR_WIDTH + MAIN_MIN_WIDTH;

/// The instant a plain render draws at: a pinned screen must not move with
/// the wall clock. Named in `docs/hugo/screens.md`.
pub const RENDER_NOW: &str = "2026-11-19T09:00:00Z";

/// The title line's text: the name of the tool, which takes its icon.
const NAME: &str = "hugo";

/// The key that reads the configuration again.
pub const RELOAD_KEY: &str = "r";
/// The key that opens the choice of the cursor's row.
pub const CHOOSE_KEY: &str = "enter";
/// The key that leaves the choice without saving.
pub const CLOSE_KEY: &str = "esc";
/// The key that starts the orchestrator over the queue.
pub const START_KEY: &str = "s";
/// The key that opens the crew-size selector.
pub const CREW_KEY: &str = "c";
/// The crew-size selector's title.
pub const CREW_SIZE_TITLE: &str = "crew size";
/// The roles a crew is made of, in the order the crew reads them: the
/// orchestrator is always there, and a crew of size N is the first N.
const CREW_ORDER: [FlowRole; 3] = [
    FlowRole::Orchestrator,
    FlowRole::Builder,
    FlowRole::Reviewer,
];
/// The hint each crew size carries in the selector, by size from one.
const CREW_HINTS: [&str; 3] = [
    "single agent",
    "orchestrator also reviews",
    "one agent per role",
];
/// The notice when `s` is pressed outside herdr.
pub const START_NEEDS_HERDR: &str = "start needs herdr";
/// The notice when `s` is pressed with no task in the queue.
pub const QUEUE_EMPTY: &str = "the queue is empty";
/// The notice when `s` is pressed with tasks waiting, none of them claimable.
pub const NONE_CLAIMABLE: &str = "no waiting task is claimable";
/// The note while the orchestrator's start runs.
pub const STARTING: &str = "starting the orchestrator…";
/// The notice when herdr already names an orchestrator.
pub const ALREADY_RUNNING: &str = "an orchestrator is already running in herdr";
/// The note's prefix when the start fails: the error follows it.
pub const START_FAILED: &str = "the orchestrator did not start";
/// The mark a harness the machine does not have carries in its choice.
pub const NOT_INSTALLED: &str = "not installed";
/// The line under the id input, while the id typed is a model id.
pub const TYPE_HINT: &str = "a model id, such as provider/model";
/// The line under the id input, while the id typed is not a model id.
pub const NOT_A_MODEL_ID: &str = "not a model id";
/// The line under the level input, while no level is typed yet or the level
/// typed has the shape of one.
pub const TYPE_LEVEL_HINT: &str = "a level, such as high";
/// The line under the level input, while the level typed is not one word.
pub const NOT_A_LEVEL: &str = "not a level";
/// The help's block title.
pub const HELP_TITLE: &str = "keys";
/// The choice's state when the catalogue cannot be read.
pub const CATALOG_UNREADABLE: &str = "the model catalog cannot be read";
/// The choice's state while the catalogue's tool runs.
pub const CATALOG_READING: &str = "reading the model catalog";
/// The choice's state when the catalogue lists no model.
pub const NO_MODELS: &str = "no models in the catalog";
/// The line an effort choice says when the agent has no model to take levels
/// from.
pub const NO_MODEL_FIRST: &str = "pick a model first";
/// The line an effort choice says when the model takes no level.
pub const NO_LEVELS: &str = "this model takes no effort level";

/// The sidebar's block title.
pub const SIDEBAR_TITLE: &str = "configuration";
/// The main pane's block title.
pub const FLOW_TITLE: &str = "flow";
/// The main pane's named empty state.
pub const FLOW_EMPTY: &str = "nothing is running";
/// The main pane's state when the record is there but cannot be read.
pub const FLOW_UNREADABLE: &str = "the runtime record cannot be read";
/// The key that moves the keys between the configuration and the queue.
pub const FOCUS_KEY: &str = "tab";
/// The key that marks the queue's task under the cursor.
pub const MARK_KEY: &str = "space";
/// The key that marks every waiting task, or unmarks them.
pub const MARK_ALL_KEY: &str = "a";
/// The captions the main pane's blocks open with: one dim line each, saying
/// what the block counts.
pub const CREW_CAPTION: &str = "the session's crew, and what each agent holds";
pub const QUEUE_CAPTION: &str = "tasks waiting, in order";
pub const ROUND_CAPTION: &str = "where the task in flight is in its build/review loop";
pub const BUDGET_CAPTION: &str = "build/review rounds used of the limit";
pub const FINDINGS_CAPTION: &str = "the task in flight's open findings, by severity";
/// The queue's marks: a marked task and an unmarked one.
const MARKED: &str = "[x]";
const UNMARKED: &str = "[ ]";

/// What hugo found when it looked for the runtime record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeRead {
    /// No project, or no record in it: the empty state.
    Absent,
    /// The record, as read.
    Record(Runtime),
    /// The record is there but cannot be read.
    Unreadable(ReadError),
}

/// A file that could not be read: the file and the error, as raul names a
/// file it cannot read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    pub file: String,
    pub error: String,
}

/// The main pane's blocks, in the order they are read.
pub const CREW_TITLE: &str = "crew";
pub const QUEUE_TITLE: &str = "queue";
pub const ROUND_TITLE: &str = "round";
pub const BUDGET_TITLE: &str = "budget";
pub const FINDINGS_TITLE: &str = "findings";

/// The narrowest the main pane draws its blocks with boxes. Under it the
/// words matter more than the chrome.
const MIN_BOXED_WIDTH: u16 = 20;
/// The cells of the budget's progress bar.
const BUDGET_CELLS: usize = 10;
/// The columns of the crew table: role, model, task, hold and state.
const CREW_COLUMNS: usize = 5;
/// The space between two cells of the crew table.
const CREW_GAP: usize = 1;

/// What a key asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Redraw,
    /// Nothing changed: the loop waits on without a draw.
    Idle,
}

/// A choice: the list of one agent's field, and the cursor on it.
struct Choice {
    /// The agent the choice is for, by its place in the configuration.
    agent: usize,
    /// The choice's title: the agent's name and the field's name.
    title: String,
    /// The role the agent fills, for its icon.
    role: FlowRole,
    /// The field the choice picks: the harness, the model or the effort.
    field: Field,
    /// The entries of the list, in order.
    entries: Vec<Entry>,
    /// The catalogue a model pick is checked against.
    catalog: Catalog,
    /// The entry the cursor is on.
    cursor: usize,
    /// What the choice has to show.
    state: ChoiceState,
    /// The state the id input opened from, so esc goes back to it: a failed
    /// catalogue keeps its reason.
    before_typing: Option<ChoiceState>,
}

/// One entry of a choice's list.
struct Entry {
    /// What the entry says: a harness's name, a model id, an effort level, or
    /// `type an id…`.
    text: String,
    /// Whether enter picks it. A harness the machine does not have is listed,
    /// and cannot be picked.
    pickable: bool,
}

impl Entry {
    fn new(text: &str, pickable: bool) -> Self {
        Self {
            text: text.to_string(),
            pickable,
        }
    }
}

/// What the choice has to show.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ChoiceState {
    /// A catalogue is being read: nothing to pick yet.
    Reading,
    /// The list is the entries.
    Listed,
    /// The catalogue is there and lists no model: only `type an id…` is listed.
    Empty,
    /// The catalogue cannot be read, or the switch cannot be made: why. A model
    /// choice that failed still lists `type an id…` under the reason.
    Failed(String),
    /// The inline input for an id is open, with the id typed so far. It takes
    /// every key but ctrl-c.
    Typing(String),
}

impl Choice {
    /// Whether enter picks what the choice has: the entry under the cursor, or
    /// the id typed.
    fn pickable(&self) -> bool {
        match &self.state {
            ChoiceState::Listed | ChoiceState::Empty => self.entry_pickable(),
            // A model that cannot be listed can still be typed.
            ChoiceState::Failed(_) if self.field == Field::Model => self.entry_pickable(),
            ChoiceState::Typing(text) if self.field == Field::Effort => pick::is_typed_level(text),
            ChoiceState::Typing(text) => pick::is_typed_id(text),
            ChoiceState::Reading | ChoiceState::Failed(_) => false,
        }
    }

    /// Whether the entry under the cursor can be picked.
    fn entry_pickable(&self) -> bool {
        self.entries
            .get(self.cursor)
            .is_some_and(|entry| entry.pickable)
    }

    /// Whether the cursor moves over the list: it is listed, and has more than
    /// one entry.
    fn moves(&self) -> bool {
        self.state == ChoiceState::Listed && self.entries.len() > 1
    }

    fn typing(&self) -> bool {
        matches!(self.state, ChoiceState::Typing(_))
    }

    /// The lines the choice's body takes, before its popup is sized.
    fn rows(&self) -> usize {
        match &self.state {
            ChoiceState::Reading | ChoiceState::Typing(_) => 2,
            ChoiceState::Failed(_) if self.field == Field::Model => 2 + self.entries.len(),
            ChoiceState::Failed(_) => 2,
            ChoiceState::Listed => self.entries.len(),
            ChoiceState::Empty => 1 + self.entries.len(),
        }
    }
}

/// What the note row says about a start: a notice, a success, or a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Note {
    /// Nothing started, or a start is running: dim.
    Notice(String),
    /// The orchestrator started: the tab it runs in.
    Done(String),
    /// The start failed: the error, named.
    Failed(String),
}

/// The pane the keys go to: the configuration in the sidebar, or the queue in
/// the main pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Configuration,
    Queue,
}

/// The roles a crew of `size` agents has: the first `size` of the crew's order.
fn crew_roles(size: usize) -> &'static [FlowRole] {
    &CREW_ORDER[..size.clamp(1, CREW_ORDER.len())]
}

/// The size of the crew a configuration runs, when it is one the selector
/// offers: the roles it has are exactly the roles of that size.
fn crew_size_of(config: &Config) -> Option<usize> {
    (1..=CREW_ORDER.len()).find(|size| {
        let roles = crew_roles(*size);
        config.agents.len() == roles.len()
            && roles
                .iter()
                .all(|role| config.agents.iter().any(|agent| agent.role == *role))
    })
}

/// The crew a session starts from: the file's, or none when the file is not
/// there to be read.
fn session_of(state: &ConfigState) -> Option<Config> {
    match state {
        ConfigState::Config(config) => Some(config.clone()),
        _ => None,
    }
}

/// Take the agent of a role out of a crew, when the crew has one.
fn take_role(agents: &mut Vec<Agent>, role: FlowRole) -> Option<Agent> {
    let at = agents.iter().position(|agent| agent.role == role)?;
    Some(agents.remove(at))
}

/// A copy of the agent of a role, when the list has one.
fn agent_of(agents: &[Agent], role: FlowRole) -> Option<Agent> {
    agents.iter().find(|agent| agent.role == role).cloned()
}

/// The regions the last frame drew, where the pointer can reach them. The draw
/// stores them and a mouse event is answered from them, so a click lands on what
/// is on screen and nothing is recomputed.
#[derive(Debug, Default)]
struct Hits {
    /// The sidebar's pane, its border included, when one is drawn.
    sidebar: Rect,
    /// The sidebar's lines a click picks, each with the cursor position it is.
    picks: Vec<(Rect, usize)>,
    /// The queue's section, its heading included, when one is drawn.
    queue_area: Rect,
    /// The queue's lines, each with the waiting task it is.
    queue: Vec<(Rect, usize)>,
    /// The choice's popup and the entries it drew, when a choice is open.
    choice: Picked,
    /// The crew-size selector's popup and the entries it drew, when it is open.
    crew: Picked,
}

/// The state of the window.
pub struct App {
    /// The colours of the window.
    pub theme: Theme,
    /// The icon set in force: every pane's title and every role draws with it.
    pub icons: IconSet,
    /// The configuration as read from the file. A reload reads it again.
    pub config: ConfigRead,
    /// The crew this session runs: the file's configuration with hugo's picks
    /// made on it. It is never written back; a reload starts it again from the
    /// file. `None` when there is no configuration to start from.
    session: Option<Config>,
    /// The agents a crew-size pick has taken off the session's crew. A role
    /// the crew takes back comes with the values it had.
    parked: Vec<Agent>,
    /// The user's home, when there is one: the configuration lives under it.
    home: Option<PathBuf>,
    /// The sidebar's row the cursor is on, as [`sidebar::cursor_of`] numbers
    /// it: an agent's harness, model or effort.
    cursor: usize,
    /// The agents group is drawn, so the cursor has rows on screen.
    agents_drawn: bool,
    /// The open model choice, when one is open.
    choice: Option<Choice>,
    /// The catalogue read in flight, when the choice waits on one. It runs
    /// off the window's thread, so no key handling waits on a harness tool.
    reading: Option<JoinHandle<Result<Catalog, String>>>,
    /// The levels each harness's models take, as the reads so far have said.
    levels: pick::Levels,
    /// The help overlay is open.
    help: bool,
    /// The crew-size selector is open, with its cursor on this size minus one.
    crew_cursor: Option<usize>,
    /// The crew in force is no size the selector names, so the cursor starts
    /// on no pick: enter does nothing until a key or a click moves it.
    crew_unsized: bool,
    /// The launcher `s` starts the orchestrator through. `None` outside herdr:
    /// the key says why, and starts nothing.
    launch: Option<Arc<dyn Launcher>>,
    /// Whether `hugo` on PATH is the running hugo, which `s` checks before it
    /// starts: agents run `hugo --skill <role>`. A refusal names the folder
    /// to add to PATH.
    hugo_on_path: fn() -> Result<(), String>,
    /// The orchestrator's start in flight. It runs off the window's thread,
    /// as the catalogue read does, so no key handling waits on herdr.
    starting: Option<JoinHandle<Result<Start, String>>>,
    /// The claimable waiting tasks' ids, as the last frame drew them: the work
    /// `s` hands the orchestrator, in queue order. A waiting task that is not
    /// claimable is drawn and not picked.
    queued: Vec<String>,
    /// How many tasks wait, claimable or not, as the last frame drew them.
    waiting: usize,
    /// The pane the keys go to.
    focus: Focus,
    /// The queue row the cursor is on, as the waiting tasks number them.
    queue_cursor: usize,
    /// The waiting tasks marked with `space`, in the order they were marked.
    /// Session-only: a reload or a restart drops them.
    marked: Vec<String>,
    /// The regions the last frame drew, that a mouse event is answered from.
    hits: Hits,
    /// The line the start last said, on the note row.
    note: Option<Note>,
    /// The project's folder as the title line names it, when one is found.
    pub project: Option<String>,
    /// The project's root: the folder the orchestrator's tab opens in.
    pub root: Option<PathBuf>,
    /// The program the catalogue is read from, in place of the harness's own
    /// list: a test's stand-in for it.
    #[cfg(test)]
    catalog_tool: Option<PathBuf>,
    /// Which harnesses the machine has, in place of `PATH`: a test's stand-in.
    #[cfg(test)]
    present: Option<fn(&Harness) -> bool>,
}

/// The row the cursor starts on: the builder's harness, when the crew has a
/// builder; otherwise the orchestrator's, the first row.
fn first_cursor(config: &Config) -> usize {
    config
        .agents
        .iter()
        .position(|agent| agent.role == FlowRole::Builder)
        .map_or(0, |agent| sidebar::cursor_of(agent, Field::Harness))
}

impl App {
    /// A window with a theme and a configuration of its own.
    pub fn new(theme: Theme, icons: IconSet, config: ConfigRead, home: Option<PathBuf>) -> Self {
        let session = session_of(&config.state);
        let cursor = first_cursor(session.as_ref().unwrap_or(&Config::default()));
        Self {
            theme,
            icons,
            config,
            session,
            parked: Vec::new(),
            home,
            cursor,
            agents_drawn: true,
            choice: None,
            reading: None,
            levels: pick::Levels::new(),
            help: false,
            crew_cursor: None,
            crew_unsized: false,
            launch: None,
            hugo_on_path: on_path::check_current,
            starting: None,
            queued: Vec::new(),
            waiting: 0,
            focus: Focus::Configuration,
            queue_cursor: 0,
            marked: Vec::new(),
            hits: Hits::default(),
            note: None,
            project: None,
            root: None,
            #[cfg(test)]
            catalog_tool: None,
            #[cfg(test)]
            present: None,
        }
    }

    /// The window with the launcher `s` starts the orchestrator through, or
    /// `None` outside herdr.
    pub fn with_launcher(mut self, launch: Option<Arc<dyn Launcher>>) -> Self {
        self.launch = launch;
        self
    }

    /// Handle a key. The window shows; it acts on its configuration, and `s`
    /// starts the orchestrator over the queue.
    ///
    /// `q` and `ctrl-c` quit from anywhere, as in raul. The sidebar's keys
    /// move the cursor between the agents' harness, model and effort rows,
    /// open the choice of the row, reload the configuration, and open the
    /// help. The choice's keys move its cursor, pick an entry, and close it.
    /// No other key does anything.
    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Redraw;
        }
        // The id input takes every key but ctrl-c: a letter typed into an id
        // is text, not the window's `q`.
        if self.typing() {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Action::Quit;
            }
            self.typing_key(key);
            return Action::Redraw;
        }
        // Quit is the one key that works in every place.
        if key.code == KeyCode::Char('q')
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return Action::Quit;
        }
        if self.help {
            // The help is a dismissal: any key closes it, and the choice it
            // was opened over is still there.
            self.help = false;
            return Action::Redraw;
        }
        if self.choice.is_some() {
            self.choice_key(key);
            return Action::Redraw;
        }
        if self.crew_cursor.is_some() {
            self.crew_key(key);
            return Action::Redraw;
        }
        if self.focus == Focus::Queue {
            self.queue_key(key);
            return Action::Redraw;
        }
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('r') => self.reload(),
            KeyCode::Char('c') => self.open_crew(),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Enter => self.open_choice(),
            KeyCode::Tab => self.focus_queue(),
            KeyCode::Char('s') => self.start_orchestrator(),
            _ => {}
        }
        Action::Redraw
    }

    /// Handle a key while the queue has focus: the cursor moves over the
    /// waiting tasks, `space` marks the one under it, `a` marks them all, `s`
    /// starts the orchestrator over the marked, and `tab` gives the keys back
    /// to the configuration. The configuration's keys do nothing here.
    fn queue_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Tab => self.focus = Focus::Configuration,
            KeyCode::Up | KeyCode::Char('k') => self.move_queue(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_queue(1),
            KeyCode::Char(' ') => self.mark_cursor(),
            KeyCode::Char('a') => self.mark_all(),
            KeyCode::Char('s') => self.start_orchestrator(),
            _ => {}
        }
    }

    /// Move the keys to the queue, when it has a waiting task to pick.
    fn focus_queue(&mut self) {
        if !self.queued.is_empty() {
            self.focus = Focus::Queue;
        }
    }

    /// Move the queue's cursor over the waiting tasks, and no further.
    fn move_queue(&mut self, delta: isize) {
        let last = self.queued.len().saturating_sub(1);
        self.queue_cursor = self.queue_cursor.saturating_add_signed(delta).min(last);
    }

    /// Mark the waiting task under the cursor, or unmark it when it is marked.
    fn mark_cursor(&mut self) {
        let Some(id) = self.queued.get(self.queue_cursor).cloned() else {
            return;
        };
        match self.marked.iter().position(|marked| *marked == id) {
            Some(at) => {
                self.marked.remove(at);
            }
            None => self.marked.push(id),
        }
    }

    /// Mark every waiting task, or unmark them all when they are all marked.
    fn mark_all(&mut self) {
        if self.queued.iter().all(|id| self.marked.contains(id)) {
            self.marked.retain(|id| !self.queued.contains(id));
        } else {
            for id in &self.queued {
                if !self.marked.contains(id) {
                    self.marked.push(id.clone());
                }
            }
        }
    }

    /// The tasks `s` hands the orchestrator, in queue order: the marked ones,
    /// or the whole queue when none is marked.
    fn starting_tasks(&self) -> Vec<String> {
        let marked: Vec<String> = self
            .queued
            .iter()
            .filter(|id| self.marked.contains(id))
            .cloned()
            .collect();
        if marked.is_empty() {
            self.queued.clone()
        } else {
            marked
        }
    }

    /// Handle a key while the crew-size selector is open: the cursor moves
    /// over the sizes, enter picks the size under it, and esc closes it.
    fn crew_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Esc => self.crew_cursor = None,
            KeyCode::Up | KeyCode::Char('k') => self.move_crew(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_crew(1),
            KeyCode::Enter => {
                if !self.crew_unsized
                    && let Some(cursor) = self.crew_cursor.take()
                {
                    self.set_crew(cursor + 1);
                }
            }
            _ => {}
        }
    }

    /// Handle a key while a choice covers the sidebar.
    fn choice_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Esc => self.close_choice(),
            KeyCode::Up | KeyCode::Char('k') => self.move_choice(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_choice(1),
            // Enter picks only what can be picked: while a catalogue is read,
            // or when a harness switch failed, enter does nothing.
            KeyCode::Enter => self.pick(),
            _ => {}
        }
    }

    /// Handle a key while the id input is open. `esc` goes back to the list;
    /// enter picks the id once it has the shape of a model id.
    fn typing_key(&mut self, key: KeyEvent) {
        let Some(choice) = self.choice.as_mut() else {
            return;
        };
        if key.code == KeyCode::Esc {
            choice.state = choice.before_typing.take().unwrap_or(ChoiceState::Listed);
            return;
        }
        let agent = choice.agent;
        let field = choice.field;
        let ChoiceState::Typing(text) = &mut choice.state else {
            return;
        };
        match key.code {
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                text.push(character)
            }
            KeyCode::Backspace => {
                text.pop();
            }
            KeyCode::Enter if field == Field::Effort && pick::is_typed_level(text) => {
                let level = text.trim().to_string();
                self.store_effort(agent, &level, false);
                self.close_choice();
            }
            KeyCode::Enter if field == Field::Model && pick::is_typed_id(text) => {
                let model = text.trim().to_string();
                let levels = self.levels_of(agent, &model);
                self.set_model(agent, model, &levels);
                self.close_choice();
            }
            _ => {}
        }
    }

    /// Handle a mouse event. The wheel scrolls what is under the pointer by
    /// `WHEEL_ROWS` a notch: the sidebar's rows, the queue, or a picker's
    /// entries. A left click picks a sidebar row, a queue row or a picker entry;
    /// a second click on the row the cursor is on answers as `enter` does (a
    /// sidebar row opens its choice) or as `space` does (a queue row marks its
    /// task). A click anywhere else does nothing.
    ///
    /// The regions are the ones the last frame drew. While the help or the id
    /// input takes the keys, the pointer waits for them. A move or a drag, and
    /// a button's release, change nothing: they answer `Idle`, so a pointer
    /// passing over the window costs no redraw.
    pub fn on_mouse(&mut self, mouse: MouseEvent) -> Action {
        if self.help || self.typing() {
            return Action::Idle;
        }
        let at = Position::new(mouse.column, mouse.row);
        let step = match mouse.kind {
            MouseEventKind::ScrollDown => WHEEL_ROWS,
            MouseEventKind::ScrollUp => -WHEEL_ROWS,
            MouseEventKind::Down(MouseButton::Left) => return self.click(at),
            _ => return Action::Idle,
        };
        if self.choice.is_some() {
            if self.hits.choice.area.contains(at) {
                self.move_choice(step);
            }
        } else if self.crew_cursor.is_some() {
            if self.hits.crew.area.contains(at) {
                self.move_crew(step);
            }
        } else if self.hits.sidebar.contains(at) {
            // The pointer takes the focus, as a click does.
            self.focus = Focus::Configuration;
            self.move_cursor(step);
        } else if self.hits.queue_area.contains(at) && !self.queued.is_empty() {
            self.focus = Focus::Queue;
            self.move_queue(step);
        }
        Action::Redraw
    }

    /// Answer a left click at `at` from the regions the last frame drew.
    fn click(&mut self, at: Position) -> Action {
        if self.choice.is_some() {
            if let Some(index) = hit(&self.hits.choice.entries, at) {
                let cursor = self.choice.as_ref().map_or(0, |choice| choice.cursor);
                if index == cursor {
                    self.pick();
                } else {
                    self.move_choice(index as isize - cursor as isize);
                }
            }
            return Action::Redraw;
        }
        if self.crew_cursor.is_some() {
            if let Some(index) = hit(&self.hits.crew.entries, at) {
                if self.crew_cursor == Some(index) && !self.crew_unsized {
                    if let Some(cursor) = self.crew_cursor.take() {
                        self.set_crew(cursor + 1);
                    }
                } else {
                    self.crew_cursor = Some(index);
                    self.crew_unsized = false;
                }
            }
            return Action::Redraw;
        }
        if let Some(row) = hit(&self.hits.picks, at) {
            // The second click on the row the cursor is on opens its choice,
            // as `enter` does on it.
            let selected = self.focus == Focus::Configuration && self.cursor == row;
            self.focus = Focus::Configuration;
            self.cursor = row;
            if selected {
                self.open_choice();
            }
        } else if let Some(task) = hit(&self.hits.queue, at) {
            // The second click on the task the cursor is on marks it, as
            // `space` does on it.
            let selected = self.focus == Focus::Queue && self.queue_cursor == task;
            self.focus = Focus::Queue;
            self.queue_cursor = task;
            if selected {
                self.mark_cursor();
            }
        }
        Action::Redraw
    }

    /// Draw the window. `now` is the instant holds are measured at, and `tick`
    /// is the frame the spinner shows.
    pub fn draw(&mut self, frame: &mut Frame, runtime: &RuntimeRead, now: &str, tick: usize) {
        // The regions are the ones this frame draws: a click answers to them.
        self.hits = Hits::default();
        // The queue `s` starts the orchestrator over is the one drawn: the
        // claimable waiting work, in order. Held and done tasks are not in it,
        // and a waiting task the record does not call claimable is not picked.
        self.queued = match runtime {
            RuntimeRead::Record(runtime) => {
                runtime.claimable().map(|item| item.id.clone()).collect()
            }
            _ => Vec::new(),
        };
        self.waiting = match runtime {
            RuntimeRead::Record(runtime) => runtime.waiting().count(),
            _ => 0,
        };
        // The queue keeps the keys only while it has a task to pick, and its
        // cursor stays on a row it has.
        if self.queued.is_empty() {
            self.focus = Focus::Configuration;
        }
        self.queue_cursor = self.queue_cursor.min(self.queued.len().saturating_sub(1));
        // The footer is one line, or two when its keys do not fit on one. A
        // window too narrow for the sidebar draws no agent, so the choice is
        // off before the footer is measured; a window wide enough holds the
        // footer on one line whatever the choice is.
        let area = frame.area();
        if area.width < SIDEBAR_MIN_WIDTH {
            self.agents_drawn = false;
        }
        let footer_rows = self.footer_lines(area.width, 2).len();
        let screen = screen(
            area,
            self.note.is_some(),
            u16::try_from(footer_rows).unwrap_or(2),
        );
        frame.render_widget(self.title_line(screen.title.width, runtime), screen.title);
        if let Some(area) = screen.announcement {
            frame.render_widget(self.hidden_line(area.width), area);
        }
        self.agents_drawn = screen
            .sidebar
            .is_some_and(|area| self.config_pane(frame, area));
        match runtime {
            RuntimeRead::Record(runtime) => self.flow(frame, screen.flow, runtime, now, tick),
            RuntimeRead::Absent => self.empty_pane(frame, screen.flow, FLOW_TITLE, FLOW_EMPTY),
            RuntimeRead::Unreadable(error) => self.unreadable_pane(frame, screen.flow, error),
        }
        if let Some(area) = screen.note {
            frame.render_widget(self.note_line(area.width), area);
        }
        frame.render_widget(
            Line::from(chrome::rule(screen.rule.width, &self.theme)),
            screen.rule,
        );
        let lines = self.footer_lines(screen.footer.width, screen.footer.height);
        frame.render_widget(Paragraph::new(lines), screen.footer);
        if self.help {
            self.help_pane(frame, frame.area());
        } else if self.choice.is_some() {
            self.choice_pane(frame, frame.area());
        } else if self.crew_cursor.is_some() {
            self.crew_pane(frame, frame.area());
        }
    }

    /// The configuration: the sidebar's content, with the cursor on an agent.
    ///
    /// A pane too short for a box gives the box up and keeps the words, as the
    /// main pane does. Returns whether the agents group — the cursor's rows —
    /// is on screen.
    fn config_pane(&mut self, frame: &mut Frame, area: Rect) -> bool {
        if area.height == 0 || area.width == 0 {
            return false;
        }
        self.hits.sidebar = area;
        let (inner, width, height) = if area.height >= 3 && area.width >= 4 {
            let block = chrome::pane(SIDEBAR_TITLE, Icon::Config, self.icons, false, &self.theme)
                .style(self.theme.chrome.surface);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            (inner, inner.width, inner.height)
        } else {
            (area, area.width, area.height)
        };
        let cursor = if self.session().is_some() && self.focus == Focus::Configuration {
            self.cursor
        } else {
            usize::MAX
        };
        let drawn = sidebar::lines(
            &self.config,
            self.session(),
            &self.levels,
            &self.theme,
            self.icons,
            usize::from(width),
            usize::from(height),
            cursor,
        );
        // Each line of an agent's field is a row a click picks.
        self.hits.picks = drawn
            .picks
            .iter()
            .filter_map(|(line, pick)| {
                let line = u16::try_from(*line)
                    .ok()
                    .filter(|line| *line < inner.height)?;
                let rect = Rect {
                    x: inner.x,
                    y: inner.y + line,
                    width: inner.width,
                    height: 1,
                };
                Some((rect, *pick))
            })
            .collect();
        frame.render_widget(
            Paragraph::new(drawn.lines).style(self.theme.chrome.surface),
            inner,
        );
        drawn.agents
    }

    /// The configuration as loaded, when the file parsed.
    fn session(&self) -> Option<&Config> {
        self.session.as_ref()
    }

    /// The agent and the field the cursor is on, when the configuration has
    /// them.
    fn cursor_row(&self) -> Option<(usize, Field)> {
        let config = self.session()?;
        let (agent, field) = sidebar::field_at(self.cursor);
        (agent < config.agents.len()).then_some((agent, field))
    }

    /// Whether `enter` has a choice to open: the cursor is on an agent's row,
    /// the configuration parsed, and its group is drawn.
    fn can_choose(&self) -> bool {
        self.agents_drawn && self.cursor_row().is_some()
    }

    /// Move the cursor over the agents' rows, and no further.
    fn move_cursor(&mut self, delta: isize) {
        if !self.agents_drawn {
            return;
        }
        let Some(rows) = self.session().map(sidebar::pick_rows) else {
            return;
        };
        if rows == 0 {
            return;
        }
        let next = self.cursor as isize + delta;
        self.cursor = next.clamp(0, rows as isize - 1) as usize;
    }

    /// Read the configuration again, and start the session over from it.
    ///
    /// The file is the session's start, so the picks made this session are
    /// dropped and the sidebar shows the file as it is now. This is the
    /// configuration only. The runtime record is already read on every redraw,
    /// and a catalogue is read when a choice opens.
    fn reload(&mut self) {
        self.config = config::load(self.home.as_deref());
        self.session = session_of(&self.config.state);
        self.parked.clear();
        self.marked.clear();
        self.cursor = match self.session() {
            Some(config) => self
                .cursor
                .min(sidebar::pick_rows(config).saturating_sub(1)),
            None => 0,
        };
    }

    /// Open the choice of the cursor's row.
    ///
    /// A harness is listed from the table, and an effort from its harness's
    /// levels. A model is read off the window's thread: the choice opens at
    /// once and says it is reading, and takes the list — or the error — when
    /// the harness's tool answers. A tool that hangs is bounded by
    /// [`Catalog::read`], and the window keeps drawing and taking keys
    /// meanwhile.
    fn open_choice(&mut self) {
        if !self.can_choose() {
            return;
        }
        let Some((agent, field)) = self.cursor_row() else {
            return;
        };
        match field {
            Field::Harness => self.open_harness_choice(agent),
            Field::Model => self.open_model_choice(agent),
            Field::Effort => self.open_effort_choice(agent),
        }
    }

    /// Open the crew-size selector, with its cursor on the crew in force. A
    /// window with no configuration has no crew to size. A crew no size names
    /// (an orchestrator and a reviewer, say) has no size in force: the cursor
    /// starts on the first size, unpicked, and enter waits for a move.
    fn open_crew(&mut self) {
        let Some(size) = self.session().map(crew_size_of) else {
            return;
        };
        self.crew_unsized = size.is_none();
        self.crew_cursor = Some(size.unwrap_or(1) - 1);
    }

    /// Move the crew-size selector's cursor over the three sizes. A real move is
    /// the first pick of a crew no size names; a move at the end of the list is
    /// no move and leaves it unpicked.
    fn move_crew(&mut self, delta: isize) {
        if let Some(cursor) = self.crew_cursor.as_mut() {
            let last = CREW_ORDER.len() as isize - 1;
            let moved = (*cursor as isize + delta).clamp(0, last) as usize;
            if moved != *cursor {
                *cursor = moved;
                self.crew_unsized = false;
            }
        }
    }

    /// Set the session's crew to the roles of `size`.
    ///
    /// A role the crew keeps keeps its values. A role it takes back comes with
    /// the values it was taken off with, else the file's, else the defaults.
    /// A role it leaves off is parked, not lost. The file is not written.
    fn set_crew(&mut self, size: usize) {
        let wanted = crew_roles(size);
        let file = session_of(&self.config.state).unwrap_or_default();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut agents = Vec::new();
        for role in CREW_ORDER {
            let present = take_role(&mut session.agents, role);
            if !wanted.contains(&role) {
                if let Some(agent) = present {
                    self.parked.push(agent);
                }
                continue;
            }
            agents.extend(
                present
                    .or_else(|| take_role(&mut self.parked, role))
                    .or_else(|| agent_of(&file.agents, role))
                    .or_else(|| agent_of(&Config::default().agents, role)),
            );
        }
        session.agents = agents;
        let rows = sidebar::pick_rows(session);
        self.cursor = self.cursor.min(rows.saturating_sub(1));
    }

    /// Open the harness choice: every harness the table holds, and one the
    /// machine does not have listed dim and unpickable.
    fn open_harness_choice(&mut self, agent: usize) {
        let Some(current) = self
            .session()
            .and_then(|config| config.agents.get(agent))
            .map(|row| row.harness.clone())
        else {
            return;
        };
        let entries: Vec<Entry> = HARNESSES
            .iter()
            .map(|harness| Entry::new(harness.name, self.installed(harness)))
            .collect();
        let cursor = entries
            .iter()
            .position(|entry| entry.text == current)
            .unwrap_or(0);
        self.show(agent, Field::Harness, entries, cursor, ChoiceState::Listed);
    }

    /// Open the model choice, reading the agent's harness's catalogue beside
    /// the window.
    fn open_model_choice(&mut self, agent: usize) {
        let Some(harness) = self
            .session()
            .and_then(|config| config.agents.get(agent))
            .and_then(|row| harness::find(&row.harness))
        else {
            return;
        };
        let reading = self.read_catalog(harness);
        self.reading = Some(reading);
        self.show(agent, Field::Model, Vec::new(), 0, ChoiceState::Reading);
    }

    /// Open the effort choice: the levels the agent's model takes, read from
    /// its harness's catalogue beside the window.
    fn open_effort_choice(&mut self, agent: usize) {
        let Some(harness) = self
            .session()
            .and_then(|config| config.agents.get(agent))
            .and_then(|row| harness::find(&row.harness))
        else {
            return;
        };
        let reading = self.read_catalog(harness);
        self.reading = Some(reading);
        self.show(agent, Field::Effort, Vec::new(), 0, ChoiceState::Reading);
    }

    /// Whether a harness is on the machine. A harness the machine does not
    /// have cannot run, so it cannot be picked.
    fn installed(&self, harness: &Harness) -> bool {
        #[cfg(test)]
        {
            if let Some(present) = self.present {
                return present(harness);
            }
        }
        harness.on_path()
    }

    /// Open a choice for an agent's field, on the entries and the cursor given.
    fn show(
        &mut self,
        agent: usize,
        field: Field,
        entries: Vec<Entry>,
        cursor: usize,
        state: ChoiceState,
    ) {
        let Some((title, role)) = self
            .session()
            .and_then(|config| config.agents.get(agent))
            .map(|row| (format!("{} {}", row.role.as_str(), field.label()), row.role))
        else {
            return;
        };
        self.help = false;
        self.choice = Some(Choice {
            agent,
            title,
            role,
            field,
            entries,
            catalog: Catalog::default(),
            cursor,
            state,
            before_typing: None,
        });
    }

    /// Read a harness's catalogue beside the window.
    fn read_catalog(&self, harness: &'static Harness) -> JoinHandle<Result<Catalog, String>> {
        #[cfg(test)]
        {
            if let Some(tool) = self.catalog_tool.clone() {
                return std::thread::spawn(move || {
                    Catalog::read_command(
                        &tool.to_string_lossy(),
                        &[],
                        coding_crew_core::harness::Listing::Lines,
                        crate::catalog::READ_BOUND,
                    )
                });
            }
        }
        std::thread::spawn(move || Catalog::read(harness))
    }

    /// Whether a choice is waiting on a catalogue.
    pub fn catalog_pending(&self) -> bool {
        self.reading.is_some()
    }

    /// Take the catalogue when the read has finished, without waiting on it.
    ///
    /// Returns whether the read finished, so the caller can draw at once. A
    /// choice the human closed while the read ran stays closed: its answer is
    /// dropped, not reopened.
    pub fn poll_choice(&mut self) -> bool {
        let Some(reading) = self.reading.as_ref() else {
            return false;
        };
        if !reading.is_finished() {
            return false;
        }
        let reading = self.reading.take().expect("the read in flight");
        let loaded = reading
            .join()
            .unwrap_or_else(|_| Err("the model catalog reader stopped".to_string()));
        match self.choice.as_ref().map(|choice| choice.field) {
            Some(Field::Harness) => self.finish_switch(loaded),
            Some(Field::Effort) => self.fill_effort(loaded),
            Some(Field::Model) => self.fill_choice(loaded),
            None => {}
        }
        true
    }

    /// Close the choice, abandoning a read in flight with it.
    fn close_choice(&mut self) {
        self.choice = None;
        self.reading = None;
    }

    /// Put a catalogue — or the reason there is none — in the open model
    /// choice.
    ///
    /// The list is the catalogue's models, with `type an id…` last. The cursor
    /// starts on the model in force, so a pick is a move from where the
    /// configuration is rather than from the top of the list.
    fn fill_choice(&mut self, loaded: Result<Catalog, String>) {
        let Some(agent) = self.choice.as_ref().map(|choice| choice.agent) else {
            return;
        };
        let current = self
            .session()
            .and_then(|config| config.agents.get(agent))
            .map(|row| row.model.clone())
            .unwrap_or_default();
        let harness = self.harness_of(agent);
        let (catalog, state) = match loaded {
            Ok(catalog) => {
                if let Some(harness) = harness {
                    self.levels.insert(harness, catalog.clone());
                }
                let state = if catalog.ids().is_empty() {
                    ChoiceState::Empty
                } else {
                    ChoiceState::Listed
                };
                (catalog, state)
            }
            Err(error) => (Catalog::default(), ChoiceState::Failed(error)),
        };
        let entries: Vec<Entry> = catalog
            .ids()
            .into_iter()
            .map(|model| Entry::new(model, true))
            .chain([Entry::new(pick::TYPE_AN_ID, true)])
            .collect();
        let cursor = catalog.index_of(&current).unwrap_or(0);
        if let Some(choice) = self.choice.as_mut() {
            choice.entries = entries;
            choice.catalog = catalog;
            choice.cursor = cursor;
            choice.state = state;
        }
    }

    /// Put the levels of the open effort choice's model in it, from the
    /// catalogue the read brought, or the reason there are none.
    ///
    /// The list is the model's levels, with the level in force under the
    /// cursor. A model that takes none says so in place of a list; a model the
    /// catalogue does not list is offered the harness's levels, `none` and
    /// `type a level…`.
    fn fill_effort(&mut self, loaded: Result<Catalog, String>) {
        let Some(agent) = self.choice.as_ref().map(|choice| choice.agent) else {
            return;
        };
        let Some((harness, model, effort)) = self.session().and_then(|config| {
            let row = config.agents.get(agent)?;
            Some((row.harness.clone(), row.model.clone(), row.effort.clone()))
        }) else {
            return;
        };
        let (entries, cursor, state) = match loaded {
            Ok(catalog) => {
                // A model the catalogue does not list has no levels of its own:
                // it is offered the harness's, then `none` and a typed level. A
                // listed model is offered its levels, and `none` when it also
                // runs bare.
                let offered = match catalog.efforts_of(&model) {
                    _ if model.is_empty() => Vec::new(),
                    // A model that takes no level is offered none to pick.
                    Some([]) => Vec::new(),
                    Some(levels) => {
                        let mut offered = levels.to_vec();
                        if catalog.takes_none(&model)
                            && !offered
                                .iter()
                                .any(|level| level == coding_crew_core::catalog::NONE_ENTRY)
                        {
                            offered.push(coding_crew_core::catalog::NONE_ENTRY.to_string());
                        }
                        offered
                    }
                    None => catalog.unlisted_choices(),
                };
                self.levels.insert(&harness, catalog);
                if offered.is_empty() {
                    (Vec::new(), 0, ChoiceState::Empty)
                } else {
                    let wanted = effort.as_deref().map(pick::effort_word);
                    let cursor = offered
                        .iter()
                        .position(|level| Some(level.as_str()) == wanted)
                        .unwrap_or(0);
                    let entries = offered
                        .iter()
                        .map(|level| Entry::new(level, true))
                        .collect();
                    (entries, cursor, ChoiceState::Listed)
                }
            }
            Err(error) => (Vec::new(), 0, ChoiceState::Failed(error)),
        };
        if let Some(choice) = self.choice.as_mut() {
            choice.entries = entries;
            choice.cursor = cursor;
            choice.state = state;
        }
    }

    /// Finish a harness switch once the new harness's catalogue has answered.
    ///
    /// The model and the effort are checked against the new harness's list
    /// before the switch is saved, so the configuration never keeps a model its
    /// harness does not list.
    fn finish_switch(&mut self, loaded: Result<Catalog, String>) {
        let Some((agent, name)) = self.entry_under_cursor() else {
            return;
        };
        match loaded {
            Ok(catalog) => {
                self.close_choice();
                let Some(target) = harness::find(&name) else {
                    return;
                };
                self.levels.insert(target.name, catalog.clone());
                self.change(move |config| {
                    if let Some(row) = config.agents.get_mut(agent) {
                        pick::switch_harness(row, target, &catalog);
                    }
                });
            }
            Err(error) => self.set_state(ChoiceState::Failed(error)),
        }
    }

    /// Set the open choice's state, when a choice is open.
    fn set_state(&mut self, state: ChoiceState) {
        if let Some(choice) = self.choice.as_mut() {
            choice.state = state;
        }
    }

    /// Open the id input over the open choice, keeping the state it opened
    /// from for esc.
    fn open_typing(&mut self) {
        if let Some(choice) = self.choice.as_mut() {
            choice.before_typing = Some(choice.state.clone());
            choice.state = ChoiceState::Typing(String::new());
        }
    }

    /// The agent of the open choice, and the text of the entry under its
    /// cursor.
    fn entry_under_cursor(&self) -> Option<(usize, String)> {
        let choice = self.choice.as_ref()?;
        let entry = choice.entries.get(choice.cursor)?;
        Some((choice.agent, entry.text.clone()))
    }

    /// Pick what the open choice's cursor is on, when it can be picked.
    fn pick(&mut self) {
        let Some(choice) = self.choice.as_ref() else {
            return;
        };
        if !choice.pickable() {
            return;
        }
        match choice.field {
            Field::Harness => self.pick_harness(),
            Field::Model => self.pick_model(),
            Field::Effort => self.pick_effort(),
        }
    }

    /// Pick the harness under the cursor. A harness that is not the one in
    /// force is switched once its catalogue has answered: its models decide
    /// what the switch keeps.
    fn pick_harness(&mut self) {
        let Some((agent, name)) = self.entry_under_cursor() else {
            return;
        };
        let Some(target) = harness::find(&name) else {
            return;
        };
        let current = self
            .session()
            .and_then(|config| config.agents.get(agent))
            .map(|row| row.harness.clone());
        if current.as_deref() == Some(name.as_str()) {
            self.close_choice();
            return;
        }
        let reading = self.read_catalog(target);
        self.reading = Some(reading);
        self.set_state(ChoiceState::Reading);
    }

    /// Pick the model under the cursor. `type an id…` opens the id input
    /// instead.
    fn pick_model(&mut self) {
        let Some((agent, model)) = self.entry_under_cursor() else {
            return;
        };
        if model == pick::TYPE_AN_ID {
            self.open_typing();
            return;
        }
        let levels = self
            .choice
            .as_ref()
            .and_then(|choice| choice.catalog.efforts_of(&model))
            .map(<[String]>::to_vec);
        if let Some(levels) = levels {
            self.set_model(agent, model, &levels);
        }
        self.close_choice();
    }

    /// Pick the effort under the cursor. `type a level…` opens the level input
    /// instead.
    fn pick_effort(&mut self) {
        let Some((agent, entry)) = self.entry_under_cursor() else {
            return;
        };
        if entry == pick::TYPE_A_LEVEL {
            self.open_typing();
            return;
        }
        // A model that also runs bare stores `none` as no level, as an
        // unlisted model does: its listing names the bare start.
        let listed = self.effort_listed(agent) && !self.takes_none_of(agent);
        self.store_effort(agent, &entry, listed);
        self.close_choice();
    }

    /// Whether an agent's model runs with no level, as its catalogue reads.
    fn takes_none_of(&self, agent: usize) -> bool {
        self.session()
            .and_then(|config| config.agents.get(agent))
            .is_some_and(|row| self.levels.takes_none(row))
    }

    /// Save an agent's effort as a picked entry: `none` on a model the
    /// catalogue does not list is stored as no effort (see
    /// [`pick::stored_effort`]).
    fn store_effort(&mut self, agent: usize, entry: &str, listed: bool) {
        let effort = pick::stored_effort(entry, listed);
        self.change(move |config| {
            if let Some(row) = config.agents.get_mut(agent) {
                row.effort = Some(effort);
            }
        });
    }

    /// Whether the catalogue lists the model an agent runs, so its levels are
    /// known.
    fn effort_listed(&self, agent: usize) -> bool {
        self.session()
            .and_then(|config| config.agents.get(agent))
            .and_then(|row| self.levels.catalog(&row.harness)?.efforts_of(&row.model))
            .is_some()
    }

    /// Save an agent's model. The effort stays only when the model takes it:
    /// `levels` are the model's, and empty when they are not known.
    fn set_model(&mut self, agent: usize, model: String, levels: &[String]) {
        self.change(move |config| {
            if let Some(row) = config.agents.get_mut(agent) {
                if !pick::keeps_effort(row.effort.as_deref(), Some(levels)) {
                    row.effort = None;
                }
                row.model = model.clone();
            }
        });
    }

    /// The levels an agent's model takes, as the reads so far have them: none
    /// when the harness's catalogue has not listed the model.
    fn levels_of(&self, agent: usize, model: &str) -> Vec<String> {
        self.session()
            .and_then(|config| config.agents.get(agent))
            .and_then(|row| self.levels.catalog(&row.harness))
            .and_then(|catalog| catalog.efforts_of(model))
            .map(<[String]>::to_vec)
            .unwrap_or_default()
    }

    /// The harness an agent runs in, when the table holds it.
    fn harness_of(&self, agent: usize) -> Option<&'static str> {
        let name = self.session()?.agents.get(agent)?.harness.clone();
        harness::find(&name).map(|harness| harness.name)
    }

    /// Apply one pick to the session's crew. The file is not written: a pick
    /// lasts the session, and a reload starts the session over from the file.
    fn change(&mut self, change: impl FnOnce(&mut Config)) {
        if let Some(session) = self.session.as_mut() {
            change(session);
        }
    }

    /// Move the open choice's cursor along its list, while the list is shown.
    fn move_choice(&mut self, delta: isize) {
        let Some(choice) = self.choice.as_mut() else {
            return;
        };
        if choice.state != ChoiceState::Listed {
            return;
        }
        let last = choice.entries.len().saturating_sub(1) as isize;
        choice.cursor = (choice.cursor as isize + delta).clamp(0, last) as usize;
    }

    /// Whether the id input is open.
    fn typing(&self) -> bool {
        self.choice.as_ref().is_some_and(Choice::typing)
    }

    /// What the configuration lacks before the flow can start.
    fn gaps(&self) -> Vec<String> {
        self.session()
            .map(|config| pick::gaps(config, &self.levels))
            .unwrap_or_default()
    }

    /// Whether `s` can act: herdr hosts hugo, the queue has work, every agent
    /// has its harness, model and effort, and no start is in flight.
    fn can_start(&self) -> bool {
        self.launch.is_some()
            && !self.queued.is_empty()
            && self.starting.is_none()
            && self.gaps().is_empty()
            && (self.hugo_on_path)().is_ok()
    }

    /// Start the orchestrator over the queue, from `s`.
    ///
    /// Outside herdr, or with nothing queued, the key says why and starts
    /// nothing. The start runs beside the window, and [`App::poll_start`]
    /// takes its answer.
    fn start_orchestrator(&mut self) {
        if self.starting.is_some() {
            self.note = Some(Note::Notice(STARTING.to_string()));
            return;
        }
        let Some(launcher) = self.launch.clone() else {
            self.note = Some(Note::Notice(START_NEEDS_HERDR.to_string()));
            return;
        };
        if self.queued.is_empty() {
            let notice = if self.waiting > 0 {
                NONE_CLAIMABLE
            } else {
                QUEUE_EMPTY
            };
            self.note = Some(Note::Notice(notice.to_string()));
            return;
        }
        let gaps = self.gaps();
        if !gaps.is_empty() {
            self.note = Some(Note::Failed(format!("{START_FAILED}: {}", gaps.join(", "))));
            return;
        }
        if let Err(why) = (self.hugo_on_path)() {
            self.note = Some(Note::Failed(format!("{START_FAILED}: {why}")));
            return;
        }
        let request = match self.launch_request() {
            Ok(request) => request,
            Err(why) => {
                self.note = Some(Note::Failed(format!("{START_FAILED}: {why}")));
                return;
            }
        };
        self.note = Some(Note::Notice(STARTING.to_string()));
        self.starting = Some(std::thread::spawn(move || {
            launch::start(launcher.as_ref(), &request).map_err(|error| format!("{error:#}"))
        }));
    }

    /// The request a start is made with: the orchestrator's agent kind and
    /// the invocation that starts it in its harness, as the configuration
    /// holds them, the project's root, and the queue in order.
    fn launch_request(&self) -> Result<LaunchRequest, String> {
        let config = self
            .session()
            .ok_or_else(|| "the configuration cannot be read".to_string())?;
        let agent = config
            .agents
            .iter()
            .find(|agent| agent.role == FlowRole::Orchestrator)
            .ok_or_else(|| "the configuration names no orchestrator".to_string())?;
        let harness = harness::find(&agent.harness)
            .ok_or_else(|| format!("{} is not a harness", agent.harness))?;
        let start = harness.invocation(agent.role.as_str(), &agent.model, agent.effort.as_deref());
        let project = self
            .root
            .clone()
            .ok_or_else(|| "the project's folder is not known".to_string())?;
        // The orchestrator's `hugo --skill` prints the crew this session runs,
        // so the session's picks reach the agents it starts.
        let mut env = start.env;
        env.push((config::CREW_ENV.to_string(), config::encode_crew(config)));
        Ok(LaunchRequest {
            kind: harness.kind.to_string(),
            args: start.args,
            env,
            project,
            tasks: self.starting_tasks(),
        })
    }

    /// Whether the orchestrator's start is in flight.
    pub fn start_pending(&self) -> bool {
        self.starting.is_some()
    }

    /// Take the start's answer when it has finished, without waiting on it.
    ///
    /// Returns whether the start finished, so the caller can draw at once.
    pub fn poll_start(&mut self) -> bool {
        let Some(starting) = self.starting.as_ref() else {
            return false;
        };
        if !starting.is_finished() {
            return false;
        }
        let starting = self.starting.take().expect("the start in flight");
        let outcome = starting
            .join()
            .unwrap_or_else(|_| Err("the orchestrator's start stopped".to_string()));
        self.note = Some(match outcome {
            Ok(Start::Started(launched)) => {
                Note::Done(format!("orchestrator started in tab {}", launched.tab))
            }
            Ok(Start::AlreadyRunning) => Note::Notice(ALREADY_RUNNING.to_string()),
            Err(error) => Note::Failed(format!("{START_FAILED}: {error}")),
        });
        true
    }

    /// The note row: what the last start said, in one line.
    fn note_line(&self, width: u16) -> Line<'static> {
        let Some(note) = &self.note else {
            return Line::from("");
        };
        let (text, style) = match note {
            Note::Notice(text) => (text, self.theme.text.dim),
            Note::Done(text) => (text, self.theme.semantic.ok),
            Note::Failed(text) => (text, self.theme.semantic.error),
        };
        Line::from(Span::styled(
            cut_to(&format!(" {text}"), usize::from(width)),
            style,
        ))
    }

    /// A block of the main pane: its title on the accent.
    fn main_title(&self, title: &'static str, icon: Icon) -> Block<'static> {
        chrome::pane_titled(
            title,
            icon,
            self.icons,
            false,
            self.theme.chrome.tab_active,
            &self.theme,
        )
    }

    /// The title line: the app's icon and name, a dim `·` and the project
    /// folder, and on the right edge the status pill: whether the automation
    /// runs.
    ///
    /// A terminal too narrow for the name shows nothing rather than a cut of
    /// it: the parts of the line are dropped whole. The pill goes before the
    /// folder is cut. The folder is cut from the front, so the folder that
    /// names the project is kept.
    fn title_line(&self, width: u16, runtime: &RuntimeRead) -> Line<'static> {
        let name = format!(" {} {NAME}", Icon::AppHugo.glyph(self.icons));
        let width = usize::from(width);
        if width < text_width(&name) {
            return Line::from("");
        }
        let name = Span::styled(name, self.theme.chrome.title);
        let separator = " · ";
        if let Some(pill) = self.pill(runtime) {
            let whole = text_width(&name.content)
                + self
                    .project
                    .as_deref()
                    .map_or(0, |project| text_width(separator) + text_width(project));
            if let Some(gap) = width
                .checked_sub(whole + pill.width())
                .filter(|gap| *gap >= 1)
            {
                let mut spans = vec![name.clone()];
                if let Some(project) = &self.project {
                    spans.push(Span::styled(separator, self.theme.text.dim));
                    spans.push(Span::styled(project.clone(), self.theme.text.dim));
                }
                spans.push(Span::raw(" ".repeat(gap)));
                spans.push(pill);
                return Line::from(spans);
            }
        }
        let Some(project) = &self.project else {
            return Line::from(name);
        };
        let head = text_width(&name.content) + text_width(separator);
        let room = width.saturating_sub(head);
        if room == 0 {
            return Line::from(name);
        }
        let path = tail_to(project, room);
        Line::from(vec![
            name,
            Span::styled(separator, self.theme.text.dim),
            Span::styled(path, self.theme.text.dim),
        ])
    }

    /// The header's status pill: whether the automation runs, in words with
    /// its icon. None when the record cannot be read, since then the state
    /// is not known. No record means nothing runs.
    fn pill(&self, runtime: &RuntimeRead) -> Option<Span<'static>> {
        let (icon, word, style) = match runtime {
            RuntimeRead::Record(runtime) if runtime.stamp.runs => {
                (Icon::Running, "running", self.theme.semantic.ok)
            }
            RuntimeRead::Record(_) | RuntimeRead::Absent => {
                (Icon::Stopped, "not running", self.theme.semantic.warn)
            }
            RuntimeRead::Unreadable(_) => return None,
        };
        Some(chrome::pill(
            &format!("{} {word}", icon.glyph(self.icons)),
            style,
        ))
    }

    /// The line that says the sidebar is not shown, and what it holds.
    ///
    /// The drop is never silent. The full line names the width the sidebar
    /// needs; where that does not fit, the short line still names the
    /// configuration; where even that does not fit, the cut is marked.
    fn hidden_line(&self, width: u16) -> Line<'static> {
        let full =
            format!(" sidebar not shown: {SIDEBAR_TITLE} (it needs {SIDEBAR_MIN_WIDTH} columns)");
        let short = format!(" sidebar not shown: {SIDEBAR_TITLE}");
        let text = if text_width(&full) <= usize::from(width) {
            full
        } else {
            short
        };
        Line::from(Span::styled(
            cut_to(&text, usize::from(width)),
            self.theme.text.dim,
        ))
    }

    /// A pane with nothing in it yet: its border, its name, and the state in
    /// one line on the dim slot.
    ///
    /// A pane too short for a box gives the box up and keeps the state: the
    /// words matter more than the frame, as on raul's Overview.
    fn empty_pane(&self, frame: &mut Frame, area: Rect, title: &'static str, says: &'static str) {
        if area.height == 0 {
            return;
        }
        if area.height < 3 {
            let line = Line::from(Span::styled(
                cut_to(says, usize::from(area.width)),
                self.theme.text.dim,
            ));
            frame.render_widget(Paragraph::new(line), area);
            return;
        }
        // No record means nothing runs, so the pane wears the stopped icon.
        let block = self.main_title(title, Icon::Stopped);
        // The inner width is the pane's own: a state that does not fit is
        // cut and marked, never drawn past the border.
        let inner = area.width.saturating_sub(2);
        let line = Line::from(Span::styled(
            cut_to(says, usize::from(inner)),
            self.theme.text.dim,
        ));
        frame.render_widget(Paragraph::new(line).block(block), area);
    }

    /// The record is there but cannot be read: the pane names the file and
    /// the error, as raul names a file it cannot read.
    ///
    /// This is not the empty state: a record that cannot be read is not a run
    /// that says nothing.
    fn unreadable_pane(&self, frame: &mut Frame, area: Rect, error: &ReadError) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let inner = if area.height >= 3 && area.width >= 4 {
            // Whether it runs is not known, so the pane keeps hugo's icon.
            let block = self.main_title(FLOW_TITLE, Icon::Config);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            inner
        } else {
            area
        };
        let detail = format!("{}: {}", error.file, error.error);
        let lines = vec![
            Line::from(Span::styled(
                cut_to(FLOW_UNREADABLE, usize::from(inner.width)),
                self.theme.text.normal,
            )),
            Line::from(Span::styled(
                cut_to(&detail, usize::from(inner.width)),
                self.theme.semantic.error,
            )),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// The main pane: the flow's blocks, with the stamp at the foot.
    fn flow(&mut self, frame: &mut Frame, area: Rect, runtime: &Runtime, now: &str, tick: usize) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        if area.height < 3 || area.width < 4 {
            self.flow_body(frame, area, runtime, now, tick);
            return;
        }
        let block = self.main_title(FLOW_TITLE, run_icon(runtime.stamp.runs));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.flow_body(frame, inner, runtime, now, tick);
    }

    /// The blocks inside the flow's box, and the stamp on the last row.
    fn flow_body(
        &mut self,
        frame: &mut Frame,
        inner: Rect,
        runtime: &Runtime,
        now: &str,
        tick: usize,
    ) {
        if inner.height == 0 || inner.width == 0 {
            return;
        }
        // The stamp is the foot of the pane: it says whether the automation
        // runs, and why when it does not.
        let stamp = Rect {
            y: inner.y + inner.height - 1,
            height: 1,
            ..inner
        };
        let body = Rect {
            height: inner.height - 1,
            ..inner
        };
        frame.render_widget(Paragraph::new(self.stamp_line(runtime, inner.width)), stamp);
        if body.height == 0 || body.width == 0 {
            return;
        }

        // The blocks keep their boxes while there is room for them. Under
        // that, the words stay and the chrome goes.
        if body.width >= MIN_BOXED_WIDTH {
            let sections = self.sections(runtime, body.width.saturating_sub(2), now, tick);
            let boxed: usize = sections.iter().map(|section| section.lines.len() + 2).sum();
            if boxed <= usize::from(body.height) {
                self.draw_boxed(frame, body, &sections);
                return;
            }
        }
        let sections = self.sections(runtime, body.width, now, tick);
        self.draw_words(frame, body, &sections);
    }

    /// The blocks in boxes, one under another, in the order they are read.
    fn draw_boxed(&mut self, frame: &mut Frame, body: Rect, sections: &[Section]) {
        let mut constraints: Vec<Constraint> = sections
            .iter()
            .map(|section| Constraint::Length((section.lines.len() + 2) as u16))
            .collect();
        constraints.push(Constraint::Min(0));
        let areas = Layout::vertical(constraints).split(body);
        for (section, area) in sections.iter().zip(areas.iter()) {
            let block = self.main_title(section.title, section.icon);
            frame.render_widget(Paragraph::new(section.lines.clone()).block(block), *area);
            // The box's border is one cell all round: its lines sit inside it.
            let inner = Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            };
            self.record_section(section, *area, inner);
        }
    }

    /// Record where a drawn section's lines are, for the pointer: the queue's
    /// section and each of its rows. `area` is the section as drawn, `inner` its
    /// lines.
    fn record_section(&mut self, section: &Section, area: Rect, inner: Rect) {
        if section.title == QUEUE_TITLE {
            self.hits.queue_area = area;
        }
        for (line, index) in &section.picks {
            let line = u16::try_from(*line).unwrap_or(u16::MAX);
            if line >= inner.height {
                continue;
            }
            let rect = Rect {
                x: inner.x,
                y: inner.y + line,
                width: inner.width,
                height: 1,
            };
            self.hits.queue.push((rect, *index));
        }
    }

    /// The blocks without their boxes: a title and its lines.
    ///
    /// A block that does not fit is left out whole, and one line names what
    /// is not shown. A block is never cut in the middle without a word.
    fn draw_words(&mut self, frame: &mut Frame, body: Rect, sections: &[Section]) {
        let heights: Vec<usize> = sections
            .iter()
            .map(|section| section.lines.len() + 1)
            .collect();
        let total: usize = heights.iter().sum();
        let room = usize::from(body.height);
        let (shown, hidden) = if total <= room {
            (sections.len(), &sections[sections.len()..])
        } else {
            // One row is the announcement's: find the prefix that fits
            // beside it.
            let mut used = 0;
            let mut shown = 0;
            for height in &heights {
                if used + height + 1 > room {
                    break;
                }
                used += height;
                shown += 1;
            }
            (shown, &sections[shown..])
        };
        let mut lines: Vec<Line<'static>> = Vec::new();
        for section in &sections[..shown] {
            // Each block's heading is the row its lines start below.
            let heading_row = lines.len();
            let heading = format!("{} {}", section.icon.glyph(self.icons), section.title);
            lines.push(Line::from(Span::styled(
                heading,
                self.theme.chrome.tab_active,
            )));
            lines.extend(section.lines.clone());
            let area = Rect {
                x: body.x,
                y: body.y + heading_row as u16,
                width: body.width,
                height: (section.lines.len() + 1) as u16,
            };
            let inner = Rect {
                y: area.y + 1,
                height: section.lines.len() as u16,
                ..area
            };
            self.record_section(section, area, inner);
        }
        if !hidden.is_empty() {
            // Each block gives way whole: its count is the rows it held.
            let names: Vec<String> = hidden
                .iter()
                .map(|section| format!("{} ({} hidden)", section.title, section.lines.len()))
                .collect();
            let says = format!("not shown: {}", names.join(", "));
            lines.push(Line::from(Span::styled(
                cut_to(&says, usize::from(body.width)),
                self.theme.text.dim,
            )));
        }
        frame.render_widget(Paragraph::new(lines), body);
    }

    /// The blocks of the main pane, in the order they are read: the crew
    /// first, then the queue, the round, the budget and the findings.
    fn sections(&self, runtime: &Runtime, width: u16, now: &str, tick: usize) -> Vec<Section> {
        let width = usize::from(width);
        let (queue, queue_picks) =
            self.captioned_picks(QUEUE_CAPTION, self.queue_lines(runtime, width), width);
        vec![
            Section {
                title: CREW_TITLE,
                icon: Icon::RoleOrchestrator,
                lines: self.captioned(
                    CREW_CAPTION,
                    self.crew_lines(runtime, width, now, tick),
                    width,
                ),
                picks: Vec::new(),
            },
            Section {
                title: QUEUE_TITLE,
                icon: Icon::TabBacklog,
                lines: queue,
                picks: queue_picks,
            },
            Section {
                title: ROUND_TITLE,
                icon: Icon::RoleReviewer,
                lines: self.captioned(ROUND_CAPTION, self.round_lines(runtime, width, tick), width),
                picks: Vec::new(),
            },
            Section {
                title: BUDGET_TITLE,
                icon: Icon::Clock,
                lines: self.captioned(BUDGET_CAPTION, self.budget_lines(runtime, width), width),
                picks: Vec::new(),
            },
            Section {
                title: FINDINGS_TITLE,
                icon: Icon::Finding,
                lines: self.captioned(FINDINGS_CAPTION, self.findings_lines(runtime, width), width),
                picks: Vec::new(),
            },
        ]
    }

    /// A block's lines under its caption: the one dim line that says what the
    /// block counts.
    fn captioned(
        &self,
        caption: &str,
        lines: Vec<Line<'static>>,
        width: usize,
    ) -> Vec<Line<'static>> {
        self.captioned_picks(caption, (lines, Vec::new()), width).0
    }

    /// [`captioned`] for a block with rows a click picks: the picks move down
    /// by the lines the caption takes, so each is the line it is drawn on.
    fn captioned_picks(
        &self,
        caption: &str,
        (lines, picks): (Vec<Line<'static>>, Vec<(usize, usize)>),
        width: usize,
    ) -> (Vec<Line<'static>>, Vec<(usize, usize)>) {
        let mut out = vec![self.fit(vec![(caption.to_string(), self.theme.text.dim)], width)];
        let at = out.len();
        out.extend(lines);
        let picks = picks
            .into_iter()
            .map(|(line, index)| (line + at, index))
            .collect();
        (out, picks)
    }

    /// The crew: a table with one row per agent, and the composition that says
    /// who reviews.
    ///
    /// The columns line up while the table fits the pane, under a dim header
    /// that names them. Where it does not, every row is drawn alone, with its
    /// task and hold in words, in one form for the whole crew. No fact is
    /// dropped with the width.
    fn crew_lines(
        &self,
        runtime: &Runtime,
        width: usize,
        now: &str,
        tick: usize,
    ) -> Vec<Line<'static>> {
        let rows: Vec<CrewRow> = runtime
            .crew
            .iter()
            .map(|member| self.crew_row(member, now, tick))
            .collect();
        let header = self.crew_header();
        let columns: Vec<[(String, Style); CREW_COLUMNS]> =
            rows.iter().map(CrewRow::columns).collect();
        let mut widths = [0; CREW_COLUMNS];
        for cells in std::iter::once(&header).chain(&columns) {
            for (column_width, (text, _)) in widths.iter_mut().zip(cells) {
                *column_width = (*column_width).max(text_width(text));
            }
        }
        let aligned = columns_width(&widths) <= width;
        let mut lines = Vec::new();
        if aligned {
            lines.push(self.fit(row_spans(&header, &widths), width));
            for cells in &columns {
                lines.push(self.fit(row_spans(cells, &widths), width));
            }
        } else {
            lines.extend(self.crew_unaligned(&rows, width));
        }
        lines.push(self.fit(
            vec![(
                runtime.composition().says().to_string(),
                self.theme.text.dim,
            )],
            width,
        ));
        lines
    }

    /// The crew table's header: the name of each column, dim.
    fn crew_header(&self) -> [(String, Style); CREW_COLUMNS] {
        ["role", "model", "task", "held", "state"]
            .map(|name| (name.to_string(), self.theme.text.dim))
    }

    /// One agent's row of the crew table.
    ///
    /// A working agent carries the spinner's frame before its state; an idle
    /// one is dim, the row and its state alike. A blocked agent is drawn on
    /// the warning slot, under the theme's blocked icon.
    fn crew_row(&self, member: &CrewMember, now: &str, tick: usize) -> CrewRow {
        let working = member.state.is_working();
        let blocked = member.state == AgentState::Blocked;
        let normal = if working || blocked {
            self.theme.text.normal
        } else {
            self.theme.text.dim
        };
        // A blocked agent names itself on the warning slot, with its own icon.
        let state = match member.state {
            AgentState::Blocked => format!(
                "{} {}",
                Icon::Blocked.glyph(self.icons),
                member.state.as_str()
            ),
            _ if working => format!("{} {}", chrome::spinner_frame(tick), member.state.as_str()),
            _ => member.state.as_str().to_string(),
        };
        let state_style = match member.state {
            AgentState::Blocked => self.theme.semantic.warn,
            _ if working => self.theme.semantic.agent,
            _ => self.theme.text.dim,
        };
        let held = held_length(member.since.as_deref(), now);
        let phrase = match &member.holds {
            Some(task) => format!("holds {task} · held {held}"),
            None => "holds nothing".to_string(),
        };
        let hold = if member.since.is_none() {
            held
        } else {
            format!("{} {held}", Icon::Clock.glyph(self.icons))
        };
        CrewRow {
            role: (
                format!(
                    "{} {}",
                    role_icon(member.role).glyph(self.icons),
                    member.role.as_str()
                ),
                role_style(&self.theme, member.role),
            ),
            model: (short_model(&member.model).to_string(), self.theme.text.dim),
            holds: (
                member.holds.clone().unwrap_or_else(|| "—".to_string()),
                normal,
            ),
            hold: (hold, self.theme.text.dim),
            state: (state, state_style),
            phrase: (phrase, normal),
        }
    }

    /// The crew without its header: one form for every row, the widest that
    /// every row fits, so the rows share one shape and one separator. Each row
    /// is one line if they all fit, else two, else three.
    fn crew_unaligned(&self, rows: &[CrewRow], width: usize) -> Vec<Line<'static>> {
        let forms: [CrewForm; 3] = [
            CrewRow::on_one_line,
            CrewRow::on_two_lines,
            CrewRow::on_three_lines,
        ];
        let fits = |form: CrewForm| {
            rows.iter()
                .all(|row| form(row).iter().all(|line| spans_width(line) <= width))
        };
        let form = forms
            .into_iter()
            .find(|form| fits(*form))
            .unwrap_or(CrewRow::on_three_lines);
        rows.iter()
            .flat_map(form)
            .map(|spans| self.fit(spans, width))
            .collect()
    }

    /// The queue: the waiting work, in order, with an id and a status, and the
    /// count with the set it counts. The task under the cursor is marked with
    /// the cursor's bar while the queue has focus; a task that is marked
    /// carries `[x]`. Also returns the line each task is drawn on, counted from
    /// the count line, with the task's place in the queue.
    fn queue_lines(
        &self,
        runtime: &Runtime,
        width: usize,
    ) -> (Vec<Line<'static>>, Vec<(usize, usize)>) {
        let items: Vec<&QueueItem> = runtime.waiting().collect();
        let done = runtime
            .queue
            .iter()
            .filter(|item| item.status == Status::Done)
            .count();
        let mut count = vec![
            ("the automation's queue · ".to_string(), self.theme.text.dim),
            (items.len().to_string(), self.theme.text.strong),
            (" waiting".to_string(), self.theme.text.dim),
        ];
        if done > 0 {
            count.push((" · ".to_string(), self.theme.text.dim));
            count.push((done.to_string(), self.theme.text.strong));
            count.push((" done".to_string(), self.theme.text.dim));
        }
        let mut lines = vec![self.fit(count, width)];
        let mut picks = Vec::new();
        let focused = self.focus == Focus::Queue;
        let mut index = 0;
        for item in items {
            // A task that is not claimable is drawn dim, with no mark and no
            // cursor: it cannot be picked.
            if !item.claimable {
                lines.push(self.fit(
                    vec![
                        (" ".to_string(), Style::default()),
                        ("  ".to_string(), Style::default()),
                        (item.id.clone(), self.theme.text.dim),
                        ("  ".to_string(), Style::default()),
                        (item.status.as_str().to_string(), self.theme.text.dim),
                        ("  not claimable".to_string(), self.theme.text.dim),
                    ],
                    width,
                ));
                continue;
            }
            picks.push((lines.len(), index));
            let selected = focused && index == self.queue_cursor;
            let (bar, bar_style) = if selected {
                (
                    Icon::CursorBar.glyph(self.icons).to_string(),
                    self.theme.chrome.border_focus,
                )
            } else {
                (" ".to_string(), Style::default())
            };
            let (mark, mark_style) = if self.marked.contains(&item.id) {
                (MARKED, self.theme.text.strong)
            } else {
                (UNMARKED, self.theme.text.dim)
            };
            let row = self.fit(
                vec![
                    (bar, bar_style),
                    (format!("{mark} "), mark_style),
                    (item.id.clone(), self.theme.text.normal),
                    ("  ".to_string(), Style::default()),
                    (
                        item.status.as_str().to_string(),
                        phase_style(&self.theme, item.status),
                    ),
                ],
                width,
            );
            lines.push(if selected {
                selected_line(row, width, &self.theme)
            } else {
                row
            });
            index += 1;
        }
        (lines, picks)
    }

    /// The review in flight, as a timeline of the task in flight: the task and
    /// its stage, then each round's build and review, and the findings each
    /// review raised. The review under way carries the spinner. A timeline too
    /// wide for one line gives each round a line of its own.
    fn round_lines(&self, runtime: &Runtime, width: usize, tick: usize) -> Vec<Line<'static>> {
        let Some(round) = &runtime.round else {
            return vec![self.fit(
                vec![("no review in flight".to_string(), self.theme.text.dim)],
                width,
            )];
        };
        let reviewing = runtime
            .crew
            .iter()
            .any(|member| member.role == FlowRole::Reviewer && member.state.is_working());
        let head = vec![
            (round.task.clone(), self.theme.text.normal),
            (
                format!(" · {}", stage_word(round.stage)),
                self.theme.text.dim,
            ),
        ];
        let mut rounds: Vec<Vec<(String, Style)>> = round
            .earlier
            .iter()
            .map(|earlier| self.finished_round(earlier.number, earlier.raised))
            .collect();
        rounds.push(self.current_round(round.number, reviewing, tick));
        self.timeline(head, &rounds, width)
    }

    /// A round that is finished: its build and review, and the findings the
    /// review raised.
    fn finished_round(&self, number: u32, raised: u32) -> Vec<(String, Style)> {
        let (verdict, style) = match raised {
            0 => ("clean".to_string(), self.theme.text.dim),
            1 => ("1 finding".to_string(), self.theme.semantic.warn),
            _ => (format!("{raised} findings"), self.theme.semantic.warn),
        };
        vec![
            (format!("round {number} "), self.theme.text.dim),
            (
                Icon::StatusDone.glyph(self.icons).to_string(),
                self.theme.semantic.ok,
            ),
            (" build".to_string(), self.theme.text.normal),
            (" · ".to_string(), self.theme.text.dim),
            (
                Icon::StatusDone.glyph(self.icons).to_string(),
                self.theme.semantic.ok,
            ),
            (" review ".to_string(), self.theme.text.normal),
            (verdict, style),
        ]
    }

    /// The round in flight: its build is done, and its review is under way
    /// when the reviewer works, else it waits.
    fn current_round(&self, number: u32, reviewing: bool, tick: usize) -> Vec<(String, Style)> {
        let (review, review_style, word_style) = if reviewing {
            (
                chrome::spinner_frame(tick).to_string(),
                self.theme.semantic.agent,
                self.theme.text.normal,
            )
        } else {
            (
                Icon::TaskOpen.glyph(self.icons).to_string(),
                self.theme.text.dim,
                self.theme.text.dim,
            )
        };
        vec![
            (format!("round {number} "), self.theme.text.dim),
            (
                Icon::StatusDone.glyph(self.icons).to_string(),
                self.theme.semantic.ok,
            ),
            (" build".to_string(), self.theme.text.normal),
            (" · ".to_string(), self.theme.text.dim),
            (review, review_style),
            (" review".to_string(), word_style),
        ]
    }

    /// The timeline: the head, then the rounds in order, on one line while
    /// they fit and one round to a line once they do not. A round that starts
    /// a line after the first is led by the arrow.
    fn timeline(
        &self,
        head: Vec<(String, Style)>,
        rounds: &[Vec<(String, Style)>],
        width: usize,
    ) -> Vec<Line<'static>> {
        let arrow = Icon::TabMarkerRight.glyph(self.icons);
        let mut lines = Vec::new();
        let mut line = head;
        for (index, round) in rounds.iter().enumerate() {
            let sep = if index == 0 {
                "  ".to_string()
            } else {
                format!(" {arrow} ")
            };
            if spans_width(&line) + text_width(&sep) + spans_width(round) <= width {
                line.push((sep, self.theme.text.dim));
                line.extend(round.iter().cloned());
            } else {
                lines.push(self.fit(line, width));
                line = if index == 0 {
                    round.clone()
                } else {
                    let mut next = vec![(format!("{arrow} "), self.theme.text.dim)];
                    next.extend(round.iter().cloned());
                    next
                };
            }
        }
        lines.push(self.fit(line, width));
        lines
    }

    /// The external rounds spent of their budget, as a bar and the numbers in
    /// words. A line too narrow for both wraps the bar under its label.
    fn budget_lines(&self, runtime: &Runtime, width: usize) -> Vec<Line<'static>> {
        let spent = runtime.budget.spent as usize;
        let total = runtime.budget.total as usize;
        let bar = chrome::progress_bar(
            spent,
            total,
            BUDGET_CELLS,
            budget_slot(spent, total),
            &self.theme,
        )
        .into_iter()
        .map(|span| (span.content.into_owned(), span.style));
        let numbers = vec![
            (" ".to_string(), Style::default()),
            (spent.to_string(), self.theme.text.strong),
            (" of ".to_string(), self.theme.text.dim),
            (total.to_string(), self.theme.text.strong),
        ];
        let label = ("rounds ".to_string(), self.theme.text.dim);
        let bar_and_numbers: Vec<(String, Style)> = bar.chain(numbers).collect();
        let mut one = vec![label.clone()];
        one.extend(bar_and_numbers.iter().cloned());
        if spans_width(&one) <= width {
            return vec![self.fit(one, width)];
        }
        vec![
            self.fit(vec![label], width),
            self.fit(bar_and_numbers, width),
        ]
    }

    /// The open findings, as a count and, when the record holds them, the
    /// count of each severity on its own slot. A line too narrow for both
    /// wraps the severities under the count.
    fn findings_lines(&self, runtime: &Runtime, width: usize) -> Vec<Line<'static>> {
        if runtime.findings == 0 {
            return vec![self.fit(
                vec![("no open findings".to_string(), self.theme.text.dim)],
                width,
            )];
        }
        let noun = if runtime.findings == 1 {
            " open finding"
        } else {
            " open findings"
        };
        let count = vec![
            (runtime.findings.to_string(), self.theme.text.strong),
            (noun.to_string(), self.theme.text.normal),
        ];
        let Some(severities) = runtime.severities else {
            return vec![self.fit(count, width)];
        };
        let parts = [
            (severities.high, self.theme.semantic.error, "high"),
            (severities.medium, self.theme.semantic.warn, "medium"),
            (severities.low, self.theme.semantic.info, "low"),
        ]
        .map(|(number, slot, word)| {
            vec![
                (number.to_string(), slot),
                (format!(" {word}"), self.theme.text.normal),
            ]
        });
        let mut breakdown = Vec::new();
        for (index, part) in parts.into_iter().enumerate() {
            if index > 0 {
                breakdown.push((" · ".to_string(), self.theme.text.dim));
            }
            breakdown.extend(part);
        }
        let mut one = count.clone();
        one.push((" · ".to_string(), self.theme.text.dim));
        one.extend(breakdown.iter().cloned());
        if spans_width(&one) <= width {
            return vec![self.fit(one, width)];
        }
        vec![self.fit(count, width), self.fit(breakdown, width)]
    }

    /// The stamp: whether the automation runs, and why when it does not.
    fn stamp_line(&self, runtime: &Runtime, width: u16) -> Line<'static> {
        let (icon, says, style) = if runtime.stamp.runs {
            (
                Icon::Running,
                format!("running since {}", runtime.started_at),
                self.theme.semantic.ok,
            )
        } else {
            let why = runtime.stamp.why.as_deref().unwrap_or("no reason given");
            (
                Icon::Stopped,
                format!("not running: {why}"),
                self.theme.semantic.warn,
            )
        };
        let says = format!("{} {says}", icon.glyph(self.icons));
        self.fit(vec![(says, style)], usize::from(width))
    }

    /// A line of spans, cut to the width with the cut marked. A line that
    /// fits is kept whole. One that does not keeps the spans that fit beside
    /// the marker, and the span that runs out is cut with it: a span that
    /// fills the width exactly is never the end of a line that had more.
    fn fit(&self, spans: Vec<(String, Style)>, width: usize) -> Line<'static> {
        if width == 0 {
            return Line::from(Vec::<Span<'static>>::new());
        }
        let total: usize = spans.iter().map(|(text, _)| text_width(text)).sum();
        if total <= width {
            let out: Vec<Span<'static>> = spans
                .into_iter()
                .map(|(text, style)| Span::styled(text, style))
                .collect();
            return Line::from(out);
        }
        let mut room = width - 1;
        let mut out = Vec::new();
        for (text, style) in spans {
            let used = text_width(&text);
            if used <= room {
                room -= used;
                out.push(Span::styled(text, style));
            } else {
                out.push(Span::styled(format!("{}…", head_to(&text, room)), style));
                break;
            }
        }
        Line::from(out)
    }

    /// A read's error with the reason first: the catalogue names the tool, then
    /// why it failed (`{tool} failed: {why}`), so a long tool path would push
    /// the reason past the popup's edge. The tool follows the reason, its
    /// program cut to its file name.
    fn reason_first(error: &str) -> String {
        let split = error
            .split_once(" failed: ")
            .map(|(tool, why)| (format!("failed: {why}"), tool))
            .or_else(|| {
                error
                    .strip_suffix(" failed")
                    .map(|tool| ("failed".to_string(), tool))
            })
            .or_else(|| {
                error
                    .split_once(": ")
                    .map(|(tool, why)| (why.to_string(), tool))
            });
        match split {
            Some((reason, tool)) => format!("{reason} · {}", tool_name(tool)),
            None => error.to_string(),
        }
    }

    /// The choice: a small list centred over the window, with its title on the
    /// agent's role.
    ///
    /// The list scrolls to keep its cursor on screen; the entry under the cursor
    /// takes the selection's paint. A choice the catalogue has not answered yet
    /// says it is reading; one that cannot be filled says why on the failure
    /// slot rather than opening empty. The id input shows the id typed, and
    /// whether it has the shape of one.
    fn choice_pane(&mut self, frame: &mut Frame, area: Rect) {
        let Some(choice) = &self.choice else {
            return;
        };
        let row = |text: &str, style: Style| picker::Row {
            text: text.to_string(),
            style,
        };
        let (lead, entries) = match &choice.state {
            ChoiceState::Reading => (vec![row(CATALOG_READING, self.theme.text.dim)], Vec::new()),
            ChoiceState::Failed(error) => {
                let lead = vec![
                    row(CATALOG_UNREADABLE, self.theme.text.normal),
                    row(&Self::reason_first(error), self.theme.semantic.error),
                ];
                let entries = if choice.field == Field::Model {
                    self.entry_rows(choice)
                } else {
                    Vec::new()
                };
                (lead, entries)
            }
            ChoiceState::Typing(text) => {
                let level = choice.field == Field::Effort;
                let shaped = if level {
                    pick::is_typed_level(text)
                } else {
                    pick::is_typed_id(text)
                };
                let (hint, style) = match (level, text.is_empty() || shaped) {
                    (true, true) => (TYPE_LEVEL_HINT, self.theme.text.dim),
                    (true, false) => (NOT_A_LEVEL, self.theme.semantic.error),
                    (false, true) => (TYPE_HINT, self.theme.text.dim),
                    (false, false) => (NOT_A_MODEL_ID, self.theme.semantic.error),
                };
                let lead = vec![
                    row(&format!("> {text}_"), self.theme.text.normal),
                    row(hint, style),
                ];
                (lead, Vec::new())
            }
            ChoiceState::Listed => (Vec::new(), self.entry_rows(choice)),
            ChoiceState::Empty => (
                vec![row(self.empty_lead(choice), self.theme.text.dim)],
                self.entry_rows(choice),
            ),
        };
        let picked = picker::draw(
            frame,
            picker::popup(area, choice.rows()),
            &choice.title,
            role_icon(choice.role),
            self.icons,
            &self.theme,
            &lead,
            &entries,
            Some(choice.cursor),
        );
        self.hits.choice = picked;
    }

    /// The crew-size selector: one line per size, the roles it has drawn as
    /// their icons and the roles it leaves out as empty circles, then the hint.
    fn crew_pane(&mut self, frame: &mut Frame, area: Rect) {
        let Some(cursor) = self.crew_cursor else {
            return;
        };
        let entries: Vec<picker::Row> = (1..=CREW_ORDER.len())
            .map(|size| picker::Row {
                text: self.crew_entry(size),
                style: self.theme.text.normal,
            })
            .collect();
        let picked = picker::draw(
            frame,
            picker::popup(area, entries.len()),
            CREW_SIZE_TITLE,
            Icon::Config,
            self.icons,
            &self.theme,
            &[],
            &entries,
            (!self.crew_unsized).then_some(cursor),
        );
        self.hits.crew = picked;
    }

    /// One size's line in the selector: the glyph of each role the size has,
    /// then the glyph of each it leaves out, then the size's hint.
    fn crew_entry(&self, size: usize) -> String {
        let roles = crew_roles(size);
        let glyphs: String = CREW_ORDER
            .iter()
            .map(|role| {
                if roles.contains(role) {
                    role_icon(*role).glyph(self.icons)
                } else {
                    Icon::RoleAbsent.glyph(self.icons)
                }
            })
            .collect();
        format!("{glyphs} {}", CREW_HINTS[size - 1])
    }

    /// The choice's entries as the picker draws them: a harness the machine
    /// does not have is dim and says so.
    /// The line an empty choice shows in place of its list. An effort choice
    /// says why it has no levels: no model yet, or one that takes none. A model
    /// the catalogue does not list is offered levels, so it is never empty.
    fn empty_lead(&self, choice: &Choice) -> &'static str {
        if choice.field == Field::Model {
            return NO_MODELS;
        }
        match self
            .session()
            .and_then(|config| config.agents.get(choice.agent))
        {
            Some(row) if row.model.is_empty() => NO_MODEL_FIRST,
            _ => NO_LEVELS,
        }
    }

    fn entry_rows(&self, choice: &Choice) -> Vec<picker::Row> {
        choice
            .entries
            .iter()
            .map(|entry| {
                if entry.pickable {
                    picker::Row {
                        text: entry.text.clone(),
                        style: self.theme.text.normal,
                    }
                } else {
                    picker::Row {
                        text: format!("{} {NOT_INSTALLED}", entry.text),
                        style: self.theme.text.dim,
                    }
                }
            })
            .collect()
    }

    /// The help: the keys that work, in the sidebar and in the choice, with
    /// the least useful lines giving way first on a small window.
    fn help_pane(&self, frame: &mut Frame, area: Rect) {
        let popup = help_popup(area);
        if popup.width < 4 || popup.height < 3 {
            return;
        }
        frame.render_widget(Clear, popup);
        let block = chrome::pane(HELP_TITLE, Icon::Config, self.icons, true, &self.theme);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let width = usize::from(inner.width);
        let room = usize::from(inner.height);
        if width == 0 || room == 0 {
            return;
        }
        let mut lines = self.help_lines(width);
        if lines.len() > room {
            // The lines give way from the least to the most useful: the keys
            // a human who is stuck needs most stay to the end.
            let mut order: Vec<usize> = (0..lines.len()).collect();
            order.sort_by_key(|index| lines[*index].0);
            let mut dropped = vec![false; lines.len()];
            let mut held = lines.len();
            for index in order {
                if held <= room {
                    break;
                }
                if lines[index].0 == u8::MAX {
                    continue;
                }
                dropped[index] = true;
                held -= 1;
            }
            lines = lines
                .into_iter()
                .enumerate()
                .filter(|(index, _)| !dropped[*index])
                .map(|(_, pair)| pair)
                .collect();
        }
        let shown: Vec<Line<'static>> = lines.into_iter().map(|(_, line)| line).collect();
        frame.render_widget(Paragraph::new(shown), inner);
    }

    /// The help's lines, with the rank that decides which one gives way
    /// first. Zero gives way first; `u8::MAX` never gives way.
    ///
    /// The keys are the ones that work where the help was opened: the
    /// sidebar's in the window, the choice's over the choice. `enter` is
    /// named only when the cursor is on the row it opens.
    fn help_lines(&self, width: usize) -> Vec<(u8, Line<'static>)> {
        let line = |key: &str, says: &str| {
            let key = cut_to(&format!(" {key} "), width);
            let used = text_width(&key);
            let says = cut_to(&format!(" {says}"), width.saturating_sub(used));
            Line::from(vec![
                Span::styled(key, self.theme.chrome.key),
                Span::styled(says, self.theme.text.normal),
            ])
        };
        if let Some(choice) = &self.choice {
            let mut lines = Vec::new();
            if choice.moves() {
                lines.push((0, line("↑↓ j/k", "move")));
            }
            lines.push((1, line("ctrl-c", "quit")));
            lines.push((2, line(CLOSE_KEY, "close")));
            if choice.pickable() {
                lines.push((3, line(CHOOSE_KEY, "pick")));
            }
            lines.push((u8::MAX, line("q", "quit")));
            lines.push((u8::MAX, line("?", "this help")));
            return lines;
        }
        if self.crew_cursor.is_some() {
            return vec![
                (0, line("↑↓ j/k", "move")),
                (1, line("ctrl-c", "quit")),
                (2, line(CLOSE_KEY, "close")),
                (3, line(CHOOSE_KEY, "pick")),
                (u8::MAX, line("q", "quit")),
                (u8::MAX, line("?", "this help")),
            ];
        }
        if self.focus == Focus::Queue {
            return vec![
                (0, line("↑↓ j/k", "move")),
                (1, line("ctrl-c", "quit")),
                (2, line(MARK_KEY, "mark the task under the cursor")),
                (
                    2,
                    line(MARK_ALL_KEY, "mark every claimable task, or unmark them"),
                ),
                (
                    3,
                    line(START_KEY, "start the orchestrator over the marked tasks"),
                ),
                (3, line(FOCUS_KEY, "back to the configuration")),
                (u8::MAX, line("q", "quit")),
                (u8::MAX, line("?", "this help")),
            ];
        }
        let mut lines = Vec::new();
        if self.agents_drawn
            && self
                .session()
                .is_some_and(|config| !config.agents.is_empty())
        {
            lines.push((0, line("↑↓ j/k", "move")));
        }
        lines.push((1, line("ctrl-c", "quit")));
        lines.push((2, line(RELOAD_KEY, "reload")));
        if self.can_choose() {
            lines.push((3, line(CHOOSE_KEY, "choose")));
        }
        if self.session().is_some() {
            lines.push((3, line(CREW_KEY, "crew size")));
        }
        if !self.queued.is_empty() {
            lines.push((3, line(FOCUS_KEY, "the queue")));
        }
        lines.push((3, line(START_KEY, "start the orchestrator")));
        lines.push((u8::MAX, line("q", "quit")));
        lines.push((u8::MAX, line("?", "this help")));
        lines
    }

    /// The footer at `width`, in `rows` rows: the keys that work, then the
    /// closing keys, which answer wherever `?` opens the help. `ctrl-c` quits
    /// as in raul, and the navigation keys are named in the help alone.
    fn footer_lines(&self, width: u16, rows: u16) -> Vec<Line<'static>> {
        let part = |key: &str, says: &str| chrome::key_chip(key, says, &self.theme);
        let closing = vec![part("?", "help"), part("q", "quit")];
        if self.help {
            let actions = vec![part(CLOSE_KEY, "close help")];
            return chrome::footer(&actions, &closing, width, rows, &self.theme);
        }
        if self.crew_cursor.is_some() {
            let actions = vec![part(CLOSE_KEY, "close"), part(CHOOSE_KEY, "pick")];
            return chrome::footer(&actions, &closing, width, rows, &self.theme);
        }
        if let Some(choice) = &self.choice {
            if choice.typing() {
                // The id input takes the letters, so `q` and `?` are text here.
                let mut actions = vec![part(CLOSE_KEY, "back")];
                if choice.pickable() {
                    actions.push(part(CHOOSE_KEY, "pick"));
                }
                actions.push(part("ctrl-c", "quit"));
                return chrome::footer(&actions, &[], width, rows, &self.theme);
            }
            let mut actions = vec![part(CLOSE_KEY, "close")];
            if choice.pickable() {
                actions.push(part(CHOOSE_KEY, "pick"));
            }
            return chrome::footer(&actions, &closing, width, rows, &self.theme);
        }
        let mut actions = Vec::new();
        if self.focus == Focus::Queue {
            actions.push(part(MARK_KEY, "mark"));
            actions.push(part(MARK_ALL_KEY, "mark all"));
            if self.can_start() {
                actions.push(part(START_KEY, "start"));
            }
            actions.push(part(FOCUS_KEY, "configuration"));
            return chrome::footer(&actions, &closing, width, rows, &self.theme);
        }
        if self.can_choose() {
            actions.push(part(CHOOSE_KEY, "choose"));
        }
        actions.push(part(RELOAD_KEY, "reload"));
        if self.session().is_some() {
            actions.push(part(CREW_KEY, "crew size"));
        }
        if !self.queued.is_empty() {
            actions.push(part(FOCUS_KEY, "queue"));
        }
        if self.can_start() {
            actions.push(part(START_KEY, "start"));
        }
        chrome::footer(&actions, &closing, width, rows, &self.theme)
    }
}

/// The tool an error names: its program cut to the file name, its arguments
/// kept.
fn tool_name(command: &str) -> String {
    let mut words = command.split(' ');
    let program = words.next().unwrap_or_default();
    let name = Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.to_string());
    std::iter::once(name.as_str())
        .chain(words)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The index a click at `at` picks, from the regions a frame drew.
fn hit(regions: &[(Rect, usize)], at: Position) -> Option<usize> {
    regions
        .iter()
        .find(|(rect, _)| rect.contains(at))
        .map(|(_, index)| *index)
}

/// Where the help sits: three fifths of the window, centred.
fn help_popup(area: Rect) -> Rect {
    let width = area.width.saturating_mul(3) / 5;
    let height = area.height.saturating_mul(3) / 5;
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// One block of the main pane, before it is drawn.
struct Section {
    title: &'static str,
    icon: Icon,
    lines: Vec<Line<'static>>,
    /// The lines a click picks, each with the queue row it is: the queue's
    /// only.
    picks: Vec<(usize, usize)>,
}

/// The slot the budget bar is filled in: `semantic.ok` under half spent,
/// `semantic.warn` up to four fifths, and `semantic.error` above that.
fn budget_slot(spent: usize, total: usize) -> Slot {
    if total == 0 || spent * 2 < total {
        Slot::SemanticOk
    } else if spent * 5 <= total * 4 {
        Slot::SemanticWarn
    } else {
        Slot::SemanticError
    }
}

/// The model's own name, without the provider the configuration names.
fn short_model(model: &str) -> &str {
    model.rsplit('/').next().unwrap_or(model)
}

/// How long an agent has held what it holds: `10d`, `5h`, `12m`, or
/// `under a minute`. `—` when it holds nothing.
///
/// The length is derived from the record's `since` and the frame's one
/// instant; the pane does not read a clock of its own.
fn held_length(since: Option<&str>, now: &str) -> String {
    let Some(since) = since else {
        return "—".to_string();
    };
    let (Some(since), Some(now)) = (parse_rfc3339(since), parse_rfc3339(now)) else {
        return "unknown".to_string();
    };
    age_text(now - since)
}

/// One agent's cells in the crew table: the text of each and its paint. The
/// phrase is the same task and hold in words, for a row drawn alone.
struct CrewRow {
    role: (String, Style),
    model: (String, Style),
    holds: (String, Style),
    hold: (String, Style),
    state: (String, Style),
    phrase: (String, Style),
}

impl CrewRow {
    /// The cells in the table's column order.
    fn columns(&self) -> [(String, Style); CREW_COLUMNS] {
        [
            self.role.clone(),
            self.model.clone(),
            self.holds.clone(),
            self.hold.clone(),
            self.state.clone(),
        ]
    }

    /// The cells of a row drawn alone, where no header names the columns.
    fn alone(&self) -> [(String, Style); 4] {
        [
            self.role.clone(),
            self.model.clone(),
            self.phrase.clone(),
            self.state.clone(),
        ]
    }

    /// The row on one line: the agent, its model, what it holds, its state.
    fn on_one_line(&self) -> Vec<Vec<(String, Style)>> {
        vec![joined(&self.alone())]
    }

    /// The row on two lines: the agent, its model and state, then what it
    /// holds under them.
    fn on_two_lines(&self) -> Vec<Vec<(String, Style)>> {
        let [role, model, phrase, state] = self.alone();
        vec![joined(&[role, model, state]), continuation(phrase)]
    }

    /// The row on three lines: the agent and its state, then its model and
    /// what it holds under them.
    fn on_three_lines(&self) -> Vec<Vec<(String, Style)>> {
        let [role, model, phrase, state] = self.alone();
        vec![
            joined(&[role, state]),
            continuation(model),
            continuation(phrase),
        ]
    }
}

/// One way of drawing a crew row: the lines it takes, each a line of spans.
type CrewForm = fn(&CrewRow) -> Vec<Vec<(String, Style)>>;

/// A row's cells one gap apart, with no padding.
fn joined(cells: &[(String, Style)]) -> Vec<(String, Style)> {
    let widths: Vec<usize> = cells.iter().map(|(text, _)| text_width(text)).collect();
    row_spans(cells, &widths)
}

/// A line under a crew row: one cell, indented the two columns under the agent.
fn continuation(cell: (String, Style)) -> Vec<(String, Style)> {
    vec![("  ".to_string(), Style::default()), cell]
}

/// A row's cells padded to the columns' widths. The last column is not
/// padded: nothing follows it.
fn row_spans(cells: &[(String, Style)], widths: &[usize]) -> Vec<(String, Style)> {
    let mut spans = Vec::new();
    for (column, (text, style)) in cells.iter().enumerate() {
        if column > 0 {
            spans.push((" ".repeat(CREW_GAP), Style::default()));
        }
        let text = if column + 1 == cells.len() {
            text.clone()
        } else {
            let pad = widths[column].saturating_sub(text_width(text));
            format!("{text}{}", " ".repeat(pad))
        };
        spans.push((text, *style));
    }
    spans
}

/// The width of a crew table's columns with the gaps between them.
fn columns_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + CREW_GAP * widths.len().saturating_sub(1)
}

/// The width of a line of spans.
fn spans_width(spans: &[(String, Style)]) -> usize {
    spans.iter().map(|(text, _)| text_width(text)).sum()
}

/// The icon of a run: the automation runs, or it stopped.
fn run_icon(runs: bool) -> Icon {
    if runs { Icon::Running } else { Icon::Stopped }
}

/// The stage of a review, in words.
fn stage_word(stage: coding_crew_core::RoundKind) -> &'static str {
    match stage {
        coding_crew_core::RoundKind::SelfReview => "self",
        coding_crew_core::RoundKind::External => "external",
    }
}

/// Where the window puts its parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// The title line.
    pub title: Rect,
    /// The line that says the sidebar is not shown, when it is not.
    pub announcement: Option<Rect>,
    /// The configuration, when there is room for it.
    pub sidebar: Option<Rect>,
    /// What is running.
    pub flow: Rect,
    /// The note row, above the rule, when there is a note to say.
    pub note: Option<Rect>,
    /// The thin rule over the footer.
    pub rule: Rect,
    /// The footer: one row, or two when its keys do not fit on one.
    pub footer: Rect,
}

/// The layout of a window: the title, the panes, and the footer.
///
/// The sidebar sits on the left and the main pane takes the rest. A narrow
/// window drops the sidebar whole before the main pane is squeezed: the
/// sidebar is there only while the main pane keeps its own least width, so
/// the window narrows by losing the sidebar, not by squeezing the flow.
///
/// The announcement of a dropped sidebar is charged to the body, not to the
/// chrome: a screen too short for everything loses its title and its footer
/// first, and the fact that the configuration is not shown is the last line
/// to give way. The footer takes `footer` rows, and its rule one more.
pub fn screen(area: Rect, noted: bool, footer: u16) -> Screen {
    let sidebar_shown = area.width >= SIDEBAR_MIN_WIDTH;
    // The note row is there only while a start has something to say: a window
    // without one keeps the rows it always had.
    let mut constraints = vec![Constraint::Length(1), Constraint::Min(3)];
    if noted {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(footer));
    let rows = Layout::vertical(constraints).split(area);
    let last = rows.len() - 1;
    let footer = rows[last];
    let rule = rows[last - 1];
    let note = noted.then(|| rows[2]);
    let (announcement, body) = if sidebar_shown {
        (None, rows[1])
    } else {
        // The fact takes the first row of the body whenever the body has
        // one: it is the last line to give way, not the first.
        let announcement = Rect {
            height: u16::from(rows[1].height > 0),
            ..rows[1]
        };
        let body = Rect {
            y: rows[1].y + announcement.height,
            height: rows[1].height - announcement.height,
            ..rows[1]
        };
        ((announcement.height > 0).then_some(announcement), body)
    };
    let (sidebar, flow) = if sidebar_shown {
        let panes = Layout::horizontal([
            Constraint::Length(SIDEBAR_WIDTH),
            Constraint::Min(MAIN_MIN_WIDTH),
        ])
        .split(body);
        (Some(panes[0]), panes[1])
    } else {
        (None, body)
    };
    Screen {
        title: rows[0],
        announcement,
        sidebar,
        flow,
        note,
        rule,
        footer,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use std::fs;

    use coding_crew_core::config::CONFIG_FILE;

    use super::*;
    use coding_crew_core::RoundKind;
    use coding_crew_core::config::LEGACY_CONFIG_FILE;
    use coding_crew_core::runtime::AgentState;
    use coding_crew_core::runtime::{
        Budget, Composition, CrewMember, EarlierRound, QueueItem, RuntimeRound, Severities, Stamp,
    };
    use coding_crew_core::{FlowRole, Status};
    use coding_crew_theme::Preset;
    use std::path::Path;

    use crate::launch::FakeLauncher;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// A key press with no modifier.
    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// A scratch home folder for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hugo-app-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch home");
        dir
    }

    fn member(
        role: FlowRole,
        model: &str,
        holds: Option<&str>,
        since: Option<&str>,
        state: AgentState,
    ) -> CrewMember {
        CrewMember {
            role,
            model: model.to_string(),
            actor: None,
            holds: holds.map(str::to_string),
            since: since.map(str::to_string),
            state,
        }
    }

    fn queue_item(id: &str, status: Status) -> QueueItem {
        QueueItem {
            id: id.to_string(),
            status,
            claimable: true,
        }
    }

    /// The record the mock writes, in its shape: three agents, one held task
    /// each for the runner and the reviewer, and one claimable task.
    fn runtime() -> Runtime {
        Runtime {
            started_at: "2026-11-09T08:30Z".to_string(),
            updated_at: None,
            crew: vec![
                member(
                    FlowRole::Orchestrator,
                    "opencode-go/glm-5.3#high",
                    None,
                    None,
                    AgentState::Idle,
                ),
                member(
                    FlowRole::Builder,
                    "opencode-go/deepseek-v4.1-flash#high",
                    Some("TSK10"),
                    Some("2026-11-09T09:00Z"),
                    AgentState::Executing,
                ),
                member(
                    FlowRole::Reviewer,
                    "opencode-go/glm-5.3#high",
                    Some("TSK13"),
                    Some("2026-11-10T16:00Z"),
                    AgentState::Reviewing,
                ),
            ],
            queue: vec![queue_item("TSK29", Status::Approved)],
            round: Some(RuntimeRound {
                task: "TSK13".to_string(),
                stage: RoundKind::External,
                number: 2,
                earlier: vec![EarlierRound {
                    number: 1,
                    raised: 3,
                }],
            }),
            budget: Budget {
                spent: 18,
                total: 45,
            },
            findings: 17,
            severities: Some(Severities {
                high: 8,
                medium: 8,
                low: 1,
            }),
            stamp: Stamp {
                runs: true,
                why: None,
            },
        }
    }

    /// The paint of one cell.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    struct Paint {
        fg: Color,
        bg: Color,
    }

    /// Draw the window and return the rows and the paint of every cell.
    fn render_with(
        width: u16,
        height: u16,
        runtime: &RuntimeRead,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        render_all(width, height, runtime, &sidebar_config())
    }

    /// The configuration the window draws: the project's own flow.
    fn sidebar_config() -> ConfigRead {
        ConfigRead::of(ConfigState::Config(Config::default()))
    }

    /// The model an agent runs, as the window's configuration holds it.
    fn model_of<'a>(window: &'a App, name: &str) -> Option<&'a str> {
        window
            .session()?
            .agents
            .iter()
            .find(|agent| agent.role.as_str() == name)
            .map(|agent| agent.model.as_str())
    }

    /// Open the orchestrator's model choice as a catalogue read that answered
    /// with `loaded` would open it.
    fn answer_choice(window: &mut App, loaded: Result<Catalog, String>) {
        window.show(0, Field::Model, Vec::new(), 0, ChoiceState::Reading);
        window.fill_choice(loaded);
    }

    /// A window on the project's own configuration.
    fn app() -> App {
        App::new(theme(), IconSet::Nerd, sidebar_config(), None)
    }

    /// Draw the window with a configuration of its own.
    fn render_all(
        width: u16,
        height: u16,
        runtime: &RuntimeRead,
        config: &ConfigRead,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        let mut app = App::new(theme(), IconSet::Nerd, config.clone(), None);
        render_app(&mut app, width, height, runtime)
    }

    /// Draw a window of the caller's own, at the first frame of the spinner.
    fn render_app(
        app: &mut App,
        width: u16,
        height: u16,
        runtime: &RuntimeRead,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        render_frame(app, width, height, runtime, 0)
    }

    /// Draw a window of the caller's own at a frame of the spinner.
    fn render_frame(
        app: &mut App,
        width: u16,
        height: u16,
        runtime: &RuntimeRead,
        tick: usize,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, runtime, RENDER_NOW, tick))
            .expect("drawing the window");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        let rows = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect();
        let paint = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| Paint {
                                fg: cell.fg,
                                bg: cell.bg,
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        (rows, paint)
    }

    fn render(width: u16, height: u16) -> (Vec<String>, Vec<Vec<Paint>>) {
        render_with(width, height, &RuntimeRead::Absent)
    }

    fn render_runtime(
        width: u16,
        height: u16,
        runtime: &Runtime,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        render_with(width, height, &RuntimeRead::Record(runtime.clone()))
    }

    /// The colour of the first cell of `needle`, on the row that holds it.
    fn colour_of(rows: &[String], paint: &[Vec<Paint>], needle: &str) -> Color {
        for (y, row) in rows.iter().enumerate() {
            if let Some(at) = row.find(needle) {
                let column = row[..at].chars().count();
                return paint[y][column].fg;
            }
        }
        panic!("{needle} is not on screen: {}", rows.join("\n"));
    }

    /// One row's index, the first that holds `needle`.
    fn row_of(rows: &[String], needle: &str) -> usize {
        rows.iter()
            .position(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("{needle} is not on screen: {}", rows.join("\n")))
    }

    /// The column where `needle` starts on row `row`.
    fn column_at(rows: &[String], row: usize, needle: &str) -> usize {
        rows[row]
            .find(needle)
            .map(|at| rows[row][..at].chars().count())
            .unwrap_or_else(|| panic!("{needle} is not on row {row}"))
    }

    /// The column where a row's content starts, past the borders and the gap.
    fn leading(row: &str) -> usize {
        row.chars()
            .position(|c| c != '│' && c != ' ')
            .unwrap_or_else(|| panic!("{row:?} has no content"))
    }

    #[test]
    fn the_sidebar_is_on_the_left_and_the_main_pane_takes_the_rest() {
        let screen = screen(Rect::new(0, 0, 100, 40), false, 1);
        let sidebar = screen.sidebar.expect("the sidebar");
        assert_eq!(sidebar.x, 0);
        assert_eq!(sidebar.width, SIDEBAR_WIDTH);
        assert_eq!(screen.flow.x, SIDEBAR_WIDTH);
        assert_eq!(screen.flow.width, 100 - SIDEBAR_WIDTH);
        assert_eq!(screen.announcement, None);
        assert_eq!(screen.title.height, 1);
        assert_eq!(screen.footer.height, 1);
    }

    #[test]
    fn the_sidebar_is_dropped_at_the_width_that_squeezes_the_main_pane() {
        // The sidebar is shown while the main pane keeps its own least
        // width: at the boundary both are there, one column narrower the
        // sidebar is gone and the main pane takes the window.
        let both = screen(Rect::new(0, 0, SIDEBAR_MIN_WIDTH, 40), false, 1);
        let sidebar = both.sidebar.expect("the sidebar at the boundary");
        assert_eq!(sidebar.width, SIDEBAR_WIDTH);
        assert_eq!(both.flow.width, MAIN_MIN_WIDTH);

        let narrow = screen(Rect::new(0, 0, SIDEBAR_MIN_WIDTH - 1, 40), false, 1);
        assert_eq!(narrow.sidebar, None, "the sidebar is still drawn");
        assert_eq!(narrow.flow.width, SIDEBAR_MIN_WIDTH - 1);
        assert!(narrow.flow.width > both.flow.width, "the main pane shrank");
        assert_eq!(
            narrow.announcement.map(|area| area.height),
            Some(1),
            "the drop is not announced"
        );

        // Narrowing past the boundary never squeezes the main pane: it grows
        // by the sidebar's width.
        for width in 1..SIDEBAR_MIN_WIDTH {
            let screen = screen(Rect::new(0, 0, width, 40), false, 1);
            assert_eq!(screen.sidebar, None, "{width}");
            assert_eq!(screen.flow.width, width, "{width}");
            assert!(screen.announcement.is_some(), "{width}: silent drop");
        }
    }

    #[test]
    fn the_dropped_sidebar_is_announced_by_a_line_that_names_it() {
        for width in [40u16, 60, 79] {
            let (rows, paint) = render(width, 12);
            let says = rows
                .iter()
                .find(|row| row.contains("sidebar not shown"))
                .unwrap_or_else(|| panic!("{width}: no line says the sidebar is not shown"));
            assert!(says.contains("configuration"), "{width}: {says}");
            assert!(
                says.chars().count() <= usize::from(width),
                "{width}: {says}"
            );
            // The line is secondary text, like raul's empty states.
            let column = says.find("sidebar").expect("the line");
            assert_eq!(
                paint[rows.iter().position(|row| row == says).expect("the row")][column].fg,
                theme().text.dim.fg.expect("the dim colour")
            );
        }
        // Where there is room, the line names the width the sidebar needs.
        let (rows, _) = render(60, 12);
        let says = rows
            .iter()
            .find(|row| row.contains("sidebar not shown"))
            .expect("the announcement");
        assert!(says.contains("needs 80 columns"), "{says}");
    }

    #[test]
    fn the_empty_states_are_named_on_the_dim_slot() {
        // No home to hold a configuration, and no runtime record: both states
        // are named, and both are painted on the dim slot.
        let (rows, paint) = render_all(
            100,
            40,
            &RuntimeRead::Absent,
            &ConfigRead::of(ConfigState::Absent),
        );
        let text = rows.join("\n");
        assert!(text.contains("no configuration yet"), "{text}");
        assert!(text.contains("nothing is running"), "{text}");
        let dim = theme().text.dim.fg.expect("the dim colour");
        assert_eq!(colour_of(&rows, &paint, "no configuration yet"), dim);
        assert_eq!(colour_of(&rows, &paint, "nothing is running"), dim);
        // The panes are named, and there is no tab bar.
        assert!(text.contains("configuration"), "{text}");
        assert!(text.contains("flow"), "{text}");
        assert!(!text.contains("Overview"), "{text}");
        assert!(!text.contains("Settings"), "{text}");
        assert!(!text.contains("1-5"), "{text}");
    }

    #[test]
    fn the_narrow_window_keeps_the_flow_and_says_the_sidebar_is_gone() {
        let (rows, _) = render(40, 12);
        let text = rows.join("\n");
        assert!(text.contains("sidebar not shown"), "{text}");
        assert!(text.contains("configuration"), "{text}");
        assert!(text.contains("nothing is running"), "{text}");
        assert!(!text.contains("no configuration yet"), "{text}");
    }

    #[test]
    fn the_footer_names_the_keys_that_work_and_no_key_that_does_nothing() {
        // The cursor starts on the builder's harness row, where enter opens a
        // choice: the footer names reload, choose, help and quit. The movement
        // keys are the help's, not the footer's.
        let (rows, _) = render(100, 20);
        let footer = rows.last().expect("a footer row");
        for works in ["q", "quit", "r", "reload", "?", "help", "choose"] {
            assert!(footer.contains(works), "{footer}: {works}");
        }
        for dead in ["open", "tab", "esc", "pick", "↑↓", "move"] {
            assert!(!footer.contains(dead), "{footer}: {dead}");
        }
        // On the orchestrator's model row, enter has a choice to open.
        let mut app = App::new(theme(), IconSet::Nerd, sidebar_config(), None);
        app.cursor = sidebar::cursor_of(0, Field::Model);
        app.agents_drawn = true;
        let line: String = app
            .footer_lines(100, 1)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(line.contains("enter") && line.contains("choose"), "{line}");
        // With the sidebar dropped, the move keys would do nothing: they are
        // not named.
        app.agents_drawn = false;
        let line: String = app
            .footer_lines(100, 1)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(!line.contains("move"), "{line}");
        // A window too narrow for the key shows no half footer.
        let (rows, _) = render(4, 8);
        assert_eq!(rows.last().expect("a footer row").trim(), "");
    }

    #[test]
    fn q_quits_and_other_keys_do_nothing() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = app();
        assert_eq!(
            app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Action::Quit
        );
        for code in [
            KeyCode::Char('x'),
            KeyCode::Char('+'),
            KeyCode::Char('-'),
            KeyCode::Char('h'),
            KeyCode::Char('l'),
            KeyCode::Esc,
            KeyCode::PageUp,
            KeyCode::PageDown,
        ] {
            assert_eq!(
                app.on_key(KeyEvent::new(code, KeyModifiers::NONE)),
                Action::Redraw,
                "{code:?}"
            );
        }
    }

    #[test]
    fn ctrl_c_quits_as_it_does_in_raul() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = app();
        assert_eq!(
            app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Action::Quit
        );
        // A plain c is not the chord: nothing behind it.
        assert_eq!(
            app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
            Action::Redraw
        );
    }

    #[test]
    fn the_title_names_the_tool_and_a_narrow_one_is_dropped_whole() {
        let app = app();
        let name = format!(" {} hugo", Icon::AppHugo.glyph(IconSet::Nerd));
        for width in 0..=20u16 {
            let line = app.title_line(width, &RuntimeRead::Absent);
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            assert!(
                text.chars().count() <= usize::from(width),
                "{width}: {text}"
            );
            assert!(
                ["", name.as_str()].contains(&text.as_str()),
                "{width}: a cut of the name: {text}"
            );
        }
        assert_eq!(
            app.title_line(100, &RuntimeRead::Absent).spans[0]
                .content
                .as_ref(),
            name
        );
    }

    #[test]
    fn the_title_names_the_project_folder_and_cuts_its_head_to_fit() {
        let mut app = app();
        app.project = Some("/home/agent/coding/coding-crew".to_string());
        let text = |line: Line<'static>| -> String {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        };
        let whole = text(app.title_line(100, &RuntimeRead::Absent));
        assert!(
            whole.contains(" · /home/agent/coding/coding-crew"),
            "{whole}"
        );
        // The folder that names the project is kept: the head is what goes.
        let narrow = text(app.title_line(30, &RuntimeRead::Absent));
        assert!(narrow.ends_with("…/coding/coding-crew"), "{narrow}");
        assert!(narrow.chars().count() <= 30, "{narrow}");
        // With no columns left for the folder, the name keeps the line and
        // the dot goes.
        let name_only = text(app.title_line(9, &RuntimeRead::Absent));
        assert_eq!(
            name_only,
            format!(" {} hugo", Icon::AppHugo.glyph(IconSet::Nerd))
        );
    }

    #[test]
    fn the_title_pill_says_whether_the_automation_runs() {
        let app = app();
        let text = |line: Line<'static>| -> String {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        };
        let mut runtime = runtime();
        let running = text(app.title_line(100, &RuntimeRead::Record(runtime.clone())));
        let running_pill = format!(" {} running ", Icon::Running.glyph(IconSet::Nerd));
        assert!(running.ends_with(&running_pill), "{running}");

        runtime.stamp = Stamp {
            runs: false,
            why: Some("the review budget is spent".to_string()),
        };
        let stopped = text(app.title_line(100, &RuntimeRead::Record(runtime)));
        let stopped_pill = format!(" {} not running ", Icon::Stopped.glyph(IconSet::Nerd));
        assert!(stopped.ends_with(&stopped_pill), "{stopped}");

        // A record that cannot be read does not say whether anything runs.
        let unreadable = RuntimeRead::Unreadable(ReadError {
            file: "runtime.json".to_string(),
            error: "unexpected end of file".to_string(),
        });
        let unknown = text(app.title_line(100, &unreadable));
        assert!(!unknown.contains("running"), "{unknown}");
    }

    #[test]
    fn the_title_pill_goes_before_the_folder_is_cut() {
        let mut app = app();
        app.project = Some("/home/agent/coding/coding-crew".to_string());
        let text = |width: u16| -> String {
            app.title_line(width, &RuntimeRead::Record(runtime()))
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        };
        // The name, the separator and the folder take 40 columns; the pill
        // takes 11, and a column between them: 52 in all.
        let whole = text(52);
        assert!(whole.ends_with(" 󰑮 running "), "{whole}");
        let narrow = text(51);
        assert!(!narrow.contains("running"), "{narrow}");
        assert!(
            narrow.ends_with(" · /home/agent/coding/coding-crew"),
            "{narrow}"
        );
    }

    #[test]
    fn a_short_window_keeps_the_announcement_and_the_state() {
        // The chrome gives way first: however short the screen, a dropped
        // sidebar is never silent, and the flow still says it is not
        // running. The box goes before the words do.
        for height in 1..=6u16 {
            let (rows, _) = render(60, height);
            let text = rows.join("\n");
            assert!(text.contains("sidebar not shown"), "{height}: {text}");
            if height >= 2 {
                assert!(text.contains("nothing is running"), "{height}: {text}");
            }
        }
    }

    #[test]
    fn the_crew_leads_the_main_pane_and_the_stamp_is_at_the_foot() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        // The block titles, on their borders: a bare word also matches the
        // sidebar's text on the same row.
        let crew = row_of(&rows, "crew ─");
        let queue = row_of(&rows, "queue ─");
        let round = row_of(&rows, "round ─");
        let budget = row_of(&rows, "budget ─");
        let findings = row_of(&rows, "findings ─");
        let stamp = row_of(&rows, "running since");
        assert!(crew < queue, "{crew} {queue}");
        assert!(queue < round, "{queue} {round}");
        assert!(round < budget, "{round} {budget}");
        assert!(budget < findings, "{budget} {findings}");
        assert!(findings < stamp, "{findings} {stamp}");
        // The stamp is the last line of the main pane, above its border.
        let footer = rows.len() - 1;
        assert!(stamp < footer);
        assert!(
            !rows[stamp..footer]
                .iter()
                .any(|row| row.contains("findings"))
        );
    }

    #[test]
    fn one_row_per_agent_carries_its_role_model_and_holding() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        for fact in [
            "orchestrator",
            "builder",
            "reviewer",
            "deepseek-v4.1-flash#high",
            "glm-5.3#high",
            "TSK10",
            "TSK13",
            "executing",
            "reviewing",
            "idle",
        ] {
            assert!(text.contains(fact), "{fact} is not on screen: {text}");
        }
        // The provider is the configuration's; the crew row names the model.
        assert!(!text.contains("opencode-go/"), "{text}");
        // One entry per agent: each role is on its own row.
        for role in ["orchestrator", "builder", "reviewer"] {
            let row = rows
                .iter()
                .find(|row| row.contains(role))
                .unwrap_or_else(|| panic!("{role} has no row"));
            assert!(row.contains(role), "{row}");
        }
    }

    #[test]
    fn the_composition_says_who_reviews_and_never_counts_agents() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert!(
            text.contains("one per role · the reviewer reviews"),
            "{text}"
        );
        for bare in ["3 agents", "2 agents", "1 agent", "agents:"] {
            assert!(!text.contains(bare), "{bare}: {text}");
        }

        let mut two = runtime.clone();
        two.crew.retain(|member| member.role != FlowRole::Reviewer);
        let (rows, _) = render_runtime(100, 40, &two);
        let text = rows.join("\n");
        assert!(text.contains("the orchestrator reviews"), "{text}");
        assert!(!text.contains("the human reviews"), "{text}");

        let mut one = two.clone();
        one.crew
            .retain(|member| member.role != FlowRole::Orchestrator);
        let (rows, _) = render_runtime(100, 40, &one);
        let text = rows.join("\n");
        assert!(text.contains("the human reviews"), "{text}");

        // A role filled twice is not one per role, and the pane says what
        // the crew is rather than claiming a rule it does not follow.
        let mut four = runtime.clone();
        four.crew.push(member(
            FlowRole::Builder,
            "opencode-go/deepseek-v4.1-flash#high",
            Some("TSK11"),
            Some("2026-11-10T09:00Z"),
            AgentState::Executing,
        ));
        let (rows, _) = render_runtime(100, 40, &four);
        let text = rows.join("\n");
        assert!(text.contains("more than one agent per role"), "{text}");
        assert!(
            !text.contains("one per role · the reviewer reviews"),
            "{text}"
        );
    }

    #[test]
    fn a_held_task_is_in_the_crew_and_never_in_the_queue() {
        // A writer that leaves a held task in the queue does not make the
        // pane paint it twice: the crew owns it.
        let mut runtime = runtime();
        runtime.round = None;
        runtime.crew[1] = member(
            FlowRole::Builder,
            "opencode-go/deepseek-v4.1-flash#high",
            Some("TSK29"),
            Some("2026-11-09T09:00Z"),
            AgentState::Executing,
        );
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert_eq!(text.matches("TSK29").count(), 1, "{text}");
        // The queue's count is the unheld set, and it is said once.
        assert!(
            text.contains("the automation's queue · 0 waiting"),
            "{text}"
        );
    }

    #[test]
    fn the_queue_names_the_set_its_count_counts() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert!(
            text.contains("the automation's queue · 1 waiting"),
            "{text}"
        );
        let queue_row = row_of(&rows, "automation's queue");
        let item_row = row_of(&rows, "TSK29");
        assert!(queue_row < item_row, "{queue_row} {item_row}");
        assert!(rows[item_row].contains("approved"), "{}", rows[item_row]);
    }

    #[test]
    fn the_round_the_budget_and_the_findings_are_one_fact_each() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        // The timeline: the task and its stage, then each round's build and
        // review, the review under way with the spinner. Too wide for one line
        // at 100 columns, so the second round leads its own line.
        let done = Icon::StatusDone.glyph(IconSet::Nerd);
        let spinner = coding_crew_theme::chrome::spinner_frame(0);
        assert!(text.contains("TSK13 · external"), "{text}");
        assert!(
            text.contains(&format!("round 1 {done} build · {done} review 3 findings")),
            "{text}"
        );
        assert!(
            text.contains(&format!("› round 2 {done} build · {spinner} review")),
            "{text}"
        );
        assert!(text.contains("rounds ████░░░░░░ 18 of 45"), "{text}");
        assert!(
            text.contains("17 open findings · 8 high · 8 medium · 1 low"),
            "{text}"
        );

        // An empty review and no findings still have words.
        let mut empty = runtime.clone();
        empty.round = None;
        empty.findings = 0;
        let (rows, _) = render_runtime(100, 40, &empty);
        let text = rows.join("\n");
        assert!(text.contains("no review in flight"), "{text}");
        assert!(text.contains("no open findings"), "{text}");
    }

    #[test]
    fn the_stamp_says_whether_it_runs_and_why_when_it_does_not() {
        let mut runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        let stamp = rows
            .iter()
            .find(|row| row.contains("running since"))
            .expect("the stamp");
        assert!(stamp.contains("2026-11-09T08:30Z"), "{stamp}");
        assert!(
            stamp.contains(Icon::Running.glyph(IconSet::Nerd)),
            "{stamp}"
        );
        assert_eq!(
            colour_of(&rows, &paint, "running since"),
            theme().semantic.ok.fg.expect("the ok colour")
        );

        runtime.stamp = Stamp {
            runs: false,
            why: Some("the review budget is spent".to_string()),
        };
        let (rows, paint) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert!(
            text.contains("not running: the review budget is spent"),
            "{text}"
        );
        assert!(!text.contains("running since"), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "not running"),
            theme().semantic.warn.fg.expect("the warn colour")
        );
        assert!(
            text.contains(Icon::Stopped.glyph(IconSet::Nerd)),
            "the stopped icon is not on screen: {text}"
        );

        // A stop with no reason still says a word.
        runtime.stamp = Stamp {
            runs: false,
            why: None,
        };
        let (rows, _) = render_runtime(100, 40, &runtime);
        assert!(
            rows.join("\n").contains("not running: no reason given"),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn the_flow_title_wears_the_icon_the_stamp_names() {
        // A live record shows running; a stopped one and no record show
        // stopped; an unreadable record shows hugo's own icon.
        let running = format!("{} flow", Icon::Running.glyph(IconSet::Nerd));
        let stopped = format!("{} flow", Icon::Stopped.glyph(IconSet::Nerd));
        let mut runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        assert!(rows.iter().any(|row| row.contains(&running)), "{rows:?}");

        runtime.stamp = Stamp {
            runs: false,
            why: Some("the review budget is spent".to_string()),
        };
        let (rows, _) = render_runtime(100, 40, &runtime);
        assert!(rows.iter().any(|row| row.contains(&stopped)), "{rows:?}");
        assert!(!rows.iter().any(|row| row.contains(&running)), "{rows:?}");

        let (rows, _) = render(100, 40);
        assert!(rows.iter().any(|row| row.contains(&stopped)), "{rows:?}");

        let read = RuntimeRead::Unreadable(ReadError {
            file: "runtime.json".to_string(),
            error: "expected value".to_string(),
        });
        let (rows, _) = render_with(100, 40, &read);
        let config = format!("{} flow", Icon::Config.glyph(IconSet::Nerd));
        assert!(rows.iter().any(|row| row.contains(&config)), "{rows:?}");
    }

    #[test]
    fn the_crew_is_a_table_whose_columns_line_up_while_they_fit() {
        // At 100 the five columns fit the pane, so every task and hold starts
        // in the same column whichever agent holds it.
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let runner = row_of(&rows, "TSK10 󰥔 10d");
        let reviewer = row_of(&rows, "TSK13 󰥔 8d");
        assert_eq!(
            column_at(&rows, runner, "TSK10"),
            column_at(&rows, reviewer, "TSK13"),
            "{rows:?}"
        );
        assert_eq!(
            column_at(&rows, runner, "󰥔 10d"),
            column_at(&rows, reviewer, "󰥔 8d"),
            "{rows:?}"
        );
    }

    #[test]
    fn the_aligned_crew_table_names_its_columns_in_a_dim_header() {
        // The header sits over the columns it names: a task starts under
        // `task`, a hold under `held`, and the header is dim.
        let runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        let header = rows
            .iter()
            .position(|row| row.contains("held") && row.contains("state"))
            .unwrap_or_else(|| panic!("no header: {rows:?}"));
        let runner = row_of(&rows, "TSK10 󰥔 10d");
        assert_eq!(
            column_at(&rows, header, "task"),
            column_at(&rows, runner, "TSK10"),
            "{rows:?}"
        );
        assert_eq!(
            column_at(&rows, header, "held"),
            column_at(&rows, runner, "󰥔 10d"),
            "{rows:?}"
        );
        let at = column_at(&rows, header, "held");
        assert_eq!(
            paint[header][at].fg,
            theme().text.dim.fg.expect("the dim colour"),
            "{}",
            rows[header]
        );
    }

    #[test]
    fn a_row_drawn_alone_says_what_it_holds_in_words() {
        // With no header to name the columns, the task and the hold are words
        // in the row at 60 and 40, and nothing says `—` for the hold.
        for width in [60u16, 40] {
            let runtime = runtime();
            let (rows, _) = render_runtime(width, 40, &runtime);
            let text = rows.join("\n");
            assert!(text.contains("holds TSK10 · held 10d"), "{width}: {text}");
            assert!(text.contains("holds TSK13 · held 8d"), "{width}: {text}");
            assert!(text.contains("holds nothing"), "{width}: {text}");
            assert!(!text.contains("model"), "{width}: {text}");
        }
    }

    #[test]
    fn the_crew_takes_one_form_for_every_row_with_one_gap() {
        // Where the table does not fit, every agent takes the same number of
        // lines and its cells share one gap: two lines each at 60, three at 40.
        let runtime = runtime();
        let (rows, _) = render_runtime(60, 40, &runtime);
        let orchestrator = row_of(&rows, "orchestrator");
        let runner = row_of(&rows, "builder");
        let reviewer = row_of(&rows, "reviewer");
        assert_eq!(runner - orchestrator, 2, "{rows:?}");
        assert_eq!(reviewer - runner, 2, "{rows:?}");
        assert!(
            rows[runner].contains("builder deepseek-v4.1-flash#high ⠋ executing"),
            "{rows:?}"
        );

        let (rows, _) = render_runtime(40, 40, &runtime);
        let orchestrator = row_of(&rows, "orchestrator");
        let runner = row_of(&rows, "builder");
        let reviewer = row_of(&rows, "reviewer");
        assert_eq!(runner - orchestrator, 3, "{rows:?}");
        assert_eq!(reviewer - runner, 3, "{rows:?}");
        assert!(rows[runner].contains("builder ⠋ executing"), "{rows:?}");
    }

    #[test]
    fn every_line_under_a_wrapped_agent_is_indented_under_it() {
        // The runner takes two lines at 60 and three at 40; each line under
        // the runner's first sits the same two columns in, so none reads as
        // an agent of its own.
        for width in [60u16, 40] {
            let runtime = runtime();
            let (rows, _) = render_runtime(width, 40, &runtime);
            let runner = row_of(&rows, "builder");
            let reviewer = row_of(&rows, "reviewer");
            assert!(reviewer > runner + 1, "{width}: {rows:?}");
            let agent = leading(&rows[runner]);
            for row in &rows[runner + 1..reviewer] {
                assert_eq!(leading(row), agent + 2, "{width}: {row:?}");
            }
        }
    }

    #[test]
    fn a_working_agent_turns_the_spinner_and_an_idle_one_does_not() {
        let runtime = runtime();
        let mut app = app();
        // The spinner turns with the frame: the third frame is ⠸.
        let (rows, paint) =
            render_frame(&mut app, 100, 40, &RuntimeRead::Record(runtime.clone()), 3);
        assert!(
            rows.join("\n").contains("⠸ executing"),
            "{}",
            rows.join("\n")
        );
        assert_eq!(
            colour_of(&rows, &paint, "⠸"),
            theme().semantic.agent.fg.expect("the agent colour")
        );
        // The idle orchestrator has no spinner, on any frame.
        let idle = row_of(&rows, "orchestrator glm-5.3#high");
        let frames = chrome::SPINNER.concat();
        assert!(
            !rows[idle].chars().any(|c| frames.contains(c)),
            "{}",
            rows[idle]
        );
        // A render is the first frame, so a pinned screen does not move.
        let (first, _) = render_runtime(100, 40, &runtime);
        assert!(
            first.join("\n").contains("⠋ executing"),
            "{}",
            first.join("\n")
        );
    }

    #[test]
    fn the_findings_name_each_severity_on_its_own_slot() {
        let runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        assert_eq!(
            colour_of(&rows, &paint, "8 high"),
            theme().semantic.error.fg.expect("the error colour")
        );
        assert_eq!(
            colour_of(&rows, &paint, "8 medium"),
            theme().semantic.warn.fg.expect("the warn colour")
        );
        assert_eq!(
            colour_of(&rows, &paint, "1 low"),
            theme().semantic.info.fg.expect("the info colour")
        );
    }

    #[test]
    fn a_record_without_severities_names_the_plain_count() {
        let mut runtime = runtime();
        runtime.severities = None;
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert!(text.contains("17 open findings"), "{text}");
        assert!(!text.contains("8 high"), "{text}");
    }

    #[test]
    fn a_narrow_pane_wraps_the_severities_and_keeps_them() {
        // At 40 the count and the breakdown do not share a line: the breakdown
        // wraps under the count, and no severity is cut.
        let runtime = runtime();
        let (rows, _) = render_runtime(40, 40, &runtime);
        let text = rows.join("\n");
        assert!(text.contains("17 open findings"), "{text}");
        assert!(text.contains("8 high · 8 medium · 1 low"), "{text}");
        // No line that names a severity is cut.
        assert!(
            rows.iter()
                .filter(|row| row.contains("high") || row.contains("low"))
                .all(|row| !row.contains("…")),
            "{text}"
        );
    }

    #[test]
    fn the_crew_state_is_painted_by_the_agent_slot_and_idle_is_dim() {
        let runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        assert_eq!(
            colour_of(&rows, &paint, "executing"),
            theme().semantic.agent.fg.expect("the agent colour")
        );
        assert_eq!(
            colour_of(&rows, &paint, "idle"),
            theme().text.dim.fg.expect("the dim colour")
        );
    }

    #[test]
    fn the_crew_names_each_role_on_its_own_slot() {
        let runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        // Each role's crew row is found by a word of its own, and the name is
        // the rightmost one on it: the sidebar draws the same name on the left.
        let roles = [
            ("idle", "orchestrator", theme().roles.orchestrator),
            ("TSK10", "builder", theme().roles.runner),
            ("TSK13 ", "reviewer", theme().roles.reviewer),
        ];
        for (row_word, name, style) in roles {
            let y = rows
                .iter()
                .position(|row| row.contains(row_word))
                .unwrap_or_else(|| panic!("{row_word} is not on screen"));
            let at = rows[y].rfind(name).expect("the role's name is on its row");
            let column = rows[y][..at].chars().count();
            assert_eq!(
                paint[y][column].fg,
                style.fg.expect("a role has a colour"),
                "{name} is not on its role's slot"
            );
        }
    }

    #[test]
    fn the_budget_is_coloured_by_what_is_spent() {
        // Under half spent is ok, up to four fifths is a warning, and past
        // that is an error. Each boundary is the edge of its own colour.
        assert_eq!(budget_slot(0, 45), Slot::SemanticOk);
        assert_eq!(budget_slot(22, 45), Slot::SemanticOk);
        assert_eq!(budget_slot(23, 45), Slot::SemanticWarn);
        assert_eq!(budget_slot(36, 45), Slot::SemanticWarn);
        assert_eq!(budget_slot(37, 45), Slot::SemanticError);
        assert_eq!(budget_slot(0, 0), Slot::SemanticOk);
    }

    #[test]
    fn the_sidebar_is_drawn_on_its_surface_and_the_main_pane_titles_on_the_accent() {
        let runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        // A word of the sidebar's group rule is on the surface, and the main
        // pane's crew title is on the accent.
        let (y, x) = rows
            .iter()
            .enumerate()
            .find_map(|(y, row)| row.find("AGENTS").map(|at| (y, row[..at].chars().count())))
            .expect("the AGENTS title is on screen");
        assert_eq!(
            paint[y][x].bg,
            theme().chrome.surface.bg.expect("the surface is a colour")
        );
        assert_eq!(
            colour_of(&rows, &paint, "crew"),
            theme()
                .chrome
                .tab_active
                .fg
                .expect("the accent is a colour")
        );
    }

    #[test]
    fn a_narrow_pane_gives_the_crew_entry_more_lines_and_keeps_every_fact() {
        // The entry is one line at 100, two at 60, three at 40; no fact is
        // dropped with the width.
        for width in [100u16, 60, 40] {
            let runtime = runtime();
            let (rows, _) = render_runtime(width, 40, &runtime);
            let text = rows.join("\n");
            for fact in [
                "builder",
                "deepseek-v4.1-flash#high",
                "TSK10",
                "10d",
                "executing",
                "⠋",
            ] {
                assert!(text.contains(fact), "{width}: {fact} is gone: {text}");
            }
        }
    }

    #[test]
    fn a_hold_is_a_length_measured_from_the_frames_one_instant() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        // Ten days and eight days before the render's instant, on the clock.
        let clock = Icon::Clock.glyph(IconSet::Nerd);
        assert!(text.contains(&format!("{clock} 10d")), "{text}");
        assert!(text.contains(&format!("{clock} 8d")), "{text}");
        // A clock reading is not a length: the row does not drop the date
        // and paint a time of day.
        assert!(!text.contains("since 09:00Z"), "{text}");
        assert!(!text.contains("since 16:00Z"), "{text}");
    }

    #[test]
    fn a_hold_under_a_minute_says_so() {
        for (since, held) in [
            ("2026-11-19T08:59:30Z", "under a minute"),
            ("2026-11-19T08:59:00Z", "1m"),
            ("2026-11-19T06:00:00Z", "3h"),
            ("2026-11-09T09:00:00Z", "10d"),
        ] {
            assert_eq!(held_length(Some(since), RENDER_NOW), held, "{since}");
        }
        // No hold has no length; it is not zero days.
        assert_eq!(held_length(None, RENDER_NOW), "—");
    }

    #[test]
    fn a_record_that_cannot_be_read_names_the_file_and_the_error() {
        let error = ReadError {
            file: "runtime.json".to_string(),
            error: "failed to parse the runtime record: expected value at line 1".to_string(),
        };
        let (rows, paint) = render_with(100, 20, &RuntimeRead::Unreadable(error));
        let text = rows.join("\n");
        assert!(text.contains("cannot be read"), "{text}");
        assert!(text.contains("runtime.json"), "{text}");
        assert!(text.contains("failed to parse"), "{text}");
        // An unreadable record is not the absent one: the pane does not say
        // nothing is running.
        assert!(!text.contains("nothing is running"), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "failed to parse"),
            theme().semantic.error.fg.expect("the error colour")
        );
    }

    #[test]
    fn a_block_that_does_not_fit_is_announced_and_never_cut_silently() {
        // The pane is shorter than its blocks: the words stay, the blocks
        // that give way are named with what they held.
        for height in 8..=16u16 {
            let runtime = runtime();
            let (rows, _) = render_runtime(60, height, &runtime);
            let text = rows.join("\n");
            if !text.contains("findings") || !text.contains("budget") {
                assert!(text.contains("not shown:"), "{height}: {text}");
                let announcement = rows
                    .iter()
                    .find(|row| row.contains("not shown:") && !row.contains("sidebar"))
                    .unwrap_or_else(|| panic!("{height}: no announcement"));
                assert!(
                    announcement.contains(" (") && announcement.contains(" hidden)"),
                    "{height}: the announcement names no count: {announcement}"
                );
            }
        }
        // The crew's eight lines go with the crew, and the announcement says so:
        // its caption, three agents of two lines each, and the composition.
        let (rows, _) = render_runtime(60, 8, &runtime());
        let text = rows.join("\n");
        assert!(text.contains("crew (8 hidden)"), "{text}");
        // The stamp survives the short pane: whether it runs is the last
        // fact to give way.
        let (rows, _) = render_runtime(60, 8, &runtime());
        assert!(
            rows.join("\n").contains("running since"),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn the_sidebar_carries_the_configuration_and_reads_it() {
        let (rows, _) = render(100, 40);
        let text = rows.join("\n");
        for fact in [
            "AGENTS",
            "builder",
            "harness",
            "claude",
            "claude-haiku-5-5",
            "effort",
            "ROUNDS",
            "5 build-and-fix rounds",
            "ORDER",
            "1 groom · creator",
            "groomed → approved",
            "6 re-review · reviewer",
        ] {
            assert!(text.contains(fact), "{fact} is not on the sidebar: {text}");
        }
        // The groups are read in the documented order.
        let at = |needle: &str| row_of(&rows, needle);
        assert!(at("AGENTS") < at("ROUNDS"), "{text}");
        assert!(at("ROUNDS") < at("ORDER"), "{text}");

        // A redraw draws the same configuration: the keys only change what
        // the cursor points at, and a cursor paints no word.
        let again = render(100, 40).0;
        assert_eq!(rows, again, "a redraw changed the configuration");
    }

    #[test]
    fn a_crew_of_one_starts_the_cursor_on_a_row_it_has() {
        // The orchestrator alone: there is no builder's row to start on, so
        // the cursor starts on the first row, and enter opens its choice.
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role == FlowRole::Orchestrator);
        let mut app = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Config(config)),
            None,
        );
        app.agents_drawn = true;
        assert_eq!(app.cursor, 0);
        assert!(app.can_choose(), "the first row is the orchestrator's");
        app.on_key(press(KeyCode::Enter));
        let choice = app.choice.as_ref().expect("enter opened no choice");
        assert_eq!(choice.title, "orchestrator harness");
    }

    #[test]
    fn the_cursor_moves_over_each_agents_rows_and_enter_opens_that_rows_choice() {
        let mut app = app();
        app.agents_drawn = true;
        // The cursor starts on the builder's harness row.
        assert_eq!(app.cursor, sidebar::cursor_of(1, Field::Harness));
        app.on_key(press(KeyCode::Enter));
        let choice = app.choice.as_ref().expect("enter opened no choice");
        assert_eq!(choice.title, "builder harness");
        assert_eq!(choice.field, Field::Harness);
        app.choice = None;
        // The rows run harness, model, effort for each agent, and the cursor
        // stops at the last row and comes back to the first.
        for _ in 0..8 {
            app.on_key(press(KeyCode::Down));
        }
        assert_eq!(app.cursor, sidebar::cursor_of(2, Field::Effort));
        assert!(app.can_choose(), "the last row is the reviewer's effort");
        app.on_key(press(KeyCode::Down));
        assert_eq!(app.cursor, sidebar::cursor_of(2, Field::Effort));
        for _ in 0..8 {
            app.on_key(press(KeyCode::Char('k')));
        }
        assert_eq!(app.cursor, 0);
        // The other keys do not move it.
        for code in [KeyCode::Char('h'), KeyCode::Char('l'), KeyCode::PageDown] {
            app.on_key(press(code));
            assert_eq!(app.cursor, 0, "{code:?} moved the cursor");
        }
    }

    #[test]
    fn the_cursor_has_nothing_to_stand_on_when_its_group_is_not_drawn() {
        // A pane too short for the agents leaves them out whole: the cursor
        // must not walk rows the pane does not draw, and enter opens nothing.
        let mut app = app();
        app.agents_drawn = false;
        let before = app.cursor;
        app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.cursor, before);
        app.cursor = 2;
        assert!(!app.can_choose(), "the hidden cursor can choose");
    }

    #[test]
    fn a_configuration_that_cannot_be_read_is_named_on_the_sidebar() {
        let error = ReadError {
            file: CONFIG_FILE.to_string(),
            error: Config::from_toml("round_limit = \n")
                .expect_err("a broken configuration")
                .to_string(),
        };
        let (rows, paint) = render_all(
            100,
            40,
            &RuntimeRead::Absent,
            &ConfigRead::of(ConfigState::Unreadable(error)),
        );
        let text = rows.join("\n");
        assert!(text.contains("configuration is unreadable"), "{text}");
        // The sidebar's 29 columns hold the reason first; the file follows it.
        assert!(text.contains("syntax error"), "{text}");
        assert!(!text.contains("no configuration yet"), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "syntax error"),
            theme().semantic.error.fg.expect("the error colour")
        );
        // The defaults are shown beside the error: what is wrong, and what
        // the tool would use.
        assert!(text.contains(sidebar::USING_DEFAULTS), "{text}");
        assert!(text.contains("5 build-and-fix rounds"), "{text}");
        assert!(text.contains("builder"), "{text}");
    }

    #[test]
    fn the_error_on_screen_is_the_one_the_pipeline_produces() {
        // The row-level wording and the real reader's wording are one: the
        // file is read through the pipeline, and what the sidebar paints is
        // what that reader said.
        let home = scratch("error-pinned");
        fs::create_dir_all(config::home_at(&home)).expect("the home");
        let path = config::config_path(&home);
        fs::write(&path, "round_limit = \n").expect("a broken configuration");
        let read = config::load_at(&home);
        let expected = Config::read(&path)
            .expect_err("a broken configuration")
            .to_string();
        match &read.state {
            ConfigState::Unreadable(error) => assert_eq!(error.error, expected),
            other => panic!("{other:?}"),
        }
        let mut app = App::new(theme(), IconSet::Nerd, read, Some(home.clone()));
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        // The sidebar's text is 29 columns: its inner width, less the bar's
        // column. The row is the reason first, then the file, cut to that
        // exactly — not a shorter wording a test invented.
        let visible = cut_to(&format!("{expected} · {CONFIG_FILE}"), 29);
        assert!(
            rows.iter().any(|row| row.contains(&visible)),
            "the screen does not carry {visible:?}: {}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_old_json_configuration_is_named_on_the_sidebar() {
        let config = ConfigRead {
            state: ConfigState::Config(Config::default()),
            legacy: Some(LEGACY_CONFIG_FILE.to_string()),
            warnings: Vec::new(),
        };
        let (rows, paint) = render_all(100, 40, &RuntimeRead::Absent, &config);
        let text = rows.join("\n");
        assert!(text.contains(LEGACY_CONFIG_FILE), "{text}");
        assert!(text.contains("not read"), "{text}");
        assert!(text.contains("5 build-and-fix rounds"), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "not read"),
            theme().text.dim.fg.expect("the dim colour")
        );
    }

    #[test]
    fn the_window_fits_its_width_at_every_size() {
        // A shell that draws past its width is a cut screen: every row is
        // measured by what it holds, at the sizes the sweep and the pins use.
        let runtime = runtime();
        for width in [20u16, 40, 60, 79, 80, 100, 120] {
            for height in [1u16, 2, 3, 4, 5, 8, 12, 20, 40] {
                for runtime in [
                    RuntimeRead::Absent,
                    RuntimeRead::Record(runtime.clone()),
                    RuntimeRead::Unreadable(ReadError {
                        file: "runtime.json".to_string(),
                        error: "failed to parse the runtime record".to_string(),
                    }),
                ] {
                    let (rows, _) = render_with(width, height, &runtime);
                    assert_eq!(rows.len(), usize::from(height), "{width}x{height}");
                    for (y, row) in rows.iter().enumerate() {
                        assert!(
                            row.chars().count() <= usize::from(width),
                            "{width}x{height} row {y}: {row:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_choice_and_the_help_fit_the_window_at_every_size() {
        // A popup that draws past its width is a cut screen, as any part is.
        for width in [20u16, 40, 60, 79, 80, 100, 120] {
            for height in [1u16, 2, 3, 5, 8, 12, 20, 40] {
                let mut window = on_orchestrator(None);
                answer_choice(&mut window, Ok(Catalog::from_lines("p/m0\np/m1\np/m2\n")));
                let (rows, _) = render_app(&mut window, width, height, &RuntimeRead::Absent);
                assert_eq!(rows.len(), usize::from(height), "{width}x{height}");
                for row in &rows {
                    assert!(
                        row.chars().count() <= usize::from(width),
                        "{width}x{height}: {row:?}"
                    );
                }

                let mut window = app();
                window.help = true;
                let (rows, _) = render_app(&mut window, width, height, &RuntimeRead::Absent);
                assert_eq!(rows.len(), usize::from(height), "{width}x{height}");
                for row in &rows {
                    assert!(
                        row.chars().count() <= usize::from(width),
                        "{width}x{height}: {row:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_height_keeps_the_empty_state_or_the_stamp() {
        for height in 1..=12u16 {
            let runtime = runtime();
            let (rows, _) = render_runtime(60, height, &runtime);
            let text = rows.join("\n");
            assert!(
                text.contains("running since") || text.contains("not shown:"),
                "{height}: {text}"
            );
        }
    }

    #[test]
    fn the_screens_doc_names_the_window_the_shell_builds() {
        // The doc is the spec of what is built: every number and every named
        // state it carries is read here, so a changed constant or a renamed
        // state fails the gate instead of drifting from the prose.
        let doc = include_str!("../../../docs/hugo/screens.md");
        let facts = [
            format!("{SIDEBAR_WIDTH} columns"),
            format!("{SIDEBAR_MIN_WIDTH} columns"),
            format!("{MAIN_MIN_WIDTH} columns"),
            SIDEBAR_TITLE.to_string(),
            FLOW_TITLE.to_string(),
            sidebar::EMPTY.to_string(),
            FLOW_EMPTY.to_string(),
            "sidebar not shown: configuration".to_string(),
            "~/.config/coding-crew/".to_string(),
            CONFIG_FILE.to_string(),
            LEGACY_CONFIG_FILE.to_string(),
            "TOML".to_string(),
            "read-only".to_string(),
            "--render --plain".to_string(),
            "100 by 40".to_string(),
            NAME.trim().to_string(),
            sidebar::AGENTS_TITLE.to_string(),
            "harness".to_string(),
            "effort".to_string(),
            sidebar::LIMIT_TITLE.to_string(),
            sidebar::ORDER_TITLE.to_string(),
            sidebar::UNREADABLE.to_string(),
            sidebar::USING_DEFAULTS.to_string(),
            RELOAD_KEY.to_string(),
            CHOOSE_KEY.to_string(),
            CLOSE_KEY.to_string(),
            format!("{START_KEY} start"),
            START_NEEDS_HERDR.to_string(),
            QUEUE_EMPTY.to_string(),
            STARTING.to_string(),
            ALREADY_RUNNING.to_string(),
            "orchestrator started in tab".to_string(),
            START_FAILED.to_string(),
            on_path::MISSING.to_string(),
            on_path::SHADOWED.to_string(),
            "builder model".to_string(),
            CATALOG_READING.to_string(),
            CATALOG_UNREADABLE.to_string(),
            format!("{} seconds", crate::catalog::READ_BOUND.as_secs()),
            HELP_TITLE.to_string(),
            "the harness's own list".to_string(),
            pick::TYPE_AN_ID.to_string(),
            NOT_INSTALLED.to_string(),
            NOT_A_MODEL_ID.to_string(),
            "pick a model".to_string(),
            "has no model".to_string(),
            CREW_TITLE.to_string(),
            QUEUE_TITLE.to_string(),
            ROUND_TITLE.to_string(),
            BUDGET_TITLE.to_string(),
            FINDINGS_TITLE.to_string(),
            "the automation's queue".to_string(),
            "one per role".to_string(),
            "the orchestrator reviews".to_string(),
            "the human reviews".to_string(),
            "more than one agent per role".to_string(),
            "runtime.json".to_string(),
            "ctrl-c".to_string(),
            FLOW_UNREADABLE.to_string(),
            RENDER_NOW.to_string(),
        ];
        for fact in facts {
            assert!(doc.contains(&fact), "the doc does not name {fact:?}");
        }
    }

    #[test]
    fn the_render_mode_uses_the_fixed_instant_in_the_doc() {
        // A pinned screen must not move with the wall clock: the render
        // draws at one fixed instant, and the doc names it.
        assert_eq!(RENDER_NOW, "2026-11-19T09:00:00Z");
        let doc = include_str!("../../../docs/hugo/screens.md");
        assert!(
            doc.contains(RENDER_NOW),
            "the doc does not name the instant"
        );
    }

    #[test]
    fn the_composition_is_the_one_in_the_doc() {
        // The doc's sizes match the code's composition, including the crew
        // it has no rule for.
        let doc = include_str!("../../../docs/hugo/screens.md");
        for (crew, says) in [
            (
                vec![
                    member(FlowRole::Orchestrator, "m", None, None, AgentState::Idle),
                    member(FlowRole::Builder, "m", None, None, AgentState::Executing),
                    member(FlowRole::Reviewer, "m", None, None, AgentState::Reviewing),
                ],
                Composition::OnePerRole,
            ),
            (
                vec![
                    member(FlowRole::Orchestrator, "m", None, None, AgentState::Idle),
                    member(FlowRole::Builder, "m", None, None, AgentState::Executing),
                ],
                Composition::OrchestratorReviews,
            ),
            (
                vec![member(
                    FlowRole::Builder,
                    "m",
                    None,
                    None,
                    AgentState::Executing,
                )],
                Composition::HumanReviews,
            ),
            (
                vec![
                    member(FlowRole::Orchestrator, "m", None, None, AgentState::Idle),
                    member(FlowRole::Builder, "m", None, None, AgentState::Executing),
                    member(FlowRole::Builder, "m", None, None, AgentState::Executing),
                    member(FlowRole::Reviewer, "m", None, None, AgentState::Reviewing),
                ],
                Composition::MoreThanOnePerRole,
            ),
        ] {
            let runtime = Runtime {
                started_at: "2026-11-09T08:30Z".to_string(),
                updated_at: None,
                crew,
                queue: Vec::new(),
                round: None,
                budget: Budget::default(),
                findings: 0,
                severities: None,
                stamp: Stamp {
                    runs: true,
                    why: None,
                },
            };
            assert_eq!(runtime.composition(), says);
            assert!(doc.contains(says.says()), "the doc does not say {says:?}");
        }
    }

    /// A window on the project's configuration, with its cursor on the
    /// orchestrator's model row.
    fn on_orchestrator(home: Option<PathBuf>) -> App {
        let mut app = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Config(Config::default())),
            home,
        );
        app.agents_drawn = true;
        app.cursor = sidebar::cursor_of(0, Field::Model);
        app
    }

    fn text_of(rows: &[String]) -> String {
        rows.join("\n")
    }

    /// Wait for a catalogue read to answer the way the loop does: by polling
    /// it, never by waiting on it.
    fn wait_for_catalog(window: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while window.catalog_pending() {
            assert!(
                std::time::Instant::now() < deadline,
                "the catalogue never answered"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
            window.poll_choice();
        }
    }

    /// A window on the orchestrator's model row, reading its catalogue from `tool`
    /// in place of the harness's own list.
    fn on_orchestrator_with(tool: &std::path::Path) -> App {
        let mut window = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Config(Config::default())),
            None,
        );
        window.agents_drawn = true;
        window.cursor = sidebar::cursor_of(0, Field::Model);
        window.catalog_tool = Some(tool.to_path_buf());
        window
    }

    #[test]
    fn enter_opens_the_choice_before_the_catalogue_is_read() {
        // The catalogue is a subprocess: the choice opens at once and the list
        // lands when the tool answers. The window is never held on the tool.
        let tool = crate::catalog::scratch_tool("app-reading", "#!/bin/sh\nsleep 0.3\necho p/m0\n");
        let mut window = on_orchestrator_with(&tool);
        assert_eq!(
            window.on_key(press(KeyCode::Enter)),
            Action::Redraw,
            "opening the choice quit the window"
        );
        assert!(
            window.catalog_pending(),
            "the window waited on the harness tool"
        );
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains(CATALOG_READING),
            "{}",
            text_of(&rows)
        );
        // The keys are still read while the tool runs: quit is one of them.
        assert_eq!(window.on_key(press(KeyCode::Char('q'))), Action::Quit);

        wait_for_catalog(&mut window);
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("p/m0"),
            "the catalogue did not land in the choice: {}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_catalogue_that_cannot_be_read_is_named_in_the_choice() {
        // A tool that fails is an error the choice shows, end to end.
        let tool = crate::catalog::scratch_tool(
            "app-failing",
            "#!/bin/sh\necho 'no harness here' >&2\nexit 9\n",
        );
        let mut window = on_orchestrator_with(&tool);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        match &window.choice.as_ref().expect("the choice").state {
            ChoiceState::Failed(why) => {
                assert!(why.contains("failed"), "{why}");
                assert!(why.contains("no harness here"), "{why}");
            }
            other => panic!("a failing tool is {other:?}"),
        }
        let (rows, paint) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(CATALOG_UNREADABLE), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "failed:"),
            theme().semantic.error.fg.expect("the error colour")
        );
        // Nothing is picked from a catalogue that could not be read: enter opens
        // the id input, and esc goes back to the list and then closes it.
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_some(), "enter closed the failed choice");
        window.on_key(press(KeyCode::Esc));
        window.on_key(press(KeyCode::Esc));
        assert!(window.choice.is_none());
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_reads_reason_leads_its_tool() {
        assert_eq!(
            App::reason_first("/tmp/a/b/a-tool failed: no harness here"),
            "failed: no harness here · a-tool"
        );
        assert_eq!(
            App::reason_first("/tmp/a/b/a-tool failed"),
            "failed · a-tool"
        );
        assert_eq!(
            App::reason_first("/tmp/a/b/a-tool: timed out after 5s"),
            "timed out after 5s · a-tool"
        );
        assert_eq!(
            App::reason_first("opencode is not on PATH"),
            "opencode is not on PATH"
        );
    }

    #[test]
    fn a_long_tool_path_does_not_hide_why_the_catalogue_failed() {
        // The macOS temp path fills the choice's line: the reason leads, so
        // `failed:` and the tool's own words stay on screen.
        let base = std::env::temp_dir().join(format!("hugo-catalog-long-{}", "x".repeat(80)));
        let tool = crate::catalog::scratch_tool_under(
            &base,
            "app-long",
            "#!/bin/sh\necho 'no harness here' >&2\nexit 9\n",
        );
        assert!(tool.to_string_lossy().len() > 100, "the path is not long");
        let mut window = on_orchestrator_with(&tool);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(CATALOG_UNREADABLE), "{text}");
        assert!(text.contains("failed:"), "the reason is cut: {text}");
        assert!(
            text.contains("no harness here"),
            "the tool's words are cut: {text}"
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_catalogue_that_lands_after_esc_does_not_reopen_the_choice() {
        let tool = crate::catalog::scratch_tool("app-late", "#!/bin/sh\nsleep 0.2\necho p/m0\n");
        let mut window = on_orchestrator_with(&tool);
        window.on_key(press(KeyCode::Enter));
        assert!(window.catalog_pending());
        window.on_key(press(KeyCode::Esc));
        assert!(window.choice.is_none(), "esc left the choice open");
        assert!(
            !window.catalog_pending(),
            "the read outlived the choice it was opened for"
        );
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn r_reads_the_configuration_again_and_the_sidebar_shows_it_at_once() {
        let home = scratch("reload");
        let path = config::config_path(&home);
        let mut app = App::new(
            theme(),
            IconSet::Nerd,
            config::load_at(&home),
            Some(home.clone()),
        );
        assert_eq!(app.session().expect("the configuration").round_limit, 5);
        // A human edits the file while hugo runs.
        let mine = Config {
            round_limit: 2,
            ..Config::default()
        };
        mine.write(&path).expect("the hand-edited file");

        assert_eq!(
            app.on_key(press(KeyCode::Char('r'))),
            Action::Redraw,
            "reload quit the window"
        );
        assert_eq!(app.session().expect("the configuration").round_limit, 2);
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("2 build-and-fix rounds"),
            "{}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_reload_that_does_not_parse_is_handled_as_at_startup() {
        // The file is named with its error, left alone, and the defaults are
        // shown beside it — the same state a broken file has at launch.
        let home = scratch("reload-broken");
        fs::create_dir_all(config::home_at(&home)).expect("the home");
        let path = config::config_path(&home);
        let broken = "round_limit = \n";
        fs::write(&path, broken).expect("a broken file");
        let mut app = App::new(theme(), IconSet::Nerd, sidebar_config(), Some(home.clone()));

        app.on_key(press(KeyCode::Char('r')));
        assert!(
            matches!(app.config.state, ConfigState::Unreadable(_)),
            "{:?}",
            app.config.state
        );
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains("configuration is unreadable"), "{text}");
        assert!(text.contains("syntax error"), "{text}");
        assert!(text.contains("using the defaults:"), "{text}");
        assert!(text.contains("5 build-and-fix rounds"), "{text}");
        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            broken,
            "the reload wrote over a file it could not read"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_reload_with_no_configuration_writes_the_defaults() {
        let home = scratch("reload-absent");
        let mut app = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Absent),
            Some(home.clone()),
        );
        let path = config::config_path(&home);
        assert!(!path.exists(), "the test home is not fresh");

        app.on_key(press(KeyCode::Char('r')));
        assert!(path.is_file(), "the reload wrote no defaults");
        assert!(matches!(app.config.state, ConfigState::Config(_)));
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("5 build-and-fix rounds"),
            "{}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_choice_lists_the_catalogue_and_starts_on_the_model_in_force() {
        let home = scratch("choice");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(
            &mut app,
            Ok(Catalog::from_lines(
                "claude-sonnet-5-5\nclaude-opus-5-5\nclaude-haiku-5-5\n",
            )),
        );
        let choice = app.choice.as_ref().expect("the choice");
        assert_eq!(
            choice.catalog.ids(),
            ["claude-sonnet-5-5", "claude-opus-5-5", "claude-haiku-5-5"],
            "the list is not the catalogue's"
        );
        // The orchestrator runs glm-5.3#high: the cursor starts there.
        assert_eq!(choice.cursor, 1);
        let models: Vec<String> = choice
            .catalog
            .ids()
            .iter()
            .map(|id| id.to_string())
            .collect();

        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains("orchestrator model"), "{text}");
        for model in models {
            assert!(text.contains(&model), "{model} is not in the list: {text}");
        }
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pick_is_made_in_the_session_and_the_file_is_not_written() {
        let home = scratch("pick");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(
            &mut app,
            Ok(Catalog::from_lines(
                "claude-sonnet-5-5\nclaude-opus-5-5\nclaude-haiku-5-5\n",
            )),
        );
        // The cursor starts on glm-5.3; move up to kimi-k3 and pick it.
        app.on_key(press(KeyCode::Char('k')));
        assert_eq!(app.choice.as_ref().expect("the choice").cursor, 0);
        assert_eq!(
            app.on_key(press(KeyCode::Enter)),
            Action::Redraw,
            "a pick quit the window"
        );
        assert!(app.choice.is_none(), "the choice stayed open");
        let saved = app.session().expect("the configuration");
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == FlowRole::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("claude-sonnet-5-5"),
            "the sidebar does not show the pick"
        );
        // The flow's own agents are untouched.
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == FlowRole::Builder)
                .map(|agent| agent.model.as_str()),
            Some("claude-haiku-5-5")
        );
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == FlowRole::Reviewer)
                .map(|agent| agent.model.as_str()),
            Some("claude-opus-5-5")
        );

        // The pick lasts the session: the file is not written, and the builder
        // keeps the model the file holds.
        assert!(
            !config::config_path(&home).exists(),
            "the pick wrote the configuration"
        );
        assert_eq!(model_of(&app, "builder"), Some("claude-haiku-5-5"));
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("claude-sonnet-5-5"),
            "the sidebar does not draw the pick: {}",
            text_of(&rows)
        );
        assert!(
            text_of(&rows).contains("orchestrator session"),
            "the sidebar does not tag the pick: {}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn picking_the_model_in_force_keeps_its_variant() {
        // The catalogue lists base ids. The orchestrator runs glm-5.3#high:
        // enter on that model in force must not save it without its variant.
        let home = scratch("pick-in-force");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(
            &mut app,
            Ok(Catalog::from_lines("claude-opus-5-5\nclaude-sonnet-5-5\n")),
        );
        assert_eq!(app.choice.as_ref().expect("the choice").cursor, 0);
        app.on_key(press(KeyCode::Enter));
        assert_eq!(
            model_of(&app, "orchestrator"),
            Some("claude-opus-5-5"),
            "the pick dropped the variant in force"
        );
        assert!(
            !config::config_path(&home).exists(),
            "the pick wrote the configuration"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn enter_picks_nothing_while_the_catalogue_is_read() {
        // The choice opens on a read that has not answered. Enter does nothing
        // then: the read is not abandoned and no model is saved. Once the list
        // is in, enter picks.
        let home = scratch("choice-enter-reading");
        let tool =
            crate::catalog::scratch_tool("app-enter-reading", "#!/bin/sh\nsleep 0.5\necho p/m0\n");
        let mut window = on_orchestrator_with(&tool);
        window.home = Some(home.clone());
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            window.choice.as_ref().expect("the choice").state,
            ChoiceState::Reading
        );
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            window
                .choice
                .as_ref()
                .expect("enter closed the choice")
                .state,
            ChoiceState::Reading
        );
        assert!(window.catalog_pending(), "enter abandoned the read");
        assert!(
            !config::config_path(&home).exists(),
            "enter picked while the catalogue was read"
        );
        let footer: String = window
            .footer_lines(100, 1)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            !footer.contains("pick"),
            "the footer offers a dead key: {footer}"
        );
        wait_for_catalog(&mut window);
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(text_of(&rows).contains("pick"), "{}", text_of(&rows));
        window.on_key(press(KeyCode::Enter));
        assert!(
            window.choice.is_none(),
            "enter did not pick the listed model"
        );
        assert_eq!(
            model_of(&window, "orchestrator"),
            Some("p/m0"),
            "the pick was not made in the session"
        );
        assert!(
            !config::config_path(&home).exists(),
            "the pick wrote the configuration"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn esc_closes_the_choice_and_saves_nothing() {
        let home = scratch("choice-esc");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(
            &mut app,
            Ok(Catalog::from_lines("claude-sonnet-5-5\nclaude-opus-5-5\n")),
        );
        app.on_key(press(KeyCode::Char('k')));
        app.on_key(press(KeyCode::Esc));
        assert!(app.choice.is_none(), "esc left the choice open");
        assert!(
            !config::config_path(&home).exists(),
            "esc wrote a configuration"
        );
        assert_eq!(
            model_of(&app, "orchestrator"),
            Some("claude-opus-5-5"),
            "esc changed the model in force"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_choice_with_no_catalogue_says_why_and_picks_nothing() {
        let home = scratch("choice-error");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(&mut app, Err("opencode models: not found".to_string()));
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(CATALOG_UNREADABLE), "{text}");
        assert!(text.contains("not found"), "{text}");
        app.on_key(press(KeyCode::Enter));
        assert!(app.choice.is_some(), "enter closed the failed choice");
        assert!(
            !config::config_path(&home).exists(),
            "a pick was saved from an empty catalogue"
        );
        app.on_key(press(KeyCode::Esc));
        app.on_key(press(KeyCode::Esc));
        assert!(app.choice.is_none());
        let _ = fs::remove_dir_all(&home);

        // A catalogue that lists nothing says so, rather than opening blank.
        let home = scratch("choice-empty");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(&mut app, Ok(Catalog::default()));
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(text_of(&rows).contains(NO_MODELS), "{}", text_of(&rows));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_choice_scrolls_to_keep_its_cursor_on_screen() {
        let home = scratch("choice-scroll");
        let mut app = on_orchestrator(Some(home.clone()));
        let list: String = (0..40)
            .map(|index| format!("provider/model-{index:02}\n"))
            .collect();
        answer_choice(&mut app, Ok(Catalog::from_lines(&list)));
        app.choice.as_mut().expect("the choice").cursor = 30;
        let (rows, _) = render_app(&mut app, 100, 20, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(
            text.contains("model-30"),
            "the cursor is off screen: {text}"
        );
        assert!(
            !text.contains("model-00"),
            "the list did not scroll: {text}"
        );
        // The footer keeps the choice's keys on screen.
        assert!(
            rows.last().expect("a footer").contains("close"),
            "{}",
            rows.last().expect("a footer")
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_choice_footer_names_its_keys_and_the_help_names_both_scopes() {
        let mut choosing = on_orchestrator(None);
        answer_choice(&mut choosing, Ok(Catalog::from_lines("a/b\nc/d\n")));
        let line: String = choosing
            .footer_lines(100, 1)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        for works in [
            CLOSE_KEY, "close", CHOOSE_KEY, "pick", "q", "quit", "?", "help",
        ] {
            assert!(line.contains(works), "{line}: {works}");
        }
        // The movement keys are the help's, not the footer's.
        assert!(!line.contains("move"), "{line}");
        // A narrow footer gives the second row to the closing keys, and the
        // close and pick chips stay on the first.
        let narrow: String = choosing
            .footer_lines(25, 2)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            narrow.contains("close") && narrow.contains("pick"),
            "{narrow}"
        );
        assert!(!narrow.contains("move"), "{narrow}");

        // The help opens over the choice, and closing it gives the choice
        // back rather than dropping it.
        choosing.on_key(press(KeyCode::Char('?')));
        assert!(choosing.help, "? did not open the help over the choice");
        assert!(choosing.choice.is_some(), "the help closed the choice");
        let (rows, _) = render_app(&mut choosing, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        for says in ["close", "pick", "move", "quit"] {
            assert!(
                text.contains(says),
                "{says} is not in the choice's help: {text}"
            );
        }
        assert!(
            !text.contains("reload"),
            "the choice's help names reload: {text}"
        );
        choosing.on_key(press(KeyCode::Char('x')));
        assert!(!choosing.help, "a key left the help open");
        assert!(
            choosing.choice.is_some(),
            "closing the help lost the choice"
        );

        // The help is one key away. In the window it names the sidebar's
        // keys: enter is on the builder's harness row, where it opens a choice.
        let mut window = app();
        window.on_key(press(KeyCode::Char('?')));
        assert!(window.help, "? did not open the help");
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(HELP_TITLE), "{text}");
        for says in ["reload", "move", "quit", "choose"] {
            assert!(text.contains(says), "{says} is not in the help: {text}");
        }
        let body = rows[..rows.len() - 1].join("\n");
        for dead in ["pick", "close"] {
            assert!(
                !body.contains(dead),
                "{dead} is in the window's help: {text}"
            );
        }
        let footer = rows.last().expect("a footer");
        assert!(footer.contains("close help"), "{footer}");
        // On the orchestrator's model row, enter has a choice to open, and the
        // help names it.
        window.on_key(press(KeyCode::Char('x')));
        window.cursor = sidebar::cursor_of(0, Field::Model);
        window.agents_drawn = true;
        window.on_key(press(KeyCode::Char('?')));
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(text_of(&rows).contains("choose"), "{}", text_of(&rows));
        // Any key closes it; q still quits.
        window.on_key(press(KeyCode::Char('x')));
        assert!(!window.help, "a key left the help open");
        window.on_key(press(KeyCode::Char('?')));
        assert_eq!(window.on_key(press(KeyCode::Char('q'))), Action::Quit);
    }

    #[test]
    fn r_starts_the_session_over_from_the_file() {
        // A pick lasts until the file is read again: reload drops the picks,
        // and the sidebar shows the file's values.
        let home = scratch("reload-picks");
        let mut app = on_orchestrator(Some(home.clone()));
        answer_choice(
            &mut app,
            Ok(Catalog::from_lines("claude-sonnet-5-5\nclaude-opus-5-5\n")),
        );
        app.on_key(press(KeyCode::Char('k')));
        app.on_key(press(KeyCode::Enter));
        assert_eq!(model_of(&app, "orchestrator"), Some("claude-sonnet-5-5"));

        app.on_key(press(KeyCode::Char('r')));
        assert_eq!(
            model_of(&app, "orchestrator"),
            Some("claude-opus-5-5"),
            "reload kept the session's pick"
        );
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(!text_of(&rows).contains("session"), "{}", text_of(&rows));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_crew_no_size_names_opens_unpicked_and_enter_waits_for_a_move() {
        // An orchestrator and a reviewer is no size the selector names: enter
        // on the opening cursor must not park the reviewer. A move picks.
        let home = scratch("crew-size-odd");
        let mut app = on_orchestrator(Some(home.clone()));
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role != FlowRole::Builder);
        app.config = ConfigRead::of(ConfigState::Config(config.clone()));
        app.session = Some(config);
        app.on_key(press(KeyCode::Char('c')));
        assert!(app.crew_cursor.is_some(), "the selector did not open");
        app.on_key(press(KeyCode::Enter));
        assert!(app.crew_cursor.is_some(), "enter picked a size unasked");
        let roles = |app: &App| -> Vec<FlowRole> {
            app.session()
                .expect("the session")
                .agents
                .iter()
                .map(|agent| agent.role)
                .collect()
        };
        assert_eq!(roles(&app), [FlowRole::Orchestrator, FlowRole::Reviewer]);
        app.on_key(press(KeyCode::Down));
        app.on_key(press(KeyCode::Enter));
        assert!(app.crew_cursor.is_none(), "enter left the selector open");
        assert_eq!(roles(&app), [FlowRole::Orchestrator, FlowRole::Builder]);
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_odd_crew_opens_with_no_size_painted_and_up_on_the_top_row_keeps_it_so() {
        // A crew no size names paints no size, so the screen shows none picked.
        // ↑ on the top row is no move: enter after it still does nothing, and
        // ↓ is the first real move that paints a size.
        let home = scratch("crew-size-unpainted");
        let mut app = on_orchestrator(Some(home.clone()));
        let mut config = Config::default();
        config
            .agents
            .retain(|agent| agent.role != FlowRole::Builder);
        app.config = ConfigRead::of(ConfigState::Config(config.clone()));
        app.session = Some(config);
        app.on_key(press(KeyCode::Char('c')));
        let selected = |app: &mut App, needle: &str| {
            let (rows, paint) = render_app(app, 100, 40, &RuntimeRead::Absent);
            let row = row_of(&rows, needle);
            let column = column_at(&rows, row, needle);
            theme().text.selected_bg.bg == Some(paint[row][column].bg)
        };
        assert!(!selected(&mut app, "single agent"), "a size is painted");
        assert!(
            !selected(&mut app, "one agent per role"),
            "a size is painted"
        );
        let roles = |app: &App| -> Vec<FlowRole> {
            app.session()
                .expect("the session")
                .agents
                .iter()
                .map(|agent| agent.role)
                .collect()
        };
        app.on_key(press(KeyCode::Up));
        app.on_key(press(KeyCode::Enter));
        assert!(app.crew_cursor.is_some(), "enter picked a size after ↑");
        assert_eq!(roles(&app), [FlowRole::Orchestrator, FlowRole::Reviewer]);
        app.on_key(press(KeyCode::Down));
        assert!(selected(&mut app, "orchestrator also reviews"));
        assert!(!selected(&mut app, "single agent"));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_crew_size_selector_sets_the_session_crew_and_the_file_stays() {
        // c opens the selector on the crew in force; enter on the first size
        // takes the builder and the reviewer off the session's crew, and the
        // file is not written.
        let home = scratch("crew-size");
        let mut app = on_orchestrator(Some(home.clone()));
        app.on_key(press(KeyCode::Char('c')));
        assert_eq!(
            app.crew_cursor,
            Some(2),
            "the selector is not on the crew in force"
        );
        app.on_key(press(KeyCode::Up));
        app.on_key(press(KeyCode::Up));
        app.on_key(press(KeyCode::Enter));
        assert!(app.crew_cursor.is_none(), "enter left the selector open");
        let roles: Vec<FlowRole> = app
            .session()
            .expect("the session")
            .agents
            .iter()
            .map(|agent| agent.role)
            .collect();
        assert_eq!(roles, [FlowRole::Orchestrator]);
        assert!(
            !config::config_path(&home).exists(),
            "the crew wrote the file"
        );
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains("AGENTS session"), "{text}");
        assert_eq!(text.matches("harness claude").count(), 1, "{text}");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_crew_size_takes_a_role_back_with_the_values_it_had() {
        // A builder's model picked this session survives leaving the crew and
        // coming back: the crew parks it rather than losing it.
        let home = scratch("crew-park");
        let mut window = on_builder(&home, Field::Model);
        answer_choice_for(
            &mut window,
            1,
            Ok(Catalog::from_lines("claude-sonnet-5-5\nclaude-haiku-5-5\n")),
        );
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(model_of(&window, "builder"), Some("claude-sonnet-5-5"));

        window.on_key(press(KeyCode::Char('c')));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(model_of(&window, "builder"), None, "the builder stayed");
        window.on_key(press(KeyCode::Char('c')));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(model_of(&window, "builder"), Some("claude-sonnet-5-5"));
        assert!(
            !config::config_path(&home).exists(),
            "the crew wrote the file"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_crew_size_selector_is_drawn_at_40_and_at_100_columns() {
        // The selector is a popup over the window: each size shows its roles
        // as icons, empty circles for the roles it leaves out, and its hint.
        for width in [40u16, 100] {
            let home = scratch("crew-picker");
            let mut window = on_orchestrator(Some(home.clone()));
            window.on_key(press(KeyCode::Char('c')));
            let (rows, _) = render_app(&mut window, width, 40, &RuntimeRead::Absent);
            let text = text_of(&rows);
            assert!(text.contains("crew size"), "{width}: {text}");
            for hint in [
                "single agent",
                "orchestrator also reviews",
                "one agent per role",
            ] {
                assert!(text.contains(hint), "{width}: {hint} is not drawn: {text}");
            }
            let one = Icon::RoleOrchestrator.glyph(IconSet::Nerd);
            let absent = Icon::RoleAbsent.glyph(IconSet::Nerd);
            assert!(
                text.contains(&format!("{one}{absent}{absent} single agent")),
                "{width}: {text}"
            );
            let _ = fs::remove_dir_all(&home);
        }
    }

    #[test]
    fn the_session_crew_is_what_s_hands_the_orchestrator_and_its_skill_prints() {
        // The orchestrator's tab carries the session's crew, so its skill
        // prints the crew the session runs, not the file's. The variable is
        // HUGO_CREW, and the tab sets no HUGO_SESSION (ADR 0015).
        let home = scratch("crew-launch");
        let mut window = on_orchestrator(Some(home.clone()));
        window.queued = vec!["TSK29".to_string()];
        window.root = Some(home.clone());
        window.on_key(press(KeyCode::Char('c')));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Enter));
        let request = window.launch_request().expect("a request");
        let value = request
            .env
            .iter()
            .find(|(key, _)| key == "HUGO_CREW")
            .map(|(_, value)| value.clone())
            .expect("the crew travels in HUGO_CREW");
        assert!(
            request.env.iter().all(|(key, _)| key != "HUGO_SESSION"),
            "the tab sets HUGO_SESSION: {:?}",
            request.env
        );
        let crew = config::decode_crew(&value).expect("the crew decodes");
        assert_eq!(crew.agents.len(), 1, "{crew:?}");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn each_crew_size_sets_the_roles_the_sidebar_draws_and_the_roles_s_starts() {
        // Sizes 1, 2 and 3 take the session's crew to their roles. The sidebar
        // draws those roles' heads and no others, and `s` hands the
        // orchestrator a session with the same roles.
        let cases: [(usize, Vec<FlowRole>); 3] = [
            (1, vec![FlowRole::Orchestrator]),
            (2, vec![FlowRole::Orchestrator, FlowRole::Builder]),
            (
                3,
                vec![
                    FlowRole::Orchestrator,
                    FlowRole::Builder,
                    FlowRole::Reviewer,
                ],
            ),
        ];
        for (size, roles) in cases {
            let home = scratch(&format!("crew-table-{size}"));
            let fake = Arc::new(FakeLauncher::default());
            let mut window = on_orchestrator(Some(home.clone())).with_launcher(Some(fake.clone()));
            window.root = Some(PathBuf::from("/work/project"));
            window.hugo_on_path = || Ok(());

            window.on_key(press(KeyCode::Char('c')));
            for _ in CREW_ORDER {
                window.on_key(press(KeyCode::Up));
            }
            for _ in 1..size {
                window.on_key(press(KeyCode::Down));
            }
            window.on_key(press(KeyCode::Enter));
            let session_roles: Vec<FlowRole> = window
                .session()
                .expect("the session")
                .agents
                .iter()
                .map(|agent| agent.role)
                .collect();
            assert_eq!(session_roles, roles, "size {size}: the session's crew");

            let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
            let text = text_of(&rows);
            for role in CREW_ORDER {
                let head = format!("{} {}", role_icon(role).glyph(IconSet::Nerd), role.as_str());
                assert_eq!(
                    text.contains(&head),
                    roles.contains(&role),
                    "size {size}: {head:?} is drawn wrongly: {text}"
                );
            }

            // Rendering clears the queue when the runtime is absent, so the
            // queue is set after it.
            window.queued = vec!["TSK29".to_string()];
            window.on_key(press(KeyCode::Char('s')));
            wait_for_start(&mut window);
            let requests = fake.requests.lock().expect("the requests");
            assert_eq!(requests.len(), 1, "size {size}: the start count");
            let value = requests[0]
                .env
                .iter()
                .find(|(key, _)| key == config::CREW_ENV)
                .map(|(_, value)| value.clone())
                .expect("the crew travels in the environment");
            assert!(
                requests[0].env.iter().all(|(key, _)| key != "HUGO_SESSION"),
                "size {size}: the tab sets HUGO_SESSION: {:?}",
                requests[0].env
            );
            let started = config::decode_crew(&value).expect("the crew decodes");
            let started_roles: Vec<FlowRole> =
                started.agents.iter().map(|agent| agent.role).collect();
            assert_eq!(started_roles, roles, "size {size}: the roles s starts");
            let _ = fs::remove_dir_all(&home);
        }
    }

    #[test]
    fn no_pick_writes_a_configuration_that_is_there() {
        // A human's configuration stays byte for byte through every pick: a
        // harness, a model, an effort and the crew size. Each pick is made
        // and checked on the session before the file is compared.
        let home = scratch("picks-keep-file");
        let path = config::config_path(&home);
        fs::create_dir_all(config::home_at(&home)).expect("the home");
        let mine = Config {
            round_limit: 3,
            ..Config::default()
        };
        mine.write(&path).expect("the human's configuration");
        let before = fs::read_to_string(&path).expect("the file");
        let tool = crate::catalog::scratch_tool("picks-keep-file", "#!/bin/sh\necho p/m0\n");
        let mut window = on_builder(&home, Field::Harness);
        window.config = config::load_at(&home);
        window.session = Some(mine);
        window.present = Some(all_but_cursor_agent);
        window.catalog_tool = Some(tool.clone());

        // A harness pick: the builder moves from claude to opencode.
        window.on_key(press(KeyCode::Enter));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        let harness_of_builder = window
            .session()
            .and_then(|config| {
                config
                    .agents
                    .iter()
                    .find(|agent| agent.role == FlowRole::Builder)
            })
            .map(|agent| agent.harness.clone());
        assert_eq!(
            harness_of_builder.as_deref(),
            Some("opencode"),
            "the harness pick did not land"
        );

        // A model pick: the builder runs sonnet. Its list is answered here,
        // as a catalogue read would answer it.
        window.cursor = sidebar::cursor_of(1, Field::Model);
        answer_choice_for(
            &mut window,
            1,
            Ok(Catalog::from_lines("claude-sonnet-5-5\nclaude-haiku-5-5\n")),
        );
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            model_of(&window, "builder"),
            Some("claude-sonnet-5-5"),
            "the model pick did not land"
        );

        // An effort pick: the orchestrator runs at medium, one level up. Its
        // levels come from the table, not from the tool that stood in for the
        // model list.
        window.catalog_tool = None;
        window.cursor = sidebar::cursor_of(0, Field::Effort);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        let effort_of_orchestrator = window
            .session()
            .and_then(|config| config.agents.first())
            .and_then(|agent| agent.effort.clone());
        assert_eq!(
            effort_of_orchestrator.as_deref(),
            Some("high"),
            "the effort pick did not land"
        );

        // A crew pick: one size down, to the orchestrator and the builder.
        window.on_key(press(KeyCode::Char('c')));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            window.session().expect("the session").agents.len(),
            2,
            "the crew pick did not land"
        );
        assert_eq!(
            window.session().expect("the session").round_limit,
            3,
            "the session lost the file's round limit"
        );

        assert_eq!(
            fs::read_to_string(&path).expect("the file"),
            before,
            "a pick wrote the configuration"
        );
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn the_footer_and_help_name_the_crew_size_key_and_its_selector_keys() {
        let mut app = on_orchestrator(None);
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(text_of(&rows).contains("c crew size"), "{}", text_of(&rows));
        app.on_key(press(KeyCode::Char('c')));
        let footer: String = app
            .footer_lines(100, 1)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            footer.contains("esc") && footer.contains("pick"),
            "{footer}"
        );
        assert!(!footer.contains("reload"), "{footer}");
        let help: String = app
            .help_lines(60)
            .into_iter()
            .flat_map(|(_, line)| line.spans)
            .map(|span| span.content.into_owned())
            .collect();
        assert!(help.contains("move") && help.contains("pick"), "{help}");
    }
    #[test]
    fn enter_opens_a_choice_on_every_agents_rows() {
        // Every agent is configured from hugo: each of its three rows opens
        // the choice of that field.
        let mut app = app();
        app.agents_drawn = true;
        for agent in 0..3 {
            for field in [Field::Harness, Field::Model, Field::Effort] {
                app.cursor = sidebar::cursor_of(agent, field);
                app.on_key(press(KeyCode::Enter));
                let choice = app.choice.take().expect("enter opened no choice");
                assert_eq!(choice.field, field, "agent {agent}");
                assert_eq!(choice.agent, agent, "agent {agent}");
            }
        }
    }

    /// Wait for a start to answer the way the loop does: by polling it, never
    /// by waiting on it.
    fn wait_for_start(window: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while window.start_pending() {
            assert!(
                std::time::Instant::now() < deadline,
                "the start never answered"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
            window.poll_start();
        }
    }

    /// A window inside herdr: the launcher is the fake, and the project is
    /// at a root of its own.
    fn in_herdr(fake: &Arc<FakeLauncher>) -> App {
        let mut window = app().with_launcher(Some(fake.clone()));
        window.root = Some(PathBuf::from("/work/project"));
        window.hugo_on_path = || Ok(());
        window
    }

    fn hugo_not_on_path() -> Result<(), String> {
        Err(format!("{}: add /opt/bin to PATH", on_path::MISSING))
    }

    #[test]
    fn s_refuses_when_hugo_is_not_on_path_and_starts_nothing() {
        // The agents run `hugo --skill`: with no hugo on PATH the orchestrator
        // would have no skill to run, so the key names the folder and stops.
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        window.hugo_on_path = hugo_not_on_path;
        let runtime = RuntimeRead::Record(runtime());
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(!rows.last().expect("a footer row").contains("start"));
        window.on_key(press(KeyCode::Char('s')));
        assert!(!window.start_pending());
        assert_eq!(
            window.note,
            Some(Note::Failed(format!(
                "{START_FAILED}: {}: add /opt/bin to PATH",
                on_path::MISSING
            )))
        );
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    #[test]
    fn s_refuses_when_another_hugo_comes_first_on_path() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        window.hugo_on_path = || Err(format!("{}: put /opt/bin ahead of it", on_path::SHADOWED));
        let runtime = RuntimeRead::Record(runtime());
        let _ = render_app(&mut window, 100, 40, &runtime);
        window.on_key(press(KeyCode::Char('s')));
        assert!(!window.start_pending());
        let Some(Note::Failed(text)) = &window.note else {
            panic!("a failed start: {:?}", window.note);
        };
        assert!(text.contains(on_path::SHADOWED), "{text}");
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    #[test]
    fn s_outside_herdr_says_why_and_starts_nothing() {
        // Outside herdr there is no launcher: the key is a notice, and the
        // footer does not name a key that cannot act.
        let mut window = app();
        window.root = Some(PathBuf::from("/work/project"));
        let runtime = RuntimeRead::Record(runtime());
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(!rows.last().expect("a footer row").contains("start"));
        window.on_key(press(KeyCode::Char('s')));
        assert!(!window.start_pending());
        assert_eq!(
            window.note,
            Some(Note::Notice(START_NEEDS_HERDR.to_string()))
        );
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            text_of(&rows).contains(START_NEEDS_HERDR),
            "{}",
            text_of(&rows)
        );
        // The help lists the key all the same: it is the window's key.
        let help: String = window
            .help_lines(60)
            .into_iter()
            .map(|(_, line)| {
                line.spans
                    .iter()
                    .map(|span| span.content.to_string())
                    .collect::<String>()
            })
            .collect();
        assert!(help.contains("start the orchestrator"), "{help}");
    }

    #[test]
    fn s_with_an_empty_queue_starts_nothing() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        let mut record = runtime();
        record.queue.clear();
        let runtime = RuntimeRead::Record(record);
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(!rows.last().expect("a footer row").contains("start"));
        window.on_key(press(KeyCode::Char('s')));
        assert!(!window.start_pending());
        assert_eq!(window.note, Some(Note::Notice(QUEUE_EMPTY.to_string())));
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    #[test]
    fn s_starts_the_orchestrator_over_the_queue_and_names_the_tab() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        let runtime = RuntimeRead::Record(runtime());
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(rows.last().expect("a footer row").contains("start"));
        window.on_key(press(KeyCode::Char('s')));
        assert!(window.start_pending());
        assert_eq!(window.note, Some(Note::Notice(STARTING.to_string())));
        // While the start runs the key starts nothing more, and the footer
        // stops naming it. The key still says the start is running.
        window.note = None;
        window.on_key(press(KeyCode::Char('s')));
        assert_eq!(window.note, Some(Note::Notice(STARTING.to_string())));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(!rows.last().expect("a footer row").contains("start"));
        wait_for_start(&mut window);
        assert_eq!(
            window.note,
            Some(Note::Done("orchestrator started in tab w1:t3".to_string()))
        );
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            text_of(&rows).contains("orchestrator started in tab w1:t3"),
            "{}",
            text_of(&rows)
        );
        let requests = fake.requests.lock().expect("the requests");
        assert_eq!(
            requests.len(),
            1,
            "a second key started a second orchestrator"
        );
        assert_eq!(requests[0].kind, "claude");
        assert_eq!(
            requests[0].args,
            [
                "--model",
                "claude-opus-5-5",
                "--effort",
                "medium",
                "--permission-mode",
                "auto"
            ]
        );
        // The only environment the start carries is the session's crew.
        assert!(
            requests[0]
                .env
                .iter()
                .all(|(key, _)| key == config::CREW_ENV),
            "{:?}",
            requests[0].env
        );
        assert!(
            requests[0].env.iter().any(|(key, _)| key == "HUGO_CREW"),
            "{:?}",
            requests[0].env
        );
        assert_eq!(requests[0].project, PathBuf::from("/work/project"));
        assert_eq!(requests[0].tasks, ["TSK29"]);
    }

    #[test]
    fn a_start_that_fails_is_named_on_the_failure_slot() {
        let fake = Arc::new(FakeLauncher {
            error: Some("herdr agent start: no pane".to_string()),
            ..Default::default()
        });
        let mut window = in_herdr(&fake);
        let runtime = RuntimeRead::Record(runtime());
        render_app(&mut window, 100, 40, &runtime);
        window.on_key(press(KeyCode::Char('s')));
        wait_for_start(&mut window);
        let (rows, paint) = render_app(&mut window, 100, 40, &runtime);
        let text = text_of(&rows);
        assert!(
            text.contains("the orchestrator did not start: herdr agent start: no pane"),
            "{text}"
        );
        assert_eq!(
            colour_of(&rows, &paint, "did not start"),
            theme().semantic.error.fg.expect("the error colour")
        );
    }

    #[test]
    fn an_orchestrator_herdr_already_names_is_a_notice_and_no_second_start() {
        let fake = Arc::new(FakeLauncher {
            named: true,
            ..Default::default()
        });
        let mut window = in_herdr(&fake);
        render_app(&mut window, 100, 40, &RuntimeRead::Record(runtime()));
        window.on_key(press(KeyCode::Char('s')));
        wait_for_start(&mut window);
        assert_eq!(window.note, Some(Note::Notice(ALREADY_RUNNING.to_string())));
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    /// The runtime with three waiting tasks behind the one in flight and the
    /// one done, in queue order: the queue the picks are tested on.
    fn queued_runtime() -> Runtime {
        let mut runtime = runtime();
        runtime.queue = vec![
            queue_item("TSK22", Status::Done),
            queue_item("TSK29", Status::Approved),
            queue_item("TSK07", Status::Approved),
            queue_item("TSK08", Status::Approved),
        ];
        runtime
    }

    /// The window's text, as its rows read at a width and height.
    fn screen_of(window: &mut App, runtime: &Runtime) -> Vec<String> {
        render_app(window, 100, 40, &RuntimeRead::Record(runtime.clone())).0
    }

    #[test]
    fn every_block_opens_with_a_dim_caption_that_says_what_it_counts() {
        let runtime = runtime();
        let (rows, paint) = render_runtime(100, 40, &runtime);
        for (title, caption) in [
            ("crew ─", CREW_CAPTION),
            ("queue ─", QUEUE_CAPTION),
            ("round ─", ROUND_CAPTION),
            ("budget ─", BUDGET_CAPTION),
            ("findings ─", FINDINGS_CAPTION),
        ] {
            let title_row = row_of(&rows, title);
            assert_eq!(
                row_of(&rows, caption),
                title_row + 1,
                "{caption} does not open its block: {}",
                rows.join("\n")
            );
            assert_eq!(
                colour_of(&rows, &paint, caption),
                theme().text.dim.fg.expect("the dim colour"),
                "{caption} is not dim"
            );
        }
    }

    #[test]
    fn the_round_is_a_timeline_of_the_task_in_flight() {
        let runtime = runtime();
        let done = Icon::StatusDone.glyph(IconSet::Nerd);
        let spinner = coding_crew_theme::chrome::spinner_frame(0);
        // At 100 columns the timeline's second round does not share the line
        // with the first: it leads the line under it.
        let (rows, _) = render_runtime(100, 40, &runtime);
        let first = row_of(&rows, "TSK13 · external");
        assert!(
            rows[first].contains(&format!("round 1 {done} build · {done} review 3 findings")),
            "{}",
            rows[first]
        );
        assert!(
            rows[first + 1].contains(&format!("› round 2 {done} build · {spinner} review")),
            "{}",
            rows[first + 1]
        );
        // At 40 columns each round has a line of its own, the head too.
        let (rows, _) = render_runtime(40, 40, &runtime);
        let head = row_of(&rows, "TSK13 · external");
        assert!(rows[head + 1].contains("round 1"), "{}", rows[head + 1]);
        assert!(rows[head + 2].contains("› round 2"), "{}", rows[head + 2]);
    }

    #[test]
    fn a_review_that_waits_is_drawn_open_and_a_first_round_has_no_arrow() {
        let mut runtime = runtime();
        runtime.crew[2].state = AgentState::Idle;
        if let Some(round) = runtime.round.as_mut() {
            round.earlier.clear();
        }
        let open = Icon::TaskOpen.glyph(IconSet::Nerd);
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert!(
            text.contains(&format!(
                "round 2 {}",
                Icon::StatusDone.glyph(IconSet::Nerd)
            )),
            "{text}"
        );
        assert!(text.contains(&format!("{open} review")), "{text}");
        assert!(
            !text.contains("›"),
            "a first round leads with the arrow: {text}"
        );
    }

    #[test]
    fn tab_moves_the_keys_to_the_queue_and_the_cursor_moves_over_the_waiting_tasks() {
        let mut window = app();
        let runtime = RuntimeRead::Record(queued_runtime());
        let rows = screen_of(&mut window, &queued_runtime());
        assert!(
            rows.iter().any(|row| row.contains("▌  harness")),
            "{rows:?}"
        );
        assert!(rows.last().expect("a footer").contains("tab queue"));
        window.on_key(press(KeyCode::Tab));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        let text = text_of(&rows);
        // The sidebar's cursor is not drawn while the queue has the keys.
        assert!(!rows.iter().any(|row| row.contains("▌  harness")), "{text}");
        assert!(
            rows[row_of(&rows, "TSK29")].contains("▌[ ] TSK29"),
            "{text}"
        );
        let footer = rows.last().expect("a footer");
        for key in ["space mark", "a mark all", "tab configuration"] {
            assert!(footer.contains(key), "{footer}");
        }
        assert!(!footer.contains("reload"), "{footer}");
        assert!(!footer.contains("choose"), "{footer}");
        // The cursor stops at the last waiting task.
        for _ in 0..5 {
            window.on_key(press(KeyCode::Down));
        }
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            rows[row_of(&rows, "TSK08")].contains("▌[ ] TSK08"),
            "{}",
            text_of(&rows)
        );
        // The configuration's keys do nothing while the queue has the keys.
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none());
        window.on_key(press(KeyCode::Char('c')));
        assert!(window.crew_cursor.is_none());
        // tab gives the keys back.
        window.on_key(press(KeyCode::Tab));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            rows.iter().any(|row| row.contains("▌  harness")),
            "{}",
            text_of(&rows)
        );
    }

    #[test]
    fn space_marks_the_task_under_the_cursor_and_a_marks_or_unmarks_them_all() {
        let mut window = app();
        let runtime = RuntimeRead::Record(queued_runtime());
        screen_of(&mut window, &queued_runtime());
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Char(' ')));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            rows[row_of(&rows, "TSK07")].contains("[x] TSK07"),
            "{}",
            text_of(&rows)
        );
        assert!(
            rows[row_of(&rows, "TSK29")].contains("[ ] TSK29"),
            "{}",
            text_of(&rows)
        );
        // space again unmarks it.
        window.on_key(press(KeyCode::Char(' ')));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            rows[row_of(&rows, "TSK07")].contains("[ ] TSK07"),
            "{}",
            text_of(&rows)
        );
        // a marks every waiting task; a again unmarks them all.
        window.on_key(press(KeyCode::Char('a')));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        for id in ["TSK29", "TSK07", "TSK08"] {
            assert!(
                rows[row_of(&rows, id)].contains(&format!("[x] {id}")),
                "{}",
                text_of(&rows)
            );
        }
        assert!(
            !text_of(&rows).contains("[x] TSK22"),
            "a done task is not markable"
        );
        window.on_key(press(KeyCode::Char('a')));
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(!text_of(&rows).contains("[x]"), "{}", text_of(&rows));
    }

    #[test]
    fn the_marks_are_the_session_s_and_a_reload_drops_them() {
        let mut window = app();
        screen_of(&mut window, &queued_runtime());
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Char(' ')));
        assert_eq!(window.marked, ["TSK29"]);
        // The configuration reloads with r, and the marks go with the session.
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Char('r')));
        assert!(window.marked.is_empty());
    }

    #[test]
    fn the_queue_footer_names_its_keys_and_the_configuration_names_the_queue_only_with_work() {
        let mut window = app();
        let mut empty = queued_runtime();
        empty.queue.clear();
        let rows = screen_of(&mut window, &empty);
        assert!(!rows.last().expect("a footer").contains("tab queue"));
        window.on_key(press(KeyCode::Tab));
        assert_eq!(
            window.focus,
            Focus::Configuration,
            "a queue with no task takes no focus"
        );

        let rows = screen_of(&mut window, &queued_runtime());
        assert!(rows.last().expect("a footer").contains("tab queue"));
    }

    #[test]
    fn the_queue_count_names_the_waiting_tasks_and_the_done_ones() {
        let mut window = app();
        let rows = screen_of(&mut window, &queued_runtime());
        let text = text_of(&rows);
        assert!(
            text.contains("the automation's queue · 3 waiting · 1 done"),
            "{text}"
        );
        // At 40 columns the count is cut, and the cut says so.
        let (rows, _) = render_runtime(40, 40, &queued_runtime());
        let count = rows[row_of(&rows, "automation's queue")].clone();
        assert!(count.contains("…"), "{count}");
    }

    /// A window in herdr whose queue is the three waiting tasks of
    /// [`queued_runtime`], with the fake launcher standing in for herdr.
    fn picking_window(fake: &Arc<FakeLauncher>) -> App {
        let mut window = in_herdr(fake);
        screen_of(&mut window, &queued_runtime());
        window
    }

    /// The tasks the fake launcher was asked to start, once a start is done.
    fn started_tasks(window: &mut App, fake: &FakeLauncher) -> Vec<String> {
        window.on_key(press(KeyCode::Char('s')));
        wait_for_start(window);
        let requests = fake.requests.lock().expect("the requests");
        assert_eq!(requests.len(), 1, "one start was asked: {requests:?}");
        requests[0].tasks.clone()
    }

    #[test]
    fn with_no_mark_s_starts_the_whole_queue_in_queue_order() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = picking_window(&fake);
        assert_eq!(
            started_tasks(&mut window, &fake),
            ["TSK29", "TSK07", "TSK08"]
        );
    }

    #[test]
    fn marks_drive_what_s_starts_and_keep_the_queue_order() {
        // One marked task: that one.
        let fake = Arc::new(FakeLauncher::default());
        let mut window = picking_window(&fake);
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Char(' ')));
        assert_eq!(started_tasks(&mut window, &fake), ["TSK07"]);

        // Marked out of order: started in queue order, not mark order.
        let fake = Arc::new(FakeLauncher::default());
        let mut window = picking_window(&fake);
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Char(' ')));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Up));
        window.on_key(press(KeyCode::Char(' ')));
        assert_eq!(started_tasks(&mut window, &fake), ["TSK29", "TSK08"]);

        // a marks them all, a again unmarks them: the whole queue again.
        let fake = Arc::new(FakeLauncher::default());
        let mut window = picking_window(&fake);
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Char('a')));
        window.on_key(press(KeyCode::Char('a')));
        assert_eq!(
            started_tasks(&mut window, &fake),
            ["TSK29", "TSK07", "TSK08"]
        );
    }

    #[test]
    fn a_blocked_agent_is_drawn_on_the_warning_slot_under_its_icon() {
        // The mock has no blocked agent; this fixture holds one, the builder.
        let mut record = runtime();
        record.crew[1].state = AgentState::Blocked;
        let (rows, paint) = render_runtime(100, 40, &record);
        let text = rows.join("\n");
        let glyph = Icon::Blocked.glyph(IconSet::Nerd);
        assert!(text.contains(&format!("{glyph} blocked")), "{text}");
        let builder = rows
            .iter()
            .find(|row| row.contains("deepseek"))
            .expect("the builder");
        assert!(!builder.contains("executing"), "{builder}");
        assert_eq!(
            colour_of(&rows, &paint, "blocked"),
            theme().semantic.warn.fg.expect("the warn colour")
        );
    }

    #[test]
    fn a_blocked_agent_is_drawn_under_the_unicode_icon_too() {
        let mut record = runtime();
        record.crew[1].state = AgentState::Blocked;
        let mut app = App::new(
            theme(),
            IconSet::Unicode,
            ConfigRead::of(ConfigState::Config(Config::default())),
            None,
        );
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Record(record));
        let text = rows.join("\n");
        assert!(text.contains("⊘ blocked"), "{text}");
    }

    #[test]
    fn a_waiting_task_the_record_does_not_call_claimable_is_drawn_and_not_picked() {
        // TSK07 waits on work not yet done: it is drawn, dim and unmarked, and
        // the cursor and the marks pass over it.
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        let mut record = runtime();
        record.queue = vec![
            QueueItem {
                id: "TSK07".to_string(),
                status: Status::Approved,
                claimable: false,
            },
            queue_item("TSK29", Status::Approved),
        ];
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Record(record.clone()));
        let text = rows.join("\n");
        let row = rows
            .iter()
            .find(|row| row.contains("TSK07"))
            .expect("the row");
        assert!(row.contains("not claimable"), "{text}");
        assert!(!row.contains("[ ]") && !row.contains("[x]"), "{row}");
        window.on_key(press(KeyCode::Tab));
        // The queue's help says what `a` marks: the claimable tasks only.
        window.on_key(press(KeyCode::Char('?')));
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Record(record.clone()));
        let text = text_of(&rows);
        assert!(text.contains("mark every claimable task"), "{text}");
        assert!(!text.contains("every waiting task"), "{text}");
        window.on_key(press(KeyCode::Char('x')));
        assert!(!window.help, "a key left the help open");
        window.on_key(press(KeyCode::Char('a')));
        assert_eq!(started_tasks(&mut window, &fake), ["TSK29"]);
    }

    #[test]
    fn a_queue_with_no_claimable_task_starts_nothing_and_says_why() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        let mut record = runtime();
        record.queue = vec![QueueItem {
            id: "TSK07".to_string(),
            status: Status::Approved,
            claimable: false,
        }];
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Record(record));
        // The queue shows the waiting task, but the keys cannot reach it.
        assert!(rows.iter().any(|row| row.contains("TSK07")), "{rows:#?}");
        assert!(!rows.last().expect("a footer row").contains("tab queue"));
        window.on_key(press(KeyCode::Char('s')));
        assert!(!window.start_pending());
        assert_eq!(window.note, Some(Note::Notice(NONE_CLAIMABLE.to_string())));
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    #[test]
    fn s_works_from_the_queue_and_from_the_configuration_alike() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = picking_window(&fake);
        window.on_key(press(KeyCode::Tab));
        window.on_key(press(KeyCode::Char('s')));
        wait_for_start(&mut window);
        let requests = fake.requests.lock().expect("the requests");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].tasks, ["TSK29", "TSK07", "TSK08"]);
    }

    /// The harnesses the test machine has: every one but cursor-agent.
    fn all_but_cursor_agent(harness: &Harness) -> bool {
        harness.name != "cursor-agent"
    }

    /// The entries of the open choice, and whether enter picks each.
    fn entries_of(window: &App) -> Vec<(String, bool)> {
        window
            .choice
            .as_ref()
            .expect("the choice")
            .entries
            .iter()
            .map(|entry| (entry.text.clone(), entry.pickable))
            .collect()
    }

    /// The window on the builder's row of the given field, with a home.
    fn on_builder(home: &Path, field: Field) -> App {
        let mut window = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Config(Config::default())),
            Some(home.to_path_buf()),
        );
        window.agents_drawn = true;
        window.cursor = sidebar::cursor_of(1, field);
        window
    }

    #[test]
    fn the_harness_choice_lists_the_table_and_a_missing_harness_cannot_be_picked() {
        let home = scratch("harness-choice");
        let mut window = on_builder(&home, Field::Harness);
        window.present = Some(all_but_cursor_agent);
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            entries_of(&window),
            [
                ("claude".to_string(), true),
                ("opencode".to_string(), true),
                ("cursor-agent".to_string(), false),
            ]
        );
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("cursor-agent not installed"),
            "{}",
            text_of(&rows)
        );
        // On the missing harness the cursor is on a row enter does not pick:
        // the footer does not name it, and enter saves nothing.
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Down));
        let footer: String = window
            .footer_lines(100, 1)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(!footer.contains("pick"), "{footer}");
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_some(), "enter closed the choice");
        assert!(
            !config::config_path(&home).exists(),
            "a missing harness was picked"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn picking_a_harness_clears_a_model_the_new_harness_does_not_list() {
        // The builder runs claude-haiku-5-5 at high. opencode does not list
        // that model: the switch clears it, keeps high, and says so on the row.
        let home = scratch("switch-clears");
        let tool = crate::catalog::scratch_tool("switch-clears", "#!/bin/sh\necho p/m0\n");
        let mut window = on_builder(&home, Field::Harness);
        window.present = Some(all_but_cursor_agent);
        window.catalog_tool = Some(tool.clone());
        window.on_key(press(KeyCode::Enter));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        assert!(window.catalog_pending(), "the switch did not read its list");
        wait_for_catalog(&mut window);
        assert!(window.choice.is_none(), "the switch left its choice open");
        let builder = window
            .session()
            .and_then(|config| {
                config
                    .agents
                    .iter()
                    .find(|agent| agent.role == FlowRole::Builder)
            })
            .expect("the builder");
        assert_eq!(builder.harness, "opencode");
        assert_eq!(builder.model, "");
        // An effort is a level of a model: with the model cleared, it goes too.
        assert_eq!(builder.effort, None);
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("pick a model"),
            "{}",
            text_of(&rows)
        );
        assert!(
            !config::config_path(&home).exists(),
            "the switch wrote the configuration"
        );
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_harness_switch_keeps_a_model_the_new_harness_lists_and_clears_an_effort_it_does_not_take()
    {
        // The builder runs at xhigh, which claude takes and opencode does not:
        // the switch resets the effort to opencode's default level and keeps
        // the model, which opencode lists.
        let home = scratch("switch-keeps");
        let tool = crate::catalog::scratch_tool(
            "switch-keeps",
            "#!/bin/sh\necho claude-haiku-5-5\necho p/m0\n",
        );
        let mut window = on_builder(&home, Field::Harness);
        window.present = Some(all_but_cursor_agent);
        window.catalog_tool = Some(tool.clone());
        let file = {
            let mut config = Config::default();
            config.agents[1].effort = Some("xhigh".to_string());
            config
        };
        window.config = ConfigRead::of(ConfigState::Config(file.clone()));
        window.session = Some(file);
        window.on_key(press(KeyCode::Enter));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        let builder = window
            .session()
            .and_then(|config| {
                config
                    .agents
                    .iter()
                    .find(|agent| agent.role == FlowRole::Builder)
            })
            .expect("the builder");
        assert_eq!(builder.harness, "opencode");
        assert_eq!(builder.model, "claude-haiku-5-5");
        // The listing gives the model no levels, so the effort does not carry.
        assert_eq!(builder.effort, None);
        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_harness_switch_whose_catalogue_cannot_be_read_saves_nothing() {
        // The switch cannot tell what the model keeps: it is refused, the
        // reason is named in the choice, and the configuration stays.
        let home = scratch("switch-refused");
        let mut window = on_builder(&home, Field::Harness);
        window.present = Some(all_but_cursor_agent);
        window.catalog_tool = Some(PathBuf::from("/nonexistent/coding-crew-tool"));
        window.on_key(press(KeyCode::Enter));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains(CATALOG_UNREADABLE),
            "{}",
            text_of(&rows)
        );
        assert!(
            window.choice.is_some(),
            "the refused switch closed the choice"
        );
        assert!(
            !config::config_path(&home).exists(),
            "a refused switch was saved"
        );
        assert_eq!(
            window.session().expect("the configuration").agents[1].harness,
            "claude"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_model_choice_lists_the_catalogue_and_type_an_id_last() {
        let home = scratch("model-choice-type");
        let mut window = on_builder(&home, Field::Model);
        answer_choice_for(&mut window, 1, Ok(Catalog::from_lines("p/m0\np/m1\n")));
        assert_eq!(
            entries_of(&window),
            [
                ("p/m0".to_string(), true),
                ("p/m1".to_string(), true),
                (pick::TYPE_AN_ID.to_string(), true),
            ]
        );
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains(pick::TYPE_AN_ID),
            "{}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn type_an_id_takes_its_letters_as_text_and_saves_a_shaped_id() {
        // The input takes every key but ctrl-c: a `q` typed into an id is a
        // letter, not the window's quit.
        let home = scratch("type-an-id");
        let mut window = on_builder(&home, Field::Model);
        answer_choice_for(&mut window, 1, Ok(Catalog::from_lines("p/m0\n")));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            window.choice.as_ref().expect("the choice").state,
            ChoiceState::Typing(String::new())
        );
        for letter in "opencode-go/qwen".chars() {
            assert_eq!(
                window.on_key(press(KeyCode::Char(letter))),
                Action::Redraw,
                "{letter:?} quit the window"
            );
        }
        // `?` is a letter too, so help does not open over the input.
        window.on_key(press(KeyCode::Char('?')));
        assert!(!window.help, "? opened the help over the id input");
        window.on_key(press(KeyCode::Backspace));
        assert_eq!(
            window.choice.as_ref().expect("the choice").state,
            ChoiceState::Typing("opencode-go/qwen".to_string())
        );
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none(), "enter did not pick the typed id");
        assert_eq!(model_of(&window, "builder"), Some("opencode-go/qwen"));
        assert!(
            !config::config_path(&home).exists(),
            "the id was written to the configuration"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn an_id_that_is_not_shaped_as_a_model_id_is_not_picked() {
        let home = scratch("type-an-id-shape");
        let mut window = on_builder(&home, Field::Model);
        answer_choice_for(&mut window, 1, Ok(Catalog::from_lines("p/m0\n")));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        for letter in "glm 5.3#high".chars() {
            window.on_key(press(KeyCode::Char(letter)));
        }
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains(NOT_A_MODEL_ID),
            "{}",
            text_of(&rows)
        );
        window.on_key(press(KeyCode::Enter));
        assert!(
            window.choice.is_some(),
            "enter picked an id that is not one"
        );
        assert!(!config::config_path(&home).exists(), "a bad id was saved");
        // Esc goes back to the list, and a second esc closes the choice.
        window.on_key(press(KeyCode::Esc));
        assert_eq!(
            window.choice.as_ref().expect("the list").state,
            ChoiceState::Listed
        );
        window.on_key(press(KeyCode::Esc));
        assert!(window.choice.is_none());
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_model_catalogue_that_cannot_be_read_still_lets_an_id_be_typed() {
        // The reason is named above the list, and `type an id…` is the way past
        // a catalogue that does not work.
        let home = scratch("model-choice-failed");
        let mut window = on_builder(&home, Field::Model);
        answer_choice_for(&mut window, 1, Err("opencode timed out".to_string()));
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains(CATALOG_UNREADABLE),
            "{}",
            text_of(&rows)
        );
        assert!(
            text_of(&rows).contains(pick::TYPE_AN_ID),
            "{}",
            text_of(&rows)
        );
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            window.choice.as_ref().expect("the choice").state,
            ChoiceState::Typing(String::new())
        );
        for letter in "opencode-go/glm-5.3".chars() {
            window.on_key(press(KeyCode::Char(letter)));
        }
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none(), "enter did not pick the typed id");
        assert_eq!(model_of(&window, "builder"), Some("opencode-go/glm-5.3"));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn esc_from_the_id_input_goes_back_to_the_failure_it_opened_from() {
        // The reason the catalogue failed stays on the choice when the id
        // input is opened and closed again.
        let home = scratch("model-choice-esc");
        let mut window = on_builder(&home, Field::Model);
        answer_choice_for(&mut window, 1, Err("opencode timed out".to_string()));
        window.on_key(press(KeyCode::Enter));
        assert!(window.typing(), "enter did not open the id input");
        window.on_key(press(KeyCode::Esc));
        assert_eq!(
            window.choice.as_ref().expect("the choice").state,
            ChoiceState::Failed("opencode timed out".to_string())
        );
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(CATALOG_UNREADABLE), "{text}");
        assert!(text.contains("opencode timed out"), "{text}");
        assert!(text.contains(pick::TYPE_AN_ID), "{text}");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_model_pick_clears_an_effort_the_new_model_does_not_take() {
        // The builder runs high. A model that takes high keeps it; one that
        // lists only low and medium, or none, clears it.
        let home = scratch("model-pick-levels");
        let mut window = on_builder(&home, Field::Model);
        window.on_key(press(KeyCode::Enter));
        let listed = Catalog::from_curated(&[
            coding_crew_core::harness::CuratedModel {
                id: "p/high",
                efforts: &["low", "high"],
            },
            coding_crew_core::harness::CuratedModel {
                id: "p/low",
                efforts: &["low", "medium"],
            },
        ]);
        answer_choice_for(&mut window, 1, Ok(listed.clone()));
        window.on_key(press(KeyCode::Enter));
        let builder = |window: &App| {
            window
                .session()
                .and_then(|config| config.agents.get(1))
                .map(|agent| (agent.model.clone(), agent.effort.clone()))
                .expect("the builder")
        };
        assert_eq!(
            builder(&window),
            ("p/high".to_string(), Some("high".to_string()))
        );

        window.cursor = sidebar::cursor_of(1, Field::Model);
        window.on_key(press(KeyCode::Enter));
        answer_choice_for(&mut window, 1, Ok(listed));
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        assert_eq!(builder(&window), ("p/low".to_string(), None));
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn the_effort_choice_lists_the_model_levels_and_saves_the_one_picked() {
        // The builder runs haiku, which takes three levels, not five.
        let home = scratch("effort-choice");
        let mut window = on_builder(&home, Field::Effort);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        assert_eq!(
            entries_of(&window),
            [
                ("low".to_string(), true),
                ("medium".to_string(), true),
                ("high".to_string(), true),
            ]
        );
        // The builder runs at high: the cursor starts there.
        assert_eq!(window.choice.as_ref().expect("the choice").cursor, 2);
        window.on_key(press(KeyCode::Char('k')));
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none());
        assert_eq!(
            window
                .session()
                .and_then(|config| config
                    .agents
                    .iter()
                    .find(|agent| agent.role == FlowRole::Builder))
                .and_then(|agent| agent.effort.as_deref()),
            Some("medium")
        );
        assert!(
            !config::config_path(&home).exists(),
            "the effort was written to the configuration"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn every_picker_renders_at_40_and_at_100_columns() {
        // The choice is a popup over the window: at the narrow design width
        // it keeps its title and its entries, and at 100 it keeps them too.
        for width in [40u16, 100] {
            for field in [Field::Harness, Field::Model, Field::Effort] {
                let home = scratch("picker-width");
                let mut window = on_builder(&home, field);
                window.present = Some(all_but_cursor_agent);
                if field == Field::Model {
                    answer_choice_for(&mut window, 1, Ok(Catalog::from_lines("p/m0\n")));
                } else {
                    window.on_key(press(KeyCode::Enter));
                }
                let (rows, _) = render_app(&mut window, width, 40, &RuntimeRead::Absent);
                let text = text_of(&rows);
                assert!(
                    text.contains(&format!("builder {}", field.label())),
                    "{width} {field:?}: {text}"
                );
                for row in &rows {
                    assert!(
                        text_width(row) <= usize::from(width),
                        "{width} {field:?}: {row:?}"
                    );
                }
                let _ = fs::remove_dir_all(&home);
            }
        }
    }

    #[test]
    fn s_refuses_while_an_agent_has_no_model_and_names_which() {
        let fake = Arc::new(FakeLauncher::default());
        let mut window = in_herdr(&fake);
        let file = {
            let mut config = Config::default();
            config.agents[1].model.clear();
            config.agents[2].effort = None;
            config
        };
        window.config = ConfigRead::of(ConfigState::Config(file.clone()));
        window.session = Some(file);
        let runtime = RuntimeRead::Record(runtime());
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            !rows.last().expect("a footer row").contains("start"),
            "the footer offers a start that cannot act"
        );
        window.on_key(press(KeyCode::Char('s')));
        assert!(!window.start_pending(), "s started with a gap");
        assert_eq!(
            window.note,
            Some(Note::Failed(format!(
                "{START_FAILED}: builder has no model, reviewer has no effort"
            )))
        );
        let (rows, _) = render_app(&mut window, 100, 40, &runtime);
        assert!(
            text_of(&rows).contains("builder has no model"),
            "{}",
            text_of(&rows)
        );
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    /// A window on the builder's effort, with the builder on a model the
    /// catalogue does not list and no effort in force.
    fn on_unlisted_builder_effort(home: &Path) -> App {
        let mut config = Config::default();
        config.agents[1].model = "claude-new-9".to_string();
        config.agents[1].effort = None;
        let mut window = on_builder(home, Field::Effort);
        window.config = ConfigRead::of(ConfigState::Config(config.clone()));
        window.session = Some(config);
        window
    }

    #[test]
    fn an_unlisted_model_is_offered_the_harness_levels_none_and_a_typed_level() {
        // The builder's model is not in the catalogue, so its levels are not
        // known: the list is the harness's levels, then `none`, then the input.
        let home = scratch("effort-unlisted");
        let mut window = on_unlisted_builder_effort(&home);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        assert_eq!(
            entries_of(&window),
            [
                ("low".to_string(), true),
                ("medium".to_string(), true),
                ("high".to_string(), true),
                ("xhigh".to_string(), true),
                ("max".to_string(), true),
                ("none".to_string(), true),
                (pick::TYPE_A_LEVEL.to_string(), true),
            ]
        );
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            !text_of(&rows).contains("does not list"),
            "{}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn none_on_an_unlisted_model_completes_its_effort_and_says_none() {
        let home = scratch("effort-unlisted-none");
        let mut window = on_unlisted_builder_effort(&home);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        for _ in 0..5 {
            window.on_key(press(KeyCode::Down));
        }
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none(), "enter did not pick none");
        let builder = window
            .session()
            .and_then(|config| config.agents.get(1))
            .expect("the builder");
        assert_eq!(builder.effort.as_deref(), Some(""));
        // The effort is set, so the flow has no gap on it.
        assert!(pick::gaps(window.session().expect("the session"), &window.levels).is_empty());
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("effort  none"),
            "{}",
            text_of(&rows)
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn type_a_level_takes_a_typed_level_for_an_unlisted_model_and_keeps_its_shape() {
        let home = scratch("effort-typed-level");
        let mut window = on_unlisted_builder_effort(&home);
        window.on_key(press(KeyCode::Enter));
        wait_for_catalog(&mut window);
        for _ in 0..6 {
            window.on_key(press(KeyCode::Down));
        }
        window.on_key(press(KeyCode::Enter));
        assert_eq!(
            window.choice.as_ref().expect("the choice").state,
            ChoiceState::Typing(String::new())
        );
        for letter in "very high".chars() {
            window.on_key(press(KeyCode::Char(letter)));
        }
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_some(), "a level of two words was taken");
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        assert!(text_of(&rows).contains(NOT_A_LEVEL), "{}", text_of(&rows));
        for _ in 0.."very high".len() {
            window.on_key(press(KeyCode::Backspace));
        }
        for letter in "ultra".chars() {
            window.on_key(press(KeyCode::Char(letter)));
        }
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none(), "enter did not take the level");
        assert_eq!(
            window
                .session()
                .and_then(|config| config.agents.get(1))
                .and_then(|agent| agent.effort.clone()),
            Some("ultra".to_string())
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_listed_model_that_also_runs_bare_offers_none_after_its_levels() {
        // gpt-5.3-codex is listed bare and at its levels. Once a level is
        // picked the list still offers `none`, and picking it stores no level,
        // so the bare start is reachable without `r`.
        let home = scratch("effort-bare-listed");
        let mut window = on_unlisted_builder_effort(&home);
        let mut config = window.session().cloned().expect("the session");
        config.agents[1].harness = "cursor-agent".to_string();
        config.agents[1].model = "gpt-5.3-codex".to_string();
        config.agents[1].effort = Some("high".to_string());
        window.config = ConfigRead::of(ConfigState::Config(config.clone()));
        window.session = Some(config);
        window.show(1, Field::Effort, Vec::new(), 0, ChoiceState::Reading);
        let listing = coding_crew_core::harness::Listing::Cursor;
        let catalog = listing
            .parse("gpt-5.3-codex-low\ngpt-5.3-codex\ngpt-5.3-codex-high\n")
            .expect("the listing");
        window.fill_effort(Ok(catalog));
        assert_eq!(
            entries_of(&window),
            [
                ("low".to_string(), true),
                ("high".to_string(), true),
                ("none".to_string(), true),
            ]
        );
        assert_eq!(
            window.choice.as_ref().expect("the choice").cursor,
            1,
            "the cursor is not on the level in force"
        );
        window.on_key(press(KeyCode::Down));
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_none(), "enter did not pick none");
        let builder = window
            .session()
            .and_then(|config| config.agents.get(1))
            .expect("the builder");
        assert_eq!(
            builder.effort.as_deref(),
            Some(coding_crew_core::catalog::NO_EFFORT)
        );
        assert!(pick::gaps(window.session().expect("the session"), &window.levels).is_empty());
        let _ = fs::remove_dir_all(&home);
    }

    /// Open the choice of an agent's model as a catalogue read that answered
    /// with `loaded` would open it, for the agent at `agent`.
    fn answer_choice_for(window: &mut App, agent: usize, loaded: Result<Catalog, String>) {
        window.show(agent, Field::Model, Vec::new(), 0, ChoiceState::Reading);
        window.fill_choice(loaded);
    }

    /// A mouse event at a cell.
    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// A left click at a cell.
    fn click(window: &mut App, column: u16, row: u16) -> Action {
        window.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row))
    }

    /// A wheel notch at a cell: down, or up.
    fn wheel(window: &mut App, down: bool, column: u16, row: u16) -> Action {
        let kind = if down {
            MouseEventKind::ScrollDown
        } else {
            MouseEventKind::ScrollUp
        };
        window.on_mouse(mouse(kind, column, row))
    }

    /// The cell where `needle` is drawn, as `(column, row)`.
    fn at(rows: &[String], needle: &str) -> (u16, u16) {
        let row = row_of(rows, needle);
        (column_at(rows, row, needle) as u16, row as u16)
    }

    #[test]
    fn a_click_on_a_sidebar_row_moves_the_cursor_and_a_second_opens_its_choice() {
        let mut window = app();
        let (column, row) = at(
            &render_app(&mut window, 100, 40, &RuntimeRead::Absent).0,
            "  harness",
        );
        let before = window.cursor;
        click(&mut window, column, row);
        assert!(window.choice.is_none(), "one click opened a choice");
        assert_ne!(window.cursor, before, "the click did not move the cursor");
        let picked = window.cursor;
        click(&mut window, column, row);
        assert!(
            window.choice.is_some(),
            "the second click did not open the choice"
        );
        assert_eq!(window.cursor, picked);
    }

    #[test]
    fn the_wheel_over_the_sidebar_moves_its_cursor_three_rows() {
        let mut window = app();
        let rows = render_app(&mut window, 100, 40, &RuntimeRead::Absent).0;
        let (column, row) = at(&rows, "  harness");
        let rows_of_config = sidebar::pick_rows(window.session().expect("a configuration"));
        let before = window.cursor;
        wheel(&mut window, true, column, row);
        assert_eq!(window.cursor, (before + 3).min(rows_of_config - 1));
        // One notch back up is three rows back: the cursor returns to where it
        // started, and a notch past the top stops there.
        wheel(&mut window, false, column, row);
        assert_eq!(window.cursor, before);
        wheel(&mut window, false, column, row);
        wheel(&mut window, false, column, row);
        assert_eq!(window.cursor, 0, "the wheel went up past the top");
    }

    #[test]
    fn no_sidebar_is_clickable_where_the_window_drops_it() {
        // Below the sidebar's width the sidebar is not drawn: the pointer over
        // its place does nothing.
        let mut window = app();
        render_app(&mut window, 40, 40, &RuntimeRead::Absent);
        let before = window.cursor;
        for row in 0..40 {
            click(&mut window, 2, row);
            wheel(&mut window, true, 2, row);
        }
        assert_eq!(window.cursor, before);
        assert!(window.choice.is_none());
    }

    #[test]
    fn a_click_on_a_queue_row_selects_it_and_a_second_marks_it_at_40_and_100() {
        for width in [40u16, 100] {
            let mut window = app();
            let runtime = RuntimeRead::Record(queued_runtime());
            let rows = render_app(&mut window, width, 40, &runtime).0;
            // The waiting tasks are TSK29, TSK07 and TSK08, in queue order.
            let (column, row) = at(&rows, "TSK07");
            click(&mut window, column, row);
            assert_eq!(
                window.focus,
                Focus::Queue,
                "{width}: the click did not take the queue"
            );
            assert_eq!(
                window.queue_cursor, 1,
                "{width}: the wrong task is selected"
            );
            assert!(
                window.marked.is_empty(),
                "{width}: one click marked the task"
            );
            click(&mut window, column, row);
            assert_eq!(
                window.marked,
                vec!["TSK07".to_string()],
                "{width}: the second click"
            );
        }
    }

    #[test]
    fn the_wheel_over_the_queue_moves_its_cursor_and_takes_the_focus() {
        for width in [40u16, 100] {
            let mut window = app();
            let runtime = RuntimeRead::Record(queued_runtime());
            let rows = render_app(&mut window, width, 40, &runtime).0;
            let (column, row) = at(&rows, "TSK29");
            wheel(&mut window, true, column, row);
            assert_eq!(window.focus, Focus::Queue, "{width}");
            assert_eq!(
                window.queue_cursor, 2,
                "{width}: the wheel stops at the last task"
            );
            wheel(&mut window, false, column, row);
            assert_eq!(window.queue_cursor, 0, "{width}: a notch is three rows up");
        }
    }

    #[test]
    fn a_pointer_that_only_moves_changes_nothing_and_asks_no_redraw() {
        // A move, a drag and a release leave the window as it is and answer
        // Idle, so a pointer passing over the window costs no draw.
        let mut window = app();
        let rows = render_app(&mut window, 100, 40, &RuntimeRead::Absent).0;
        let (column, row) = at(&rows, "  harness");
        let (cursor, focus) = (window.cursor, window.focus);
        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            assert_eq!(
                window.on_mouse(mouse(kind, column, row)),
                Action::Idle,
                "{kind:?} asked for a redraw"
            );
        }
        assert_eq!(window.cursor, cursor, "a move changed the cursor");
        assert_eq!(window.focus, focus, "a move changed the focus");
        assert!(window.choice.is_none(), "a move opened a choice");
    }

    #[test]
    fn each_queue_pick_is_the_line_its_task_is_drawn_on() {
        // A pick is the line the draw recorded for its task, under the caption:
        // the click answers to what is drawn there. The held task sits ahead of
        // the waiting ones, so a waiting task's place in the queue is not its
        // place among the waiting: a pick that counted the queue would land on
        // another row.
        let window = app();
        let mut runtime = queued_runtime();
        runtime
            .queue
            .insert(1, queue_item("TSK13", Status::Approved));
        assert!(
            runtime.is_held("TSK13"),
            "the fixture no longer holds TSK13"
        );
        let waiting: Vec<String> = runtime.waiting().map(|item| item.id.clone()).collect();
        assert_eq!(waiting, ["TSK29", "TSK07", "TSK08"]);
        let queue = window
            .sections(&runtime, 100, RENDER_NOW, 0)
            .into_iter()
            .find(|section| section.title == QUEUE_TITLE)
            .expect("the queue section");
        assert_eq!(queue.picks.len(), waiting.len());
        for (line, index) in &queue.picks {
            let text: String = queue.lines[*line]
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            assert!(
                text.contains(waiting[*index].as_str()),
                "task {index} is picked on line {line}, drawn as {text:?}"
            );
        }
    }

    #[test]
    fn a_click_lands_on_its_task_past_a_waiting_row_that_is_not_claimable_at_40_and_100() {
        // A waiting task the record does not call claimable is drawn with no
        // pick, so the claimable rows below it sit a line lower than a fixed
        // offset from the count line would put them. Each click answers to the
        // row drawn: the one on TSK07 selects and marks TSK07, not the row a
        // fixed offset would name.
        for width in [40u16, 100] {
            let mut window = app();
            let mut runtime = queued_runtime();
            let mut parked = queue_item("TSK27", Status::Approved);
            parked.claimable = false;
            runtime.queue.insert(2, parked);
            let rows = render_app(&mut window, width, 40, &RuntimeRead::Record(runtime)).0;
            let (column, row) = at(&rows, "TSK07");
            click(&mut window, column, row);
            assert_eq!(
                window.focus,
                Focus::Queue,
                "{width}: the click did not take the queue"
            );
            assert_eq!(
                window.queue_cursor, 1,
                "{width}: the click on TSK07 selected another task"
            );
            click(&mut window, column, row);
            assert_eq!(
                window.marked,
                vec!["TSK07".to_string()],
                "{width}: the second click did not mark TSK07"
            );
        }
    }

    #[test]
    fn a_caption_moves_every_pick_down_by_its_own_lines_not_a_fixed_offset() {
        // The queue's picks sit one count line below the caption. A block with
        // two count lines puts its rows one line lower, so a pick that kept the
        // fixed offset of the queue would land on the second count line.
        let window = app();
        let block = vec![
            Line::from("count one"),
            Line::from("count two"),
            Line::from("TSK21 waiting"),
            Line::from("TSK22 waiting"),
        ];
        let (lines, picks) = window.captioned_picks("caption", (block, vec![(2, 0), (3, 1)]), 100);
        assert_eq!(picks, [(3, 0), (4, 1)]);
        for (line, index) in &picks {
            let text: String = lines[*line]
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            let task = ["TSK21", "TSK22"][*index];
            assert!(
                text.contains(task),
                "task {task} is picked on line {line}, drawn as {text:?}"
            );
        }
    }

    #[test]
    fn a_click_on_a_choice_entry_moves_the_cursor_and_a_second_picks_it_at_40_and_100() {
        let home = scratch("choice-mouse");
        for width in [40u16, 100] {
            let mut window = on_orchestrator(Some(home.clone()));
            answer_choice(
                &mut window,
                Ok(Catalog::from_lines(
                    "claude-sonnet-5-5\nclaude-opus-5-5\nclaude-haiku-5-5\n",
                )),
            );
            // The choice opens on the model in force, Opus. The first entry is
            // the one a first click moves to: its name is drawn only in the
            // picker, as the sidebar shows the other agents' models.
            let models = ["claude-sonnet-5-5", "claude-opus-5-5", "claude-haiku-5-5"];
            let target = 0;
            assert_ne!(window.choice.as_ref().expect("the choice").cursor, target);
            let rows = render_app(&mut window, width, 40, &RuntimeRead::Absent).0;
            let (column, row) = at(&rows, models[target]);
            click(&mut window, column, row);
            assert_eq!(
                window.choice.as_ref().expect("the choice").cursor,
                target,
                "{width}: the first click did not move"
            );
            assert!(window.choice.is_some(), "{width}: the first click picked");
            click(&mut window, column, row);
            assert!(
                window.choice.is_none(),
                "{width}: the second click did not pick"
            );
            assert_eq!(
                model_of(&window, "orchestrator"),
                Some(models[target]),
                "{width}"
            );
        }
    }

    #[test]
    fn a_click_on_a_crew_size_entry_sets_the_crew_on_a_second_click_at_40_and_100() {
        for width in [40u16, 100] {
            let mut window = app();
            window.on_key(press(KeyCode::Char('c')));
            let rows = render_app(&mut window, width, 40, &RuntimeRead::Absent).0;
            let (column, row) = at(&rows, "single agent");
            click(&mut window, column, row);
            assert_eq!(
                window.crew_cursor,
                Some(0),
                "{width}: the first click did not move"
            );
            click(&mut window, column, row);
            assert_eq!(
                window.crew_cursor, None,
                "{width}: the second click did not pick"
            );
            assert_eq!(
                crew_size_of(window.session().expect("a configuration")),
                Some(1),
                "{width}: the crew is not the single agent"
            );
        }
    }

    #[test]
    fn a_click_outside_the_panes_does_nothing() {
        let mut window = app();
        let runtime = RuntimeRead::Record(queued_runtime());
        render_app(&mut window, 100, 40, &runtime);
        let before = (window.cursor, window.focus, window.choice.is_some());
        // The title row, the rule over the footer, and the footer itself.
        for row in [0u16, 37, 39] {
            click(&mut window, 60, row);
        }
        assert_eq!(
            (window.cursor, window.focus, window.choice.is_some()),
            before
        );
    }

    #[test]
    fn the_help_takes_the_pointer_away() {
        let mut window = app();
        let rows = render_app(&mut window, 100, 40, &RuntimeRead::Absent).0;
        let (column, row) = at(&rows, "  harness");
        window.on_key(press(KeyCode::Char('?')));
        let before = window.cursor;
        click(&mut window, column, row);
        wheel(&mut window, true, column, row);
        assert_eq!(window.cursor, before);
    }
}
