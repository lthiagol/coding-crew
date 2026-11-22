//! The window: one sidebar, one main pane, a title line and a footer.
//!
//! The sidebar is the configuration and the main pane is what is running.
//! There is no tab and no tab bar. The main pane leads with the crew — the
//! reading the human picked — then the unclaimed queue, the round, the budget
//! and the findings, with the stamp at its foot.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use coding_crew_core::config::{CONFIG_FILE, Config};
use coding_crew_core::harness::{self, Harness};
use coding_crew_core::runtime::{CrewMember, Runtime};
use coding_crew_core::{Role, age_text, parse_rfc3339};
use coding_crew_theme::text::{cut_to, tail_to, text_width};
use coding_crew_theme::{Icon, IconSet, Slot, Theme, chrome, phase_style, selected_line};

use crate::catalog::Catalog;
use crate::config::{self, ConfigRead, ConfigState};
use crate::launch::{self, LaunchRequest, Launcher, Start};
use crate::sidebar::{self, role_icon};

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
/// The key that opens the orchestrator's model choice.
pub const CHOOSE_KEY: &str = "enter";
/// The key that leaves the choice without saving.
pub const CLOSE_KEY: &str = "esc";
/// The key that starts the orchestrator over the queue.
pub const START_KEY: &str = "s";
/// The notice when `s` is pressed outside herdr.
pub const START_NEEDS_HERDR: &str = "start needs herdr";
/// The notice when `s` is pressed with no task in the queue.
pub const QUEUE_EMPTY: &str = "the queue is empty";
/// The note while the orchestrator's start runs.
pub const STARTING: &str = "starting the orchestrator…";
/// The notice when herdr already names an orchestrator.
pub const ALREADY_RUNNING: &str = "an orchestrator is already running in herdr";
/// The note's prefix when the start fails: the error follows it.
pub const START_FAILED: &str = "the orchestrator did not start";
/// The choice's block title.
pub const CHOICE_TITLE: &str = "orchestrator model";
/// The help's block title.
pub const HELP_TITLE: &str = "keys";
/// The choice's state when the catalogue cannot be read.
pub const CATALOG_UNREADABLE: &str = "the model catalog cannot be read";
/// The choice's state while the catalogue's tool runs.
pub const CATALOG_READING: &str = "reading the model catalog";
/// The choice's state when the catalogue lists no model.
pub const NO_MODELS: &str = "no models in the catalog";

/// The sidebar's block title.
pub const SIDEBAR_TITLE: &str = "configuration";
/// The main pane's block title.
pub const FLOW_TITLE: &str = "flow";
/// The main pane's named empty state.
pub const FLOW_EMPTY: &str = "nothing is running";
/// The main pane's state when the record is there but cannot be read.
pub const FLOW_UNREADABLE: &str = "the runtime record cannot be read";

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
}

/// The model choice: the catalogue's models, and the cursor on them.
struct Choice {
    /// The catalogue the list came from, and the list a pick is checked
    /// against.
    catalog: Catalog,
    /// The model the cursor is on.
    cursor: usize,
    /// What the choice has to show.
    state: ChoiceState,
}

/// What the choice has to show, while the catalogue may not be there yet.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ChoiceState {
    /// The catalogue's tool is still running: nothing to pick yet.
    Reading,
    /// The catalogue's models are the list.
    Listed,
    /// The catalogue is there and lists nothing.
    Empty,
    /// The catalogue cannot be read: why.
    Failed(String),
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

/// The state of the window.
pub struct App {
    /// The colours of the window.
    pub theme: Theme,
    /// The icon set in force: every pane's title and every role draws with it.
    pub icons: IconSet,
    /// The configuration as read, and what a pick is saved into.
    pub config: ConfigRead,
    /// The user's home, when there is one: the configuration lives under it.
    home: Option<PathBuf>,
    /// The agents group's row the cursor is on.
    cursor: usize,
    /// The agents group is drawn, so the cursor has rows on screen.
    agents_drawn: bool,
    /// The open model choice, when one is open.
    choice: Option<Choice>,
    /// The catalogue read in flight, when the choice waits on one. It runs
    /// off the window's thread, so no key handling waits on a harness tool.
    reading: Option<JoinHandle<Result<Catalog, String>>>,
    /// The help overlay is open.
    help: bool,
    /// A pick that could not be saved, named so nothing fails silently.
    notice: Option<ReadError>,
    /// The launcher `s` starts the orchestrator through. `None` outside herdr:
    /// the key says why, and starts nothing.
    launch: Option<Arc<dyn Launcher>>,
    /// The orchestrator's start in flight. It runs off the window's thread,
    /// as the catalogue read does, so no key handling waits on herdr.
    starting: Option<JoinHandle<Result<Start, String>>>,
    /// The unheld queue's task ids, as the last frame drew them: the work
    /// `s` hands the orchestrator, in order.
    queued: Vec<String>,
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
}

impl App {
    /// A window with a theme and a configuration of its own.
    pub fn new(theme: Theme, icons: IconSet, config: ConfigRead, home: Option<PathBuf>) -> Self {
        Self {
            theme,
            icons,
            config,
            home,
            cursor: 0,
            agents_drawn: true,
            choice: None,
            reading: None,
            help: false,
            notice: None,
            launch: None,
            starting: None,
            queued: Vec::new(),
            note: None,
            project: None,
            root: None,
            #[cfg(test)]
            catalog_tool: None,
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
    /// move the cursor between the agents, open the orchestrator's model
    /// choice, reload the configuration, and open the help. The choice's keys
    /// move its cursor, pick a model, and close it. No other key does
    /// anything.
    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
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
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('r') => self.reload(),
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Enter => self.open_choice(),
            KeyCode::Char('s') => self.start_orchestrator(),
            _ => {}
        }
        Action::Redraw
    }

