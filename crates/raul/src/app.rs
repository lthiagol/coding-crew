//! The window: the tab bar, the content pane and the footer.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::backlog;
use crate::data::Work;
use crate::detail::{self, Detail};
use crate::needs;
use crate::overview;
use crate::path;
use crate::settings;
use crate::tabs::Tab;
use crate::theme::{Theme, block_title_style};

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
    /// The open detail pane, when one is open.
    pub detail: Option<Detail>,
    /// The help overlay is open.
    pub help: bool,
    /// The colours of the window.
    pub theme: Theme,
}

impl App {
    /// A window with a theme of its own.
    pub fn new(root: PathBuf, theme: Theme) -> Self {
        Self {
            root,
            tab: Tab::Overview,
            selection: None,
            detail: None,
            help: false,
            theme,
        }
    }

    /// Handle a key. Returns what the window does next.
    pub fn on_key(&mut self, key: KeyEvent, work: &Work) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Redraw;
        }
        if let Some(detail) = self.detail.as_mut() {
            // The pane is modal: the keys its footer names are the only
            // ones that do something, and every other key is left alone.
            return match key.code {
                KeyCode::Esc => {
                    self.close_detail();
                    Action::Redraw
                }
                KeyCode::Char('q') => Action::Quit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
                // The scroll keys: the content pane's bindings, named in
                // the pane's footer.
                KeyCode::Up | KeyCode::Char('k') => {
                    detail.scroll_rows(-1);
                    Action::Redraw
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    detail.scroll_rows(1);
                    Action::Redraw
                }
                KeyCode::PageUp => {
                    detail.scroll_pages(-1);
                    Action::Redraw
                }
                KeyCode::PageDown => {
                    detail.scroll_pages(1);
                    Action::Redraw
                }
                KeyCode::Char('g') => {
                    detail.scroll_to_top();
                    Action::Redraw
                }
                KeyCode::Char('G') => {
                    detail.scroll_to_bottom();
                    Action::Redraw
                }
                _ => Action::Redraw,
            };
        }
        if self.help {
            self.help = false;
            return match key.code {
                KeyCode::Char('q') => Action::Quit,
                _ => Action::Redraw,
            };
        }
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Esc => Action::Quit,
            KeyCode::Char('?') => {
                self.help = true;
                Action::Redraw
            }
            KeyCode::Enter => {
                // Enter opens the row that is selected, or the tab's first
                // row: the pane opens from every tab that has rows, and
                // nowhere else.
                if let Some(id) = self.open_target(work) {
                    self.selection = Some(id.clone());
                    self.detail = Some(Detail::opened(id, self.tab, self.selection.clone()));
                }
                Action::Redraw
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(work, 1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(work, -1),
            KeyCode::Char(digit @ '1'..='6') => {
                self.go_to(Tab::from_index(digit as usize - '1' as usize))
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => self.go_to(self.tab.next()),
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.go_to(self.tab.previous())
            }
            _ => Action::Redraw,
        }
    }

    /// Switch to a tab. The selection starts over: the first row of the
    /// tab is the one enter opens.
    fn go_to(&mut self, tab: Tab) -> Action {
        self.tab = tab;
        self.selection = None;
        Action::Redraw
    }

    /// The record enter opens: the selected row, or the tab's first row.
    fn open_target(&self, work: &Work) -> Option<String> {
        self.cursor(work)
    }

    /// The row the cursor is on: the selection, or the tab's first row when
    /// nothing is selected yet.
    fn cursor(&self, work: &Work) -> Option<String> {
        let ids = rows_of(work, self.tab)?;
        let selected = self.selection.clone().filter(|id| ids.contains(id));
        selected.or_else(|| ids.into_iter().next())
    }

    /// Move the cursor one row, clamped to the tab's rows. A tab without
    /// rows has nowhere to move.
    fn move_selection(&mut self, work: &Work, step: isize) -> Action {
        let Some(ids) = rows_of(work, self.tab) else {
            return Action::Redraw;
        };
        if ids.is_empty() {
            return Action::Redraw;
        }
        let current = self
            .cursor(work)
            .and_then(|id| ids.iter().position(|other| *other == id))
            .unwrap_or(0);
        let next = (current as isize + step).clamp(0, ids.len() as isize - 1) as usize;
        self.selection = Some(ids[next].clone());
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
    pub fn draw(&mut self, frame: &mut Frame, work: &Work) {
        let area = frame.area();
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(self.title_line(rows[0].width), rows[0]);
        frame.render_widget(self.tab_bar(rows[1].width), rows[1]);
        // The row the cursor is on is the row enter opens: the highlight
        // and the key agree, including before the first move.
        let selected = self.cursor(work);
        let selected = selected.as_deref();
        match self.tab {
            Tab::Overview => overview::render(frame, rows[2], work, &self.theme),
            Tab::NeedsYou => needs::render(frame, rows[2], work, &self.theme, selected),
            Tab::Path => path::render(frame, rows[2], work, &self.theme, selected),
            Tab::Backlog => backlog::render(frame, rows[2], work, &self.theme, selected),
            Tab::Settings => settings::render(frame, rows[2], &self.theme),
            tab => frame.render_widget(placeholder(tab, &self.theme), rows[2]),
        }
        if let Some(detail) = self.detail.as_mut() {
            // The pane covers the whole window: its own footer is the one
            // on display, and the help overlay waits behind it. The pane
            // takes the mutable detail: the draw clamps its scroll to the
            // document and measures the page the next keys move by.
            detail::render(frame, area, work, detail, &self.theme);
            return;
        }
        frame.render_widget(self.footer(rows[3].width, work), rows[3]);

        if self.help {
            render_help(frame, area, &self.theme);
        }
    }

    /// The title line: the name of the tool and the folder it reads.
    ///
    /// A terminal too narrow for the name gives the whole line to the
    /// folder: a part of the line is never cut in half.
    fn title_line(&self, width: u16) -> Line<'static> {
        const NAME: &str = " coding-lanes ";
        let name = Span::styled(NAME, self.theme.chrome.title);
        let folder = self.theme.text.dim;
        let name_width = NAME.chars().count() as u16;
        if width < name_width {
            // The name does not fit: the folder it reads takes its place.
            let path = self.path_tail(usize::from(width));
            let padding = " ".repeat(usize::from(width).saturating_sub(path.chars().count()));
            return Line::from(vec![Span::raw(padding), Span::styled(path, folder)]);
        }
        let columns = usize::from(width - name_width);
        let path = self.path_tail(columns);
        let padding = " ".repeat(columns.saturating_sub(path.chars().count()));
        Line::from(vec![name, Span::raw(padding), Span::styled(path, folder)])
    }

    /// The path, in the columns it has: the tail, which names the folder,
    /// is what is kept.
    fn path_tail(&self, columns: usize) -> String {
        let path = self.root.display().to_string();
        let length = path.chars().count();
        if length <= columns {
            return path;
        }
        if columns == 0 {
            return String::new();
        }
        let tail: String = path.chars().skip(length - (columns - 1)).collect();
        format!("…{tail}")
    }

    /// The tab bar, with the active tab marked.
    ///
    /// A terminal too narrow for the six tabs gets the active tab alone.
    /// The parts of the line are dropped whole: no label is ever cut in
    /// half.
    fn tab_bar(&self, width: u16) -> Line<'static> {
        let full: usize = Tab::ALL
            .iter()
            .map(|tab| tab.label().chars().count() + 2)
            .sum::<usize>()
            + Tab::ALL.len()
            - 1;
        if full <= width as usize {
            let mut spans = Vec::new();
            for (index, tab) in Tab::ALL.iter().enumerate() {
                if index > 0 {
                    spans.push(Span::raw(" "));
                }
                spans.push(Span::styled(
                    format!(" {} ", tab.label()),
                    self.tab_style(*tab),
                ));
            }
            return Line::from(spans);
        }
        // Too narrow for the six tabs: the active tab, marked. The parts
        // are dropped whole, from the least to the most useful.
        let marked = format!(" ‹ {} › ", self.tab.label());
        let plain = format!(" {} ", self.tab.label());
        let index = format!("{}/{}", self.tab.index() + 1, Tab::ALL.len());
        let parts = [
            (marked, self.tab_style(self.tab)),
            (plain, self.tab_style(self.tab)),
            (index, self.theme.chrome.tab_index),
        ];
        for (part, style) in parts {
            if part.chars().count() <= width as usize {
                return Line::from(vec![Span::styled(part, style)]);
            }
        }
        Line::from(Span::raw(""))
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
    /// A terminal too narrow for every key keeps the keys that matter most,
    /// and no key is ever cut in half. Enter is named only where it opens:
    /// a tab without rows never offers it.
    fn footer(&self, width: u16, work: &Work) -> Line<'static> {
        let key = self.theme.chrome.key;
        let label = self.theme.chrome.footer;
        let space = || Span::raw("   ");
        // (where it sits in the line, its spans, whether it matters most)
        let mut parts: Vec<(usize, Vec<Span<'static>>, bool)> = vec![
            (
                0,
                vec![Span::styled(" q ", key), Span::styled(" quit", label)],
                true,
            ),
            (
                1,
                vec![
                    space(),
                    Span::styled(" 1-6 ", key),
                    Span::styled(" tab", label),
                ],
                true,
            ),
            (
                2,
                vec![
                    space(),
                    Span::styled(" ←→ ", key),
                    Span::styled(" switch tab", label),
                ],
                false,
            ),
        ];
        let mut next = 3;
        if rows_of(work, self.tab).is_some_and(|ids| !ids.is_empty()) {
            parts.push((
                next,
                vec![
                    space(),
                    Span::styled(" enter ", key),
                    Span::styled(" open", label),
                ],
                true,
            ));
            next += 1;
        }
        parts.push((
            next,
            vec![
                space(),
                Span::styled(" ? ", key),
                Span::styled(" help", label),
            ],
            true,
        ));
        if self.help {
            parts.push((
                next + 1,
                vec![
                    space(),
                    Span::styled(" esc ", key),
                    Span::styled(" close help", label),
                ],
                true,
            ));
        }
        footer_line(parts, width)
    }
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
            let columns = spans
                .iter()
                .map(|span| span.content.chars().count())
                .sum::<usize>();
            if used + columns <= usize::from(width) {
                chosen[index] = true;
                used += columns;
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

/// The rows a tab holds, when it holds rows the detail pane can open.
///
/// The Overview is a summary, not a list: it has no rows to open. The
/// Autopilot tab is a placeholder, and the Settings tab lists nothing yet.
fn rows_of(work: &Work, tab: Tab) -> Option<Vec<String>> {
    match tab {
        Tab::NeedsYou => Some(needs::row_ids(work, &coding_lanes_core::now_rfc3339())),
        Tab::Path => Some(path::row_ids(work)),
        Tab::Backlog => Some(backlog::row_ids(work)),
        Tab::Overview | Tab::Autopilot | Tab::Settings => None,
    }
}

/// The tab that is not built yet.
fn placeholder(tab: Tab, theme: &Theme) -> Paragraph<'static> {
    Paragraph::new(Line::from(Span::styled(
        format!("the {} tab is not built yet", tab.label()),
        theme.text.dim,
    )))
    .wrap(Wrap { trim: true })
    .block(
        Block::bordered()
            .title(tab.label())
            .title_style(block_title_style(theme))
            .border_style(theme.chrome.border),
    )
}

