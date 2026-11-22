//! The window: the tab bar, the content pane and the footer.

use std::path::PathBuf;

use crossterm::event::{KeyEvent, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};

use crate::backlog;
use crate::data::Work;
use crate::detail::{self, Detail};
use crate::filter::Filter;
use crate::keys::{self, Act, Scope};
use crate::needs;
use crate::overview;
use crate::path;
use crate::settings;
use crate::tabs::Tab;
use crate::tasks::{self, Order};
use crate::view::View;
use coding_crew_theme::chrome;
use coding_crew_theme::text::text_width;
use coding_crew_theme::{Icon, IconSet, Theme, block_title_style};

/// What a key asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Redraw,
}

/// The state of the window.
pub struct App {
    /// The work folder raul reads.
    pub root: PathBuf,
    /// The active tab.
    pub tab: Tab,
    /// The row selected on the active tab, when the tab has rows.
    pub selection: Option<String>,
    /// The window over the active tab's rows.
    pub view: View,
    /// The open detail pane, when one is open.
    pub detail: Option<Detail>,
    /// The help overlay is open.
    pub help: bool,
    /// The colours of the window.
    pub theme: Theme,
    /// The icon set in force.
    pub icons: IconSet,
    /// The order the Tasks tab keeps its rows in. It lives for the session.
    pub order: Order,
    /// The filter text the Tasks tab narrows its rows by. It lives for the
    /// session.
    pub filter: String,
    /// The filter line is open: the keys type into it.
    pub typing: bool,
    /// A line the window shows under the rule, until the session ends: the
    /// configuration problem raul started past.
    pub notice: Option<String>,
    /// The configuration file, as the Settings tab shows and edits it.
    pub settings: settings::Panel,
}

impl App {
    /// A window with a theme of its own.
    pub fn new(root: PathBuf, theme: Theme, icons: IconSet) -> Self {
        Self {
            root,
            tab: Tab::Overview,
            selection: None,
            view: View::default(),
            detail: None,
            help: false,
            theme,
            icons,
            order: Order::default(),
            filter: String::new(),
            typing: false,
            notice: None,
            settings: settings::Panel::open(None, settings::Overrides::default()),
        }
    }