    /// Handle a key while the choice covers the sidebar.
    fn choice_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Esc => self.close_choice(),
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(choice) = self.choice.as_mut() {
                    choice.cursor = choice.cursor.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(choice) = self.choice.as_mut()
                    && choice.cursor + 1 < choice.catalog.models().len()
                {
                    choice.cursor += 1;
                }
            }
            // Only a listed catalogue has a model to pick: while it is read,
            // or when it failed or is empty, enter does nothing.
            KeyCode::Enter
                if self
                    .choice
                    .as_ref()
                    .is_some_and(|choice| choice.state == ChoiceState::Listed) =>
            {
                self.pick()
            }
            _ => {}
        }
    }

    /// Draw the window. `now` is the instant holds are measured at, and `tick`
    /// is the frame the spinner shows.
    pub fn draw(&mut self, frame: &mut Frame, runtime: &RuntimeRead, now: &str, tick: usize) {
        // The queue `s` starts the orchestrator over is the one drawn: the
        // unclaimed work, in order. Held tasks are the crew's, not the queue's.
        self.queued = match runtime {
            RuntimeRead::Record(runtime) => {
                runtime.unheld_queue().map(|item| item.id.clone()).collect()
            }
            _ => Vec::new(),
        };
        let screen = screen(frame.area(), self.note.is_some());
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
        frame.render_widget(self.footer_line(screen.footer.width), screen.footer);
        if self.help {
            self.help_pane(frame, frame.area());
        } else if self.choice.is_some() {
            self.choice_pane(frame, frame.area());
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
        let (inner, width, height) = if area.height >= 3 && area.width >= 4 {
            let block = chrome::pane(SIDEBAR_TITLE, Icon::Config, self.icons, false, &self.theme);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            (inner, inner.width, inner.height)
        } else {
            (area, area.width, area.height)
        };
        let cursor = if self.loaded().is_some() {
            self.cursor
        } else {
            usize::MAX
        };
        // A pick that could not be saved is named, never swallowed: the
        // configuration on screen is the one still in force. It is drawn
        // inside the sidebar's rows, so the fit budgets for it.
        let drawn = sidebar::lines(
            &self.config,
            self.notice.as_ref(),
            &self.theme,
            self.icons,
            usize::from(width),
            usize::from(height),
            cursor,
        );
        frame.render_widget(Paragraph::new(drawn.lines), inner);
        drawn.agents
    }

    /// The configuration as loaded, when the file parsed.
    fn loaded(&self) -> Option<&Config> {
        match &self.config.state {
            ConfigState::Config(config) => Some(config),
            _ => None,
        }
    }

    /// The agent the cursor is on, when there is one.
    fn cursor_agent(&self) -> Option<&coding_crew_core::config::Agent> {
        self.loaded()
            .and_then(|config| config.agents.get(self.cursor))
    }

    /// The orchestrator's model, as the configuration holds it.
    fn orchestrator_model(&self) -> Option<&str> {
        self.loaded()
            .and_then(|config| {
                config
                    .agents
                    .iter()
                    .find(|agent| agent.role == Role::Orchestrator)
            })
            .map(|agent| agent.model.as_str())
    }

    /// Whether `enter` has a choice to open: the cursor is on the
    /// orchestrator's row, the configuration parsed, and its group is drawn.
    fn can_choose(&self) -> bool {
        self.agents_drawn
            && self
                .cursor_agent()
                .is_some_and(|agent| agent.role == Role::Orchestrator)
    }

    /// Move the cursor between the agents, and no further.
    fn move_cursor(&mut self, delta: isize) {
        if !self.agents_drawn {
            return;
        }
        let Some(count) = self.loaded().map(|config| config.agents.len()) else {
            return;
        };
        if count == 0 {
            return;
        }
        let next = self.cursor as isize + delta;
        self.cursor = next.clamp(0, count as isize - 1) as usize;
    }

    /// Read the configuration again: the sidebar shows the file as it is now.
    ///
    /// This is the configuration only. The runtime record is already read on
    /// every redraw, and the catalogue is read when a choice opens.
    fn reload(&mut self) {
        self.config = config::load(self.home.as_deref());
        self.notice = None;
        self.cursor = match self.loaded() {
            Some(config) => self.cursor.min(config.agents.len().saturating_sub(1)),
            None => 0,
        };
    }

    /// Open the orchestrator's model choice, and start reading the catalogue.
    ///
    /// The read runs beside the window: the choice opens at once, says it is
    /// reading, and takes the list — or the error — when the harness's tool
    /// answers. A tool that hangs is bounded by [`Catalog::read`], and the
    /// window keeps drawing and taking keys meanwhile.
    fn open_choice(&mut self) {
        if !self.can_choose() {
            return;
        }
        let Some(harness) = self.orchestrator_harness() else {
            return;
        };
        let reading = self.read_catalog(harness);
        self.begin_choice(reading);
    }

    /// The harness the orchestrator runs in, when the table holds it.
    fn orchestrator_harness(&self) -> Option<&'static Harness> {
        let agent = self
            .loaded()?
            .agents
            .iter()
            .find(|agent| agent.role == Role::Orchestrator)?;
        harness::find(&agent.harness)
    }

    /// Read a harness's catalogue beside the window.
    fn read_catalog(&self, harness: &'static Harness) -> JoinHandle<Result<Catalog, String>> {
        #[cfg(test)]
        {
            if let Some(tool) = self.catalog_tool.clone() {
                return std::thread::spawn(move || {
                    Catalog::read_command(&tool.to_string_lossy(), &[], crate::catalog::READ_BOUND)
                });
            }
        }
        std::thread::spawn(move || Catalog::read(harness))
    }

    /// Open the choice on a read that has not answered yet.
    fn begin_choice(&mut self, reading: JoinHandle<Result<Catalog, String>>) {
        self.help = false;
        self.reading = Some(reading);
        self.choice = Some(Choice {
            catalog: Catalog::default(),
            cursor: 0,
            state: ChoiceState::Reading,
        });
    }

    /// Whether the choice is waiting on the catalogue.
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
        if self.choice.is_some() {
            // The choice is already open: the answer fills it rather than
            // opening it again.
            self.fill_choice(loaded);
        }
        true
    }

    /// Close the choice, abandoning a read in flight with it.
    fn close_choice(&mut self) {
        self.choice = None;
        self.reading = None;
    }

    /// Put a catalogue — or the reason there is none — in the open choice.
    ///
    /// The cursor starts on the model in force, so a pick is a move from
    /// where the configuration is rather than from the top of the list.
    fn fill_choice(&mut self, loaded: Result<Catalog, String>) {
        let (catalog, state) = match loaded {
            Ok(catalog) if catalog.models().is_empty() => (catalog, ChoiceState::Empty),
            Ok(catalog) => (catalog, ChoiceState::Listed),
            Err(error) => (Catalog::default(), ChoiceState::Failed(error)),
        };
        let cursor = self
            .orchestrator_model()
            .and_then(|current| catalog.index_of(current))
            .unwrap_or(0);
        self.choice = Some(Choice {
            catalog,
            cursor,
            state,
        });
    }

    /// Save the model under the choice's cursor and return to the sidebar.
    ///
    /// The pick is checked against the catalogue the list came from, so a
    /// configuration can never hold a model that does not exist.
    fn pick(&mut self) {
        let Some(choice) = self.choice.as_ref() else {
            return;
        };
        let Some(model) = choice.catalog.models().get(choice.cursor).cloned() else {
            self.close_choice();
            return;
        };
        if !choice.catalog.knows(&model) {
            self.close_choice();
            return;
        }
        let Some(mut config) = self.loaded().cloned() else {
            self.close_choice();
            return;
        };
        let Some(agent) = config
            .agents
            .iter_mut()
            .find(|agent| agent.role == Role::Orchestrator)
        else {
            self.close_choice();
            return;
        };
        agent.model = model.clone();
        let saved = match &self.home {
            Some(home) => config::save_pick_at(home, &config, &model),
            None => Err(ReadError {
                file: CONFIG_FILE.to_string(),
                error: "there is no home to save the configuration in".to_string(),
            }),
        };
        match saved {
            Ok(saved) => {
                self.config.state = ConfigState::Config(saved);
                self.notice = None;
            }
            Err(error) => self.notice = Some(error),
        }
        self.close_choice();
        self.cursor = self.cursor.min(config.agents.len().saturating_sub(1));
    }

    /// Whether `s` can act: herdr hosts hugo, the queue has work, and no start
    /// is in flight.
    fn can_start(&self) -> bool {
        self.launch.is_some() && !self.queued.is_empty() && self.starting.is_none()
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
            self.note = Some(Note::Notice(QUEUE_EMPTY.to_string()));
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
            .loaded()
            .ok_or_else(|| "the configuration cannot be read".to_string())?;
        let agent = config
            .agents
            .iter()
            .find(|agent| agent.role == Role::Orchestrator)
            .ok_or_else(|| "the configuration names no orchestrator".to_string())?;
        let harness = harness::find(&agent.harness)
            .ok_or_else(|| format!("{} is not a harness", agent.harness))?;
        let start = harness.invocation(&agent.name, &agent.model, agent.effort.as_deref());
        let project = self
            .root
            .clone()
            .ok_or_else(|| "the project's folder is not known".to_string())?;
        Ok(LaunchRequest {
            kind: harness.kind.to_string(),
            args: start.args,
            env: start.env,
            project,
            tasks: self.queued.clone(),
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
        let block = chrome::pane(title, Icon::Stopped, self.icons, false, &self.theme);
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
            let block = chrome::pane(FLOW_TITLE, Icon::Config, self.icons, false, &self.theme);
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
    fn flow(&self, frame: &mut Frame, area: Rect, runtime: &Runtime, now: &str, tick: usize) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        if area.height < 3 || area.width < 4 {
            self.flow_body(frame, area, runtime, now, tick);
            return;
        }
        let block = chrome::pane(
            FLOW_TITLE,
            run_icon(runtime.stamp.runs),
            self.icons,
            false,
            &self.theme,
        );
        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.flow_body(frame, inner, runtime, now, tick);
    }

    /// The blocks inside the flow's box, and the stamp on the last row.
    fn flow_body(&self, frame: &mut Frame, inner: Rect, runtime: &Runtime, now: &str, tick: usize) {
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
    fn draw_boxed(&self, frame: &mut Frame, body: Rect, sections: &[Section]) {
        let mut constraints: Vec<Constraint> = sections
            .iter()
            .map(|section| Constraint::Length((section.lines.len() + 2) as u16))
            .collect();
        constraints.push(Constraint::Min(0));
        let areas = Layout::vertical(constraints).split(body);
        for (section, area) in sections.iter().zip(areas.iter()) {
            let block = chrome::pane(section.title, section.icon, self.icons, false, &self.theme);
            frame.render_widget(Paragraph::new(section.lines.clone()).block(block), *area);
        }
    }

    /// The blocks without their boxes: a title and its lines.
    ///
    /// A block that does not fit is left out whole, and one line names what
    /// is not shown. A block is never cut in the middle without a word.
    fn draw_words(&self, frame: &mut Frame, body: Rect, sections: &[Section]) {
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
            let heading = format!("{} {}", section.icon.glyph(self.icons), section.title);
            lines.push(Line::from(Span::styled(heading, self.theme.text.strong)));
            lines.extend(section.lines.clone());
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
        vec![
            Section {
                title: CREW_TITLE,
                icon: Icon::RoleOrchestrator,
                lines: self.crew_lines(runtime, width, now, tick),
            },
            Section {
                title: QUEUE_TITLE,
                icon: Icon::TabBacklog,
                lines: self.queue_lines(runtime, width),
            },
            Section {
                title: ROUND_TITLE,
                icon: Icon::RoleReviewer,
                lines: vec![self.round_line(runtime, width)],
            },
            Section {
                title: BUDGET_TITLE,
                icon: Icon::Clock,
                lines: self.budget_lines(runtime, width),
            },
            Section {
                title: FINDINGS_TITLE,
                icon: Icon::Finding,
                lines: self.findings_lines(runtime, width),
            },
        ]
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
    /// one is dim, the row and its state alike.
    fn crew_row(&self, member: &CrewMember, now: &str, tick: usize) -> CrewRow {
        let working = member.state.is_working();
        let normal = if working {
            self.theme.text.normal
        } else {
            self.theme.text.dim
        };
        let state = if working {
            format!("{} {}", chrome::spinner_frame(tick), member.state.as_str())
        } else {
            member.state.as_str().to_string()
        };
        let state_style = if working {
            self.theme.semantic.agent
        } else {
            self.theme.text.dim
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
                normal,
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

    /// The queue: the unclaimed work, an id and a status, and the count with
    /// the set it counts.
    fn queue_lines(&self, runtime: &Runtime, width: usize) -> Vec<Line<'static>> {
        let items: Vec<&coding_crew_core::runtime::QueueItem> = runtime.unheld_queue().collect();
        let count = items.len();
        let noun = if count == 1 { "task" } else { "tasks" };
        let mut lines = vec![self.fit(
            vec![
                ("the automation's queue · ".to_string(), self.theme.text.dim),
                (count.to_string(), self.theme.text.strong),
                (format!(" {noun}"), self.theme.text.dim),
            ],
            width,
        )];
        for item in items {
            lines.push(self.fit(
                vec![
                    (item.id.clone(), self.theme.text.normal),
                    ("  ".to_string(), Style::default()),
                    (
                        item.status.as_str().to_string(),
                        phase_style(&self.theme, item.status),
                    ),
                ],
                width,
            ));
        }
        lines
    }

    /// The review in flight: the task, the stage and the number.
    fn round_line(&self, runtime: &Runtime, width: usize) -> Line<'static> {
        match &runtime.round {
            Some(round) => self.fit(
                vec![
                    (round.task.clone(), self.theme.text.normal),
                    (
                        format!(" · {} · round {}", stage_word(round.stage), round.number),
                        self.theme.text.dim,
                    ),
                ],
                width,
            ),
            None => self.fit(
                vec![("no review in flight".to_string(), self.theme.text.dim)],
                width,
            ),
        }
    }

    /// The external rounds spent of their budget, as a bar and the numbers in
    /// words. A line too narrow for both wraps the bar under its label.
    fn budget_lines(&self, runtime: &Runtime, width: usize) -> Vec<Line<'static>> {
        let spent = runtime.budget.spent as usize;
        let total = runtime.budget.total as usize;
        let bar = chrome::progress_bar(spent, total, BUDGET_CELLS, Slot::TextStrong, &self.theme)
            .into_iter()
            .map(|span| (span.content.into_owned(), span.style));
        let numbers = vec![
            (" ".to_string(), Style::default()),
            (spent.to_string(), self.theme.text.strong),
            (" of ".to_string(), self.theme.text.dim),
            (total.to_string(), self.theme.text.strong),
        ];
        let label = ("external rounds ".to_string(), self.theme.text.dim);
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
        let count = vec![
            (runtime.findings.to_string(), self.theme.text.strong),
            (" open findings".to_string(), self.theme.text.normal),
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

    /// A line of spans, cut to the width with the cut marked.
    fn fit(&self, spans: Vec<(String, Style)>, width: usize) -> Line<'static> {
        let mut out = Vec::new();
        let mut used = 0;
        for (text, style) in spans {
            if used >= width {
                break;
            }
            let piece = cut_to(&text, width - used);
            used += text_width(&piece);
            let cut = piece != text;
            out.push(Span::styled(piece, style));
            if cut {
                break;
            }
        }
        Line::from(out)
    }

    /// The model choice: a small list of the catalogue's models, centred.
    ///
    /// The list scrolls to keep its cursor on screen; the row under the
    /// cursor takes the selection's paint. A choice the catalogue has not
    /// answered yet says it is reading; one that cannot be filled says why on
    /// the failure slot rather than opening empty.
    fn choice_pane(&self, frame: &mut Frame, area: Rect) {
        let Some(choice) = &self.choice else {
            return;
        };
        let rows = match &choice.state {
            ChoiceState::Reading | ChoiceState::Failed(_) => 2,
            ChoiceState::Empty => 0,
            ChoiceState::Listed => choice.catalog.models().len().min(CHOICE_ROWS),
        };
        let popup = choice_popup(area, rows);
        if popup.width < 4 || popup.height < 3 {
            return;
        }
        frame.render_widget(Clear, popup);
        let block = chrome::pane(
            CHOICE_TITLE,
            Icon::RoleOrchestrator,
            self.icons,
            true,
            &self.theme,
        );
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let width = usize::from(inner.width);
        if width == 0 || inner.height == 0 {
            return;
        }
        let mut lines = Vec::new();
        match &choice.state {
            ChoiceState::Reading => lines.push(Line::from(Span::styled(
                cut_to(CATALOG_READING, width),
                self.theme.text.dim,
            ))),
            ChoiceState::Failed(error) => {
                lines.push(Line::from(Span::styled(
                    cut_to(CATALOG_UNREADABLE, width),
                    self.theme.text.normal,
                )));
                lines.push(Line::from(Span::styled(
                    cut_to(error, width),
                    self.theme.semantic.error,
                )));
            }
            ChoiceState::Empty => lines.push(Line::from(Span::styled(
                cut_to(NO_MODELS, width),
                self.theme.text.dim,
            ))),
            ChoiceState::Listed => {
                let models = choice.catalog.models();
                let height = usize::from(inner.height);
                let start = if choice.cursor < height {
                    0
                } else {
                    choice.cursor + 1 - height
                };
                let end = (start + height).min(models.len());
                for (offset, model) in models[start..end].iter().enumerate() {
                    let line =
                        Line::from(Span::styled(cut_to(model, width), self.theme.text.normal));
                    if start + offset == choice.cursor {
                        lines.push(selected_line(line, width, &self.theme));
                    } else {
                        lines.push(line);
                    }
                }
            }
        }
        frame.render_widget(Paragraph::new(lines), inner);
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
            if choice.catalog.models().len() > 1 {
                lines.push((0, line("↑↓ j/k", "move")));
            }
            lines.push((1, line("ctrl-c", "quit")));
            lines.push((2, line(CLOSE_KEY, "close")));
            if choice.state == ChoiceState::Listed {
                lines.push((3, line(CHOOSE_KEY, "pick")));
            }
            lines.push((u8::MAX, line("q", "quit")));
            lines.push((u8::MAX, line("?", "this help")));
            return lines;
        }
        let mut lines = Vec::new();
        if self.agents_drawn
            && self
                .loaded()
                .is_some_and(|config| !config.agents.is_empty())
        {
            lines.push((0, line("↑↓ j/k", "move")));
        }
        lines.push((1, line("ctrl-c", "quit")));
        lines.push((2, line(RELOAD_KEY, "reload")));
        if self.can_choose() {
            lines.push((3, line(CHOOSE_KEY, "choose")));
        }
        lines.push((3, line(START_KEY, "start the orchestrator")));
        lines.push((u8::MAX, line("q", "quit")));
        lines.push((u8::MAX, line("?", "this help")));
        lines
    }

    /// The footer: the keys that work. The window has `q quit`, and `ctrl-c`
    /// quits as in raul; the footer names what a line can carry, and the
    /// choice's keys while the choice is open.
    fn footer_line(&self, width: u16) -> Line<'static> {
        let part = |key: &str, says: &str| chrome::key_chip(key, says, &self.theme);
        if self.help {
            return footer(
                vec![
                    (0, part("q", "quit"), true),
                    (1, part(CLOSE_KEY, "close help"), true),
                    (2, part("?", "help"), false),
                ],
                width,
            );
        }
        if let Some(choice) = &self.choice {
            let mut parts = vec![
                (0, part(CLOSE_KEY, "close"), true),
                (2, part("q", "quit"), false),
                (4, part("?", "help"), false),
            ];
            if choice.state == ChoiceState::Listed {
                parts.push((1, part(CHOOSE_KEY, "pick"), true));
            }
            if choice.catalog.models().len() > 1 {
                parts.push((3, part("↑↓ j/k", "move"), false));
            }
            return footer(parts, width);
        }
        let mut parts = vec![
            (0, part("q", "quit"), true),
            (1, part(RELOAD_KEY, "reload"), false),
            (3, part("?", "help"), true),
        ];
        if self.agents_drawn
            && self
                .loaded()
                .is_some_and(|config| !config.agents.is_empty())
        {
            parts.push((2, part("↑↓ j/k", "move"), false));
        }
        if self.can_choose() {
            parts.push((4, part(CHOOSE_KEY, "choose"), false));
        }
        if self.can_start() {
            parts.push((5, part(START_KEY, "start"), false));
        }
        footer(parts, width)
    }
}

/// The footer line: the parts that fit, the ones that matter most first.
///
/// Chips are parted from the chip before them by two spaces; the columns a
/// chip takes after the first include those two.
fn footer(parts: Vec<(u8, Vec<Span<'static>>, bool)>, width: u16) -> Line<'static> {
    const PART: usize = 2;
    let mut chosen = vec![false; parts.len()];
    let mut used = 0usize;
    let mut count = 0usize;
    for matters in [true, false] {
        for (index, (_, spans, needed)) in parts.iter().enumerate() {
            if *needed != matters {
                continue;
            }
            let columns = spans
                .iter()
                .map(|span| text_width(span.content.as_ref()))
                .sum::<usize>()
                + if count > 0 { PART } else { 0 };
            if used + columns <= usize::from(width) {
                chosen[index] = true;
                used += columns;
                count += 1;
            }
        }
    }
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by_key(|index| parts[*index].0);
    let mut spans = Vec::new();
    for index in order {
        if chosen[index] {
            if !spans.is_empty() {
                spans.push(Span::raw(" ".repeat(PART)));
            }
            spans.extend(parts[index].1.clone());
        }
    }
    Line::from(spans)
}

/// The rows the choice's popup holds when the catalogue is longer.
const CHOICE_ROWS: usize = 18;

/// Where the choice sits: a small list, centred.
fn choice_popup(area: Rect, rows: usize) -> Rect {
    // The title and the footer stay on screen: the choice's keys are the
    // window's footer, and a popup must not cover them.
    let room = area.height.saturating_sub(2);
    let height = u16::try_from(rows.saturating_add(2))
        .unwrap_or(u16::MAX)
        .min(CHOICE_ROWS as u16 + 2)
        .min(room)
        .max(3)
        .min(area.height);
    let width = 56.min(area.width);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
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
    /// The note row, above the footer, when there is a note to say.
    pub note: Option<Rect>,
    /// The footer.
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
/// to give way.
pub fn screen(area: Rect, noted: bool) -> Screen {
    let sidebar_shown = area.width >= SIDEBAR_MIN_WIDTH;
    // The note row is there only while a start has something to say: a window
    // without one keeps the rows it always had.
    let mut constraints = vec![Constraint::Length(1), Constraint::Min(3)];
    if noted {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1));
    let rows = Layout::vertical(constraints).split(area);
    let footer = rows[rows.len() - 1];
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
        footer,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use std::fs;

    use super::*;
    use coding_crew_core::RoundKind;
    use coding_crew_core::config::LEGACY_CONFIG_FILE;
    use coding_crew_core::runtime::AgentState;
    use coding_crew_core::runtime::{
        Budget, Composition, CrewMember, QueueItem, RuntimeRound, Severities, Stamp,
    };
    use coding_crew_core::{Role, Status};
    use coding_crew_theme::Preset;

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
        role: Role,
        model: &str,
        holds: Option<&str>,
        since: Option<&str>,
        state: AgentState,
    ) -> CrewMember {
        CrewMember {
            role,
            model: model.to_string(),
            holds: holds.map(str::to_string),
            since: since.map(str::to_string),
            state,
        }
    }

    fn queue_item(id: &str, status: Status) -> QueueItem {
        QueueItem {
            id: id.to_string(),
            status,
        }
    }

    /// The record the mock writes, in its shape: three agents, one held task
    /// each for the runner and the reviewer, and one claimable task.
    fn runtime() -> Runtime {
        Runtime {
            started_at: "2026-11-09T08:30Z".to_string(),
            crew: vec![
                member(
                    Role::Orchestrator,
                    "opencode-go/glm-5.3#high",
                    None,
                    None,
                    AgentState::Idle,
                ),
                member(
                    Role::Runner,
                    "opencode-go/deepseek-v4.1-flash#high",
                    Some("TSK10"),
                    Some("2026-11-09T09:00Z"),
                    AgentState::Executing,
                ),
                member(
                    Role::Reviewer,
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
        let screen = screen(Rect::new(0, 0, 100, 40), false);
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
        let both = screen(Rect::new(0, 0, SIDEBAR_MIN_WIDTH, 40), false);
        let sidebar = both.sidebar.expect("the sidebar at the boundary");
        assert_eq!(sidebar.width, SIDEBAR_WIDTH);
        assert_eq!(both.flow.width, MAIN_MIN_WIDTH);

        let narrow = screen(Rect::new(0, 0, SIDEBAR_MIN_WIDTH - 1, 40), false);
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
            let screen = screen(Rect::new(0, 0, width, 40), false);
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
        // The cursor starts on the builder, where enter opens nothing: the
        // footer names quit, reload, move and help, and not choose.
        let (rows, _) = render(100, 20);
        let footer = rows.last().expect("a footer row");
        for works in ["q", "quit", "r", "reload", "↑↓ j/k", "move", "?", "help"] {
            assert!(footer.contains(works), "{footer}: {works}");
        }
        for dead in ["open", "tab", "esc", "choose", "pick"] {
            assert!(!footer.contains(dead), "{footer}: {dead}");
        }
        // On the orchestrator's row, enter has a choice to open.
        let mut app = App::new(theme(), IconSet::Nerd, sidebar_config(), None);
        app.cursor = 2;
        app.agents_drawn = true;
        let line: String = app
            .footer_line(100)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(line.contains("enter") && line.contains("choose"), "{line}");
        // With the sidebar dropped, the move keys would do nothing: they are
        // not named.
        app.agents_drawn = false;
        let line: String = app
            .footer_line(100)
            .spans
            .iter()
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
        let crew = row_of(&rows, "crew");
        let queue = row_of(&rows, "queue");
        let round = row_of(&rows, "round");
        let budget = row_of(&rows, "budget");
        let findings = row_of(&rows, "findings");
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
            "runner",
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
        for role in ["orchestrator", "runner", "reviewer"] {
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
        two.crew.retain(|member| member.role != Role::Reviewer);
        let (rows, _) = render_runtime(100, 40, &two);
        let text = rows.join("\n");
        assert!(text.contains("the orchestrator reviews"), "{text}");
        assert!(!text.contains("the human reviews"), "{text}");

        let mut one = two.clone();
        one.crew.retain(|member| member.role != Role::Orchestrator);
        let (rows, _) = render_runtime(100, 40, &one);
        let text = rows.join("\n");
        assert!(text.contains("the human reviews"), "{text}");

        // A role filled twice is not one per role, and the pane says what
        // the crew is rather than claiming a rule it does not follow.
        let mut four = runtime.clone();
        four.crew.push(member(
            Role::Runner,
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
            Role::Runner,
            "opencode-go/deepseek-v4.1-flash#high",
            Some("TSK29"),
            Some("2026-11-09T09:00Z"),
            AgentState::Executing,
        );
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert_eq!(text.matches("TSK29").count(), 1, "{text}");
        // The queue's count is the unheld set, and it is said once.
        assert!(text.contains("the automation's queue · 0 tasks"), "{text}");
    }

    #[test]
    fn the_queue_names_the_set_its_count_counts() {
        let runtime = runtime();
        let (rows, _) = render_runtime(100, 40, &runtime);
        let text = rows.join("\n");
        assert!(text.contains("the automation's queue · 1 task"), "{text}");
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
        assert!(text.contains("TSK13 · external · round 2"), "{text}");
        assert!(
            text.contains("external rounds ████░░░░░░ 18 of 45"),
            "{text}"
        );
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
        let runner = row_of(&rows, "runner");
        let reviewer = row_of(&rows, "reviewer");
        assert_eq!(runner - orchestrator, 2, "{rows:?}");
        assert_eq!(reviewer - runner, 2, "{rows:?}");
        assert!(
            rows[runner].contains("runner deepseek-v4.1-flash#high ⠋ executing"),
            "{rows:?}"
        );

        let (rows, _) = render_runtime(40, 40, &runtime);
        let orchestrator = row_of(&rows, "orchestrator");
        let runner = row_of(&rows, "runner");
        let reviewer = row_of(&rows, "reviewer");
        assert_eq!(runner - orchestrator, 3, "{rows:?}");
        assert_eq!(reviewer - runner, 3, "{rows:?}");
        assert!(rows[runner].contains("runner ⠋ executing"), "{rows:?}");
    }

    #[test]
    fn every_line_under_a_wrapped_agent_is_indented_under_it() {
        // The runner takes two lines at 60 and three at 40; each line under
        // the runner's first sits the same two columns in, so none reads as
        // an agent of its own.
        for width in [60u16, 40] {
            let runtime = runtime();
            let (rows, _) = render_runtime(width, 40, &runtime);
            let runner = row_of(&rows, "runner");
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
        assert!(!text.contains("…"), "{text}");
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
    fn a_narrow_pane_gives_the_crew_entry_more_lines_and_keeps_every_fact() {
        // The entry is one line at 100, two at 60, three at 40; no fact is
        // dropped with the width.
        for width in [100u16, 60, 40] {
            let runtime = runtime();
            let (rows, _) = render_runtime(width, 40, &runtime);
            let text = rows.join("\n");
            for fact in [
                "runner",
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
        // The crew's seven lines go with the crew, and the announcement says so:
        // three agents of two lines each, and the composition.
        let (rows, _) = render_runtime(60, 8, &runtime());
        let text = rows.join("\n");
        assert!(text.contains("crew (7 hidden)"), "{text}");
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
            "builder · runner",
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
    fn the_cursor_moves_between_the_agents_and_enter_opens_only_the_orchestrator() {
        let mut app = app();
        app.agents_drawn = true;
        // The cursor starts on the first agent, the builder: enter has
        // nothing to open there.
        assert_eq!(app.cursor, 0);
        assert_eq!(
            app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Action::Redraw
        );
        assert!(app.choice.is_none(), "enter opened a choice on the builder");
        // It stops at the last row and comes back to the first.
        for _ in 0..5 {
            app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        }
        assert_eq!(app.cursor, 2);
        assert!(app.can_choose(), "the last row is the orchestrator's");
        for _ in 0..5 {
            app.on_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        }
        assert_eq!(app.cursor, 0);
        // The other keys do not move it.
        for code in [KeyCode::Char('h'), KeyCode::Char('l'), KeyCode::PageDown] {
            app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
            assert_eq!(app.cursor, 0, "{code:?} moved the cursor");
        }
    }

    #[test]
    fn the_cursor_has_nothing_to_stand_on_when_its_group_is_not_drawn() {
        // A pane too short for the agents leaves them out whole: the cursor
        // must not walk rows the pane does not draw, and enter opens nothing.
        let mut app = app();
        app.agents_drawn = false;
        app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.cursor, 0);
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
        assert!(text.contains(CONFIG_FILE), "{text}");
        assert!(!text.contains("no configuration yet"), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "syntax error"),
            theme().semantic.error.fg.expect("the error colour")
        );
        // The defaults are shown beside the error: what is wrong, and what
        // the tool would use.
        assert!(text.contains(sidebar::USING_DEFAULTS), "{text}");
        assert!(text.contains("5 build-and-fix rounds"), "{text}");
        assert!(text.contains("builder · runner"), "{text}");
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
        // column. The row is the error cut to that, exactly, not a shorter
        // wording a test invented.
        let visible = cut_to(&format!("{CONFIG_FILE}: {expected}"), 29);
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
                window.fill_choice(Ok(Catalog::from_lines("p/m0\np/m1\np/m2\n")));
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
            CHOICE_TITLE.to_string(),
            CATALOG_READING.to_string(),
            CATALOG_UNREADABLE.to_string(),
            format!("{} seconds", crate::catalog::READ_BOUND.as_secs()),
            HELP_TITLE.to_string(),
            "the harness's own list".to_string(),
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
                    member(Role::Orchestrator, "m", None, None, AgentState::Idle),
                    member(Role::Runner, "m", None, None, AgentState::Executing),
                    member(Role::Reviewer, "m", None, None, AgentState::Reviewing),
                ],
                Composition::OnePerRole,
            ),
            (
                vec![
                    member(Role::Orchestrator, "m", None, None, AgentState::Idle),
                    member(Role::Runner, "m", None, None, AgentState::Executing),
                ],
                Composition::OrchestratorReviews,
            ),
            (
                vec![member(Role::Runner, "m", None, None, AgentState::Executing)],
                Composition::HumanReviews,
            ),
            (
                vec![
                    member(Role::Orchestrator, "m", None, None, AgentState::Idle),
                    member(Role::Runner, "m", None, None, AgentState::Executing),
                    member(Role::Runner, "m", None, None, AgentState::Executing),
                    member(Role::Reviewer, "m", None, None, AgentState::Reviewing),
                ],
                Composition::MoreThanOnePerRole,
            ),
        ] {
            let runtime = Runtime {
                started_at: "2026-11-09T08:30Z".to_string(),
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
    /// orchestrator's row: the only row a pick can be made from.
    fn on_orchestrator(home: Option<PathBuf>) -> App {
        let mut app = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Config(Config::default())),
            home,
        );
        app.agents_drawn = true;
        app.cursor = 2;
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

    /// A window on the orchestrator's row, reading its catalogue from `tool`
    /// in place of the harness's own list.
    fn on_orchestrator_with(tool: &std::path::Path) -> App {
        let mut window = App::new(
            theme(),
            IconSet::Nerd,
            ConfigRead::of(ConfigState::Config(Config::default())),
            None,
        );
        window.agents_drawn = true;
        window.cursor = 2;
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
        // Nothing is picked from a catalogue that could not be read: enter does
        // nothing, and esc is the way out.
        window.on_key(press(KeyCode::Enter));
        assert!(window.choice.is_some(), "enter closed the failed choice");
        window.on_key(press(KeyCode::Esc));
        assert!(window.choice.is_none());
        let _ = fs::remove_dir_all(tool.parent().expect("the scratch folder"));
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
        assert_eq!(app.loaded().expect("the configuration").round_limit, 5);
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
        assert_eq!(app.loaded().expect("the configuration").round_limit, 2);
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
        assert!(text.contains(CONFIG_FILE), "{text}");
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
        app.fill_choice(Ok(Catalog::from_lines(
            "claude-sonnet-5-5\nclaude-opus-5-5\nclaude-haiku-5-5\n",
        )));
        let choice = app.choice.as_ref().expect("the choice");
        assert_eq!(
            choice.catalog.models(),
            ["claude-sonnet-5-5", "claude-opus-5-5", "claude-haiku-5-5"],
            "the list is not the catalogue's"
        );
        // The orchestrator runs glm-5.3#high: the cursor starts there.
        assert_eq!(choice.cursor, 1);
        let models: Vec<String> = choice.catalog.models().to_vec();

        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(CHOICE_TITLE), "{text}");
        for model in models {
            assert!(text.contains(&model), "{model} is not in the list: {text}");
        }
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_pick_is_saved_and_the_sidebar_shows_what_it_saved() {
        let home = scratch("pick");
        let mut app = on_orchestrator(Some(home.clone()));
        app.fill_choice(Ok(Catalog::from_lines(
            "claude-sonnet-5-5\nclaude-opus-5-5\nclaude-haiku-5-5\n",
        )));
        // The cursor starts on glm-5.3; move up to kimi-k3 and pick it.
        app.on_key(press(KeyCode::Char('k')));
        assert_eq!(app.choice.as_ref().expect("the choice").cursor, 0);
        assert_eq!(
            app.on_key(press(KeyCode::Enter)),
            Action::Redraw,
            "a pick quit the window"
        );
        assert!(app.choice.is_none(), "the choice stayed open");
        let saved = app.loaded().expect("the configuration");
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == Role::Orchestrator)
                .map(|agent| agent.model.as_str()),
            Some("claude-sonnet-5-5"),
            "the sidebar does not show the pick"
        );
        // The flow's own agents are untouched.
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == Role::Runner)
                .map(|agent| agent.model.as_str()),
            Some("claude-haiku-5-5")
        );
        assert_eq!(
            saved
                .agents
                .iter()
                .find(|agent| agent.role == Role::Reviewer)
                .map(|agent| agent.model.as_str()),
            Some("claude-opus-5-5")
        );

        // The pick survives a restart: the file holds it, and only it.
        let on_disk = Config::read(&config::config_path(&home)).expect("the saved configuration");
        assert_eq!(on_disk, *saved);
        let text = fs::read_to_string(config::config_path(&home)).expect("the file");
        assert!(text.contains("model = \"claude-sonnet-5-5\""), "{text}");
        assert!(text.contains("model = \"claude-haiku-5-5\""), "{text}");
        let (rows, _) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        assert!(
            text_of(&rows).contains("claude-sonnet-5-5"),
            "the sidebar does not draw the pick: {}",
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
        app.fill_choice(Ok(Catalog::from_lines(
            "claude-opus-5-5\nclaude-sonnet-5-5\n",
        )));
        assert_eq!(app.choice.as_ref().expect("the choice").cursor, 0);
        app.on_key(press(KeyCode::Enter));
        assert_eq!(
            app.orchestrator_model(),
            Some("claude-opus-5-5"),
            "the pick dropped the variant in force"
        );
        let text = fs::read_to_string(config::config_path(&home)).expect("the saved file");
        assert!(text.contains("model = \"claude-opus-5-5\""), "{text}");
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
            .footer_line(100)
            .spans
            .iter()
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
        assert!(
            config::config_path(&home).exists(),
            "the pick was not saved"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn esc_closes_the_choice_and_saves_nothing() {
        let home = scratch("choice-esc");
        let mut app = on_orchestrator(Some(home.clone()));
        app.fill_choice(Ok(Catalog::from_lines(
            "claude-sonnet-5-5\nclaude-opus-5-5\n",
        )));
        app.on_key(press(KeyCode::Char('k')));
        app.on_key(press(KeyCode::Esc));
        assert!(app.choice.is_none(), "esc left the choice open");
        assert!(
            !config::config_path(&home).exists(),
            "esc wrote a configuration"
        );
        assert_eq!(
            app.orchestrator_model(),
            Some("claude-opus-5-5"),
            "esc changed the model in force"
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn a_choice_with_no_catalogue_says_why_and_picks_nothing() {
        let home = scratch("choice-error");
        let mut app = on_orchestrator(Some(home.clone()));
        app.fill_choice(Err("opencode models: not found".to_string()));
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
        assert!(app.choice.is_none());
        let _ = fs::remove_dir_all(&home);

        // A catalogue that lists nothing says so, rather than opening blank.
        let home = scratch("choice-empty");
        let mut app = on_orchestrator(Some(home.clone()));
        app.fill_choice(Ok(Catalog::default()));
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
        app.fill_choice(Ok(Catalog::from_lines(&list)));
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
        choosing.fill_choice(Ok(Catalog::from_lines("a/b\nc/d\n")));
        let line: String = choosing
            .footer_line(100)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        for works in [
            CLOSE_KEY, "close", CHOOSE_KEY, "pick", "move", "q", "quit", "?", "help",
        ] {
            assert!(line.contains(works), "{line}: {works}");
        }
        // A narrow footer gives the movement up before the keys that close
        // and pick.
        let narrow: String = choosing
            .footer_line(25)
            .spans
            .iter()
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
        // keys only: enter is on the builder's row, where it opens nothing.
        let mut window = app();
        window.on_key(press(KeyCode::Char('?')));
        assert!(window.help, "? did not open the help");
        let (rows, _) = render_app(&mut window, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(HELP_TITLE), "{text}");
        for says in ["reload", "move", "quit"] {
            assert!(text.contains(says), "{says} is not in the help: {text}");
        }
        let body = rows[..rows.len() - 1].join("\n");
        for dead in ["choose", "pick", "close"] {
            assert!(
                !body.contains(dead),
                "{dead} is in the window's help: {text}"
            );
        }
        let footer = rows.last().expect("a footer");
        assert!(footer.contains("close help"), "{footer}");
        // On the orchestrator's row, enter has a choice to open, and the
        // help names it.
        window.on_key(press(KeyCode::Char('x')));
        window.cursor = 2;
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
    fn a_pick_that_cannot_be_saved_is_named_on_the_sidebar() {
        // The configuration keeps the model in force, and the sidebar says
        // the pick did not land.
        let home = scratch("pick-failure");
        fs::write(
            home.join(coding_crew_core::config::CONFIG_DIR),
            "not a folder",
        )
        .expect("a blocking file");
        let mut app = on_orchestrator(Some(home.clone()));
        app.fill_choice(Ok(Catalog::from_lines(
            "claude-sonnet-5-5\nclaude-opus-5-5\n",
        )));
        app.on_key(press(KeyCode::Char('k')));
        app.on_key(press(KeyCode::Enter));
        assert!(app.choice.is_none());
        assert!(app.notice.is_some(), "the failed save is silent");
        assert_eq!(
            app.orchestrator_model(),
            Some("claude-opus-5-5"),
            "a failed save changed the configuration on screen"
        );
        let (rows, paint) = render_app(&mut app, 100, 40, &RuntimeRead::Absent);
        let text = text_of(&rows);
        assert!(text.contains(sidebar::NOT_SAVED), "{text}");
        // The home is what could not be made, and it is what is named.
        let home_name = config::home_at(&home).display().to_string();
        let shown = cut_to(&home_name, 29);
        assert!(text.contains(&shown), "{text}");
        assert!(!text.contains(CONFIG_FILE), "{text}");
        // The failure is painted on the failure slot.
        assert_eq!(
            colour_of(&rows, &paint, "hugo-app-pick-failure"),
            theme().semantic.error.fg.expect("the error colour")
        );
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn enter_on_a_read_only_agent_opens_nothing() {
        // The builder and the reviewer are the flow's: their rows name a
        // model, and the screen changes none of it.
        let mut app = app();
        app.agents_drawn = true;
        for cursor in [0usize, 1] {
            app.cursor = cursor;
            assert!(!app.can_choose(), "agent {cursor} is choosable");
            app.on_key(press(KeyCode::Enter));
            assert!(app.choice.is_none(), "agent {cursor} opened a choice");
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
        window
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
        assert!(requests[0].env.is_empty());
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
}