/// The help overlay.
fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let width = area.width.saturating_mul(3) / 5;
    let height = area.height.saturating_mul(3) / 5;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    let key = theme.chrome.key;
    let label = theme.text.normal;
    let lines = vec![
        Line::from(vec![
            Span::styled(" 1-6 ", key),
            Span::styled(" jump to a tab", label),
        ]),
        Line::from(vec![
            Span::styled(" ← → ", key),
            Span::styled(" switch tab", label),
        ]),
        Line::from(vec![
            Span::styled(" h l ", key),
            Span::styled(" switch tab", label),
        ]),
        Line::from(vec![
            Span::styled(" tab ", key),
            Span::styled(" next tab", label),
        ]),
        Line::from(vec![
            Span::styled(" shift-tab ", key),
            Span::styled(" previous tab", label),
        ]),
        Line::from(vec![Span::styled(" q ", key), Span::styled(" quit", label)]),
        Line::from(vec![
            Span::styled(" ? ", key),
            Span::styled(" this help", label),
        ]),
    ];
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .title("keys")
                .title_style(block_title_style(theme))
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
    use ratatui::style::{Color, Modifier};

    use super::*;
    use crate::data::Work;
    use crate::theme::Preset;

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-lanes/work"))
    }

    /// A window on the mock data, with the default theme.
    fn app() -> App {
        App::new(
            PathBuf::from("/tmp/coding-lanes/work"),
            Theme::preset(Preset::Dark),
        )
    }

    /// The window as it is drawn: a theme, a tab, and the help overlay.
    fn window(theme: &Theme, tab: Tab, help: bool) -> App {
        let mut app = App::new(PathBuf::from("/tmp/coding-lanes/work"), *theme);
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
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, &work))
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
            let title = app.title_line(width);
            let text = title
                .spans
                .iter()
                .map(|span| span.content.to_string())
                .collect::<String>();
            assert!(text.chars().count() <= width as usize, "{width}: {text}");
            // The name is whole, or the folder is.
            if text.contains("coding-lane") {
                assert!(text.contains("coding-lanes"), "{width}: {text}");
            }
        }
    }

    #[test]
    fn a_narrow_tab_bar_drops_whole_parts() {
        let app = app();
        for width in 0..=24 {
            let bar = app.tab_bar(width);
            let text = bar
                .spans
                .iter()
                .map(|span| span.content.to_string())
                .collect::<String>();
            assert!(text.chars().count() <= width as usize, "{width}: {text}");
            // The active tab alone, the index alone, or nothing: no part
            // of the line is cut in half.
            assert!(
                [" ‹ Overview › ", " Overview ", "1/6", ""].contains(&text.as_str()),
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
    fn a_narrow_footer_does_not_cut_a_key() {
        let rows = draw(30, 8);
        let footer = rows.last().expect("a footer row").trim_end().to_string();
        // Every key shown is whole: nothing is cut in the middle of a word.
        assert!(footer.contains("quit"), "{footer}");
        assert!(!footer.contains("hel"), "{footer}");
        assert!(footer.chars().count() <= 30, "{footer}");
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
    fn the_backlog_tab_is_built() {
        let rows = render(
            &mut window(&Theme::preset(Preset::Dark), Tab::Backlog, false),
            100,
            40,
        )
        .0;
        let text = rows.join("\n");
        assert!(text.contains("IDEAS · 1"), "{text}");
        assert!(text.contains("SMALL TASKS · 4"), "{text}");
        assert!(text.contains("DRAFTS · 5"), "{text}");
        assert!(!text.contains("not built yet"), "{text}");
    }

    #[test]
    fn the_settings_tab_says_what_it_will_hold() {
        let rows = render(
            &mut window(&Theme::preset(Preset::Dark), Tab::Settings, false),
            100,
            20,
        )
        .0;
        let text = rows.join("\n");
        // A named empty state, not a blank screen.
        assert!(text.contains("nothing to set yet"), "{text}");
        // What the spec says the tab will hold.
        assert!(text.contains("keybinds"), "{text}");
        assert!(text.contains("sort keys"), "{text}");
        assert!(text.contains("theme"), "{text}");
        // The Settings placeholder is its own, not the generic one.
        assert!(!text.contains("not built yet"), "{text}");
    }

    #[test]
    fn the_settings_tab_has_no_rows_to_open() {
        let work = mock_work();
        assert!(
            rows_of(&work, Tab::Settings).is_none(),
            "Settings holds rows"
        );
        let mut app = window(&Theme::preset(Preset::Dark), Tab::Settings, false);
        app.on_key(key(KeyCode::Enter), &work);
        assert!(app.detail.is_none(), "enter opened a record on Settings");
        assert!(app.selection.is_none(), "Settings selected a row");
    }

    #[test]
    fn the_settings_footer_names_only_keys_that_work() {
        let rows = render(
            &mut window(&Theme::preset(Preset::Dark), Tab::Settings, false),
            100,
            20,
        )
        .0;
        let footer = rows.last().expect("a footer row").trim_end().to_string();
        for works in ["quit", "1-6", "switch tab", "help"] {
            assert!(footer.contains(works), "{footer}: {works} is missing");
        }
        // Nothing on this tab is opened or scrolled: the footer says so by
        // leaving those keys out.
        for nothing in ["enter", "open", " j ", " k ", "pgup", "pgdn"] {
            assert!(
                !footer.contains(nothing),
                "{footer}: {nothing} does nothing here"
            );
        }
    }

    #[test]
    fn the_settings_placeholder_is_painted_dim() {
        let theme = Theme::preset(Preset::Dark);
        let (rows, paint) = render(&mut window(&theme, Tab::Settings, false), 100, 20);
        let dim = theme.text.dim.fg.expect("the dim colour");
        for words in ["nothing to set yet", "will hold keybinds, sort keys, theme"] {
            let row = rows
                .iter()
                .position(|row| row.contains(words))
                .unwrap_or_else(|| panic!("{words} is not on screen"));
            let column = rows[row].find(words).expect("the words");
            assert_eq!(paint[row][column].fg, dim, "{words} is not painted dim");
        }
        // The box is the theme's, as on the other tabs.
        let border = theme.chrome.border.fg.expect("the border colour");
        assert_eq!(
            paint[2][0].fg, border,
            "the settings box is not the theme's"
        );
    }

    #[test]
    fn the_settings_tab_is_reached_and_marked_like_the_others() {
        let work = mock_work();
        let theme = Theme::preset(Preset::Dark);
        let mut app = window(&theme, Tab::Overview, false);
        app.on_key(key(KeyCode::Char('6')), &work);
        assert_eq!(app.tab, Tab::Settings, "6 did not reach Settings");
        let (rows, paint) = render(&mut app, 100, 20);
        let bar = rows.get(1).expect("the tab bar");
        let column = bar.find(" Settings ").expect("the Settings label") + 1;
        assert_eq!(
            paint[1][column].fg,
            theme.chrome.tab_active.fg.expect("the active tab colour"),
            "Settings is not marked as the active tab"
        );
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
            for (width, height) in [(100, 20), (60, 16), (40, 12)] {
                let dark = render(
                    &mut window(&Theme::preset(Preset::Dark), tab, help),
                    width,
                    height,
                )
                .0;
                let terminal = render(
                    &mut window(&Theme::preset(Preset::Terminal), tab, help),
                    width,
                    height,
                )
                .0;
                // A theme changes the paint, never the words.
                assert_eq!(dark, terminal, "{tab:?} help={help} {width}x{height}");
            }
        }
    }

    #[test]
    fn the_presets_differ_in_paint() {
        for (tab, help) in screens() {
            let dark = render(
                &mut window(&Theme::preset(Preset::Dark), tab, help),
                100,
                20,
            )
            .1;
            let terminal = render(
                &mut window(&Theme::preset(Preset::Terminal), tab, help),
                100,
                20,
            )
            .1;
            assert_ne!(
                dark, terminal,
                "{tab:?} help={help}: the presets paint the same"
            );
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
            let ids = rows_of(&work, tab).expect("a row tab");
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
            app.on_key(key(KeyCode::Enter), &work);
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
            let ids = rows_of(&work, tab).expect("a row tab");
            assert!(ids.len() >= 3, "{tab:?}: not enough rows to move through");
            let mut app = window(&theme, tab, false);
            // Down moves the paint to the second row.
            app.on_key(key(KeyCode::Down), &work);
            assert_eq!(app.selection.as_deref(), Some(ids[1].as_str()), "{tab:?}");
            let (rows, paint) = render(&mut app, 100, 40);
            let (y, x) = cell_of(&rows, &ids[1]).expect("the second row");
            assert_eq!(
                paint[y][x], selection,
                "{tab:?}: the paint did not follow the key"
            );
            // The row the paint is on is the row enter opens.
            app.on_key(key(KeyCode::Enter), &work);
            assert_eq!(
                app.detail.as_ref().expect("the pane opens").id,
                ids[1],
                "{tab:?}: enter opened another row than the painted one"
            );
            app.on_key(key(KeyCode::Esc), &work);
            // Up returns the paint and the cursor to the first row.
            app.on_key(key(KeyCode::Up), &work);
            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()), "{tab:?}");
            let (rows, paint) = render(&mut app, 100, 40);
            let (y, x) = cell_of(&rows, &ids[0]).expect("the first row");
            assert_eq!(paint[y][x], selection, "{tab:?}: up did not repaint");
            // `j` and `k` are the same movement as down and up.
            app.on_key(key(KeyCode::Char('j')), &work);
            assert_eq!(app.selection.as_deref(), Some(ids[1].as_str()), "{tab:?}");
            app.on_key(key(KeyCode::Char('k')), &work);
            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()), "{tab:?}");
            // The cursor stops at the ends rather than running off them.
            app.on_key(key(KeyCode::Up), &work);
            assert_eq!(app.selection.as_deref(), Some(ids[0].as_str()), "{tab:?}");
            for _ in 0..ids.len() + 2 {
                app.on_key(key(KeyCode::Down), &work);
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
            let ids = rows_of(&work, tab).expect("a row tab");
            let target = &ids[2];
            let mut app = window(&theme, tab, false);
            app.on_key(key(KeyCode::Down), &work);
            app.on_key(key(KeyCode::Down), &work);
            app.on_key(key(KeyCode::Enter), &work);
            assert_eq!(
                app.detail.as_ref().expect("the pane opens").id,
                *target,
                "{tab:?}"
            );
            app.on_key(key(KeyCode::Esc), &work);
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
            (Tab::Backlog, true),
            (Tab::Autopilot, false),
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
                app.on_key(key(KeyCode::Enter), &work);
                assert!(
                    app.detail.is_none(),
                    "{tab:?}: the pane opened where it does not"
                );
                continue;
            }
            app.on_key(key(KeyCode::Enter), &work);
            let detail = app
                .detail
                .as_ref()
                .unwrap_or_else(|| panic!("{tab:?}: the pane did not open"));
            assert_eq!(detail.tab, tab, "{tab:?}");
            let id = detail.id.clone();
            app.on_key(key(KeyCode::Esc), &work);
            assert!(app.detail.is_none(), "{tab:?}: the pane did not close");
            assert_eq!(app.tab, tab, "{tab:?}: esc did not return to the tab");
            assert_eq!(
                app.selection.as_deref(),
                Some(id.as_str()),
                "{tab:?}: esc did not return to the selection"
            );
            app.on_key(key(KeyCode::Enter), &work);
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
        let ids = rows_of(&work, Tab::Backlog).expect("rows");
        assert!(ids.len() >= 2, "{ids:?}");
        app.selection = Some(ids[1].clone());
        app.on_key(key(KeyCode::Enter), &work);
        assert_eq!(
            app.detail.as_ref().expect("opened").id,
            ids[1],
            "enter opened another row than the one selected"
        );
        app.on_key(key(KeyCode::Esc), &work);
        assert_eq!(
            app.selection.as_deref(),
            Some(ids[1].as_str()),
            "esc did not return to the selection it came from"
        );
    }

    #[test]
    fn enter_opens_nothing_without_rows() {
        // A folder with nothing in it: the row tabs have no rows either,
        // and enter does nothing anywhere.
        let work = Work::load(Path::new("/nowhere/coding-lanes/work"));
        for (tab, _) in reachability() {
            let mut app = app();
            app.tab = tab;
            app.on_key(key(KeyCode::Enter), &work);
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
        app.on_key(key(KeyCode::Enter), &work);
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
}