    /// Handle a key. Returns what the window does next.
    ///
    /// Every key is looked up in the key table: a key the table does not list
    /// does nothing, and the gate checks that every row does its act.
    ///
    /// `now` is the frame's instant: the one the screen on display was drawn
    /// with. The rows a key moves through are the rows that were painted, so
    /// a clock read between the draw and the key cannot reorder the list.
    pub fn on_key(&mut self, key: KeyEvent, work: &Work, now: &str) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Redraw;
        }
        if self.settings.asking() {
            // Quit waits for an answer: the keys that answer it are the only
            // ones that do something.
            return match keys::binding(Scope::Asking, &key).map(|binding| binding.act) {
                Some(Act::Quit | Act::Exit) => Action::Quit,
                Some(Act::Save) => {
                    if self.settings.save() {
                        Action::Quit
                    } else {
                        // The refusal is said in the Settings footer, so the
                        // window goes there: asked from another tab, the
                        // save would fail with no reason on screen.
                        self.settings.stay();
                        self.detail = None;
                        if self.tab != Tab::Settings {
                            self.go_to(Tab::Settings);
                        }
                        Action::Redraw
                    }
                }
                Some(Act::Stay) => {
                    self.settings.stay();
                    Action::Redraw
                }
                _ => Action::Redraw,
            };
        }
        if let Some(detail) = self.detail.as_mut() {
            // The pane is modal: the keys its footer names are the only
            // ones that do something, and every other key is left alone.
            return match keys::binding(Scope::Detail, &key).map(|binding| binding.act) {
                Some(Act::Quit) => self.quit(),
                Some(Act::Exit) => Action::Quit,
                Some(Act::Back) => {
                    self.close_detail();
                    Action::Redraw
                }
                Some(Act::Move(rows)) => {
                    detail.scroll_rows(rows);
                    Action::Redraw
                }
                Some(Act::Page(pages)) => {
                    detail.scroll_pages(pages);
                    Action::Redraw
                }
                Some(Act::Top) => {
                    detail.scroll_to_top();
                    Action::Redraw
                }
                Some(Act::Bottom) => {
                    detail.scroll_to_bottom();
                    Action::Redraw
                }
                _ => Action::Redraw,
            };
        }
        if self.help {
            // The help is a dismissal: any key closes it. The key that
            // returns from the detail pane closes it too — the footer says
            // so — and the keys that quit still leave raul.
            self.help = false;
            let overlay = keys::leaves_overlay(&key);
            return match keys::binding(Scope::Window, &key).map(|binding| binding.act) {
                Some(Act::Quit) if !overlay => self.quit(),
                Some(Act::Exit) => Action::Quit,
                _ => Action::Redraw,
            };
        }
        if self.typing {
            // The filter line takes the keys as its text: no other key of the
            // window answers while it is open.
            return match keys::binding(Scope::Filter, &key).map(|binding| binding.act) {
                Some(Act::Exit) => Action::Quit,
                Some(Act::KeepFilter) => {
                    self.typing = false;
                    Action::Redraw
                }
                Some(Act::ClearFilter) => {
                    self.typing = false;
                    self.filter.clear();
                    Action::Redraw
                }
                Some(Act::EraseFilter) => {
                    erase_grapheme(&mut self.filter);
                    Action::Redraw
                }
                Some(Act::TypeFilter) => {
                    if let Some(letter) = keys::typed(&key) {
                        self.filter.push(letter);
                    }
                    Action::Redraw
                }
                _ => Action::Redraw,
            };
        }
        if self.settings.editing() {
            // A value of the Settings tab takes the keys as its text, as the
            // filter line does.
            return match keys::binding(Scope::Value, &key).map(|binding| binding.act) {
                Some(Act::Exit) => Action::Quit,
                Some(Act::Keep) => {
                    self.settings.keep();
                    self.apply_look();
                    Action::Redraw
                }
                Some(Act::Drop) => {
                    self.settings.drop_edit();
                    Action::Redraw
                }
                Some(Act::EraseValue) => {
                    if let Some(text) = self.settings.typing() {
                        erase_grapheme(text);
                    }
                    Action::Redraw
                }
                Some(Act::TypeValue) => {
                    if let (Some(letter), Some(text)) = (keys::typed(&key), self.settings.typing())
                    {
                        text.push(letter);
                    }
                    Action::Redraw
                }
                _ => Action::Redraw,
            };
        }
        // The row keys are for the tabs that are lists; the window keys work
        // everywhere. The Settings tab is a list too, though enter opens
        // nothing there: enter edits, and the keys that change a choice
        // answer before the window's tab keys do.
        // The Tasks tab's order and filter keys are its own scope: no other
        // tab answers them, so the footer and the help name them there alone.
        // With a filter in force, esc clears it before it quits the window.
        let clearing = (self.tab == Tab::Tasks && self.filter_in_force())
            .then(|| keys::binding(Scope::Tasks, &key))
            .flatten()
            .filter(|binding| binding.act == Act::ClearFilter);
        let act = clearing
            .or_else(|| {
                (self.tab == Tab::Settings)
                    .then(|| keys::binding(Scope::Settings, &key))
                    .flatten()
            })
            .or_else(|| keys::binding(Scope::Window, &key))
            .or_else(|| {
                self.tab
                    .lists()
                    .then(|| keys::binding(Scope::Rows, &key))
                    .flatten()
            })
            .or_else(|| {
                (self.tab == Tab::Tasks)
                    .then(|| keys::binding(Scope::Tasks, &key))
                    .flatten()
            })
            .map(|binding| binding.act);
        match act {
            Some(Act::Quit) => self.quit(),
            Some(Act::Exit) => Action::Quit,
            Some(Act::Help) => {
                self.help = true;
                Action::Redraw
            }
            Some(Act::NextTab) => self.go_to(self.tab.next()),
            Some(Act::PreviousTab) => self.go_to(self.tab.previous()),
            Some(Act::JumpTab) => match keys::digit(&key) {
                Some(digit) if (1..=Tab::ALL.len()).contains(&digit) => {
                    self.go_to(Tab::from_index(digit - 1))
                }
                _ => Action::Redraw,
            },
            Some(Act::Move(rows)) => self.move_selection(work, rows, now),
            Some(Act::Page(pages)) => {
                let page = self.view.page().max(1) as isize;
                self.move_selection(work, pages * page, now)
            }
            Some(Act::Top) => self.move_to(work, 0, now),
            Some(Act::Bottom) => self.move_to(work, usize::MAX, now),
            Some(Act::Open) => {
                // Enter opens the row that is selected, or the tab's first
                // row: the pane opens from every tab whose rows are tasks,
                // and nowhere else.
                if let Some(id) = self.open_target(work, now) {
                    self.selection = Some(id.clone());
                    self.detail = Some(Detail::opened(id, self.tab, self.selection.clone()));
                }
                Action::Redraw
            }
            Some(Act::NextSort) => {
                // The cursor stays on its task: the window follows it to the
                // task's new place.
                self.order = self.order.cycled();
                Action::Redraw
            }
            Some(Act::ReverseSort) => {
                self.order = self.order.reversed();
                Action::Redraw
            }
            Some(Act::Filter) => {
                self.typing = true;
                Action::Redraw
            }
            Some(Act::ClearFilter) => {
                self.filter.clear();
                Action::Redraw
            }
            Some(Act::Change(step)) => {
                if let Some(id) = self.cursor(work, now) {
                    self.settings.change(&id, step);
                }
                self.apply_look();
                Action::Redraw
            }
            Some(Act::Begin) => {
                if let Some(id) = self.cursor(work, now) {
                    self.settings.begin(&id);
                }
                Action::Redraw
            }
            Some(Act::Save) => {
                self.settings.save();
                self.apply_look();
                Action::Redraw
            }
            Some(Act::Undo) => {
                self.settings.undo();
                self.apply_look();
                Action::Redraw
            }
            Some(Act::Reload) => {
                self.settings.reload();
                self.apply_look();
                Action::Redraw
            }
            // The keys of a value being typed and of the question answer in
            // their own scopes, and the filter's keys answer in theirs.
            Some(
                Act::KeepFilter
                | Act::EraseFilter
                | Act::TypeFilter
                | Act::Back
                | Act::Keep
                | Act::Drop
                | Act::EraseValue
                | Act::TypeValue
                | Act::Stay,
            )
            | None => Action::Redraw,
        }
    }

    /// Quit, unless there are unsaved changes: then the footer asks first, and
    /// the next key answers.
    fn quit(&mut self) -> Action {
        if self.settings.unsaved() {
            self.settings.ask();
            Action::Redraw
        } else {
            Action::Quit
        }
    }

    /// Repaint the window from the Settings draft: the theme and the icons at
    /// once, and the Tasks order when the draft's changed.
    fn apply_look(&mut self) {
        self.theme = Theme::preset(self.settings.theme());
        self.icons = self.settings.icons();
        if let Some(order) = self.settings.order_changed() {
            self.order = order;
        }
    }

    /// The rows the cursor moves through on the tab, when it has rows. The
    /// Settings tab's rows are the value lines of the file.
    fn ids(&self, work: &Work, now: &str) -> Option<Vec<String>> {
        match self.tab {
            Tab::Settings => Some(self.settings.ids()),
            tab => rows_of(work, tab, now, self.order, &self.filter),
        }
    }

    /// Whether a filter narrows the Tasks rows: its text reads as words or
    /// tokens. Blank text is no filter.
    fn filter_in_force(&self) -> bool {
        !Filter::parse(&self.filter).is_empty()
    }

    /// Switch to a tab. The selection starts over: the first row of the
    /// tab is the one enter opens.
    fn go_to(&mut self, tab: Tab) -> Action {
        self.tab = tab;
        self.selection = None;
        self.view.reset();
        Action::Redraw
    }

    /// The record enter opens: the selected row, or the tab's first row.
    ///
    /// Only the tabs whose rows are tasks open one; the Settings tab lists
    /// facts and reads.
    fn open_target(&self, work: &Work, now: &str) -> Option<String> {
        if !self.tab.opens() {
            return None;
        }
        self.cursor(work, now)
    }

    /// The row the cursor is on: the selection, or the tab's first row when
    /// nothing is selected yet.
    fn cursor(&self, work: &Work, now: &str) -> Option<String> {
        let ids = self.ids(work, now)?;
        let selected = self.selection.clone().filter(|id| ids.contains(id));
        selected.or_else(|| ids.into_iter().next())
    }

    /// Move the cursor one row, clamped to the tab's rows. A tab without
    /// rows has nowhere to move.
    fn move_selection(&mut self, work: &Work, step: isize, now: &str) -> Action {
        let Some(ids) = self.ids(work, now) else {
            return Action::Redraw;
        };
        if ids.is_empty() {
            return Action::Redraw;
        }
        let current = self
            .cursor(work, now)
            .and_then(|id| ids.iter().position(|other| *other == id))
            .unwrap_or(0);
        let next = (current as isize + step).clamp(0, ids.len() as isize - 1) as usize;
        self.selection = Some(ids[next].clone());
        Action::Redraw
    }

    /// Move the cursor to a row, clamped to the tab's rows. The first and
    /// the last row are the ends the `g` and `G` keys reach.
    fn move_to(&mut self, work: &Work, index: usize, now: &str) -> Action {
        let Some(ids) = self.ids(work, now) else {
            return Action::Redraw;
        };
        let Some(id) = ids.get(index.min(ids.len().saturating_sub(1))) else {
            return Action::Redraw;
        };
        self.selection = Some(id.clone());
        Action::Redraw
    }

    /// Close the detail pane and return to the tab, and the selection, it
    /// came from.
    fn close_detail(&mut self) {
        if let Some(detail) = self.detail.take() {
            self.tab = detail.tab;
            self.selection = detail.selection;
        }
    }

    /// Draw the window.
    ///
    /// `now` is read once per frame by the caller and passed through: the
    /// cursor's row order, the rows that are painted and the footer's
    /// question all answer from one instant.
    pub fn draw(&mut self, frame: &mut Frame, work: &Work, now: &str) {
        let area = frame.area();
        // The notice row is there only while a notice is: a window without one
        // keeps the layout it always had.
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(u16::from(self.notice.is_some())),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);
        let (tabs, rule) = self.tab_bar(rows[1].width, work, now);

        frame.render_widget(self.title_line(rows[0].width, work, now), rows[0]);
        frame.render_widget(tabs, rows[1]);
        frame.render_widget(rule, rows[2]);
        if let Some(notice) = &self.notice {
            let line = Span::styled(notice.clone(), self.theme.semantic.warn);
            frame.render_widget(Line::from(line), rows[3]);
        }
        let body = rows[4];
        // The row the cursor is on is the row enter opens: the highlight
        // and the key agree, including before the first move.
        let selected = self.cursor(work, now);
        let selected = selected.as_deref();
        let hidden = match self.tab {
            Tab::Overview => {
                overview::render(frame, body, work, &self.theme, self.icons, now);
                None
            }
            Tab::NeedsYou => needs::render_view(
                frame,
                body,
                work,
                &self.theme,
                self.icons,
                selected,
                &mut self.view,
                now,
            ),
            Tab::Path => path::render_view(
                frame,
                body,
                work,
                &self.theme,
                self.icons,
                selected,
                &mut self.view,
                now,
            ),
            Tab::Tasks => tasks::render_view(
                frame,
                body,
                work,
                &self.theme,
                self.icons,
                selected,
                &mut self.view,
                self.order,
                &self.filter,
            ),
            Tab::Backlog => backlog::render_view(
                frame,
                body,
                work,
                &self.theme,
                self.icons,
                selected,
                &mut self.view,
            ),
            Tab::Settings => settings::render_view(
                frame,
                body,
                &self.theme,
                self.icons,
                selected,
                &mut self.view,
                &self.settings,
            ),
        };
        if let Some(detail) = self.detail.as_mut() {
            // The pane covers the whole window: its own footer is the one
            // on display, and the help overlay waits behind it. The pane
            // takes the mutable detail: the draw clamps its scroll to the
            // document and measures the page the next keys move by.
            detail::render(frame, area, work, detail, self.icons, &self.theme);
            // A question about unsaved changes is asked over the pane's footer.
            if self.settings.asking() {
                frame.render_widget(self.footer(rows[5].width, work, now, None), rows[5]);
            }
            return;
        }
        frame.render_widget(self.footer(rows[5].width, work, now, hidden), rows[5]);

        if self.help {
            let has_rows = self.ids(work, now).is_some_and(|ids| !ids.is_empty());
            let tasks = self.tab == Tab::Tasks;
            let settings = self.tab == Tab::Settings;
            render_help(frame, area, &self.theme, has_rows, tasks, tasks, settings);
        }
    }

    /// The title line: the app's icon and name, a dim `·`, and the folder it
    /// reads. The right edge holds the status pill: how many tasks need a
    /// human, or that none do.
    ///
    /// A terminal too narrow for the whole folder drops the pill first; too
    /// narrow for the name, the folder takes the line. A part of the line is
    /// never cut in half.
    fn title_line(&self, width: u16, work: &Work, now: &str) -> Line<'static> {
        let name = format!(" {} raul", Icon::AppRaul.glyph(self.icons));
        let separator = " · ";
        let head = text_width(&name) + text_width(separator);
        let folder = self.theme.text.dim;
        let whole = head + text_width(&self.root.display().to_string());
        if let Some(pill) = self.pill(work, now) {
            let room = usize::from(width).checked_sub(whole + pill.width());
            if let Some(gap) = room.filter(|gap| *gap >= 1) {
                return Line::from(vec![
                    Span::styled(name, self.theme.chrome.title),
                    Span::styled(separator, self.theme.text.dim),
                    Span::styled(self.path_tail(usize::MAX), folder),
                    Span::raw(" ".repeat(gap)),
                    pill,
                ]);
            }
        }
        if usize::from(width) < head {
            // The name does not fit: the folder it reads takes the line.
            let path = self.path_tail(usize::from(width));
            return Line::from(Span::styled(path, folder));
        }
        let path = self.path_tail(usize::from(width) - head);
        Line::from(vec![
            Span::styled(name, self.theme.chrome.title),
            Span::styled(separator, self.theme.text.dim),
            Span::styled(path, folder),
        ])
    }

    /// The header's status pill: the tasks that need a human, in the count
    /// the Needs you tab carries, or that nothing does. None when the work
    /// folder is missing: there is no count to show.
    fn pill(&self, work: &Work, now: &str) -> Option<Span<'static>> {
        if work.missing.is_some() {
            return None;
        }
        let count = self.badge(Tab::NeedsYou, work, now)?;
        let verb = if count == 1 { "needs" } else { "need" };
        Some(if count == 0 {
            chrome::pill("nothing needs you", self.theme.semantic.ok)
        } else {
            chrome::pill(&format!("{count} {verb} you"), self.theme.semantic.warn)
        })
    }

    /// The path, in the columns it has: the tail, which names the folder,
    /// is what is kept. The cut is on grapheme boundaries, and the columns
    /// are display columns, so a wide character takes two.
    fn path_tail(&self, columns: usize) -> String {
        let path = self.root.display().to_string();
        if text_width(&path) <= columns {
            return path;
        }
        if columns == 0 {
            return String::new();
        }
        let span = Span::raw(path.as_str());
        let graphemes: Vec<&str> = span
            .styled_graphemes(Style::default())
            .map(|grapheme| grapheme.symbol)
            .collect();
        let mut used = 0;
        let mut kept: Vec<&str> = Vec::new();
        for grapheme in graphemes.iter().rev() {
            let width = text_width(grapheme);
            if used + width > columns - 1 {
                break;
            }
            used += width;
            kept.push(grapheme);
        }
        format!("…{}", kept.iter().rev().copied().collect::<String>())
    }

    /// The tab bar, and the rule under it.
    ///
    /// Each tab is its icon and its name; the two tabs whose count matters
    /// carry it as a badge. The active tab's label is underlined: the rule
    /// is `─`, broken by a `▔` under that label.
    ///
    /// A terminal too narrow for the six tabs gets the active tab alone.
    /// The parts are dropped whole: no label is ever cut in half.
    fn tab_bar(&self, width: u16, work: &Work, now: &str) -> (Line<'static>, Line<'static>) {
        let cells: Vec<Vec<Span<'static>>> = Tab::ALL
            .iter()
            .map(|tab| self.tab_cell(*tab, work, now))
            .collect();
        let full: usize = cells
            .iter()
            .map(|cell| cell.iter().map(Span::width).sum::<usize>())
            .sum::<usize>()
            + Tab::ALL.len()
            - 1;
        if full <= width as usize {
            let mut spans = Vec::new();
            let mut start = 0;
            let mut underline = None;
            for (index, (tab, cell)) in Tab::ALL.iter().zip(cells).enumerate() {
                if index > 0 {
                    spans.push(Span::raw(" "));
                    start += 1;
                }
                let columns: usize = cell.iter().map(Span::width).sum();
                if *tab == self.tab {
                    underline = Some((start, columns));
                }
                start += columns;
                spans.extend(cell);
            }
            return (Line::from(spans), self.rule(width, underline));
        }
        // Too narrow for the six tabs: the active tab, marked. The parts
        // are dropped whole, from the least to the most useful.
        let name = format!(
            "{} {}",
            tab_icon(self.tab).glyph(self.icons),
            self.tab.label()
        );
        let left = Icon::TabMarkerLeft.glyph(self.icons);
        let right = Icon::TabMarkerRight.glyph(self.icons);
        let parts = [
            (
                format!(" {left} {name} {right} "),
                self.tab_style(self.tab),
                true,
            ),
            (format!(" {name} "), self.tab_style(self.tab), true),
            (
                format!("{}/{}", self.tab.index() + 1, Tab::ALL.len()),
                self.theme.chrome.tab_index,
                false,
            ),
        ];
        for (part, style, underlined) in parts {
            let columns = text_width(&part);
            if columns <= usize::from(width) {
                let underline = underlined.then_some((0, columns));
                return (
                    Line::from(Span::styled(part, style)),
                    self.rule(width, underline),
                );
            }
        }
        (Line::from(Span::raw("")), self.rule(width, None))
    }

    /// One tab in the bar: its icon and name, and its badge when it has one.
    fn tab_cell(&self, tab: Tab, work: &Work, now: &str) -> Vec<Span<'static>> {
        let style = self.tab_style(tab);
        let name = format!("{} {}", tab_icon(tab).glyph(self.icons), tab.label());
        let mut spans = vec![Span::styled(format!(" {name}"), style)];
        if let Some(count) = self.badge(tab, work, now) {
            spans.push(Span::styled(" ", style));
            spans.push(chrome::badge(count, &self.theme));
        }
        spans.push(Span::styled(" ", style));
        spans
    }

    /// The count a tab carries as its badge.
    ///
    /// The count is the one the tab's rows are drawn from, so the badge and
    /// the rows cannot disagree.
    fn badge(&self, tab: Tab, work: &Work, now: &str) -> Option<usize> {
        match tab {
            Tab::NeedsYou | Tab::Backlog => {
                rows_of(work, tab, now, self.order, &self.filter).map(|ids| ids.len())
            }
            Tab::Overview | Tab::Path | Tab::Tasks | Tab::Settings => None,
        }
    }

    /// The rule under the tab bar: `─` in the border slot, with the columns
    /// of the active label drawn as `▔` in its slot.
    fn rule(&self, width: u16, underline: Option<(usize, usize)>) -> Line<'static> {
        let width = usize::from(width);
        let (start, columns) = underline.unwrap_or((width, 0));
        let start = start.min(width);
        let columns = columns.min(width - start);
        Line::from(vec![
            Span::styled("─".repeat(start), self.theme.chrome.border),
            Span::styled(
                Icon::TabUnderline.glyph(self.icons).repeat(columns),
                self.theme.chrome.tab_active,
            ),
            Span::styled(
                "─".repeat(width - start - columns),
                self.theme.chrome.border,
            ),
        ])
    }

    /// The style of a tab in the bar.
    fn tab_style(&self, tab: Tab) -> Style {
        if tab == self.tab {
            self.theme.chrome.tab_active
        } else {
            self.theme.chrome.tab_inactive
        }
    }

    /// The footer: the keys that do something on this tab.
    ///
    /// While the filter line is open, the footer is the line: its keys do not
    /// answer, so the footer names none. A filter in force puts its text, the
    /// count it leaves and the token it cannot read in the footer beside the
    /// keys.
    ///
    /// A terminal too narrow for every key keeps the keys that matter most,
    /// and no key is ever cut in half. Enter is named only where it opens:
    /// a tab without rows never offers it, and the Settings tab reads even
    /// though it lists rows. When the content pane had no room to say what
    /// it hides, `hidden` is that count: the footer carries it beside the
    /// keys, and the content pane says nothing about it — the same fact is
    /// never painted twice.
    fn footer(&self, width: u16, work: &Work, now: &str, hidden: Option<usize>) -> Line<'static> {
        if self.settings.asking() {
            // Quit is waiting for an answer: the footer is the question.
            let parts = vec![
                self.part(0, "w", "save", true),
                self.part(1, "q", "quit", true),
                self.part(2, "esc", "stay", true),
            ];
            return footer_line(parts, width);
        }
        if self.typing {
            return footer_line(self.filter_parts(work, usize::from(width)), width);
        }
        if self.settings.editing() {
            let parts = vec![
                self.part(0, "enter", "keep", true),
                self.part(1, "esc", "drop", true),
            ];
            return footer_line(parts, width);
        }
        if self.tab == Tab::Settings {
            return self.settings_footer(width, hidden);
        }
        // A key chip, parted from the chip before it by two spaces.
        let chip = |key: &str, action: &str| {
            [
                vec![Span::raw("  ")],
                chrome::key_chip(key, action, &self.theme),
            ]
            .concat()
        };
        // (where it sits in the line, its spans, whether it matters most)
        let mut parts: Vec<(usize, Vec<Span<'static>>, bool)> =
            vec![(0, chrome::key_chip("q", "quit", &self.theme), true)];
        if let Some(hidden) = hidden {
            parts.push((
                1,
                vec![
                    Span::raw("  "),
                    Span::styled(hidden.to_string(), self.theme.text.strong),
                    Span::styled(" hidden", self.theme.chrome.footer),
                ],
                true,
            ));
        }
        // The filter parts go here, once the room they take is known.
        let filter_at = parts.len();
        parts.push((1, chip("1-6", "tab"), true));
        let mut next = 3;
        let has_rows = rows_of(work, self.tab, now, self.order, &self.filter)
            .is_some_and(|ids| !ids.is_empty());
        // Enter is named only where it opens: the Settings tab is a list
        // whose rows are facts, and the tab reads.
        if has_rows && self.tab.opens() {
            parts.push((next, chip("enter", "open"), true));
            next += 1;
        }
        if has_rows {
            // The movement keys work on every tab with rows: the single
            // rows, the ends and a page. `footer_line` first-fits the
            // optional parts in index order — g/G, then ↑↓ j/k, then
            // pgup/pgdn, then the tab pair — so a narrowing footer gives the
            // pair up first, the page keys next, the ends next, and the
            // single rows last.
            parts.push((next, chip("g/G", "top/bottom"), false));
            next += 1;
            parts.push((next, chip("↑↓ j/k", "move"), false));
            next += 1;
            parts.push((next, chip("pgup/pgdn", "page"), false));
            next += 1;
        }
        if has_rows && self.tab == Tab::Tasks {
            parts.push((next, chip("o/O", "sort"), false));
            next += 1;
        }
        if self.tab == Tab::Tasks {
            parts.push((next, chip("/", "filter"), false));
            next += 1;
        }
        parts.push((next, chip("?", "help"), true));
        if self.help {
            parts.push((next + 1, chip("esc", "close help"), true));
        }
        // The filter parts take the room the needed parts before them leave;
        // the keys after them give way first in a narrow footer.
        let before: usize = parts[..filter_at]
            .iter()
            .filter(|(_, _, needed)| *needed)
            .map(|(_, spans, _)| columns(spans))
            .sum();
        let room = usize::from(width).saturating_sub(before);
        parts.splice(filter_at..filter_at, self.filter_parts(work, room));
        // The tab pair is the last optional part: a pane too narrow for
        // everything gives it up before a key that works on the tab.
        parts.push((2, chip("←→", "switch tab"), false));
        footer_line(parts, width)
    }

    /// A key chip as a footer part, parted from the chip before it by two
    /// spaces unless it is the first.
    fn part(
        &self,
        at: usize,
        key: &str,
        action: &str,
        needed: bool,
    ) -> (usize, Vec<Span<'static>>, bool) {
        let mut spans = Vec::new();
        if at > 0 {
            spans.push(Span::raw("  "));
        }
        spans.extend(chrome::key_chip(key, action, &self.theme));
        (at, spans, needed)
    }

    /// The Settings tab's footer: the keys that change the file, the hidden
    /// count, and the notice of the last refused change or save. A file that
    /// cannot be used offers no change, and says the reload is what it can do.
    fn settings_footer(&self, width: u16, hidden: Option<usize>) -> Line<'static> {
        let mut parts = vec![self.part(0, "q", "quit", true)];
        if let Some(hidden) = hidden {
            parts.push((
                1,
                vec![
                    Span::raw("  "),
                    Span::styled(hidden.to_string(), self.theme.text.strong),
                    Span::styled(" hidden", self.theme.chrome.footer),
                ],
                true,
            ));
        }
        if let Some(notice) = self.settings.notice() {
            // A refused save or change says why, and the reload is offered
            // when the file changed on disk under the draft.
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(notice.to_string(), self.theme.semantic.warn),
            ];
            if self.settings.stale() {
                spans.extend(self.part(0, "r", "reload", true).1);
            }
            parts.push((1, spans, true));
        }
        parts.push(self.part(2, "1-6", "tab", true));
        if !self.settings.read_only() {
            parts.push(self.part(3, "←→", "change", true));
            parts.push(self.part(4, "enter", "edit", true));
            parts.push(self.part(5, "w", "save", true));
        }
        // The movement keys come before the keys that change the file again:
        // a narrow footer gives up the rarer keys first.
        parts.push(self.part(6, "g/G", "top/bottom", false));
        parts.push(self.part(7, "↑↓ j/k", "move", false));
        parts.push(self.part(8, "pgup/pgdn", "page", false));
        if !self.settings.read_only() {
            parts.push(self.part(9, "u", "undo", false));
        }
        if !self.settings.stale() {
            parts.push(self.part(10, "r", "reload", false));
        }
        parts.push(self.part(11, "?", "help", true));
        if self.help {
            parts.push(self.part(12, "esc", "close help", true));
        }
        footer_line(parts, width)
    }

    /// The footer's parts for the filter: the line, with a cursor while it is
    /// open; the count a filter in force leaves; and the token it cannot
    /// read. Empty when no filter is open or in force.
    ///
    /// The parts take `room` columns between them. The token comes first, then
    /// the count if it fits beside it; the line takes what is left, cut from
    /// the left so its tail and its cursor stay in view.
    fn filter_parts(&self, work: &Work, room: usize) -> Vec<(usize, Vec<Span<'static>>, bool)> {
        let open = self.typing;
        if !open && !self.filter_in_force() {
            return Vec::new();
        }
        let theme = &self.theme;
        let count = if self.filter_in_force() {
            let total = work.tasks.len();
            let shown = tasks::row_ids(work, self.order, &self.filter).len();
            vec![
                Span::raw("  "),
                Span::styled(format!("{shown} of {total}"), theme.text.strong),
                Span::styled(
                    format!(" · {} filtered out", total - shown),
                    theme.chrome.footer,
                ),
            ]
        } else {
            Vec::new()
        };
        let error = match Filter::parse(&self.filter).unknown() {
            Some(error) => vec![
                Span::raw("  "),
                Span::styled(error.to_string(), theme.semantic.error),
            ],
            None => Vec::new(),
        };
        // The line's own columns: "/ " and, while open, the cursor.
        let cursor = usize::from(open);
        let fixed = 2 + cursor + columns(&error);
        // The error names the token that matches nothing, so the count gives
        // way to it when both do not fit beside the line.
        let count = if error.is_empty() || fixed + columns(&count) <= room {
            count
        } else {
            Vec::new()
        };
        let text_room = room.saturating_sub(fixed + columns(&count));
        let mut line = vec![
            Span::styled("/ ", theme.text.dim),
            Span::styled(tail(&self.filter, text_room), theme.text.strong),
        ];
        if open {
            line.push(Span::styled(
                " ",
                Style::default().add_modifier(Modifier::REVERSED),
            ));
        }
        let mut parts = vec![(1, line, true)];
        if !count.is_empty() {
            parts.push((1, count, true));
        }
        if !error.is_empty() {
            parts.push((1, error, true));
        }
        parts
    }
}

