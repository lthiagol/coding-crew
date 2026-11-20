//! The window: one sidebar, one main pane, a title line and a footer.
//!
//! The sidebar is the configuration and the main pane is what is running.
//! There is no tab and no tab bar. The main pane leads with the crew — the
//! reading the human picked — then the unclaimed queue, the round, the budget
//! and the findings, with the stamp at its foot.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use coding_crew_core::parse_rfc3339;
use coding_crew_core::runtime::{AgentState, CrewMember, Runtime};
use coding_crew_theme::text::{cut_to, text_width};
use coding_crew_theme::{Theme, block_title_style, phase_style};

use crate::config::ConfigRead;
use crate::sidebar;

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

/// The title line's text: the name of the tool.
const NAME: &str = " hugo ";

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

/// What a key asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Redraw,
}

/// The state of the window.
pub struct App {
    /// The colours of the window.
    pub theme: Theme,
}

impl App {
    /// A window with a theme of its own.
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Handle a key. The window shows; it does not act on the flow.
    ///
    /// `q` quits, and `ctrl-c` quits as it does in raul; every other key is
    /// left alone. The footer names the only key a screen has room for, and
    /// no key is painted with nothing behind it.
    pub fn on_key(&self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Redraw;
        }
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Redraw,
        }
    }

    /// Draw the window.
    pub fn draw(&self, frame: &mut Frame, runtime: &RuntimeRead, config: &ConfigRead, now: &str) {
        let screen = screen(frame.area());
        frame.render_widget(self.title_line(screen.title.width), screen.title);
        if let Some(area) = screen.announcement {
            frame.render_widget(self.hidden_line(area.width), area);
        }
        if let Some(area) = screen.sidebar {
            self.config_pane(frame, area, config);
        }
        match runtime {
            RuntimeRead::Record(runtime) => self.flow(frame, screen.flow, runtime, now),
            RuntimeRead::Absent => self.empty_pane(frame, screen.flow, FLOW_TITLE, FLOW_EMPTY),
            RuntimeRead::Unreadable(error) => self.unreadable_pane(frame, screen.flow, error),
        }
        frame.render_widget(self.footer_line(screen.footer.width), screen.footer);
    }

    /// The configuration: the sidebar's content, read-only.
    ///
    /// A pane too short for a box gives the box up and keeps the words, as the
    /// main pane does.
    fn config_pane(&self, frame: &mut Frame, area: Rect, config: &ConfigRead) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let (inner, width, height) = if area.height >= 3 && area.width >= 4 {
            let block = Block::bordered()
                .title(SIDEBAR_TITLE)
                .title_style(block_title_style(&self.theme))
                .border_style(self.theme.chrome.border);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            (inner, inner.width, inner.height)
        } else {
            (area, area.width, area.height)
        };
        let lines = sidebar::lines(config, &self.theme, usize::from(width), usize::from(height));
        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// The title line: the name of the tool.
    ///
    /// A terminal too narrow for the name shows nothing rather than a cut of
    /// it: the parts of the line are dropped whole.
    fn title_line(&self, width: u16) -> Line<'static> {
        if usize::from(width) < text_width(NAME) {
            return Line::from("");
        }
        Line::from(Span::styled(NAME, self.theme.chrome.title))
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
        let block = Block::bordered()
            .title(title)
            .title_style(block_title_style(&self.theme))
            .border_style(self.theme.chrome.border);
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
            let block = Block::bordered()
                .title(FLOW_TITLE)
                .title_style(block_title_style(&self.theme))
                .border_style(self.theme.chrome.border);
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
    fn flow(&self, frame: &mut Frame, area: Rect, runtime: &Runtime, now: &str) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        if area.height < 3 || area.width < 4 {
            self.flow_body(frame, area, runtime, now);
            return;
        }
        let block = Block::bordered()
            .title(FLOW_TITLE)
            .title_style(block_title_style(&self.theme))
            .border_style(self.theme.chrome.border);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.flow_body(frame, inner, runtime, now);
    }

    /// The blocks inside the flow's box, and the stamp on the last row.
    fn flow_body(&self, frame: &mut Frame, inner: Rect, runtime: &Runtime, now: &str) {
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
            let sections = self.sections(runtime, body.width.saturating_sub(2), now);
            let boxed: usize = sections.iter().map(|section| section.lines.len() + 2).sum();
            if boxed <= usize::from(body.height) {
                self.draw_boxed(frame, body, &sections);
                return;
            }
        }
        let sections = self.sections(runtime, body.width, now);
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
            let block = Block::bordered()
                .title(section.title)
                .title_style(block_title_style(&self.theme))
                .border_style(self.theme.chrome.border);
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
            lines.push(Line::from(Span::styled(
                section.title,
                self.theme.text.strong,
            )));
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
    fn sections(&self, runtime: &Runtime, width: u16, now: &str) -> Vec<Section> {
        let width = usize::from(width);
        vec![
            Section {
                title: CREW_TITLE,
                lines: self.crew_lines(runtime, width, now),
            },
            Section {
                title: QUEUE_TITLE,
                lines: self.queue_lines(runtime, width),
            },
            Section {
                title: ROUND_TITLE,
                lines: vec![self.round_line(runtime, width)],
            },
            Section {
                title: BUDGET_TITLE,
                lines: vec![self.budget_line(runtime, width)],
            },
            Section {
                title: FINDINGS_TITLE,
                lines: vec![self.findings_line(runtime, width)],
            },
        ]
    }

    /// The crew: one entry per agent, and the composition that says who
    /// reviews.
    ///
    /// The entry is one line while the facts fit it; a narrower pane gives
    /// it two lines, then three. The facts are never dropped.
    fn crew_lines(&self, runtime: &Runtime, width: usize, now: &str) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for member in &runtime.crew {
            lines.extend(self.crew_entry(member, width, now));
        }
        lines.push(self.fit(
            vec![(
                runtime.composition().says().to_string(),
                self.theme.text.normal,
            )],
            width,
        ));
        lines
    }

    /// One agent's entry.
    fn crew_entry(&self, member: &CrewMember, width: usize, now: &str) -> Vec<Line<'static>> {
        let role = member.role.as_str();
        let model = short_model(&member.model);
        let holds = member.holds.as_deref().unwrap_or("—");
        let held = held_length(member.since.as_deref(), now);
        let state = member.state.as_str();
        let state_style = match member.state {
            AgentState::Idle => self.theme.text.dim,
            AgentState::Executing | AgentState::Reviewing => self.theme.semantic.agent,
        };
        let normal = self.theme.text.normal;
        let dim = self.theme.text.dim;

        // One line: role, model, what it holds, how long, and its state.
        let one = format!("{role}  {model}  {holds}  held {held}  {state}");
        if text_width(&one) <= width {
            return vec![self.fit(
                vec![
                    (role.to_string(), normal),
                    ("  ".to_string(), Style::default()),
                    (model.to_string(), dim),
                    ("  ".to_string(), Style::default()),
                    (holds.to_string(), normal),
                    ("  ".to_string(), Style::default()),
                    (format!("held {held}"), dim),
                    ("  ".to_string(), Style::default()),
                    (state.to_string(), state_style),
                ],
                width,
            )];
        }

        // Two lines: the agent and its state, then what it holds.
        let head = format!("{role}  {model}  {state}");
        let tail = format!("  holds {holds} · held {held}");
        if text_width(&head) <= width && text_width(&tail) <= width {
            return vec![
                self.fit(
                    vec![
                        (role.to_string(), normal),
                        ("  ".to_string(), Style::default()),
                        (model.to_string(), dim),
                        ("  ".to_string(), Style::default()),
                        (state.to_string(), state_style),
                    ],
                    width,
                ),
                self.fit(vec![(tail, dim)], width),
            ];
        }

        // Three lines: the agent's state, its model, and what it holds.
        vec![
            self.fit(vec![(format!("{role} · {state}"), normal)], width),
            self.fit(vec![(format!("  {model}"), dim)], width),
            self.fit(vec![(tail, dim)], width),
        ]
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

    /// The external rounds spent of their budget.
    fn budget_line(&self, runtime: &Runtime, width: usize) -> Line<'static> {
        self.fit(
            vec![
                ("external rounds ".to_string(), self.theme.text.dim),
                (runtime.budget.spent.to_string(), self.theme.text.strong),
                (" of ".to_string(), self.theme.text.dim),
                (runtime.budget.total.to_string(), self.theme.text.strong),
            ],
            width,
        )
    }

    /// The open findings, as a count.
    fn findings_line(&self, runtime: &Runtime, width: usize) -> Line<'static> {
        if runtime.findings == 0 {
            return self.fit(
                vec![("no open findings".to_string(), self.theme.text.dim)],
                width,
            );
        }
        self.fit(
            vec![
                (runtime.findings.to_string(), self.theme.text.strong),
                (" open findings".to_string(), self.theme.text.normal),
            ],
            width,
        )
    }

    /// The stamp: whether the automation runs, and why when it does not.
    fn stamp_line(&self, runtime: &Runtime, width: u16) -> Line<'static> {
        let (says, style) = if runtime.stamp.runs {
            (
                format!("● running since {}", runtime.started_at),
                self.theme.semantic.ok,
            )
        } else {
            let why = runtime.stamp.why.as_deref().unwrap_or("no reason given");
            (format!("● not running: {why}"), self.theme.semantic.error)
        };
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

    /// The footer: the keys that work. The window has `q quit`, and `ctrl-c`
    /// quits as in raul; the footer names the key a narrow line can carry.
    fn footer_line(&self, width: u16) -> Line<'static> {
        let spans = vec![
            Span::styled(" q ", self.theme.chrome.key),
            Span::styled(" quit", self.theme.chrome.footer),
        ];
        let used = spans
            .iter()
            .map(|span| text_width(span.content.as_ref()))
            .sum::<usize>();
        if used > usize::from(width) {
            return Line::from("");
        }
        Line::from(spans)
    }
}

