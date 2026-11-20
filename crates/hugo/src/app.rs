//! The window: one sidebar, one main pane, a title line and a footer.
//!
//! The sidebar is the configuration and the main pane is what is running.
//! There is no tab and no tab bar. The shell has no content yet, so each
//! pane says plainly that it has none.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use coding_crew_theme::text::{cut_to, text_width};
use coding_crew_theme::{Theme, block_title_style};

/// The columns the sidebar takes when it is shown, its border included.
pub const SIDEBAR_WIDTH: u16 = 32;
/// The narrowest the main pane is drawn beside the sidebar.
pub const MAIN_MIN_WIDTH: u16 = 48;
/// The narrowest window that holds both panes: the sidebar and a main pane
/// that is not squeezed. Below it the sidebar is dropped, whole.
pub const SIDEBAR_MIN_WIDTH: u16 = SIDEBAR_WIDTH + MAIN_MIN_WIDTH;

/// The title line's text: the name of the tool.
const NAME: &str = " hugo ";

/// The sidebar's block title.
pub const SIDEBAR_TITLE: &str = "configuration";
/// The main pane's block title.
pub const FLOW_TITLE: &str = "flow";
/// The sidebar's named empty state.
pub const SIDEBAR_EMPTY: &str = "no configuration yet";
/// The main pane's named empty state.
pub const FLOW_EMPTY: &str = "nothing is running";

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

    /// Handle a key. The shell shows; it does not act on the flow.
    ///
    /// `q` quits, and every other key is left alone: the footer names the
    /// only key that works, and no key is painted with nothing behind it.
    pub fn on_key(&self, key: KeyEvent) -> Action {
        if key.kind != KeyEventKind::Press {
            return Action::Redraw;
        }
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            _ => Action::Redraw,
        }
    }

    /// Draw the window.
    pub fn draw(&self, frame: &mut Frame) {
        let screen = screen(frame.area());
        frame.render_widget(self.title_line(screen.title.width), screen.title);
        if let Some(area) = screen.announcement {
            frame.render_widget(self.hidden_line(area.width), area);
        }
        if let Some(area) = screen.sidebar {
            self.empty_pane(frame, area, SIDEBAR_TITLE, SIDEBAR_EMPTY);
        }
        self.empty_pane(frame, screen.flow, FLOW_TITLE, FLOW_EMPTY);
        frame.render_widget(self.footer_line(screen.footer.width), screen.footer);
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

    /// The footer: the keys that work. In the shell, that is `q quit`.
    ///
    /// A terminal too narrow for the key shows no footer rather than half of
    /// one.
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
    use coding_crew_theme::Preset;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// The paint of one cell.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    struct Paint {
        fg: Color,
        bg: Color,
    }

    /// Draw the window and return the rows and the paint of every cell.
    fn render(width: u16, height: u16) -> (Vec<String>, Vec<Vec<Paint>>) {
        let app = App::new(theme());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame))
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
        let (rows, paint) = render(100, 40);
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
            SIDEBAR_EMPTY.to_string(),
            FLOW_EMPTY.to_string(),
            "sidebar not shown: configuration".to_string(),
            "~/.config/coding-crew/".to_string(),
            "--render --plain".to_string(),
            "100 by 40".to_string(),
            NAME.trim().to_string(),
        ];
        for fact in facts {
            assert!(doc.contains(&fact), "the doc does not name {fact:?}");
        }
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
    fn the_window_fits_its_width_at_every_size() {
        // A shell that draws past its width is a cut screen: every row is
        // measured by what it holds, at the sizes the sweep and the pins use.
        for width in [20u16, 40, 60, 79, 80, 100, 120] {
            for height in [1u16, 2, 3, 4, 5, 8, 12, 40] {
                let (rows, _) = render(width, height);
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
