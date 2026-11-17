//! The detail pane: the record, close up.
//!
//! Enter on a row opens the record the row holds as a full-screen overlay;
//! esc returns to the tab and the selection it came from, and enter again
//! reopens. The pane opens from every tab that has rows — Path, Needs you,
//! Backlog — and nowhere else: the Overview is a summary, not a list, and
//! Autopilot and Settings are placeholders.
//!
//! The document is the next unit: the body is a placeholder until then.
//! The scroll keys are the document unit's work too, and are not built
//! here.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use coding_lanes_core::escape_literal;

use crate::app::footer_line;
use crate::data::Work;
use crate::tabs::Tab;
use crate::text::{cut_to, text_width};
use crate::theme::{Theme, block_title_style};

/// The body while the document is not built.
const BODY: &str = "the document is the next unit";

/// The open pane: the record it shows, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    /// The id of the record the pane shows.
    pub id: String,
    /// The tab the pane was opened from.
    pub tab: Tab,
    /// The row selected on that tab, so esc returns to it.
    pub selection: Option<String>,
}

impl Detail {
    /// The pane a row opened.
    pub fn opened(id: String, tab: Tab, selection: Option<String>) -> Self {
        Self { id, tab, selection }
    }
}

/// Draw the pane over the whole window.
pub fn render(frame: &mut Frame, area: Rect, work: &Work, detail: &Detail, theme: &Theme) {
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    let width = usize::from(rows[0].width).saturating_sub(2);
    // The pane covers the whole window: what is under it is not on
    // display, and nothing of it shows through.
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(header_labels(detail, work, width, theme))
        .border_style(theme.chrome.border_focus);
    frame.render_widget(
        Paragraph::new(body_line(work, detail, width, theme))
            .wrap(Wrap { trim: false })
            .block(block),
        rows[0],
    );
    frame.render_widget(footer(rows[1].width, theme), rows[1]);
}

/// The body of the pane: one line, until the document is built.
///
/// The record is looked up again on every redraw. A record that left the
/// folder is said, not a blank screen.
fn body_line(work: &Work, detail: &Detail, width: usize, theme: &Theme) -> Line<'static> {
    match work.record_title(&detail.id) {
        Some(_) => Line::from(Span::styled(cut_to(BODY, width), theme.text.normal)),
        None => Line::from(Span::styled(
            cut_to(
                &format!(
                    "the record {} is not in the work folder",
                    escape_literal(&detail.id)
                ),
                width,
            ),
            theme.text.dim,
        )),
    }
}

/// The block's title: the id and the title of the record, both labels.
///
/// A label is a field the document must not interpret, so both are escaped
/// by the projection's own rule. The title is `text.strong` — a title that
/// matters. The labels are cut on grapheme boundaries: a long title is
/// never cut through a grapheme, and a cut title is never shown as if it
/// were the whole one.
fn header_labels(detail: &Detail, work: &Work, width: usize, theme: &Theme) -> Line<'static> {
    let mut labels = vec![
        Span::raw(" "),
        Span::styled(escape_literal(&detail.id), theme.text.normal),
    ];
    if let Some(title) = work.record_title(&detail.id) {
        labels.push(Span::styled(" · ".to_string(), theme.text.dim));
        labels.push(Span::styled(
            escape_literal(&title),
            block_title_style(theme),
        ));
    }
    labels.push(Span::raw(" "));
    cut_labels(labels, width)
}

/// The labels, cut to `width` columns grapheme by grapheme.
///
/// Each kept grapheme keeps the paint of the label it came from. When the
/// labels do not fit, the cut ends with an ellipsis in the paint of the
/// label it lands in: no part of a label is shown as if it were the whole
/// label.
fn cut_labels(labels: Vec<Span<'static>>, width: usize) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    let total: usize = labels.iter().map(Span::width).sum();
    if total <= width {
        return Line::from(labels);
    }
    let budget = width - 1;
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for label in labels {
        let remaining = budget - used;
        if label.width() <= remaining {
            // The label fits whole: it is kept whole.
            used += label.width();
            out.push(label);
            continue;
        }
        // The cut lands in this label: it is cut here, and nothing after
        // it is shown.
        let mut piece = String::new();
        for grapheme in label.styled_graphemes(Style::default()) {
            let columns = text_width(grapheme.symbol);
            if used + columns > budget {
                break;
            }
            used += columns;
            piece.push_str(grapheme.symbol);
        }
        if !piece.is_empty() {
            out.push(Span::styled(piece, label.style));
        }
        break;
    }
    let style = out.last().map_or(Style::default(), |span| span.style);
    out.push(Span::styled("…", style));
    Line::from(out)
}