/// One block of the main pane, before it is drawn.
struct Section {
    title: &'static str,
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
    age_text((now - since).max(0))
}

/// The length of a hold, in the units raul's ages use.
fn age_text(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < 60 {
        return "under a minute".to_string();
    }
    let days = seconds / 86_400;
    if days > 0 {
        return format!("{days}d");
    }
    let hours = seconds / 3600;
    if hours > 0 {
        return format!("{hours}h");
    }
    format!("{}m", seconds / 60)
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
pub fn screen(area: Rect) -> Screen {
    let sidebar_shown = area.width >= SIDEBAR_MIN_WIDTH;
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);
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
        footer: rows[2],
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;
    use coding_crew_core::RoundKind;
    use coding_crew_core::config::Config;
    use coding_crew_core::runtime::{
        Budget, Composition, CrewMember, QueueItem, RuntimeRound, Stamp,
    };
    use coding_crew_core::{Role, Status};
    use coding_crew_theme::Preset;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
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
            findings: 21,
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
        ConfigRead::Config(Config::default())
    }

    /// Draw the window with a configuration of its own.
    fn render_all(
        width: u16,
        height: u16,
        runtime: &RuntimeRead,
        config: &ConfigRead,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        let app = App::new(theme());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, runtime, config, RENDER_NOW))
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

    #[test]
    fn the_sidebar_is_on_the_left_and_the_main_pane_takes_the_rest() {
        let screen = screen(Rect::new(0, 0, 100, 40));
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
        let both = screen(Rect::new(0, 0, SIDEBAR_MIN_WIDTH, 40));
        let sidebar = both.sidebar.expect("the sidebar at the boundary");
        assert_eq!(sidebar.width, SIDEBAR_WIDTH);
        assert_eq!(both.flow.width, MAIN_MIN_WIDTH);

        let narrow = screen(Rect::new(0, 0, SIDEBAR_MIN_WIDTH - 1, 40));
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
            let screen = screen(Rect::new(0, 0, width, 40));
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
        let (rows, paint) = render_all(100, 40, &RuntimeRead::Absent, &ConfigRead::Absent);
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
    fn the_footer_names_quit_and_no_key_that_does_nothing() {
        let (rows, _) = render(100, 20);
        let footer = rows.last().expect("a footer row");
        assert!(footer.contains('q'), "{footer}");
        assert!(footer.contains("quit"), "{footer}");
        for dead in ["help", "open", "tab", "esc", "move"] {
            assert!(!footer.contains(dead), "{footer}: {dead}");
        }
        // A window too narrow for the key shows no half footer.
        let (rows, _) = render(4, 8);
        assert_eq!(rows.last().expect("a footer row").trim(), "");
    }

    #[test]
    fn q_quits_and_other_keys_do_nothing() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let app = App::new(theme());
        assert_eq!(
            app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Action::Quit
        );
        for code in [
            KeyCode::Char('x'),
            KeyCode::Char('?'),
            KeyCode::Enter,
            KeyCode::Esc,
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

        let app = App::new(theme());
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
        let app = App::new(theme());
        for width in 0..=8u16 {
            let line = app.title_line(width);
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
                ["", " hugo "].contains(&text.as_str()),
                "{width}: a cut of the name: {text}"
            );
        }
        assert_eq!(app.title_line(100).spans[0].content.as_ref(), " hugo ");
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
        assert!(text.contains("external rounds 18 of 45"), "{text}");
        assert!(text.contains("21 open findings"), "{text}");

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
            theme().semantic.error.fg.expect("the error colour")
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
                "held 10d",
                "executing",
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
        // Ten days and eight days before the render's instant.
        assert!(text.contains("held 10d"), "{text}");
        assert!(text.contains("held 8d"), "{text}");
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
        // The crew's five rows go with the crew, and the announcement says so.
        let (rows, _) = render_runtime(60, 8, &runtime());
        let text = rows.join("\n");
        assert!(text.contains("crew (5 hidden)"), "{text}");
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
    fn the_sidebar_carries_the_configuration_and_no_key_changes_it() {
        let (rows, _) = render(100, 40);
        let text = rows.join("\n");
        for fact in [
            "agents",
            "builder · runner",
            "opencode-go",
            "deepseek-v4.1-flash#high",
            "harness",
            "opencode",
            "--standalone · role config",
            "review-round limit",
            "5 build-and-fix rounds",
            "order",
            "1 groom · creator",
            "groomed → approved",
            "6 re-review · reviewer",
        ] {
            assert!(text.contains(fact), "{fact} is not on the sidebar: {text}");
        }
        // The groups are read in the documented order.
        let at = |needle: &str| row_of(&rows, needle);
        assert!(at("agents") < at("harness"), "{text}");
        assert!(at("harness") < at("review-round limit"), "{text}");
        assert!(at("review-round limit") < at("order"), "{text}");

        // Nothing on the sidebar is changed from the screen: no key but quit
        // does anything, and a redraw draws the same configuration.
        let app = App::new(theme());
        for code in [
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char('+'),
            KeyCode::Char('-'),
            KeyCode::Char('h'),
            KeyCode::Char('l'),
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::PageUp,
            KeyCode::PageDown,
        ] {
            assert_eq!(
                app.on_key(KeyEvent::new(code, KeyModifiers::NONE)),
                Action::Redraw,
                "{code:?} acts on the configuration"
            );
        }
        let again = render(100, 40).0;
        assert_eq!(rows, again, "a redraw changed the configuration");
    }

    #[test]
    fn a_configuration_that_cannot_be_read_is_named_on_the_sidebar() {
        let error = ReadError {
            file: "config.json".to_string(),
            error: "failed to parse the configuration: expected value at line 1".to_string(),
        };
        let (rows, paint) = render_all(
            100,
            20,
            &RuntimeRead::Absent,
            &ConfigRead::Unreadable(error),
        );
        let text = rows.join("\n");
        assert!(text.contains("configuration is unreadable"), "{text}");
        assert!(text.contains("config.json"), "{text}");
        assert!(!text.contains("no configuration yet"), "{text}");
        assert_eq!(
            colour_of(&rows, &paint, "failed to parse"),
            theme().semantic.error.fg.expect("the error colour")
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
            "config.json".to_string(),
            "read-only".to_string(),
            "--render --plain".to_string(),
            "100 by 40".to_string(),
            NAME.trim().to_string(),
            sidebar::AGENTS_TITLE.to_string(),
            sidebar::HARNESS_TITLE.to_string(),
            sidebar::LIMIT_TITLE.to_string(),
            sidebar::ORDER_TITLE.to_string(),
            sidebar::UNREADABLE.to_string(),
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
                stamp: Stamp {
                    runs: true,
                    why: None,
                },
            };
            assert_eq!(runtime.composition(), says);
            assert!(doc.contains(says.says()), "the doc does not say {says:?}");
        }
    }
}