/// The icon of a tab.
fn tab_icon(tab: Tab) -> Icon {
    match tab {
        Tab::Overview => Icon::TabOverview,
        Tab::NeedsYou => Icon::TabNeeds,
        Tab::Path => Icon::TabPath,
        Tab::Tasks => Icon::TabTasks,
        Tab::Backlog => Icon::TabBacklog,
        Tab::Settings => Icon::TabSettings,
    }
}

/// Delete the last grapheme of the text: what a backspace takes off the line.
fn erase_grapheme(text: &mut String) {
    let length = Span::raw(text.clone())
        .styled_graphemes(Style::default())
        .last()
        .map(|grapheme| grapheme.symbol.len());
    if let Some(length) = length {
        text.truncate(text.len() - length);
    }
}

/// The text cut from the left to `room` columns, with a `…` before the part
/// kept. The cut falls on a grapheme boundary: no character is split.
fn tail(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_string();
    }
    let graphemes: Vec<String> = Span::raw(text.to_string())
        .styled_graphemes(Style::default())
        .map(|grapheme| grapheme.symbol.to_string())
        .collect();
    // The ellipsis takes a column.
    let mut used = 1;
    let mut kept = Vec::new();
    for grapheme in graphemes.iter().rev() {
        let width = grapheme.chars().count();
        if used + width > room {
            break;
        }
        used += width;
        kept.push(grapheme.as_str());
    }
    format!("…{}", kept.into_iter().rev().collect::<String>())
}