/// The footer of the pane: the keys that do something while it is open.
///
/// Every other key is left alone while the pane is open, so none of them
/// is named: not the tabs, not help, and not enter — it opens, and it has
/// opened.
fn footer(width: u16, theme: &Theme) -> Line<'static> {
    let key = theme.chrome.key;
    let label = theme.chrome.footer;
    let parts = vec![
        (
            0,
            vec![Span::styled(" q ", key), Span::styled(" quit", label)],
            true,
        ),
        (
            1,
            vec![
                Span::raw("   "),
                Span::styled(" esc ", key),
                Span::styled(" close", label),
            ],
            true,
        ),
    ];
    footer_line(parts, width)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Modifier};

    use super::*;
    use crate::app::App;
    use crate::data::Work;
    use crate::theme::Preset;

    /// The title fixture: one wide, one combining and one joined grapheme.
    const GRAPHEME_TITLE: &str = "界e\u{0301}👨‍👩‍👧x";

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-lanes/work"))
    }

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    fn pane(id: &str) -> Detail {
        Detail::opened(id.to_string(), Tab::Path, None)
    }

    /// The paint of one cell: its colours and its attributes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    struct Paint {
        fg: Color,
        bg: Color,
        modifier: Modifier,
    }

    /// Draw the window with the pane open and return the rows and paints.
    fn draw_with(
        theme: &Theme,
        work: &Work,
        detail: &Detail,
        width: u16,
        height: u16,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        let mut app = App::new(PathBuf::from("/tmp/coding-lanes/work"), *theme);
        app.detail = Some(detail.clone());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, work))
            .expect("drawing the pane");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        let rows = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect();
        let paints = (0..area.height)
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
        (rows, paints)
    }

    fn draw(
        work: &Work,
        detail: &Detail,
        width: u16,
        height: u16,
    ) -> (Vec<String>, Vec<Vec<Paint>>) {
        draw_with(&theme(), work, detail, width, height)
    }

    /// The text of the rows, without trailing padding.
    fn text_of(rows: &[String]) -> String {
        rows.iter()
            .map(|row| row.trim_end().to_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// Whether `prefix` is a whole number of graphemes at the start of
    /// `text`, by the same grapheme iterator ratatui draws with.
    fn is_grapheme_prefix(prefix: &str, text: &str) -> bool {
        let span = Span::raw(text.to_string());
        let mut built = String::new();
        for grapheme in span.styled_graphemes(Style::default()) {
            if built.chars().count() >= prefix.chars().count() {
                break;
            }
            built.push_str(grapheme.symbol);
        }
        built == prefix
    }

    /// One task with the given title, groomed and waiting.
    fn titled_work(title: &str) -> Work {
        let mut task = coding_lanes_core::Task::new("TSK01", title.to_string());
        task.status = coding_lanes_core::Status::Groomed;
        task.status_since = "2026-11-07T13:00:00Z".to_string();
        Work {
            root: PathBuf::from("/nowhere/coding-lanes/work"),
            tasks: vec![task],
            ..Work::default()
        }
    }

    #[test]
    fn the_pane_shows_the_record_as_labels() {
        let work = mock_work();
        let task = work.task("TSK13").expect("the task");
        let (rows, paints) = draw(&work, &pane("TSK13"), 100, 40);
        let text = text_of(&rows);
        assert!(text.contains("TSK13"), "{text}");
        assert!(text.contains(&task.title), "{text}");
        // The pane is full screen: the window under it is not on display.
        assert!(!text.contains("Autopilot"), "{text}");
        // The body is the placeholder, not a blank screen.
        assert!(text.contains(BODY), "{text}");
        // The footer says what the pane does.
        assert!(text.contains("esc") && text.contains("close"), "{text}");
        // The title is `text.strong`.
        let strong = theme().text.strong.fg.expect("the strong colour");
        let row_at = rows
            .iter()
            .position(|row| row.contains(&task.title))
            .expect("the title is on screen");
        let column = rows[row_at].find(&task.title).expect("the title's column");
        assert_eq!(
            paints[row_at][column].fg, strong,
            "the title is not on text.strong"
        );
    }

    #[test]
    fn the_body_is_on_text_normal() {
        let work = mock_work();
        let normal = theme().text.normal.fg.expect("the normal colour");
        let (rows, paints) = draw(&work, &pane("TSK13"), 100, 40);
        let row_at = rows
            .iter()
            .position(|row| row.contains(BODY))
            .expect("the body is on screen");
        let column = rows[row_at].find(BODY).expect("the body's column");
        assert_eq!(
            paints[row_at][column].fg, normal,
            "the body is not on text.normal"
        );
    }

    #[test]
    fn the_title_is_escaped() {
        // A field that must not be interpreted is escaped by the
        // projection's own rule, wherever it is shown.
        let work = titled_work("fix: `code` *bold* [link]");
        let (rows, _) = draw(&work, &pane("TSK01"), 100, 40);
        let text = text_of(&rows);
        assert!(text.contains(r"fix: \`code\` \*bold\* \[link\]"), "{text}");
        assert!(!text.contains("`code`"), "{text}");
    }

    #[test]
    fn a_long_label_is_cut_on_grapheme_boundaries() {
        // The cut is grapheme by grapheme: no cut ever splits one, at any
        // width. The check can fail: a scalar cut leaves a dangling
        // combining mark behind, which is not a cut on a grapheme
        // boundary.
        let full = format!(" TSK01 · {GRAPHEME_TITLE} ");
        let width_of_full = text_width(&full);
        assert_eq!(
            width_of_full,
            text_width(" TSK01 · ") + text_width(GRAPHEME_TITLE) + 1,
            "the fixture measures"
        );
        for width in 1..=width_of_full {
            let line = cut_labels(
                vec![
                    Span::raw(" "),
                    Span::raw(full.trim().to_string()),
                    Span::raw(" "),
                ],
                width,
            );
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            assert!(text_width(&text) <= width, "{width}: {text:?}");
            let head = text.strip_suffix('…').unwrap_or(&text);
            assert!(
                is_grapheme_prefix(head, &full),
                "{width}: {text:?} is not cut on the grapheme boundaries of {full:?}"
            );
        }
    }

    #[test]
    fn a_long_title_is_cut_and_the_cut_is_marked() {
        // The header holds a title far longer than the pane: it is cut,
        // and the cut says so. The check can fail: a pass-through cut
        // would show the whole title at a width that cannot hold it.
        let work = titled_work(&"x".repeat(300));
        let (rows, _) = draw(&work, &pane("TSK01"), 100, 40);
        let header = &rows[0];
        assert!(header.chars().count() <= 100, "{header:?}");
        assert!(header.contains('…'), "{header:?}");
        assert!(!header.contains(&"x".repeat(300)), "{header:?}");
    }

    #[test]
    fn the_body_is_never_a_blank_screen() {
        let work = mock_work();
        for width in 0..=100 {
            let line = body_line(&work, &pane("TSK13"), width, &theme());
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            if width == 0 {
                assert!(text.is_empty(), "{width}: {text:?}");
            } else {
                assert!(!text.is_empty(), "{width}: the body is blank");
                assert_eq!(text, cut_to(BODY, width), "{width}");
            }
        }
    }

    #[test]
    fn the_footer_names_only_keys_that_do_something() {
        let work = mock_work();
        let (rows, _) = draw(&work, &pane("TSK13"), 100, 40);
        let footer = rows.last().expect("a footer row").trim_end().to_string();
        assert!(footer.contains("quit"), "{footer}");
        assert!(
            footer.contains("esc") && footer.contains("close"),
            "{footer}"
        );
        // Enter has opened, the tabs are left alone, and help is not
        // offered over the pane: none of them is named.
        assert!(!footer.contains("open"), "{footer}");
        assert!(!footer.contains("tab"), "{footer}");
        assert!(!footer.contains("help"), "{footer}");
        assert!(!footer.contains("1-6"), "{footer}");
    }

    #[test]
    fn a_record_that_left_the_folder_is_said() {
        let work = mock_work();
        let (rows, _) = draw(&work, &pane("TSK99"), 100, 40);
        let text = text_of(&rows);
        assert!(text.contains("TSK99"), "{text}");
        assert!(text.contains("not in the work folder"), "{text}");
        assert!(!text.contains(BODY), "{text}");
    }

    #[test]
    fn the_selection_slot_stays_unused() {
        // The first user of text.selected is the document unit: nothing in
        // the pane paints with it yet. The accent background is the
        // selection's alone.
        let theme = theme();
        let selection = theme.text.selected.bg.expect("the selection background");
        let work = mock_work();
        for width in [100u16, 60, 30] {
            let (_, paints) = draw_with(&theme, &work, &pane("TSK13"), width, 20);
            for cell in paints.iter().flatten() {
                assert_ne!(cell.bg, selection, "{width}: the selection slot is painted");
            }
        }
    }

    #[test]
    fn the_presets_say_the_same_thing_and_paint_differently() {
        let work = mock_work();
        for (width, height) in [(100u16, 20u16), (60, 16), (40, 12), (24, 8)] {
            let (dark_rows, dark_paints) = draw_with(
                &Theme::preset(Preset::Dark),
                &work,
                &pane("TSK13"),
                width,
                height,
            );
            let (terminal_rows, terminal_paints) = draw_with(
                &Theme::preset(Preset::Terminal),
                &work,
                &pane("TSK13"),
                width,
                height,
            );
            // A theme changes the paint, never the words.
            assert_eq!(dark_rows, terminal_rows, "{width}x{height}");
            assert_ne!(dark_paints, terminal_paints, "{width}x{height}");
        }
    }
}