/// The columns the spans take in the footer.
fn columns(spans: &[Span]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

/// The footer line: the parts that fit, the ones that matter most first.
pub(crate) fn footer_line(
    parts: Vec<(usize, Vec<Span<'static>>, bool)>,
    width: u16,
) -> Line<'static> {
    // The keys that matter most are kept; the rest only if they fit.
    let mut chosen = vec![false; parts.len()];
    let mut used = 0usize;
    for matters in [true, false] {
        for (index, (_, spans, needed)) in parts.iter().enumerate() {
            if *needed != matters {
                continue;
            }
            let width_of_part = columns(spans);
            if used + width_of_part <= usize::from(width) {
                chosen[index] = true;
                used += width_of_part;
            }
        }
    }
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by_key(|index| parts[*index].0);
    let mut spans = Vec::new();
    for index in order {
        if chosen[index] {
            spans.extend(parts[index].1.clone());
        }
    }
    Line::from(spans)
}

/// The rows a tab holds, when it holds rows the cursor moves through.
///
/// The rows a tab holds, when it holds rows the cursor moves through. The
/// Overview is a summary, not a list: it has no rows.
fn rows_of(work: &Work, tab: Tab, now: &str, order: Order, filter: &str) -> Option<Vec<String>> {
    match tab {
        Tab::NeedsYou => Some(needs::row_ids(work, now)),
        Tab::Path => Some(path::row_ids(work, now)),
        Tab::Tasks => Some(tasks::row_ids(work, order, filter)),
        Tab::Backlog => Some(backlog::row_ids(work)),
        // The Settings rows are the file's value lines: see `App::ids`.
        Tab::Settings | Tab::Overview => None,
    }
}

/// Where the help popup sits: three fifths of the window, centred.
fn help_popup(area: Rect) -> Rect {
    let width = area.width.saturating_mul(3) / 5;
    let height = area.height.saturating_mul(3) / 5;
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

/// The help lines, with the rank that decides which one is dropped first.
///
/// A popup three fifths of a short window cannot hold every line, so the
/// lines give way from the least to the most useful: `shift-tab`, then
/// `h l`, then `tab`, then `← →` (each has a twin that remains), then
/// `g/G`, `o O` and the filter's keys, then `pgup`/`pgdn` (the arrows still
/// move), then `↑↓`/`j`/`k`, then `1-6`. `q` and `?` are dropped last: a
/// human who is stuck needs to leave and to ask what the keys are.
fn help_lines(
    theme: &Theme,
    has_rows: bool,
    sorts: bool,
    filter: bool,
    settings: bool,
) -> Vec<(u8, Line<'static>)> {
    let line = |key: &str, says: &str| {
        Line::from(vec![
            Span::styled(format!(" {key} "), theme.chrome.key),
            Span::styled(format!(" {says}"), theme.text.normal),
        ])
    };
    let mut lines = vec![(7, line("1-6", "jump to a tab"))];
    // On the Settings tab the arrows and h l change a choice, so they do not
    // switch tabs there.
    if settings {
        lines.push((4, line("← →", "change")));
        lines.push((4, line("enter", "edit")));
        lines.push((5, line("w", "save")));
        lines.push((5, line("u", "undo")));
        lines.push((5, line("r", "reload")));
    } else {
        lines.push((3, line("← →", "switch tab")));
        lines.push((1, line("h l", "switch tab")));
    }
    lines.push((2, line("tab", "next tab")));
    lines.push((0, line("shift-tab", "previous tab")));
    if has_rows {
        // The movement keys work on a tab with rows: the help names the
        // same keys as the footer, and never one that does nothing.
        lines.push((6, line("↑↓ j/k", "move")));
        lines.push((5, line("pgup/pgdn", "page")));
        lines.push((4, line("g/G", "top/bottom")));
    }
    if sorts {
        lines.push((4, line("o O", "sort")));
    }
    if filter {
        lines.push((4, line("/", "filter")));
        lines.push((4, line("enter", "keep the filter")));
        lines.push((4, line("esc", "clear the filter")));
    }
    lines.push((u8::MAX, line("q", "quit")));
    lines.push((u8::MAX, line("?", "this help")));
    lines
}

/// The rows a help line takes in an `inner`-column popup.
///
/// The height is the one ratatui draws: the paragraph that will be shown is
/// the one that is measured. A column count is only the minimum: word breaks
/// add rows, so counting columns undercounts and the drop loop could stop
/// with a line still below the popup's last row.
fn help_rows(line: &Line<'_>, inner: usize) -> usize {
    if inner == 0 {
        return usize::MAX;
    }
    let width = u16::try_from(inner).unwrap_or(u16::MAX);
    Paragraph::new(line.clone())
        .wrap(Wrap { trim: true })
        .line_count(width)
        .max(1)
}

/// The help overlay.
fn render_help(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    has_rows: bool,
    sorts: bool,
    filter: bool,
    settings: bool,
) {
    let popup = help_popup(area);
    // The border takes one row at the top and one at the bottom.
    let inner_width = usize::from(popup.width).saturating_sub(2);
    let room = usize::from(popup.height).saturating_sub(2);
    let lines = help_lines(theme, has_rows, sorts, filter, settings);
    let lines: Vec<Line<'static>> = if inner_width == 0 {
        Vec::new()
    } else {
        let rows = |line: &Line<'_>| help_rows(line, inner_width);
        let mut used = lines
            .iter()
            .fold(0usize, |used, (_, line)| used.saturating_add(rows(line)));
        if used <= room {
            lines.into_iter().map(|(_, line)| line).collect()
        } else {
            let mut order: Vec<usize> = (0..lines.len()).collect();
            order.sort_by_key(|index| lines[*index].0);
            let mut dropped = vec![false; lines.len()];
            for index in order {
                if used <= room {
                    break;
                }
                used = used.saturating_sub(rows(&lines[index].1));
                dropped[index] = true;
            }
            lines
                .into_iter()
                .enumerate()
                .filter(|(index, _)| !dropped[*index])
                .map(|(_, (_, line))| line)
                .collect()
        }
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(Line::from(vec![
                    Span::styled("─", theme.chrome.border_focus),
                    Span::styled(" keys ", block_title_style(theme)),
                ]))
                .border_style(theme.chrome.border_focus),
        ),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    // The colour type is named here to read the paint back: no colour is
    // written outside the theme.
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::style::{Color, Modifier};

    use super::*;
    use crate::data::Work;
    use coding_crew_theme::{Icon, IconSet, Preset};

    /// The tests' clock: one instant for the rows and the draw that shows
    /// them. Every fixture entered its status before it.
    const NOW: &str = "2026-11-19T08:00:00Z";

    fn now() -> &'static str {
        NOW
    }

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    /// A window on the mock data, with the default theme.
    fn app() -> App {
        App::new(
            PathBuf::from("/tmp/coding-crew/work"),
            Theme::preset(Preset::Dark),
            IconSet::Nerd,
        )
    }

    /// The window as it is drawn: a theme, a tab, and the help overlay.
    fn window(theme: &Theme, tab: Tab, help: bool) -> App {
        let mut app = App::new(
            PathBuf::from("/tmp/coding-crew/work"),
            *theme,
            IconSet::Nerd,
        );
        app.tab = tab;
        app.help = help;
        app
    }

    /// The paint of one cell: its colours and its attributes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    struct Paint {
        fg: Color,
        bg: Color,
        modifier: Modifier,
    }

    /// Draw the window and return what it shows: the rows, and the paint
    /// of every cell.
    fn render(app: &mut App, width: u16, height: u16) -> (Vec<String>, Vec<Vec<Paint>>) {
        let work = mock_work();
        render_work(app, &work, width, height, now())
    }

    /// Draw the window over `work` at a given instant.
    fn render_work(
        app: &mut App,
        work: &Work,
        width: u16,
        height: u16,
        now: &str,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, work, now))
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
                                modifier: cell.modifier,
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        (rows, paint)
    }

    /// Draw the window with a theme and return the rows it shows.
    fn draw_with(theme: &Theme, width: u16, height: u16) -> Vec<String> {
        let mut app = window(theme, Tab::Overview, false);
        render(&mut app, width, height).0
    }

    /// Draw the window and return the rows it shows.
    fn draw(width: u16, height: u16) -> Vec<String> {
        draw_with(&Theme::preset(Preset::Dark), width, height)
    }
    #[test]
    fn a_narrow_title_drops_whole_parts() {
        let app = app();
        for width in 0..=24 {
            let title = app.title_line(width, &mock_work(), now());
            let text = title
                .spans
                .iter()
                .map(|span| span.content.to_string())
                .collect::<String>();
            assert!(title.width() <= width as usize, "{width}: {text}");
            // The name is whole, or the folder is: a cut of the name never
            // stands alone. Any line that holds the icon must hold all of it.
            let icon = Icon::AppRaul.glyph(IconSet::Nerd);
            if text.contains(icon) {
                assert!(text.contains(&format!(" {icon} raul")), "{width}: {text}");
            }
        }
    }

    #[test]
    fn the_title_names_raul_and_its_pill_sits_on_the_right_edge() {
        let app = app();
        let title = app.title_line(100, &mock_work(), now());
        let text = title
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect::<String>();
        assert!(
            text.starts_with(&format!(
                " {} raul · /tmp/coding-crew/work",
                Icon::AppRaul.glyph(IconSet::Nerd)
            )),
            "{text}"
        );
        assert!(text.ends_with(" 8 need you "), "{text}");
        assert_eq!(title.width(), 100, "{text}");
    }

    #[test]
    fn the_title_drops_the_pill_before_the_folder_is_cut() {
        let app = app();
        let work = mock_work();
        let text = |width: u16| -> String {
            app.title_line(width, &work, now())
                .spans
                .iter()
                .map(|span| span.content.to_string())
                .collect()
        };
        // The name, the separator and the folder take 31 columns, the pill
        // 12, and a column between them: at 43 the pill goes, and the folder
        // keeps its tail.
        let whole = text(44);
        assert!(whole.contains("8 need you"), "{whole}");
        let narrow = text(43);
        assert!(!narrow.contains("need you"), "{narrow}");
        assert!(narrow.ends_with("/tmp/coding-crew/work"), "{narrow}");
    }

    #[test]
    fn a_title_agrees_the_pill_with_one_task() {
        let app = app();
        let mut task = coding_crew_core::Task::new("TSK01", "the one task");
        task.status = coding_crew_core::Status::Groomed;
        let work = Work {
            root: PathBuf::new(),
            tasks: vec![task],
            items: Vec::new(),
            errors: Vec::new(),
            activity: None,
            missing: None,
        };
        let title = app.title_line(100, &work, now());
        let text: String = title
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect();
        assert!(text.ends_with(" 1 needs you "), "{text}");
    }

    #[test]
    fn a_title_with_nothing_to_need_a_human_says_so() {
        let app = app();
        let title = app.title_line(100, &Work::default(), now());
        let text: String = title
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect();
        assert!(text.ends_with(" nothing needs you "), "{text}");
    }

    #[test]
    fn a_title_fits_and_keeps_the_folder_in_columns() {
        // Each of the four wide characters takes two columns: the path is 28
        // characters but 32 columns, so a count of characters lets the whole
        // path through at a width of 41, four columns past the edge.
        let app = App::new(
            PathBuf::from("/home/用户/项目/coding-crew/work"),
            Theme::preset(Preset::Dark),
            IconSet::Nerd,
        );
        for width in 0..=60u16 {
            let title = app.title_line(width, &mock_work(), now());
            let text = title
                .spans
                .iter()
                .map(|span| span.content.to_string())
                .collect::<String>();
            assert!(title.width() <= usize::from(width), "{width}: {text:?}");
            // Once the folder's name fits in the columns it has, the tail is
            // kept whole: the name and its separator take 10 columns, and the
            // folder's name with its ellipsis 17.
            if usize::from(width) >= 10 + 17 {
                assert!(text.contains("coding-crew/work"), "{width}: {text:?}");
            }
        }
    }

    #[test]
    fn a_narrow_tab_bar_drops_whole_parts() {
        let app = app();
        for width in 0..=24 {
            let (bar, _) = app.tab_bar(width, &mock_work(), now());
            let text = bar
                .spans
                .iter()
                .map(|span| span.content.to_string())
                .collect::<String>();
            assert!(text.chars().count() <= width as usize, "{width}: {text}");
            // The active tab alone, the index alone, or nothing: no part
            // of the line is cut in half.
            let glyph = Icon::TabOverview.glyph(IconSet::Nerd);
            let left = Icon::TabMarkerLeft.glyph(IconSet::Nerd);
            let right = Icon::TabMarkerRight.glyph(IconSet::Nerd);
            assert!(
                [
                    format!(" {left} {glyph} Overview {right} "),
                    format!(" {glyph} Overview "),
                    "1/6".to_string(),
                    String::new(),
                ]
                .contains(&text),
                "{width}: {text}"
            );
        }
    }

    #[test]
    fn a_narrow_window_does_not_cut_a_label() {
        let rows = draw(40, 12);
        let text = rows.join("\n");
        assert!(text.contains("Overview"), "{text}");
        assert!(!text.contains("Settings"), "{text}");
        assert!(text.contains("quit"), "{text}");
        assert!(text.contains("help"), "{text}");
    }

    #[test]
    fn a_notice_is_drawn_under_the_rule_and_the_footer_stays_last() {
        // The notice takes a row between the rule and the content. A window
        // without one keeps its rows, and the footer is the last row either way.
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Overview, false);
        let plain = render(&mut app, 100, 20).0;
        app.notice = Some("config.toml [raul] theme: unknown theme light".to_string());
        let noticed = render(&mut app, 100, 20).0;
        assert!(noticed[3].contains("unknown theme light"), "{noticed:?}");
        assert_eq!(noticed.len(), plain.len());
        assert_eq!(noticed.last(), plain.last());
    }

    #[test]
    fn a_narrow_footer_does_not_cut_a_key() {
        let rows = draw(24, 8);
        let footer = rows.last().expect("a footer row").trim_end().to_string();
        // Every key shown is whole: nothing is cut in the middle of a word.
        assert!(footer.contains("quit"), "{footer}");
        assert!(!footer.contains("hel"), "{footer}");
        let line = app().footer(24, &mock_work(), now(), None);
        assert!(line.width() <= 30, "{line:?}");
    }

    #[test]
    fn a_wide_window_shows_every_tab() {
        let rows = draw(100, 20);
        let text = rows.join("\n");
        for tab in Tab::ALL {
            assert!(text.contains(tab.label()), "{tab:?} is missing");
        }
        assert!(text.contains("switch tab"), "{text}");
    }

    #[test]
    fn the_tab_bar_shows_six_tabs_in_key_order() {
        // The six tabs, in the order the keys 1 to 6 reach them: the bar
        // is the visible face of Tab::ALL. Each carries its icon, and the two
        // whose counts matter carry the count of their rows.
        let work = mock_work();
        let needs =
            rows_of(&work, Tab::NeedsYou, now(), Order::default(), "").map_or(0, |ids| ids.len());
        let backlog =
            rows_of(&work, Tab::Backlog, now(), Order::default(), "").map_or(0, |ids| ids.len());
        let glyph = |icon: Icon| icon.glyph(IconSet::Nerd);
        let rows = draw(100, 20);
        let bar = rows.get(1).expect("the tab bar").trim_end();
        let expected = [
            format!(" {} Overview ", glyph(Icon::TabOverview)),
            format!(" {} Needs you {needs} ", glyph(Icon::TabNeeds)),
            format!(" {} Path ", glyph(Icon::TabPath)),
            format!(" {} Tasks ", glyph(Icon::TabTasks)),
            format!(" {} Backlog {backlog} ", glyph(Icon::TabBacklog)),
            format!(" {} Settings ", glyph(Icon::TabSettings)),
        ]
        .join(" ");
        // A bar that drops, adds or reorders a tab disagrees with a window
        // that has six.
        assert_eq!(
            bar,
            expected.trim_end(),
            "the tab bar disagrees with the tabs"
        );
        // The dropped tab is not back.
        assert!(!bar.contains("Autopilot"), "{bar}");
        // Too narrow for the bar, the index counts to six.
        let (narrow, _) = app().tab_bar(8, &work, now());
        let text = narrow
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect::<String>();
        assert_eq!(text, "1/6", "the narrow tab bar counts another window");
    }

    #[test]
    fn the_rule_under_the_tabs_is_broken_under_the_active_tab() {
        // The `▔` sits under the active label, and the rest of the rule is
        // `─`: the rule is one row, the full width of the window.
        let theme = Theme::preset(Preset::Dark);
        let rows = render(&mut window(&theme, Tab::Path, false), 100, 20).0;
        let label = format!(" {} Path ", Icon::TabPath.glyph(IconSet::Nerd));
        let start = rows[1].find(&label).expect("the active label");
        let start = rows[1][..start].chars().count();
        let len = label.chars().count();
        let expected = format!(
            "{}{}{}",
            "─".repeat(start),
            Icon::TabUnderline.glyph(IconSet::Nerd).repeat(len),
            "─".repeat(100 - start - len)
        );
        assert_eq!(rows[2], expected);
    }

    #[test]
    fn the_keys_one_to_six_reach_the_six_tabs_and_seven_does_not() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for (offset, tab) in Tab::ALL.iter().enumerate() {
            let mut app = window(&theme, Tab::Overview, false);
            let digit = char::from(b'1' + offset as u8);
            app.on_key(key(KeyCode::Char(digit)), &work, now());
            assert_eq!(app.tab, *tab, "the key {digit} did not reach {tab:?}");
        }
        // 7 names no tab: the window stays where it is.
        let mut app = window(&theme, Tab::Path, false);
        app.on_key(key(KeyCode::Char('7')), &work, now());
        assert_eq!(app.tab, Tab::Path, "7 moved the window");
    }

    #[test]
    fn the_footer_names_no_dead_tab_key() {
        // The footer and the help name the keys that work: 1-6, never a
        // numbering that a tab is not reached by.
        let theme = Theme::preset(Preset::Dark);
        for tab in Tab::ALL {
            for help in [false, true] {
                let rows = render(&mut window(&theme, tab, help), 100, 20).0;
                let text = rows.join("\n");
                assert!(text.contains("1-6"), "{tab:?} help={help}: {text}");
                assert!(
                    !text.contains("1-7"),
                    "{tab:?} help={help}: the footer names a dead key: {text}"
                );
                assert!(
                    !text.contains("Autopilot"),
                    "{tab:?} help={help}: a dropped tab is named: {text}"
                );
            }
        }
    }

    #[test]
    fn the_backlog_tab_is_built() {
        let rows = render(
            &mut window(&Theme::preset(Preset::Dark), Tab::Backlog, false),
            100,
            40,
        )
        .0;
        let text = rows.join("\n");
        let work = mock_work();
        let ideas = work
            .items
            .iter()
            .filter(|item| item.kind == coding_crew_core::ItemKind::Idea)
            .count();
        let small = work.items.len() - ideas;
        let drafts = work
            .tasks
            .iter()
            .filter(|task| task.status == coding_crew_core::Status::Draft)
            .count();
        assert!(text.contains(&format!("IDEAS  {ideas}")), "{text}");
        assert!(text.contains(&format!("SMALL TASKS  {small}")), "{text}");
        assert!(text.contains(&format!("DRAFTS  {drafts}")), "{text}");
        assert!(!text.contains("not built yet"), "{text}");
    }

    #[test]
    fn the_settings_tab_shows_the_file_as_its_lines() {
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("shows");
        let rows = render(&mut app, 100, 200).0;
        let text = rows.join("\n");
        for line in ["[raul.tasks]", "round_limit = 5", "# raul's own settings"] {
            assert!(text.contains(line), "{line} is missing: {text}");
        }
        assert!(text.contains("~/.config/coding-crew/config.toml"), "{text}");
        assert!(!text.contains("nothing to set yet"), "{text}");
    }

    #[test]
    fn enter_edits_a_value_on_the_settings_tab_and_opens_no_record() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("enter");
        app.selection = Some("harness.driven".to_string());
        app.on_key(key(KeyCode::Enter), &work, now());
        assert!(app.settings.editing(), "enter did not start the edit");
        assert!(app.detail.is_none(), "enter opened a record on Settings");
        app.on_key(key(KeyCode::Esc), &work, now());
        assert!(!app.settings.editing(), "esc did not drop the edit");
        // The rows still move: the tab scrolls like the other lists.
        app.on_key(key(KeyCode::Down), &work, now());
        assert_ne!(app.selection.as_deref(), Some("harness.driven"));
    }

    #[test]
    fn the_settings_footer_names_the_keys_that_change_the_file() {
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("footer");
        let footer = render(&mut app, 160, 20)
            .0
            .last()
            .expect("a footer row")
            .trim_end()
            .to_string();
        for works in [
            "quit",
            "1-6",
            "change",
            "edit",
            "save",
            "undo",
            "reload",
            "move",
            "page",
            "top/bottom",
            "help",
        ] {
            assert!(footer.contains(works), "{footer}: {works} is missing");
        }
        // The arrows change a choice here, so they do not switch tabs.
        assert!(!footer.contains("switch tab"), "{footer}");
        // A file that cannot be used offers no change: the reload is left.
        let mut read_only = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        let footer = render(&mut read_only, 160, 20)
            .0
            .last()
            .expect("a footer row")
            .trim_end()
            .to_string();
        assert!(!footer.contains("save"), "{footer}");
        assert!(footer.contains("reload"), "{footer}");
    }

    #[test]
    fn a_choice_repaints_the_whole_window_and_the_tasks_order() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("repaint");
        app.selection = Some("raul.theme".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        assert_eq!(app.theme.preset, Preset::Nord);
        // The rule under the tab bar is the new theme's border.
        let (_, paint) = render(&mut app, 100, 20);
        assert_eq!(
            paint[2][0].fg,
            Theme::preset(Preset::Nord)
                .chrome
                .border
                .fg
                .expect("the border colour"),
            "the window kept the old theme"
        );
        app.selection = Some("raul.tasks.sort".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        assert_eq!(
            app.order,
            Order {
                key: crate::tasks::Key::Created,
                reverse: false
            },
            "the Tasks order did not follow the draft"
        );
    }

    /// The footer of a window at 120 columns, as text.
    fn footer_of(app: &mut App) -> String {
        render(app, 120, 20)
            .0
            .last()
            .expect("a footer row")
            .trim_end()
            .to_string()
    }

    #[test]
    fn quitting_with_unsaved_changes_asks_in_the_footer_and_stay_keeps_them() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("asks");
        app.selection = Some("raul.theme".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        // q does not quit: the footer asks, and the question is the footer.
        assert_eq!(
            app.on_key(key(KeyCode::Char('q')), &work, now()),
            Action::Redraw
        );
        let footer = footer_of(&mut app);
        for word in ["save", "quit", "stay"] {
            assert!(footer.contains(word), "{footer}: {word} is missing");
        }
        // esc stays, and the change is kept.
        assert_eq!(app.on_key(key(KeyCode::Esc), &work, now()), Action::Redraw);
        assert!(!app.settings.asking());
        assert!(app.settings.unsaved(), "stay dropped the change");
        // ctrl-c quits at once, whatever is unsaved.
        let mut control = key(KeyCode::Char('c'));
        control.modifiers = crossterm::event::KeyModifiers::CONTROL;
        assert_eq!(app.on_key(control, &work, now()), Action::Quit);
    }

    #[test]
    fn quit_then_q_quits_without_saving_and_quit_then_w_saves_first() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("leave");
        app.selection = Some("raul.theme".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        app.on_key(key(KeyCode::Char('q')), &work, now());
        assert_eq!(
            app.on_key(key(KeyCode::Char('q')), &work, now()),
            Action::Quit,
            "q did not quit without saving"
        );
        let path = coding_crew_core::config::config_path(&settings_home("leave"));
        assert!(
            std::fs::read_to_string(&path)
                .expect("the file")
                .contains("theme = \"dark\""),
            "q wrote the change"
        );
        // Quit again, and w saves the change and quits.
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("leave-save");
        app.selection = Some("raul.theme".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        app.on_key(key(KeyCode::Char('q')), &work, now());
        assert_eq!(
            app.on_key(key(KeyCode::Char('w')), &work, now()),
            Action::Quit
        );
        let path = coding_crew_core::config::config_path(&settings_home("leave-save"));
        assert!(
            std::fs::read_to_string(&path)
                .expect("the file")
                .contains("theme = \"nord\""),
            "w did not save before quitting"
        );
    }

    #[test]
    fn a_refused_save_says_why_in_the_footer_and_offers_the_reload() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("refused");
        app.selection = Some("raul.theme".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        let path = coding_crew_core::config::config_path(&settings_home("refused"));
        std::fs::write(
            &path,
            coding_crew_core::config::Config::default()
                .to_toml()
                .replace("round_limit = 5", "round_limit = 6"),
        )
        .expect("someone else's edit");
        app.on_key(key(KeyCode::Char('w')), &work, now());
        let footer = footer_of(&mut app);
        assert!(footer.contains("the file changed on disk"), "{footer}");
        assert!(footer.contains("reload"), "{footer}");
        // The reload takes the file, and the notice goes with the refusal.
        app.on_key(key(KeyCode::Char('r')), &work, now());
        assert!(!app.settings.unsaved());
        assert!(!footer_of(&mut app).contains("the file changed on disk"));
    }

    #[test]
    fn a_save_refused_from_the_quit_question_on_another_tab_says_why_on_settings() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("refused-elsewhere");
        app.selection = Some("raul.theme".to_string());
        app.on_key(key(KeyCode::Right), &work, now());
        app.on_key(key(KeyCode::Char('4')), &work, now());
        assert_eq!(app.tab, Tab::Tasks);
        app.on_key(key(KeyCode::Char('q')), &work, now());
        assert!(app.settings.asking(), "q did not ask");
        let path = coding_crew_core::config::config_path(&settings_home("refused-elsewhere"));
        std::fs::write(
            &path,
            coding_crew_core::config::Config::default()
                .to_toml()
                .replace("round_limit = 5", "round_limit = 6"),
        )
        .expect("someone else's edit");
        assert_eq!(
            app.on_key(key(KeyCode::Char('w')), &work, now()),
            Action::Redraw
        );
        assert_eq!(app.tab, Tab::Settings, "the refusal stayed on Tasks");
        assert!(!app.settings.asking());
        let footer = footer_of(&mut app);
        assert!(footer.contains("the file changed on disk"), "{footer}");
        assert!(footer.contains("reload"), "{footer}");
    }

    #[test]
    fn the_settings_help_names_the_keys_that_change_the_file() {
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, true);
        app.settings = settings_panel("help");
        let rows = render(&mut app, 100, 40).0;
        let help = help_text(&rows, 100, 40);
        for word in ["change", "edit", "save", "undo", "reload"] {
            assert!(help.contains(word), "{help}: {word} is missing");
        }
        assert!(!help.contains("switch tab"), "{help}");
    }

    #[test]
    fn the_settings_tab_is_reached_and_marked_like_the_others() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Overview, false);
        app.on_key(key(KeyCode::Char('6')), &work, now());
        assert_eq!(app.tab, Tab::Settings, "6 did not reach Settings");
        let (rows, paint) = render(&mut app, 100, 20);
        let bar = rows.get(1).expect("the tab bar");
        let column = bar.find(" Settings ").expect("the Settings label");
        let column = bar[..column].chars().count() + 1;
        assert_eq!(
            paint[1][column].fg,
            theme.chrome.tab_active.fg.expect("the active tab colour"),
            "Settings is not marked as the active tab"
        );
    }

    /// A press of the key a binding's first chord names.
    fn press_for(binding: &keys::Binding) -> KeyEvent {
        match binding.chords[0] {
            keys::Chord::Key(code, control) => {
                let mut event = KeyEvent::from(code);
                if control {
                    event.modifiers = KeyModifiers::CONTROL;
                }
                event
            }
            keys::Chord::Digits => KeyEvent::from(KeyCode::Char('6')),
            keys::Chord::Typed => KeyEvent::from(KeyCode::Char('x')),
        }
    }

    /// The scratch home a Settings panel of the tests is under.
    fn settings_home(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("raul-app-{name}-{}", std::process::id()))
    }

    /// A Settings panel over a fresh file of the defaults, in a scratch home.
    fn settings_panel(name: &str) -> settings::Panel {
        let home = settings_home(name);
        let _ = std::fs::remove_dir_all(&home);
        settings::Panel::open(Some(&home), settings::Overrides::default())
    }

    /// The name of a binding in a failure message: its act and its scopes.
    fn named(binding: &keys::Binding) -> String {
        format!("{:?} in {:?}", binding.act, binding.scopes)
    }

    #[test]
    fn every_binding_in_the_table_does_its_act() {
        // The keys group lists the table; the handling reads the table. A
        // row that does nothing, or a key that does something the table
        // does not say, fails here.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for binding in &keys::BINDINGS {
            for scope in binding.scopes {
                // The sort and filter keys answer on the Tasks tab alone; the
                // Settings keys on the Settings tab; the rest on Path.
                let tab = match scope {
                    Scope::Tasks | Scope::Filter => Tab::Tasks,
                    Scope::Settings | Scope::Value | Scope::Asking => Tab::Settings,
                    _ => Tab::Path,
                };
                let mut app = window(&theme, tab, false);
                let ids = rows_of(&work, tab, now(), Order::default(), "")
                    .unwrap_or_else(|| app.ids(&work, now()).expect("a row tab"));
                if *scope == Scope::Detail {
                    let mut detail = Detail::opened("TSK13".to_string(), tab, None);
                    detail.scroll = 5;
                    app.detail = Some(detail);
                } else if *scope == Scope::Filter {
                    // The filter's keys answer while the line is open.
                    app.filter = "ab".to_string();
                    app.on_key(key(KeyCode::Char('/')), &work, now());
                } else if matches!(scope, Scope::Settings | Scope::Value | Scope::Asking) {
                    app.settings = settings_panel(&format!("every-{scope:?}"));
                    app.selection = Some(
                        if binding.act == Act::Begin {
                            "harness.driven"
                        } else {
                            "raul.theme"
                        }
                        .to_string(),
                    );
                    match scope {
                        Scope::Value => app.settings.begin("harness.driven"),
                        Scope::Asking => {
                            app.settings.change("raul.theme", 1);
                            app.settings.ask();
                        }
                        _ => {}
                    }
                    if binding.act == Act::Undo || binding.act == Act::Save {
                        app.settings.change("raul.theme", 1);
                    }
                    if binding.act == Act::Reload {
                        app.settings.change("raul.theme", 1);
                    }
                } else {
                    match binding.act {
                        Act::ClearFilter => app.filter = "x".to_string(),
                        Act::Move(-1) => app.selection = Some(ids[1].clone()),
                        Act::Move(1) => app.selection = Some(ids[0].clone()),
                        Act::Page(pages) => {
                            render(&mut app, 100, 10);
                            let page = app.view.page().min(ids.len() - 1);
                            let at = if pages < 0 { page } else { 0 };
                            app.selection = Some(ids[at].clone());
                        }
                        Act::Top => app.selection = Some(ids[2].clone()),
                        Act::Bottom => app.selection = Some(ids[0].clone()),
                        _ => {}
                    }
                }
                let action = app.on_key(press_for(binding), &work, now());
                match binding.act {
                    Act::Quit | Act::Exit => {
                        assert_eq!(action, Action::Quit, "{}", named(binding));
                    }
                    Act::Help => {
                        assert!(app.help, "{} did not open the help", named(binding));
                    }
                    Act::NextTab => assert_eq!(app.tab, Tab::Tasks, "{}", named(binding)),
                    Act::PreviousTab => assert_eq!(app.tab, Tab::NeedsYou, "{}", named(binding)),
                    Act::JumpTab => assert_eq!(app.tab, Tab::Settings, "{}", named(binding)),
                    Act::NextSort => {
                        assert_eq!(app.order, Order::default().cycled(), "{}", named(binding));
                    }
                    Act::ReverseSort => {
                        assert_eq!(app.order, Order::default().reversed(), "{}", named(binding));
                    }
                    Act::Open => {
                        assert!(app.detail.is_some(), "{} did not open", named(binding));
                    }
                    Act::Back => {
                        assert!(app.detail.is_none(), "{} did not close", named(binding));
                    }
                    Act::Filter => {
                        assert!(app.typing, "{} did not open the filter", named(binding));
                    }
                    Act::KeepFilter => {
                        assert!(!app.typing, "{} did not keep the filter", named(binding));
                        assert_eq!(app.filter, "ab", "{}", named(binding));
                    }
                    Act::ClearFilter => {
                        assert!(!app.typing, "{} kept the line open", named(binding));
                        assert!(app.filter.is_empty(), "{} kept the text", named(binding));
                    }
                    Act::EraseFilter => {
                        assert_eq!(app.filter, "a", "{}", named(binding));
                    }
                    Act::TypeFilter => {
                        assert_eq!(app.filter, "abx", "{}", named(binding));
                    }
                    Act::Change(step) => {
                        // From the Dark theme, a step back is the last theme
                        // and a step forward is the next one.
                        let expected = if step < 0 {
                            Preset::Terminal
                        } else {
                            Preset::Nord
                        };
                        assert_eq!(app.theme.preset, expected, "{}", named(binding));
                    }
                    Act::Begin => {
                        assert!(app.settings.editing(), "{} did not start", named(binding));
                    }
                    Act::Save => match scope {
                        Scope::Asking => {
                            assert_eq!(action, Action::Quit, "{} did not quit", named(binding));
                        }
                        _ => assert!(!app.settings.unsaved(), "{} did not save", named(binding)),
                    },
                    Act::Undo => {
                        assert!(
                            !app.settings.unsaved(),
                            "{} kept the change",
                            named(binding)
                        );
                        assert_eq!(app.theme.preset, Preset::Dark, "{}", named(binding));
                    }
                    Act::Reload => {
                        assert!(!app.settings.unsaved(), "{} kept the draft", named(binding));
                    }
                    Act::Keep | Act::Drop => {
                        assert!(
                            !app.settings.editing(),
                            "{} kept the value open",
                            named(binding)
                        );
                    }
                    Act::EraseValue => {
                        assert!(
                            app.settings.editing(),
                            "{} closed the value",
                            named(binding)
                        );
                    }
                    Act::TypeValue => {
                        let typed = app.settings.typing().expect("the value");
                        assert!(typed.ends_with('x'), "{}: {typed}", named(binding));
                    }
                    Act::Stay => {
                        assert!(
                            !app.settings.asking(),
                            "{} kept the question",
                            named(binding)
                        );
                    }
                    Act::Move(step) => {
                        if *scope == Scope::Detail {
                            let scroll = 5i64 + step as i64;
                            assert_eq!(
                                app.detail.as_ref().expect("the pane").scroll,
                                scroll as u16,
                                "{}",
                                named(binding)
                            );
                        } else {
                            let expected = if step < 0 { &ids[0] } else { &ids[1] };
                            assert_eq!(
                                app.selection.as_deref(),
                                Some(expected.as_str()),
                                "{}",
                                named(binding)
                            );
                        }
                    }
                    Act::Page(pages) => {
                        if *scope == Scope::Detail {
                            let scroll = 5i64 + pages as i64;
                            assert_eq!(
                                app.detail.as_ref().expect("the pane").scroll,
                                scroll as u16,
                                "{}",
                                named(binding)
                            );
                        } else {
                            let page = app.view.page();
                            let expected = if pages < 0 { 0 } else { page };
                            assert_eq!(
                                app.selection.as_deref(),
                                Some(ids[expected].as_str()),
                                "{}",
                                named(binding)
                            );
                        }
                    }
                    Act::Top => {
                        if *scope == Scope::Detail {
                            assert_eq!(app.detail.as_ref().expect("the pane").scroll, 0);
                        } else {
                            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()));
                        }
                    }
                    Act::Bottom => {
                        if *scope == Scope::Detail {
                            assert_eq!(app.detail.as_ref().expect("the pane").scroll, u16::MAX);
                        } else {
                            assert_eq!(app.selection.as_deref(), ids.last().map(String::as_str));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_window_handles_no_key_the_table_does_not_list() {
        // Input handling goes through the table: the window's own source
        // names no key code, so a key cannot act without a row that says so.
        let source = include_str!("app.rs");
        let production = source.split("#[cfg(test)]").next().expect("the tests");
        assert!(
            !production.contains("KeyCode::"),
            "app.rs names a key code outside the table"
        );
        assert!(
            !production.contains("KeyModifiers::"),
            "app.rs names a modifier outside the table"
        );
    }

    #[test]
    fn a_key_the_table_does_not_list_does_nothing() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for code in [KeyCode::Char('7'), KeyCode::Char('x'), KeyCode::Char('c')] {
            let mut app = window(&theme, Tab::Path, false);
            let before = (app.tab, app.selection.clone(), app.detail.clone(), app.help);
            let action = app.on_key(key(code), &work, now());
            assert_eq!(action, Action::Redraw, "{code:?}");
            assert_eq!(
                (app.tab, app.selection.clone(), app.detail.clone(), app.help),
                before,
                "{code:?} moved the window"
            );
        }
    }

    #[test]
    fn esc_quits_in_the_window_and_returns_from_the_detail_pane() {
        // The orchestrator's ruling: esc quits in the window, as it did on
        // HEAD, and returns to the tab from the detail pane. A refactor that
        // drops the quit path fails here.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Path, false);
        assert_eq!(
            app.on_key(key(KeyCode::Esc), &work, now()),
            Action::Quit,
            "esc no longer quits in the window"
        );
        let mut app = window(&theme, Tab::Path, false);
        app.detail = Some(Detail::opened("TSK13".to_string(), Tab::Path, None));
        assert_eq!(
            app.on_key(key(KeyCode::Esc), &work, now()),
            Action::Redraw,
            "esc quit from the detail pane"
        );
        assert!(app.detail.is_none(), "esc did not return to the tab");
    }

    #[test]
    fn esc_closes_the_help_without_leaving_raul() {
        // The help footer names esc as the close key: it closes the overlay
        // and keeps raul open. q still leaves.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Path, true);
        assert_eq!(app.on_key(key(KeyCode::Esc), &work, now()), Action::Redraw);
        assert!(!app.help, "esc did not close the help");
        let mut app = window(&theme, Tab::Path, true);
        assert_eq!(
            app.on_key(key(KeyCode::Char('q')), &work, now()),
            Action::Quit
        );
        assert!(!app.help, "q left the help open");
    }

    #[test]
    fn the_settings_cursor_scrolls_and_stays_drawn() {
        // The Settings tab keeps the row tabs' rule: the cursor is never on
        // a row the pane does not draw, and its line is painted.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let selection = paint_of(theme.text.selected);
        let mut app = window(&theme, Tab::Settings, false);
        app.settings = settings_panel("scrolls");
        let ids = app.settings.ids();
        for height in [12u16, 8, 6] {
            render(&mut app, 100, height);
            for _ in 0..ids.len() + 2 {
                app.on_key(key(KeyCode::Down), &work, now());
                let (rows, paint) = render(&mut app, 100, height);
                let cursor = app.selection.clone().expect("a cursor");
                let index = ids
                    .iter()
                    .position(|id| *id == cursor)
                    .expect("the cursor's row");
                assert!(
                    index >= app.view.first() && index < app.view.first() + app.view.page(),
                    "{cursor} is at {index}, the window is {}..{} at {height}",
                    app.view.first(),
                    app.view.first() + app.view.page()
                );
                // The cursor's line is on the screen, its value named, and
                // painted with the selection after the selection bar.
                let key_name = cursor.rsplit('.').next().expect("a key");
                let (_, row) = rows
                    .iter()
                    .enumerate()
                    .skip(2)
                    .take(rows.len().saturating_sub(3))
                    .find(|(y, _)| paint[*y][2] == selection)
                    .unwrap_or_else(|| panic!("{cursor} is not painted: {rows:?}"));
                assert!(
                    row.contains(&format!("{key_name} = ")),
                    "{cursor}: the selected line is {row:?}"
                );
            }
        }
    }

    /// Every screen the window can show.
    fn screens() -> Vec<(Tab, bool)> {
        let mut screens: Vec<(Tab, bool)> = Tab::ALL.iter().map(|tab| (*tab, false)).collect();
        // The help overlay is a screen of its own.
        screens.push((Tab::Overview, true));
        screens
    }

    #[test]
    fn the_presets_say_the_same_thing() {
        for (tab, help) in screens() {
            // Settings names the theme in use: its words move with the
            // preset by design, and its own test pins the mark.
            if tab == Tab::Settings {
                continue;
            }
            for (width, height) in [(100, 20), (60, 16), (40, 12)] {
                let mut rendered = Vec::new();
                for preset in Preset::ALL {
                    let rows = render(
                        &mut window(&Theme::preset(preset), tab, help),
                        width,
                        height,
                    )
                    .0;
                    rendered.push((preset.name(), rows));
                }
                let (first_name, first) = &rendered[0];
                for (name, rows) in &rendered[1..] {
                    // A theme changes the paint, never the words.
                    assert_eq!(
                        first, rows,
                        "{tab:?} help={help} {width}x{height}: {first_name} and {name} say different things"
                    );
                }
            }
        }
    }

    #[test]
    fn the_presets_differ_in_paint() {
        for (tab, help) in screens() {
            let mut paints = Vec::new();
            for preset in Preset::ALL {
                let paint = render(&mut window(&Theme::preset(preset), tab, help), 100, 20).1;
                paints.push((preset.name(), paint));
            }
            for (index, (one, paint)) in paints.iter().enumerate() {
                for (other, other_paint) in &paints[index + 1..] {
                    assert_ne!(
                        paint, other_paint,
                        "{tab:?} help={help}: {one} and {other} paint the same"
                    );
                }
            }
        }
    }

    #[test]
    fn the_terminal_preset_paints_with_attributes() {
        // In the terminal preset the colours are the terminal's: what
        // tells the presets apart is the attributes.
        let terminal = render(
            &mut window(&Theme::preset(Preset::Terminal), Tab::Overview, false),
            100,
            20,
        )
        .1;
        let attributes = terminal
            .iter()
            .flatten()
            .any(|paint| paint.modifier != Modifier::empty());
        assert!(attributes, "the terminal preset paints no attributes");
    }

    /// A press of the key, as the terminal sends it.
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    /// What a style paints a cell with.
    fn paint_of(style: Style) -> Paint {
        Paint {
            fg: style.fg.unwrap_or(Color::Reset),
            bg: style.bg.unwrap_or(Color::Reset),
            modifier: style.add_modifier,
        }
    }

    /// The cell of the first occurrence of `needle`.
    fn cell_of(rows: &[String], needle: &str) -> Option<(usize, usize)> {
        for (y, row) in rows.iter().enumerate() {
            if let Some(byte) = row.find(needle) {
                return Some((y, row[..byte].chars().count()));
            }
        }
        None
    }

    /// The tabs whose rows the cursor moves through.
    fn row_tabs() -> Vec<Tab> {
        reachability()
            .into_iter()
            .filter_map(|(tab, opens)| opens.then_some(tab))
            .collect()
    }

    /// Every tab that is a list with rows: the row tabs and Settings.
    fn list_tabs() -> Vec<Tab> {
        let work = mock_work();
        Tab::ALL
            .into_iter()
            .filter(|tab| {
                rows_of(&work, *tab, now(), Order::default(), "").is_some_and(|ids| !ids.is_empty())
            })
            .collect()
    }

    #[test]
    fn the_lists_draw_through_the_shared_window() {
        // The window's fields and `settle` are private, so a tab cannot keep
        // its own window. A tab could still stop drawing through
        // `View::draw` and keep local state; `page` is set only by `draw`, so
        // a fresh window that has a page after a render proves the tab went
        // through the shared helper.
        for tab in list_tabs() {
            let mut app = window(&Theme::preset(Preset::Dark), tab, false);
            assert_eq!(
                app.view.page(),
                0,
                "{tab:?}: a fresh window already has a page"
            );
            render(&mut app, 100, 10);
            assert!(
                app.view.page() > 0,
                "{tab:?}: the tab does not draw through View::draw"
            );
        }
    }

    #[test]
    fn the_viewport_clamps_at_both_ends() {
        // At a short pane the tabs have rows below the last visible one.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for tab in list_tabs() {
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a list tab");
            assert!(ids.len() > 4, "{tab:?}: not enough rows to scroll");
            let mut app = window(&theme, tab, false);
            render(&mut app, 100, 10);
            assert!(app.view.page() > 0, "{tab:?}: the pane shows no rows");
            assert!(
                app.view.page() < ids.len(),
                "{tab:?}: every row fits, nothing scrolls"
            );
            // G reaches the last page and the last row.
            app.on_key(key(KeyCode::Char('G')), &work, now());
            render(&mut app, 100, 10);
            assert_eq!(
                app.selection.as_deref(),
                ids.last().map(String::as_str),
                "{tab:?}: G did not reach the last row"
            );
            let bottom = app.view;
            assert_eq!(
                bottom.first() + bottom.page(),
                ids.len(),
                "{tab:?}: the last page is not at the end"
            );
            // Down at the end moves nothing: the viewport clamps.
            app.on_key(key(KeyCode::Down), &work, now());
            render(&mut app, 100, 10);
            assert_eq!(app.view, bottom, "{tab:?}: down ran past the end");
            // g reaches the first row and the top.
            app.on_key(key(KeyCode::Char('g')), &work, now());
            render(&mut app, 100, 10);
            assert_eq!(
                app.selection.as_deref(),
                Some(ids[0].as_str()),
                "{tab:?}: g did not reach the first row"
            );
            assert_eq!(app.view.first(), 0, "{tab:?}: g did not reach the top");
            // Up at the top moves nothing.
            app.on_key(key(KeyCode::Up), &work, now());
            render(&mut app, 100, 10);
            assert_eq!(app.view.first(), 0, "{tab:?}: up ran past the top");
            assert_eq!(
                app.selection.as_deref(),
                Some(ids[0].as_str()),
                "{tab:?}: up left the first row"
            );
        }
    }

    #[test]
    fn one_now_per_frame_keeps_the_row_order_and_the_draw_in_step() {
        // Two tasks at a boundary: before it both have waited zero seconds
        // and the id breaks the tie; after it TSK02 has waited five and
        // comes first. The instant is passed into the frame, so the row the
        // cursor moves through is the row the frame drew. A fresh clock in
        // the draw (the real one is before both stamps) would put TSK01
        // first and fail the check.
        let mut later = coding_crew_core::Task::new("TSK01", "the later stamp");
        later.status = coding_crew_core::Status::Groomed;
        later.status_since = "2030-01-01T00:00:10Z".to_string();
        later.created = later.status_since.clone();
        let mut earlier = coding_crew_core::Task::new("TSK02", "the earlier stamp");
        earlier.status = coding_crew_core::Status::Groomed;
        earlier.status_since = "2030-01-01T00:00:00Z".to_string();
        earlier.created = earlier.status_since.clone();
        let work = Work {
            root: PathBuf::new(),
            tasks: vec![later, earlier],
            ..Work::default()
        };
        let before = "2029-12-31T23:59:55Z";
        let after = "2030-01-01T00:00:05Z";
        assert_eq!(
            rows_of(&work, Tab::NeedsYou, before, Order::default(), "").expect("rows"),
            ["TSK01", "TSK02"],
            "the boundary moved: before it the id breaks a zero tie"
        );
        assert_eq!(
            rows_of(&work, Tab::NeedsYou, after, Order::default(), "").expect("rows"),
            ["TSK02", "TSK01"],
            "the boundary moved: after it TSK02 has waited"
        );
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::NeedsYou, false);
        let (rows, paint) = render_work(&mut app, &work, 100, 40, after);
        let text = rows.join("\n");
        let first = text.find("TSK02").expect("TSK02 is drawn");
        let second = text.find("TSK01").expect("TSK01 is drawn");
        assert!(
            first < second,
            "the draw did not follow the frame's now: {text}"
        );
        // The cursor is painted on the drawn first row, and the next key
        // moves to the drawn second one: one instant for the draw and both
        // keys.
        let (y, x) = cell_of(&rows, "TSK02").expect("the drawn first row");
        assert_eq!(
            paint[y][x],
            paint_of(theme.text.selected),
            "the cursor was not painted on the row the frame drew first: {text}"
        );
        assert_eq!(app.cursor(&work, after).as_deref(), Some("TSK02"));
        app.on_key(key(KeyCode::Down), &work, after);
        assert_eq!(app.selection.as_deref(), Some("TSK01"));
    }

    #[test]
    fn a_pane_with_one_content_row_shows_the_cut_announcement() {
        // One content row, and the first Path row wraps below it: no window
        // can draw a row, so the pane shows only the cut announcement.
        let rows = render(
            &mut window(&Theme::preset(Preset::Dark), Tab::Path, false),
            20,
            6,
        )
        .0;
        let text = rows.join("\n");
        // The entry is wider than the pane, so it gives way whole and the
        // marker says more is hidden.
        assert!(
            rows.iter()
                .any(|row| row.trim_matches(|symbol| symbol == '│' || symbol == ' ') == "…"),
            "the marker is not the pane's only row: {text}"
        );
        assert!(!text.contains("TSK29"), "a row was drawn: {text}");
        // view.rs says what it renders, not that it keeps a window with a
        // row: at this height there is no such window.
        let source = include_str!("view.rs");
        assert!(
            source.contains("one content row"),
            "view.rs does not name the case that renders"
        );
        assert!(
            source.contains("cut announcement"),
            "view.rs does not name what renders"
        );
        assert!(
            !source.contains("keeps the last window"),
            "view.rs still promises a window that draws a row"
        );
    }

    #[test]
    fn the_help_title_leads_with_a_rule_after_the_corner_as_a_pane_does() {
        // The check can fail: the old title reads `╭keys`, not `╭─ keys`.
        let theme = Theme::preset(Preset::Dark);
        let rows = render(&mut window(&theme, Tab::NeedsYou, true), 100, 20).0;
        let popup = help_popup(Rect::new(0, 0, 100, 20));
        let top: String = rows[usize::from(popup.y)]
            .chars()
            .skip(usize::from(popup.x))
            .take(usize::from(popup.width))
            .collect();
        assert!(top.starts_with("╭─ keys "), "{top:?}");
    }

    #[test]
    fn the_help_popup_keeps_q_and_help_on_a_short_window() {
        let theme = Theme::preset(Preset::Dark);
        for height in [14u16, 10] {
            let rows = render(&mut window(&theme, Tab::NeedsYou, true), 100, height).0;
            let text = help_text(&rows, 100, height);
            assert!(
                text.contains("q  quit"),
                "{height}: q is not in the popup: {text}"
            );
            assert!(
                text.contains("?  this help"),
                "{height}: ? is not in the popup: {text}"
            );
        }
        // The drop order the comment states: at 14 the ends survive; at 10
        // they are gone before q and ?.
        let at_14 = help_text(
            &render(&mut window(&theme, Tab::NeedsYou, true), 100, 14).0,
            100,
            14,
        );
        assert!(at_14.contains("g/G"), "{at_14}");
        let at_10 = help_text(
            &render(&mut window(&theme, Tab::NeedsYou, true), 100, 10).0,
            100,
            10,
        );
        assert!(!at_10.contains("g/G"), "{at_10}");
        assert!(!at_10.contains("pgup/pgdn"), "{at_10}");
    }

    #[test]
    fn help_rows_is_the_height_the_line_wraps_to() {
        // Twenty columns of text in ten: the words break at three rows,
        // though the column count says two. The estimate is the height, not
        // a count.
        let line = Line::from(Span::raw("aaaaaa bbbbbb cccccc"));
        assert_eq!(
            Paragraph::new(line.clone())
                .wrap(Wrap { trim: true })
                .line_count(10),
            3
        );
        assert_eq!(help_rows(&line, 10), 3);
    }

    #[test]
    fn the_help_popup_keeps_q_and_help_in_a_narrow_short_window() {
        // A narrow popup wraps its lines, so a column count undercounts the
        // rows: at 24x8 it would stop dropping too early and cut q. The last
        // two lines are the ones a stuck human needs.
        let theme = Theme::preset(Preset::Dark);
        for (width, height) in [(40u16, 10u16), (30, 10), (24, 8)] {
            let rows = render(&mut window(&theme, Tab::NeedsYou, true), width, height).0;
            let text = help_text(&rows, width, height);
            assert!(text.contains("q  quit"), "{width}x{height}: {text}");
            assert!(text.contains("?  this help"), "{width}x{height}: {text}");
        }
    }

    /// The text inside the help popup, without the window beneath it.
    fn help_text(rows: &[String], width: u16, height: u16) -> String {
        let popup = help_popup(Rect::new(0, 0, width, height));
        rows.iter()
            .skip(usize::from(popup.y))
            .take(usize::from(popup.height))
            .map(|row| {
                row.chars()
                    .skip(usize::from(popup.x))
                    .take(usize::from(popup.width))
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[test]
    fn the_cursor_is_never_on_a_row_the_pane_does_not_show() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let selection = paint_of(theme.text.selected);
        for tab in row_tabs() {
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a row tab");
            let mut app = window(&theme, tab, false);
            render(&mut app, 100, 10);
            for _ in 0..ids.len() + 3 {
                app.on_key(key(KeyCode::Down), &work, now());
                let (rows, paint) = render(&mut app, 100, 10);
                let cursor = app.selection.clone().expect("a cursor");
                let index = ids
                    .iter()
                    .position(|id| *id == cursor)
                    .expect("the cursor's row");
                assert!(
                    index >= app.view.first() && index < app.view.first() + app.view.page(),
                    "{tab:?}: {cursor} is at {index}, the window is {}..{}",
                    app.view.first(),
                    app.view.first() + app.view.page()
                );
                assert!(
                    painted_selection(&rows, &paint, &cursor, selection),
                    "{tab:?}: {cursor} is not painted on the screen: {rows:?}"
                );
            }
        }
    }

    /// Whether the row an id names is painted with the selection.
    fn painted_selection(
        rows: &[String],
        paint: &[Vec<Paint>],
        id: &str,
        selection: Paint,
    ) -> bool {
        rows.iter().enumerate().any(|(y, row)| {
            row.match_indices(id).any(|(byte, _)| {
                let x = row[..byte].chars().count();
                paint
                    .get(y)
                    .and_then(|cells| cells.get(x))
                    .is_some_and(|cell| *cell == selection)
            })
        })
    }

    #[test]
    fn a_page_key_moves_by_the_rows_the_pane_shows() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for tab in list_tabs() {
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a list tab");
            let mut app = window(&theme, tab, false);
            render(&mut app, 100, 15);
            let page = app.view.page();
            assert!(page > 1, "{tab:?}: the pane shows {page} rows");
            // One page down moves the cursor by exactly the rows on
            // display.
            app.on_key(key(KeyCode::PageDown), &work, now());
            assert_eq!(
                app.selection.as_deref(),
                Some(ids[page.min(ids.len() - 1)].as_str()),
                "{tab:?}: pgdn did not move a page"
            );
            // Further pages clamp at the last row.
            for _ in 0..ids.len() {
                app.on_key(key(KeyCode::PageDown), &work, now());
            }
            assert_eq!(
                app.selection.as_deref(),
                ids.last().map(String::as_str),
                "{tab:?}: pgdn ran past the last row"
            );
            // A page up moves back by the same distance, and clamps at the
            // first row.
            app.on_key(key(KeyCode::PageUp), &work, now());
            assert_eq!(
                app.selection.as_deref(),
                ids.get(ids.len() - 1 - page).map(String::as_str),
                "{tab:?}: pgup did not move a page back"
            );
            for _ in 0..ids.len() {
                app.on_key(key(KeyCode::PageUp), &work, now());
            }
            assert_eq!(
                app.selection.as_deref(),
                Some(ids[0].as_str()),
                "{tab:?}: pgup ran past the first row"
            );
        }
    }

    #[test]
    fn a_page_key_measured_before_the_first_draw_still_moves_a_row() {
        // The page is what the last draw held; a key pressed before any
        // draw moves one row rather than nothing.
        let work = mock_work();
        let ids = rows_of(&work, Tab::NeedsYou, now(), Order::default(), "").expect("a row tab");
        let mut app = window(&Theme::preset(Preset::Dark), Tab::NeedsYou, false);
        app.on_key(key(KeyCode::PageDown), &work, now());
        assert_eq!(
            app.selection.as_deref(),
            Some(ids[1].as_str()),
            "pgdn before a draw did not move"
        );
    }

    #[test]
    fn the_footer_and_the_help_name_the_movement_keys_that_work() {
        let theme = Theme::preset(Preset::Dark);
        for tab in list_tabs() {
            let mut app = window(&theme, tab, false);
            let rows = render(&mut app, 100, 20).0;
            let footer = rows.last().expect("a footer row").trim_end().to_string();
            for key in ["↑↓", "j/k", "pgup/pgdn", "g/G"] {
                assert!(footer.contains(key), "{tab:?}: {key} is missing: {footer}");
            }
            for words in ["move", "page", "top/bottom"] {
                assert!(
                    footer.contains(words),
                    "{tab:?}: {words} is missing: {footer}"
                );
            }
            // The help overlay names the same keys.
            app.help = true;
            let help = render(&mut app, 100, 20).0.join("\n");
            for key in ["↑↓", "j/k", "pgup/pgdn", "g/G"] {
                assert!(help.contains(key), "{tab:?}: help omits {key}: {help}");
            }
        }
        // The Overview is a summary, not a list: no movement key does
        // anything there, and neither the footer nor the help names one.
        for tab in [Tab::Overview] {
            let rows = render(&mut window(&theme, tab, false), 100, 20).0;
            let footer = rows.last().expect("a footer row").trim_end().to_string();
            for key in ["↑↓", "j/k", "pgup/pgdn", "g/G"] {
                assert!(
                    !footer.contains(key),
                    "{tab:?}: {key} does nothing here: {footer}"
                );
            }
            let help = render(&mut window(&theme, tab, true), 100, 20).0.join("\n");
            for key in ["↑↓", "j/k", "pgup/pgdn", "g/G"] {
                assert!(
                    !help.contains(key),
                    "{tab:?}: help names {key}, which does nothing here: {help}"
                );
            }
        }
    }

    #[test]
    fn the_footer_carries_the_hidden_count_when_the_pane_cannot_announce() {
        // A one-line content pane draws the cursor's row alone. The content
        // pane says nothing about what it hides; the footer carries the count
        // beside the keys, so the same fact is never painted twice.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for (tab, width, height) in [
            (Tab::Backlog, 38u16, 6u16),
            (Tab::NeedsYou, 31, 6),
            (Tab::Path, 80, 6),
            (Tab::Settings, 38, 6),
        ] {
            let mut app = window(&theme, tab, false);
            let rows = render(&mut app, width, height).0;
            let footer = rows.last().expect("a footer row").trim_end().to_string();
            let ids = app.ids(&work, now()).expect("a row tab");
            let hidden = ids.len() - app.view.page();
            assert!(hidden > 0, "{tab:?}: nothing is hidden");
            assert!(
                footer.contains(&format!("{hidden} hidden")),
                "{tab:?}: the footer does not carry {hidden} hidden: {footer}"
            );
            let content = rows[2..rows.len() - 1].join("\n");
            assert!(
                !content.contains("hidden") && !content.contains("not shown"),
                "{tab:?}: the content pane announces too: {content}"
            );
        }
        // Where the content pane does announce, the footer says nothing.
        for tab in [Tab::Backlog, Tab::NeedsYou, Tab::Path, Tab::Settings] {
            let mut app = window(&theme, tab, false);
            let rows = render(&mut app, 100, 12).0;
            let footer = rows.last().expect("a footer row").trim_end().to_string();
            let content = rows[2..rows.len() - 1].join("\n");
            assert!(
                content.contains("hidden") || content.contains("not shown"),
                "{tab:?}: the content pane says nothing: {content}"
            );
            assert!(
                !footer.contains("hidden"),
                "{tab:?}: the footer paints it twice: {footer}"
            );
        }
    }

    #[test]
    fn the_row_tab_footer_degrades_in_the_built_order() {
        // `footer_line` first-fits the optional parts in index order — g/G,
        // then ↑↓ j/k, then pgup/pgdn, then the tab pair — so the pair is the
        // first to go in a narrowing footer, the page keys next, the ends
        // next, and the single rows last. The widths are the ones the review
        // rendered.
        let work = mock_work();
        let app = window(&Theme::preset(Preset::Dark), Tab::Path, false);
        let text = |width: u16| {
            app.footer(width, &work, now(), None)
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        };
        // At 68 the open key and the ends hold; the single rows, the page
        // keys and the tab pair are gone.
        let at_68 = text(68);
        assert!(at_68.contains("enter"), "{at_68}");
        assert!(at_68.contains("open"), "{at_68}");
        assert!(at_68.contains("g/G"), "{at_68}");
        assert!(at_68.contains("top/bottom"), "{at_68}");
        assert!(!at_68.contains("↑↓") && !at_68.contains("j/k"), "{at_68}");
        assert!(!at_68.contains("pgup/pgdn"), "{at_68}");
        assert!(!at_68.contains("switch tab"), "{at_68}");
        // At 56 the ends are gone too; help still fits.
        let at_56 = text(56);
        assert!(at_56.contains("enter") && at_56.contains("open"), "{at_56}");
        assert!(at_56.contains(" help"), "{at_56}");
        assert!(!at_56.contains("g/G"), "{at_56}");
        assert!(!at_56.contains("↑↓") && !at_56.contains("j/k"), "{at_56}");
        assert!(!at_56.contains("pgup/pgdn"), "{at_56}");
        assert!(!at_56.contains("switch tab"), "{at_56}");
        // At 40 help gives way too; the keys that matter most hold, and no
        // movement key is named.
        let at_40 = text(40);
        assert!(at_40.contains(" q ") && at_40.contains("quit"), "{at_40}");
        assert!(at_40.contains("1-6") && at_40.contains(" tab"), "{at_40}");
        assert!(at_40.contains("enter") && at_40.contains("open"), "{at_40}");
        assert!(!at_40.contains("help"), "{at_40}");
        assert!(!at_40.contains("g/G"), "{at_40}");
        assert!(!at_40.contains("↑↓") && !at_40.contains("j/k"), "{at_40}");
        assert!(!at_40.contains("pgup/pgdn"), "{at_40}");
        assert!(!at_40.contains("switch tab"), "{at_40}");
    }

    #[test]
    fn a_scrolled_short_pane_announces_what_it_hides() {
        // The window moves the rows; what is above it and below it is
        // counted in the tab's own announcement. The counts are pinned: a
        // count off by one is a count that lies.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        for tab in row_tabs() {
            let mut app = window(&theme, tab, false);
            render(&mut app, 100, 12);
            app.on_key(key(KeyCode::Char('G')), &work, now());
            let (rows, _) = render(&mut app, 100, 12);
            let text = rows.join("\n");
            // Each line of the announcement is checked on its own: a long one
            // wraps onto the next row of the pane.
            let expected = match tab {
                Tab::NeedsYou => vec![
                    "not shown: approve (2 hidden), review (3 hidden), escalated (0 hidden)"
                        .to_string(),
                ],
                Tab::Path => vec![
                    "not shown: EXECUTION (1 hidden), AWAITING APPROVAL (2 hidden), BLOCKED (3 hidden)"
                        .to_string(),
                    "RUNNING (6 hidden), REVIEW (3 hidden)".to_string(),
                ],
                Tab::Backlog => {
                    // The hidden count is what the group holds minus the rows
                    // the scrolled window draws, both read from the fixtures
                    // and the screen rather than kept by hand.
                    let groups = backlog::groups(&work);
                    let hidden = |group: &backlog::Group| {
                        let shown = group
                            .rows
                            .iter()
                            .filter(|row| text.lines().any(|line| line.contains(row.id.as_str())))
                            .count();
                        group.rows.len() - shown
                    };
                    vec![format!(
                        "not shown: IDEAS ({} hidden), SMALL TASKS ({} hidden), DRAFTS ({} hidden)",
                        hidden(&groups[0]),
                        hidden(&groups[1]),
                        hidden(&groups[2])
                    )]
                }
                Tab::Tasks => {
                    // The count is the rows the scrolled window does not draw,
                    // read from the screen: a task row names its id.
                    let shown = text.lines().filter(|line| line.contains("TSK")).count();
                    vec![format!("{} hidden", work.tasks.len() - shown)]
                }
                _ => unreachable!("a tab without rows"),
            };
            for line in &expected {
                assert!(
                    text.contains(line),
                    "{tab:?}: the scrolled count is not {line:?}: {text}"
                );
            }
        }
    }

    #[test]
    fn the_cursor_is_painted_on_a_short_pane_in_both_directions() {
        // A short pane is where a window hides the cursor: the rows above and
        // below give way to the count. Walking down and up again, the row the
        // cursor lands on is always on the screen, with the highlight on it.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let selection = paint_of(theme.text.selected);
        for tab in row_tabs() {
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a row tab");
            for height in [7u16, 8, 10] {
                let mut app = window(&theme, tab, false);
                render(&mut app, 100, height);
                for code in [KeyCode::Down, KeyCode::Up] {
                    for _ in 0..ids.len() + 2 {
                        app.on_key(key(code), &work, now());
                        let (rows, paint) = render(&mut app, 100, height);
                        let cursor = app.selection.clone().expect("a cursor");
                        let index = ids
                            .iter()
                            .position(|id| *id == cursor)
                            .expect("the cursor's row");
                        assert!(
                            index >= app.view.first() && index < app.view.first() + app.view.page(),
                            "{tab:?} {height}: {cursor} is at {index}, the window is {}..{}",
                            app.view.first(),
                            app.view.first() + app.view.page()
                        );
                        assert!(
                            painted_selection(&rows, &paint, &cursor, selection),
                            "{tab:?} {height}: {cursor} is not painted on the screen: {rows:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_tab_switch_returns_the_viewport_to_the_top() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Path, false);
        render(&mut app, 100, 10);
        app.on_key(key(KeyCode::Char('G')), &work, now());
        render(&mut app, 100, 10);
        assert!(app.view.first() > 0, "the path did not scroll");
        app.on_key(key(KeyCode::Char('1')), &work, now());
        assert_eq!(app.view, View::default(), "the window outlived its tab");
    }

    #[test]
    fn the_cursor_starts_on_the_row_enter_opens_and_the_paint_says_so() {
        // The first row of a tab is what enter opens before any key: the
        // highlight is on it, so the cursor and the key agree.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let selection = paint_of(theme.text.selected);
        for tab in row_tabs() {
            let mut app = window(&theme, tab, false);
            let (rows, paint) = render(&mut app, 100, 40);
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a row tab");
            let first = ids.first().expect("a row");
            let (y, x) = cell_of(&rows, first)
                .unwrap_or_else(|| panic!("{tab:?}: {first} is not on screen"));
            assert_eq!(
                paint[y][x], selection,
                "{tab:?}: the first row is not painted with the selection"
            );
            // The bar runs to the edge of the content pane.
            assert_eq!(
                paint[y][98], selection,
                "{tab:?}: the selection bar stops short of the pane's edge"
            );
            app.on_key(key(KeyCode::Enter), &work, now());
            assert_eq!(
                app.detail.as_ref().expect("the pane opens").id,
                *first,
                "{tab:?}: enter opened another row than the painted one"
            );
        }
    }

    #[test]
    fn the_cursor_follows_the_keys_and_lands_where_enter_will_open() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let selection = paint_of(theme.text.selected);
        for tab in row_tabs() {
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a row tab");
            assert!(ids.len() >= 3, "{tab:?}: not enough rows to move through");
            let mut app = window(&theme, tab, false);
            // Down moves the paint to the second row.
            app.on_key(key(KeyCode::Down), &work, now());
            assert_eq!(app.selection.as_deref(), Some(ids[1].as_str()), "{tab:?}");
            let (rows, paint) = render(&mut app, 100, 40);
            let (y, x) = cell_of(&rows, &ids[1]).expect("the second row");
            assert_eq!(
                paint[y][x], selection,
                "{tab:?}: the paint did not follow the key"
            );
            // The row the paint is on is the row enter opens.
            app.on_key(key(KeyCode::Enter), &work, now());
            assert_eq!(
                app.detail.as_ref().expect("the pane opens").id,
                ids[1],
                "{tab:?}: enter opened another row than the painted one"
            );
            app.on_key(key(KeyCode::Esc), &work, now());
            // Up returns the paint and the cursor to the first row.
            app.on_key(key(KeyCode::Up), &work, now());
            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()), "{tab:?}");
            let (rows, paint) = render(&mut app, 100, 40);
            let (y, x) = cell_of(&rows, &ids[0]).expect("the first row");
            assert_eq!(paint[y][x], selection, "{tab:?}: up did not repaint");
            // `j` and `k` are the same movement as down and up.
            app.on_key(key(KeyCode::Char('j')), &work, now());
            assert_eq!(app.selection.as_deref(), Some(ids[1].as_str()), "{tab:?}");
            app.on_key(key(KeyCode::Char('k')), &work, now());
            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()), "{tab:?}");
            // The cursor stops at the ends rather than running off them.
            app.on_key(key(KeyCode::Up), &work, now());
            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()), "{tab:?}");
            for _ in 0..ids.len() + 2 {
                app.on_key(key(KeyCode::Down), &work, now());
            }
            assert_eq!(
                app.selection.as_deref(),
                ids.last().map(String::as_str),
                "{tab:?}"
            );
        }
    }

    #[test]
    fn esc_returns_the_cursor_and_its_paint_to_the_row_it_came_from() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let selection = paint_of(theme.text.selected);
        for tab in row_tabs() {
            let ids = rows_of(&work, tab, now(), Order::default(), "").expect("a row tab");
            let target = &ids[2];
            let mut app = window(&theme, tab, false);
            app.on_key(key(KeyCode::Down), &work, now());
            app.on_key(key(KeyCode::Down), &work, now());
            app.on_key(key(KeyCode::Enter), &work, now());
            assert_eq!(
                app.detail.as_ref().expect("the pane opens").id,
                *target,
                "{tab:?}"
            );
            app.on_key(key(KeyCode::Esc), &work, now());
            assert_eq!(
                app.selection.as_deref(),
                Some(target.as_str()),
                "{tab:?}: esc lost the selection"
            );
            let (rows, paint) = render(&mut app, 100, 40);
            let (y, x) = cell_of(&rows, target).expect("the returned row");
            assert_eq!(
                paint[y][x], selection,
                "{tab:?}: esc lost the paint it came from"
            );
            let other = &ids[1];
            let (y, x) = cell_of(&rows, other).expect("another row");
            assert_ne!(
                paint[y][x], selection,
                "{tab:?}: the paint did not move back"
            );
        }
    }

    /// The tabs the detail pane opens from, and the tabs it does not.
    fn reachability() -> Vec<(Tab, bool)> {
        vec![
            (Tab::Overview, false),
            (Tab::NeedsYou, true),
            (Tab::Path, true),
            (Tab::Tasks, true),
            (Tab::Backlog, true),
            (Tab::Settings, false),
        ]
    }

    #[test]
    fn enter_opens_the_row_and_esc_returns_to_it() {
        // Enter on a row opens the task as a full-screen overlay; esc
        // returns to the tab and the selection it came from; enter again
        // reopens.
        let work = mock_work();
        for (tab, opens) in reachability() {
            let mut app = app();
            app.tab = tab;
            if !opens {
                app.on_key(key(KeyCode::Enter), &work, now());
                assert!(
                    app.detail.is_none(),
                    "{tab:?}: the pane opened where it does not"
                );
                continue;
            }
            app.on_key(key(KeyCode::Enter), &work, now());
            let detail = app
                .detail
                .as_ref()
                .unwrap_or_else(|| panic!("{tab:?}: the pane did not open"));
            assert_eq!(detail.tab, tab, "{tab:?}");
            let id = detail.id.clone();
            app.on_key(key(KeyCode::Esc), &work, now());
            assert!(app.detail.is_none(), "{tab:?}: the pane did not close");
            assert_eq!(app.tab, tab, "{tab:?}: esc did not return to the tab");
            assert_eq!(
                app.selection.as_deref(),
                Some(id.as_str()),
                "{tab:?}: esc did not return to the selection"
            );
            app.on_key(key(KeyCode::Enter), &work, now());
            assert_eq!(
                app.detail.as_ref().expect("reopened").id,
                id,
                "{tab:?}: enter did not reopen the record"
            );
        }
    }

    #[test]
    fn the_selection_it_came_from_is_returned_to() {
        let work = mock_work();
        let mut app = app();
        app.tab = Tab::Backlog;
        let ids = rows_of(&work, Tab::Backlog, now(), Order::default(), "").expect("rows");
        assert!(ids.len() >= 2, "{ids:?}");
        app.selection = Some(ids[1].clone());
        app.on_key(key(KeyCode::Enter), &work, now());
        assert_eq!(
            app.detail.as_ref().expect("opened").id,
            ids[1],
            "enter opened another row than the one selected"
        );
        app.on_key(key(KeyCode::Esc), &work, now());
        assert_eq!(
            app.selection.as_deref(),
            Some(ids[1].as_str()),
            "esc did not return to the selection it came from"
        );
    }

    #[test]
    fn the_sort_keys_answer_on_the_tasks_tab_alone() {
        // o cycles the key and O reverses it on the Tasks tab; the other tabs
        // do not sort by them.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('o')), &work, now());
        assert_eq!(app.order, Order::default().cycled());
        app.on_key(key(KeyCode::Char('O')), &work, now());
        assert_eq!(app.order, Order::default().cycled().reversed());
        let mut path = window(&theme, Tab::Path, false);
        path.on_key(key(KeyCode::Char('o')), &work, now());
        path.on_key(key(KeyCode::Char('O')), &work, now());
        assert_eq!(
            path.order,
            Order::default(),
            "a sort key sorted another tab"
        );
    }

    #[test]
    fn the_cursor_stays_on_its_task_when_the_order_changes() {
        // The window follows the task: the cursor, and the row enter opens,
        // are the same task in whatever order the rows are in.
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Tasks, false);
        app.selection = Some("TSK13".to_string());
        app.on_key(key(KeyCode::Char('o')), &work, now());
        assert_eq!(app.selection.as_deref(), Some("TSK13"));
        app.on_key(key(KeyCode::Enter), &work, now());
        assert_eq!(
            app.detail.as_ref().map(|detail| detail.id.as_str()),
            Some("TSK13")
        );
    }

    #[test]
    fn the_tasks_footer_and_help_name_the_sort_keys_and_no_other_tab_does() {
        let theme = Theme::preset(Preset::Dark);
        // The footer gives the sort chip up before the page keys at 100
        // columns, as it gives up any optional key; the help names it there.
        let footer = render(&mut window(&theme, Tab::Tasks, false), 120, 20)
            .0
            .join("\n");
        assert!(footer.contains("o/O"), "{footer}");
        assert!(footer.contains("sort"), "{footer}");
        let help = render(&mut window(&theme, Tab::Tasks, true), 100, 20)
            .0
            .join("\n");
        assert!(help.contains("o O"), "{help}");
        for tab in [Tab::Path, Tab::NeedsYou, Tab::Backlog] {
            let text = render(&mut window(&theme, tab, false), 100, 20)
                .0
                .join("\n");
            assert!(!text.contains("o/O"), "{tab:?} names a sort key: {text}");
        }
    }

    #[test]
    fn enter_opens_nothing_without_rows() {
        // A folder with nothing in it: the row tabs have no rows either,
        // and enter does nothing anywhere.
        let work = Work::load(Path::new("/nowhere/coding-crew/work"));
        for (tab, _) in reachability() {
            let mut app = app();
            app.tab = tab;
            app.on_key(key(KeyCode::Enter), &work, now());
            assert!(app.detail.is_none(), "{tab:?}: the pane opened");
        }
    }

    #[test]
    fn the_footer_offers_open_only_where_a_row_can_open() {
        // The footer never names a key that does nothing.
        for (tab, offers) in reachability() {
            let rows = render(
                &mut window(&Theme::preset(Preset::Dark), tab, false),
                100,
                20,
            )
            .0;
            let footer = rows.last().expect("a footer row").trim_end().to_string();
            assert_eq!(
                footer.contains("enter") && footer.contains("open"),
                offers,
                "{tab:?}: {footer}"
            );
        }
    }

    #[test]
    fn the_footer_says_close_while_the_pane_is_open() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Path, false);
        app.on_key(key(KeyCode::Enter), &work, now());
        let rows = render(&mut app, 100, 20).0;
        let footer = rows.last().expect("a footer row").trim_end().to_string();
        assert!(
            footer.contains("esc") && footer.contains("close"),
            "{footer}"
        );
        assert!(
            !(footer.contains("enter") && footer.contains("open")),
            "{footer}: enter has already opened"
        );
    }

    /// Type the text into the filter line, one key at a time.
    fn type_text(app: &mut App, work: &Work, text: &str) {
        for letter in text.chars() {
            app.on_key(key(KeyCode::Char(letter)), work, now());
        }
    }

    #[test]
    fn slash_opens_the_filter_on_the_tasks_tab_alone() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut tasks = window(&theme, Tab::Tasks, false);
        tasks.on_key(key(KeyCode::Char('/')), &work, now());
        assert!(tasks.typing, "/ did not open the filter on Tasks");
        let mut path = window(&theme, Tab::Path, false);
        path.on_key(key(KeyCode::Char('/')), &work, now());
        assert!(!path.typing, "/ opened a filter on Path");
    }

    #[test]
    fn typing_narrows_the_rows_live_and_the_cursor_stays_on_a_visible_row() {
        let work = mock_work();
        let total = rows_of(&work, Tab::Tasks, now(), Order::default(), "").expect("rows");
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(&mut app, &work, "TSK13");
        assert_eq!(app.filter, "TSK13");
        let shown = rows_of(&work, Tab::Tasks, now(), Order::default(), &app.filter).expect("rows");
        assert_eq!(shown, ["TSK13"], "{shown:?} of {} rows", total.len());
        assert_eq!(app.cursor(&work, now()).as_deref(), Some("TSK13"));
        // A selection the narrowing hides is not the cursor: the cursor falls
        // to the first visible row.
        app.selection = Some(total[0].clone());
        assert_eq!(app.cursor(&work, now()).as_deref(), Some("TSK13"));
        // Nothing matches: no row, and no cursor.
        type_text(&mut app, &work, "zzzz");
        assert_eq!(app.cursor(&work, now()), None);
    }

    #[test]
    fn while_typing_the_other_keys_of_the_window_do_not_fire() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        let action = app.on_key(key(KeyCode::Char('q')), &work, now());
        assert_eq!(action, Action::Redraw, "q quit while typing");
        app.on_key(key(KeyCode::Char('2')), &work, now());
        assert_eq!(app.tab, Tab::Tasks, "a digit changed the tab");
        app.on_key(key(KeyCode::Char('o')), &work, now());
        assert_eq!(app.order, Order::default(), "o sorted while typing");
        app.on_key(key(KeyCode::Enter), &work, now());
        assert_eq!(app.filter, "q2o", "the keys were not typed");
        assert!(app.detail.is_none(), "enter opened a row inside the filter");
    }

    #[test]
    fn while_a_settings_value_is_typed_no_other_key_fires() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.settings = settings_panel("typing-keys");
        // An unsaved change for w and u to act on, and a reload for r.
        app.settings.change("raul.theme", 1);
        app.apply_look();
        app.selection = Some("harness.driven".to_string());
        app.on_key(key(KeyCode::Enter), &work, now());
        assert!(app.settings.editing(), "enter did not start the edit");
        for code in [
            KeyCode::Char('q'),
            KeyCode::Char('w'),
            KeyCode::Char('u'),
            KeyCode::Char('r'),
            KeyCode::Char('2'),
            KeyCode::Char('h'),
            KeyCode::Left,
        ] {
            assert_eq!(
                app.on_key(key(code), &work, now()),
                Action::Redraw,
                "{code:?} quit the window"
            );
        }
        assert!(!app.settings.asking(), "q asked about quitting");
        assert!(app.settings.editing(), "a key closed the value");
        assert!(app.settings.unsaved(), "w saved or r reloaded the change");
        assert_eq!(app.theme.preset, Preset::Nord, "u undid the change");
        assert_eq!(app.tab, Tab::Settings, "a digit or h changed the tab");
        let typed = app.settings.typing().expect("the value");
        assert!(typed.ends_with("qwur2h"), "{typed}");
    }

    #[test]
    fn enter_keeps_the_filter_and_returns_to_the_rows() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(&mut app, &work, "TSK13");
        app.on_key(key(KeyCode::Enter), &work, now());
        assert!(!app.typing, "enter did not return to the rows");
        assert_eq!(app.filter, "TSK13", "enter cleared the filter");
        // The rows answer again: q quits.
        assert_eq!(
            app.on_key(key(KeyCode::Char('q')), &work, now()),
            Action::Quit
        );
    }

    #[test]
    fn esc_clears_the_line_while_typing_and_a_kept_filter_before_it_quits() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(&mut app, &work, "x");
        app.on_key(key(KeyCode::Esc), &work, now());
        assert!(
            !app.typing && app.filter.is_empty(),
            "esc while typing kept the text"
        );
        // A kept filter: the first esc clears it and the window stays open.
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(&mut app, &work, "TSK13");
        app.on_key(key(KeyCode::Enter), &work, now());
        assert_eq!(app.on_key(key(KeyCode::Esc), &work, now()), Action::Redraw);
        assert!(app.filter.is_empty(), "esc did not clear the kept filter");
        // With no filter in force, esc quits as it always did.
        assert_eq!(app.on_key(key(KeyCode::Esc), &work, now()), Action::Quit);
    }

    #[test]
    fn backspace_deletes_a_grapheme_not_a_scalar() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        // An e with a combining accent is one grapheme: one backspace takes both.
        type_text(&mut app, &work, "ae\u{301}");
        app.on_key(key(KeyCode::Backspace), &work, now());
        assert_eq!(app.filter, "a");
        app.on_key(key(KeyCode::Backspace), &work, now());
        assert_eq!(app.filter, "");
        app.on_key(key(KeyCode::Backspace), &work, now());
        assert_eq!(app.filter, "", "backspace on an empty line");
    }

    #[test]
    fn the_footer_counts_the_rows_a_filter_leaves() {
        let work = mock_work();
        let total = work.tasks.len();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.filter = "TSK13".to_string();
        let shown = rows_of(&work, Tab::Tasks, now(), Order::default(), &app.filter)
            .expect("rows")
            .len();
        let footer = render_work(&mut app, &work, 100, 20, now()).0;
        let footer = footer.last().expect("a footer row").trim_end().to_string();
        assert!(
            footer.contains(&format!(
                "{shown} of {total} · {} filtered out",
                total - shown
            )),
            "{footer}"
        );
        assert!(
            footer.contains("/ TSK13"),
            "the filter text is not in the footer: {footer}"
        );
    }

    #[test]
    fn the_footer_names_an_unknown_token() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.filter = "colour:red".to_string();
        let footer = render_work(&mut app, &work, 100, 20, now()).0;
        let footer = footer.last().expect("a footer row").trim_end().to_string();
        assert!(footer.contains("unknown key: colour"), "{footer}");
        assert!(footer.contains("0 of"), "{footer}");
    }

    #[test]
    fn a_narrow_footer_keeps_the_tail_of_the_line_and_the_count_while_typing() {
        // The spec's example is longer than the footer at 40 columns: the
        // line gives way from the left, the cursor and the count stay.
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(
            &mut app,
            &work,
            "status:review status:remediating priority:high",
        );
        let footer = render_work(&mut app, &work, 40, 20, now()).0;
        let footer = footer.last().expect("a footer row").to_string();
        assert!(footer.contains("…"), "the line is not cut: {footer}");
        assert!(footer.contains("…rity:high "), "tail or cursor: {footer}");
        assert!(footer.contains(" of "), "the count is dropped: {footer}");
    }

    #[test]
    fn a_narrow_footer_names_an_unknown_token_while_typing() {
        // At 40 columns the count does not fit beside the error: the error
        // stays, and the typed text keeps its cursor.
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(&mut app, &work, "colour:red");
        let footer = render_work(&mut app, &work, 40, 20, now()).0;
        let footer = footer.last().expect("a footer row").trim_end().to_string();
        assert!(footer.contains("unknown key: colour"), "{footer}");
        assert!(footer.contains("colour:red"), "{footer}");
    }

    #[test]
    fn a_narrow_footer_keeps_the_count_once_the_filter_is_kept() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(
            &mut app,
            &work,
            "status:review status:remediating priority:high",
        );
        app.on_key(key(KeyCode::Enter), &work, now());
        let footer = render_work(&mut app, &work, 40, 20, now()).0;
        let footer = footer.last().expect("a footer row").trim_end().to_string();
        assert!(footer.contains("/ …"), "the line is not cut: {footer}");
        assert!(footer.contains(" of "), "the count is dropped: {footer}");
    }

    #[test]
    fn the_tail_cuts_on_a_grapheme_boundary() {
        assert_eq!(tail("abc", 3), "abc");
        assert_eq!(tail("abcdef", 4), "…def");
        // The e with its accent is one grapheme: it is kept whole or not at all.
        assert_eq!(tail("abe\u{301}f", 4), "…e\u{301}f");
        assert_eq!(tail("abe\u{301}f", 3), "…f");
    }

    #[test]
    fn an_empty_result_is_named_in_the_pane_with_the_filter() {
        let work = mock_work();
        for width in [40u16, 100] {
            let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
            app.filter = "zzzz".to_string();
            let rows = render_work(&mut app, &work, width, 20, now()).0.join("\n");
            assert!(rows.contains("no task matches"), "{width}: {rows}");
            assert!(rows.contains("/ zzzz"), "{width}: {rows}");
        }
    }

    #[test]
    fn the_filter_line_is_drawn_with_the_text_while_typing() {
        let work = mock_work();
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Tasks, false);
        app.on_key(key(KeyCode::Char('/')), &work, now());
        type_text(&mut app, &work, "TSK1");
        for width in [40u16, 100] {
            let footer = render_work(&mut app, &work, width, 20, now()).0;
            let footer = footer.last().expect("a footer row").trim_end().to_string();
            assert!(footer.starts_with("/ TSK1"), "{width}: {footer}");
            assert!(
                !footer.contains("quit"),
                "{width}: the keys show while typing: {footer}"
            );
        }
    }

    #[test]
    fn the_help_names_the_filter_keys_on_the_tasks_tab_and_no_other() {
        let theme = Theme::preset(Preset::Dark);
        let help = render(&mut window(&theme, Tab::Tasks, true), 100, 20)
            .0
            .join("\n");
        for words in ["filter", "keep the filter", "clear the filter"] {
            assert!(help.contains(words), "{words} is missing: {help}");
        }
        let path = render(&mut window(&theme, Tab::Path, true), 100, 20)
            .0
            .join("\n");
        assert!(
            !path.contains("keep the filter"),
            "Path names the filter: {path}"
        );
    }
}
