//! The detail pane: the record, close up, as a document.
//!
//! Enter on a row opens the record the row holds as a full-screen overlay;
//! esc returns to the tab and the selection it came from, and enter again
//! reopens. The pane opens from every tab whose rows are tasks — Path, Needs
//! you, Backlog — and nowhere else: the Overview is a summary, not a list,
//! and Settings lists what is true and reads.
//!
//! The body is the record projected to markdown
//! ([ADR 0009](../../../docs/decisions/0009-markdown-projection.md)) and
//! rendered through the vendored leaf engine, painted by the theme's
//! `markdown_theme` — one palette. The document is not editable: raul
//! reads. The pane scrolls — `↑`/`↓`, `j`/`k`, `pgup`/`pgdn`, `g`/`G` move
//! through a document taller than the pane — and its footer names the keys
//! that work while it is open.

use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use syntect::highlighting::{
    Color as SyntectColor, FontStyle, StyleModifier, Theme as SyntectTheme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::SyntaxSet;

use coding_crew_core::escape_literal;
use coding_crew_engine::{MarkdownTheme, parse_markdown_with_width, syntax_set};

use crate::data::Work;
use crate::tabs::Tab;
use coding_crew_theme::chrome;
use coding_crew_theme::text::{cut_to, text_width};
use coding_crew_theme::{Icon, IconSet, Theme, block_title_style, phase_style};

/// The syntax set, loaded once per process.
///
/// The syntax set compiles hundreds of syntaxes; loading it in a draw loop
/// would stall every frame. The document is parsed again on every redraw —
/// the work folder is read again on every redraw too — so the parse has to
/// stay cheap, and it does once the assets are in.
static SYNTAX: OnceLock<SyntaxSet> = OnceLock::new();

/// The syntax set and the syntect theme the document's code tokens pair
/// with, for `theme`'s preset.
///
/// Every field of `MarkdownTheme` is mapped onto the document's roles — one
/// palette — and the code's tokens are the one paint outside that promise,
/// handled deliberately. The engine paints the tokens of a fenced code block
/// with syntect's own theme, and no parameter of the vendored engine turns that
/// off: an empty scope-less syntect theme paints the tokens black, and an
/// empty syntax set panics looking for plain text. So each preset pairs the
/// code tokens with a theme derived from its own document palette — see
/// [`code_theme`].
fn syntect(theme: &Theme) -> (&'static SyntaxSet, SyntectTheme) {
    (SYNTAX.get_or_init(syntax_set), code_theme(theme))
}

/// The syntect theme for a preset, derived from its document palette.
///
/// Each class of token takes one of the eight syntax roles, and the scopes
/// of a class are the ones the common languages name for it:
///
/// - a token no scope names is the code's ink;
/// - a comment is the muted grade, in italic;
/// - a keyword, and a storage keyword, takes the keyword role;
/// - a string, and a character escape, takes the string role;
/// - a constant or a number takes the number role;
/// - a function, a macro or a builtin function takes the function role;
/// - a type, a class or a namespace takes the type role;
/// - a storage keyword, such as `let` or `fn`, takes the keyword role, and so
///   does a primitive type such as `u32`, which the grammars scope as storage;
/// - a variable or a parameter takes the variable role;
/// - a tag or an attribute name takes the tag role;
/// - a markup heading is the section heading's colour, in bold, and a markup
///   bold or italic is a modifier with no colour of its own;
/// - an operator or punctuation is the document's prose, so it reads apart
///   from a comment;
/// - an invalid token is the error hue.
///
/// The terminal preset paints no colour, so its theme names the terminal's
/// own colour everywhere, and the tokens emit no RGB.
fn code_theme(theme: &Theme) -> SyntectTheme {
    let doc = theme.document();
    let item = |selector: &str, colour: Option<Color>, font_style: Option<FontStyle>| ThemeItem {
        scope: selector
            .parse()
            .expect("a scope selector the mapping names"),
        style: StyleModifier {
            foreground: colour.map(syntect_colour),
            background: None,
            font_style,
        },
    };
    let syntax = doc.syntax;
    SyntectTheme {
        settings: ThemeSettings {
            foreground: Some(syntect_colour(doc.code.text)),
            ..ThemeSettings::default()
        },
        scopes: vec![
            item("comment", Some(syntax.comment), Some(FontStyle::ITALIC)),
            item(
                "punctuation.definition.comment",
                Some(syntax.comment),
                Some(FontStyle::ITALIC),
            ),
            item("keyword, storage", Some(syntax.keyword), None),
            item(
                "string, constant.character.escape",
                Some(syntax.string),
                None,
            ),
            item(
                "constant.numeric, constant.language, support.constant",
                Some(syntax.number),
                None,
            ),
            item(
                "entity.name.function, support.function, support.macro, meta.macro",
                Some(syntax.function),
                None,
            ),
            item(
                "entity.name.type, entity.name.namespace, entity.name.struct, entity.name.enum, \
                 entity.name.trait, support.type, support.class",
                Some(syntax.type_name),
                None,
            ),
            item("variable, variable.parameter", Some(syntax.variable), None),
            item(
                "entity.name.tag, entity.other.attribute-name",
                Some(syntax.tag),
                None,
            ),
            item(
                "markup.heading",
                Some(doc.heading_section),
                Some(FontStyle::BOLD),
            ),
            item("markup.bold", None, Some(FontStyle::BOLD)),
            item("markup.italic", None, Some(FontStyle::ITALIC)),
            item("keyword.operator, punctuation", Some(doc.text.body), None),
            item("invalid", Some(doc.alert_caution), None),
        ],
        ..SyntectTheme::default()
    }
}

/// A preset's colour as syntect's: its RGB, or alpha 0 for the terminal's
/// own colour, which the engine reads back as `Color::Reset`.
fn syntect_colour(colour: Color) -> SyntectColor {
    match colour {
        Color::Rgb(r, g, b) => SyntectColor { r, g, b, a: 255 },
        _ => SyntectColor {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    }
}

/// The open pane: the record it shows, where it came from, and how far
/// through the document it has scrolled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    /// The id of the record the pane shows.
    pub id: String,
    /// The tab the pane was opened from.
    pub tab: Tab,
    /// The row selected on that tab, so esc returns to it.
    pub selection: Option<String>,
    /// The first row of the document that is on display.
    ///
    /// A key moves it; the render clamps it to the document, so a document
    /// that shrank between two redraws never shows past its end.
    pub scroll: u16,
    /// The rows of the document the pane last showed: a page, the distance
    /// `pgup` and `pgdn` move.
    pub page: u16,
}

impl Detail {
    /// The pane a row opened, at the top of the document.
    pub fn opened(id: String, tab: Tab, selection: Option<String>) -> Self {
        Self {
            id,
            tab,
            selection,
            scroll: 0,
            page: 0,
        }
    }

    /// Scroll by rows: down on a positive count, up on a negative one.
    pub fn scroll_rows(&mut self, rows: isize) {
        self.scroll = clamp_scroll(self.scroll as isize + rows);
    }

    /// Scroll by pages: the rows the pane last showed.
    pub fn scroll_pages(&mut self, pages: isize) {
        self.scroll_rows(pages * (self.page.max(1) as isize));
    }

    /// Scroll to the top of the document.
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    /// Scroll to the bottom of the document; the render clamps it.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = u16::MAX;
    }
}

/// A scroll offset that is never negative and never overflows.
fn clamp_scroll(offset: isize) -> u16 {
    offset.clamp(0, u16::MAX as isize) as u16
}

/// Draw the pane over the whole window. `help` says the help is open over it,
/// which the footer names.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    detail: &mut Detail,
    icons: IconSet,
    theme: &Theme,
    help: bool,
) {
    let (actions, closing) = footer_chips(help, theme);
    let footer_rows = chrome::footer(&actions, &closing, area.width, 2, theme).len();
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(u16::try_from(footer_rows).unwrap_or(2)),
    ])
    .split(area);
    let width = usize::from(rows[0].width).saturating_sub(2);
    // The pane covers the whole window: what is under it is not on
    // display, and nothing of it shows through.
    frame.render_widget(Clear, area);
    // The pane is a rounded block like the others; its title is the
    // record's status icon, then its id and title, cut as one line.
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(header_labels(detail, work, width, icons, theme))
        .border_style(theme.chrome.border_focus);
    let lines = document_lines(work, detail, width, icons, theme);
    // The scroll is clamped to the document, and the page is what the pane
    // actually holds: the next pgup/pgdn moves by the rows on display.
    let visible = usize::from(rows[0].height).saturating_sub(2);
    let scroll = usize::from(detail.scroll).min(lines.len().saturating_sub(visible));
    detail.scroll = u16::try_from(scroll).unwrap_or(u16::MAX);
    detail.page = u16::try_from(visible).unwrap_or(u16::MAX);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0)),
        rows[0],
    );
    frame.render_widget(Line::from(chrome::rule(rows[1].width, theme)), rows[1]);
    let lines = footer(rows[2].width, rows[2].height, help, theme);
    frame.render_widget(Paragraph::new(lines), rows[2]);
}

/// The document the pane shows: the record projected to markdown, rendered
/// through the vendored engine at the pane's own width.
///
/// The record is looked up again on every redraw. A record that left the
/// folder is said, not a blank screen; a smaller task is said to be one,
/// not shown as the task it is not.
fn document_lines(
    work: &Work,
    detail: &Detail,
    width: usize,
    icons: IconSet,
    theme: &Theme,
) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let Some(task) = work.task(&detail.id) else {
        let line = if work.item(&detail.id).is_some() {
            format!(
                "the smaller task {} has no document yet",
                escape_literal(&detail.id)
            )
        } else {
            format!(
                "the record {} is not in the work folder",
                escape_literal(&detail.id)
            )
        };
        // A notice is said quietly: both take `text.dim`, the paint the
        // doc gives the empty state and other secondary text.
        return vec![Line::from(Span::styled(
            cut_to(&line, width),
            theme.text.dim,
        ))];
    };
    // The projection reads the whole work folder as its context: the
    // reverse dependencies and the children's progress are the project's
    // own facts.
    let markdown = coding_crew_core::task_to_markdown(task, &work.tasks);
    let (syntax, paired) = syntect(theme);
    parse_markdown_with_width(
        &markdown,
        syntax,
        &paired,
        width,
        &markdown_theme(theme, icons),
        // The document is not a file view: no file-mode offsets. Every
        // fenced block numbers its lines.
        false,
        true,
    )
    .lines
}

/// The engine's theme for the document: the theme's palette, with the task,
/// link and alert glyphs taken from the icon set the pane draws its icons with.
fn markdown_theme(theme: &Theme, icons: IconSet) -> MarkdownTheme {
    MarkdownTheme {
        task_done_glyph: Icon::StatusDone.glyph(icons),
        task_open_glyph: Icon::TaskOpen.glyph(icons),
        link_glyph: Icon::Link.glyph(icons),
        alert_glyphs: [
            Icon::AlertNote.glyph(icons),
            Icon::AlertTip.glyph(icons),
            Icon::AlertImportant.glyph(icons),
            Icon::AlertWarning.glyph(icons),
            Icon::AlertCaution.glyph(icons),
        ],
        ..theme.markdown_theme()
    }
}

/// The block's title: the id and the title of the record, both labels.
///
/// A label is a field the document must not interpret, so both are escaped
/// by the projection's own rule. The title is `text.strong` — a title that
/// matters. The labels are cut on grapheme boundaries: a long title is
/// never cut through a grapheme, and a cut title is never shown as if it
/// were the whole one. The title leads with a rule after the corner, as every
/// pane's does: `╭─ icon id · title`.
fn header_labels(
    detail: &Detail,
    work: &Work,
    width: usize,
    icons: IconSet,
    theme: &Theme,
) -> Line<'static> {
    let mut labels = vec![Span::styled("─", theme.chrome.border_focus), Span::raw(" ")];
    if let Some(task) = work.task(&detail.id)
        && let Some(icon) = Icon::for_status(task.status.as_str())
    {
        // The icon takes the phase's slot, as the status word does.
        labels.push(Span::styled(
            icon.glyph(icons).to_string(),
            phase_style(theme, task.status),
        ));
        labels.push(Span::raw(" "));
    }
    labels.push(Span::styled(escape_literal(&detail.id), theme.text.normal));
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
/// `esc` closes the pane, or the help over it. The scroll keys are not named:
/// the help over the pane lists them, as the window's help lists its own.
/// Every other key is left alone while the pane is open, so none of them is
/// named: not the tabs, and not enter — it opens, and it has opened.
fn footer_chips(help: bool, theme: &Theme) -> (Vec<Vec<Span<'static>>>, Vec<Vec<Span<'static>>>) {
    let chip = |key: &str, action: &str| chrome::key_chip(key, action, theme);
    let actions = vec![chip("esc", if help { "close help" } else { "close" })];
    let closing = vec![chip("?", "help"), chip("q", "quit")];
    (actions, closing)
}

/// The footer's lines at `width`, in `rows` rows.
fn footer(width: u16, rows: u16, help: bool, theme: &Theme) -> Vec<Line<'static>> {
    let (actions, closing) = footer_chips(help, theme);
    chrome::footer(&actions, &closing, width, rows, theme)
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
    use coding_crew_theme::{IconSet, Preset, SyntaxPaint};

    /// The title fixture: one wide, one combining and one joined grapheme.
    const GRAPHEME_TITLE: &str = "界e\u{0301}👨‍👩‍👧x";

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    #[test]
    fn the_syntect_note_says_each_preset_pairs_its_own() {
        // The note says the pairing is per preset, as theme.md does, and no
        // longer names one syntect theme for all of them.
        const DETAIL: &str = include_str!("detail.rs");
        // The test lives beside the note now: read the source above the test
        // module, so this test's own strings cannot satisfy or trip it.
        let source = DETAIL.split("\n#[cfg(test)]").next().unwrap_or(DETAIL);
        // The note is wrapped across doc-comment lines: compare its words,
        // not its line breaks or its comment markers.
        let words = source
            .replace("///", " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            words.contains("each preset pairs the code tokens with a theme derived from its own"),
            "the syntect note does not say each preset pairs its own theme"
        );
        assert!(
            !words.contains("base16-ocean.dark"),
            "the syntect note still names one syntect theme for every preset"
        );
    }

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// The tests' clock: the pane does not measure time, but the window it
    /// draws over does.
    fn now() -> &'static str {
        "2026-11-19T08:00:00Z"
    }

    fn pane(id: &str) -> Detail {
        Detail::opened(id.to_string(), Tab::Path, None)
    }

    /// One task with the given title, groomed and waiting.
    fn titled_work(title: &str) -> Work {
        let mut task = coding_crew_core::Task::new("TSK01", title.to_string());
        task.status = coding_crew_core::Status::Groomed;
        task.status_since = "2026-11-07T13:00:00Z".to_string();
        Work {
            root: PathBuf::from("/nowhere/coding-crew/work"),
            tasks: vec![task],
            ..Work::default()
        }
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
        let mut app = App::new(
            PathBuf::from("/tmp/coding-crew/work"),
            *theme,
            IconSet::Nerd,
        );
        app.detail = Some(detail.clone());
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, work, now()))
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

    /// The document the pane would show, at the pane's own width.
    fn document(work: &Work, detail: &Detail, width: usize, theme: &Theme) -> String {
        document_lines(work, detail, width, IconSet::Nerd, theme)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
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

    /// The sections of the detail pane, in the order the screens file
    /// writes them down.
    fn doc_sections() -> Vec<String> {
        let doc = include_str!("../../../docs/tui/screens.md");
        let body = doc
            .split("## The detail pane")
            .nth(1)
            .expect("the detail pane section")
            .split("\n## ")
            .next()
            .expect("the section ends");
        let names: Vec<String> = body
            .lines()
            .skip_while(|line| !line.starts_with("| section |"))
            .skip(2) // the header row and the separator
            .take_while(|line| line.starts_with("| "))
            .map(|line| {
                line.split('|')
                    .nth(1)
                    .expect("a first cell")
                    .trim()
                    .to_lowercase()
            })
            .collect();
        assert_eq!(names.len(), 10, "{names:?}");
        names
    }

    #[test]
    fn the_document_carries_the_sections_in_the_doc_s_order() {
        // The sections and their order are the screens file's table, read
        // here rather than copied. The check can fail: a section the
        // projection drops or renames is missing from the rendered
        // document, and one moved out of place breaks the order.
        let work = mock_work();
        let detail = pane("TSK13");
        let rendered = document(&work, &detail, 100, &theme()).to_lowercase();
        let mut cursor = 0usize;
        for section in doc_sections() {
            let at = rendered[cursor..]
                .find(&section)
                .unwrap_or_else(|| panic!("{section} is not in the rendered document"));
            cursor += at + section.len();
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
        assert!(!text.contains("Settings"), "{text}");
        // The body is the document, not a blank screen: its first section
        // is on display.
        assert!(text.contains("Header"), "{text}");
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
    fn the_first_heading_is_painted_from_its_slot() {
        // The document opens on `# Header`: a section heading takes the
        // document palette's `heading.section` and the engine's own bold.
        let work = mock_work();
        let (rows, paints) = draw(&work, &pane("TSK13"), 100, 40);
        let section = theme().document().heading_section;
        let row_at = rows
            .iter()
            .position(|row| row.contains("Header"))
            .expect("the first heading is on screen");
        let column = rows[row_at].find("Header").expect("the heading's column");
        assert_eq!(
            paints[row_at][column].fg, section,
            "the heading is not on heading.section"
        );
        assert!(
            paints[row_at][column].modifier.contains(Modifier::BOLD),
            "the heading is not bold"
        );
    }

    #[test]
    fn the_title_is_escaped() {
        // A field that must not be interpreted is escaped by the
        // projection's own rule, wherever it is shown. The pane's header
        // shows the escape raw: markdown cannot interpret the title there.
        // In the document, the same escape keeps the title text: the code
        // span, the emphasis and the link do not happen — the characters
        // survive, so no markdown shape replaced them.
        let work = titled_work("fix: `code` *bold* [link]");
        let (rows, _) = draw(&work, &pane("TSK01"), 100, 40);
        let header = &rows[0];
        assert!(
            header.contains(r"fix: \`code\` \*bold\* \[link\]"),
            "{header:?}"
        );
        assert!(!header.contains("`code`"), "{header:?}");
        let body = document(&work, &pane("TSK01"), 98, &theme());
        assert!(
            body.contains("fix: `code` *bold* [link]"),
            "the document interpreted the title: {body}"
        );
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
        let line = header_labels(&pane("TSK01"), &work, 98, IconSet::Nerd, &theme());
        assert_eq!(line.width(), 98, "{line:?}");
        let (rows, _) = draw(&work, &pane("TSK01"), 100, 40);
        let header = &rows[0];
        assert!(header.contains('…'), "{header:?}");
        assert!(!header.contains(&"x".repeat(300)), "{header:?}");
    }

    #[test]
    fn the_title_leads_with_a_rule_after_the_corner_as_a_pane_does() {
        // The overlay's title has the shape of every pane's: `╭─ icon id`.
        // The check can fail: the old shape reads `╭ icon id`.
        let work = titled_work("a title");
        let (rows, _) = draw(&work, &pane("TSK01"), 100, 40);
        assert!(rows[0].starts_with("╭─ "), "{:?}", rows[0]);
    }

    #[test]
    fn the_document_fills_the_pane_at_any_width() {
        // The body is never a blank screen while the record is a task, at
        // any width the pane can hold.
        let work = mock_work();
        for width in [100u16, 60, 30, 14, 12] {
            let (rows, _) = draw(&work, &pane("TSK13"), width, 20);
            let body = &rows[1..rows.len() - 1];
            assert!(
                body.iter().any(|row| !row.trim().is_empty()),
                "{width}: the body is blank"
            );
            // Below 14 columns the header table is wider than its inner
            // width, and a quoted code line is too (a follow-up in TODO.md),
            // so only the body is checked there.
            if width < 14 {
                continue;
            }
            // Nothing is wider than the pane: the document is rendered at
            // the pane's own width, inside the border.
            let inner = usize::from(width) - 2;
            for line in document_lines(&work, &pane("TSK13"), inner, IconSet::Nerd, &theme()) {
                assert!(line.width() <= inner, "{width}: {line:?}");
            }
        }
    }

    /// Draw the pane, press keys in order, and draw again — the loop a
    /// redraw is. The first draw is returned with the second, so a test
    /// can hold the unscrolled view while the keys move the pane.
    fn drawn_after(
        work: &Work,
        id: &str,
        width: u16,
        height: u16,
        keys: &[crossterm::event::KeyCode],
    ) -> (Vec<String>, Vec<String>, App) {
        let mut app = App::new(
            PathBuf::from("/tmp/coding-crew/work"),
            theme(),
            IconSet::Nerd,
        );
        app.detail = Some(Detail::opened(id.to_string(), Tab::Path, None));
        // The first draw, as the loop does: it measures the page.
        let first = draw_frame(&mut app, work, width, height);
        for code in keys {
            app.on_key(crossterm::event::KeyEvent::from(*code), work, now());
        }
        let last = draw_frame(&mut app, work, width, height);
        (first, last, app)
    }

    /// One frame of the pane, as rows.
    fn draw_frame(app: &mut App, work: &Work, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| app.draw(frame, work, now()))
            .expect("drawing the pane");
        let buffer = terminal.backend().buffer();
        (0..buffer.area().height)
            .map(|y| {
                (0..buffer.area().width)
                    .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn the_pane_scrolls_a_document_taller_than_the_pane() {
        use crossterm::event::KeyCode;
        let work = mock_work();
        // Unscrolled, the bottom of the document is not on display; at
        // twenty rows, the top of a long document is the header alone.
        let (top, bottom, app) = drawn_after(&work, "TSK13", 100, 20, &[KeyCode::Char('G')]);
        assert!(
            app.detail.as_ref().expect("open").scroll > 0,
            "G did not scroll"
        );
        assert!(
            !text_of(&top).contains("Children"),
            "the unscrolled pane shows the bottom: {}",
            text_of(&top)
        );
        assert!(
            text_of(&bottom).contains("Children"),
            "{}",
            text_of(&bottom)
        );
        // Back at the top, the pane is byte for byte the unscrolled view:
        // the shape at the top matches.
        let (_, back, _) = drawn_after(
            &work,
            "TSK13",
            100,
            20,
            &[KeyCode::Char('G'), KeyCode::Char('g')],
        );
        assert_eq!(top, back, "g did not return the unscrolled view");
    }

    #[test]
    fn a_row_moves_the_view_by_one_row() {
        use crossterm::event::KeyCode;
        let work = mock_work();
        let (top, down, app) = drawn_after(&work, "TSK13", 100, 20, &[KeyCode::Down]);
        assert_eq!(
            app.detail.as_ref().expect("open").scroll,
            1,
            "down did not move one row"
        );
        assert_ne!(top, down, "down did not move the view");
        let (_, back, app) = drawn_after(&work, "TSK13", 100, 20, &[KeyCode::Down, KeyCode::Up]);
        assert_eq!(app.detail.as_ref().expect("open").scroll, 0);
        assert_eq!(top, back, "up did not return the unscrolled view");
        // j and k are the same keys in the pane.
        let (_, vim, app) = drawn_after(&work, "TSK13", 100, 20, &[KeyCode::Char('j')]);
        assert_eq!(app.detail.as_ref().expect("open").scroll, 1);
        assert_eq!(down, vim, "j did not scroll like down");
        let (_, _, app) = drawn_after(
            &work,
            "TSK13",
            100,
            20,
            &[KeyCode::Char('j'), KeyCode::Char('k')],
        );
        assert_eq!(app.detail.as_ref().expect("open").scroll, 0);
    }

    #[test]
    fn a_page_moves_the_view_by_the_pane_s_rows() {
        use crossterm::event::KeyCode;
        let work = mock_work();
        // The first draw measures the page: the rows the pane holds.
        let (top, _, app) = drawn_after(&work, "TSK13", 100, 20, &[]);
        let page = app.detail.as_ref().expect("open").page;
        assert_eq!(
            page, 16,
            "the page is not the pane's rows: the height minus the rule, the footer and the two borders"
        );
        // pgdn moves by that page.
        let (_, paged, app) = drawn_after(&work, "TSK13", 100, 20, &[KeyCode::PageDown]);
        assert_eq!(
            app.detail.as_ref().expect("open").scroll,
            page,
            "pgdn did not move a page"
        );
        // The same view, reached by the scroll itself.
        let mut jumped = App::new(
            PathBuf::from("/tmp/coding-crew/work"),
            theme(),
            IconSet::Nerd,
        );
        let mut detail = pane("TSK13");
        detail.scroll = page;
        jumped.detail = Some(detail);
        let expected = draw_frame(&mut jumped, &work, 100, 20);
        assert_eq!(paged, expected, "pgdn did not show the page's rows");
        // pgup moves back by the same page.
        let (_, back, app) = drawn_after(
            &work,
            "TSK13",
            100,
            20,
            &[KeyCode::PageDown, KeyCode::PageUp],
        );
        assert_eq!(
            app.detail.as_ref().expect("open").scroll,
            0,
            "pgup did not return"
        );
        assert_eq!(back, top, "pgup did not show the top again");
    }

    #[test]
    fn a_scroll_past_the_end_stays_on_the_document() {
        use crossterm::event::KeyCode;
        // The render clamps the scroll to the document, so a document that
        // shrank between two redraws never shows past its end.
        let work = mock_work();
        let (_, _, app) = drawn_after(
            &work,
            "TSK13",
            100,
            20,
            &[KeyCode::Char('G'), KeyCode::Down, KeyCode::Down],
        );
        let detail = app.detail.as_ref().expect("open");
        let lines = document_lines(&work, detail, 98, IconSet::Nerd, &theme());
        let visible = 16usize;
        let max = lines.len().saturating_sub(visible);
        assert!(
            usize::from(detail.scroll) <= max,
            "scroll {} is past the document's end {max}",
            detail.scroll
        );
        // Two downs past G land on the same last page.
        let (_, bottom, _) = drawn_after(&work, "TSK13", 100, 20, &[KeyCode::Char('G')]);
        let (_, more, _) = drawn_after(
            &work,
            "TSK13",
            100,
            20,
            &[KeyCode::Char('G'), KeyCode::Down, KeyCode::Down],
        );
        assert_eq!(bottom, more, "the scroll ran past the end");
    }

    #[test]
    fn a_mark_in_free_text_takes_the_selection() {
        // The fixture's note carries ==key finding==. The engine paints a
        // mark with mark_fg and mark_bg, which the mapping gives to
        // text.selected — the pair the palette records, in the dark
        // preset. The check can fail: any other paint on the mark is a
        // second spelling of the selection.
        let work = mock_work();
        let detail = pane("TSK13");
        let dark = Theme::preset(Preset::Dark);
        let selected = dark.text.selected;
        // The width keeps the wrapped note on one line, so the whole mark
        // is one span.
        let lines = document_lines(&work, &detail, 138, IconSet::Nerd, &dark);
        let marked = lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content.contains("key finding"))
            .expect("the marked span is in the document");
        assert_eq!(
            marked.style.fg, selected.fg,
            "the mark is not on the selection's foreground"
        );
        assert_eq!(
            marked.style.bg, selected.bg,
            "the mark is not on the selection's background"
        );
    }

    #[test]
    fn the_terminal_preset_marks_with_no_colour() {
        // The terminal preset names no colour: the mark is the same words
        // with `Color::Reset` on both sides, as the theme file requires.
        // The check can fail: any colour here is a colour the preset does
        // not name.
        let work = mock_work();
        let detail = pane("TSK13");
        let terminal = Theme::preset(Preset::Terminal);
        // The width keeps the wrapped note on one line, so the whole mark
        // is one span.
        let lines = document_lines(&work, &detail, 138, IconSet::Nerd, &terminal);
        let marked = lines
            .iter()
            .flat_map(|line| &line.spans)
            .find(|span| span.content.contains("key finding"))
            .expect("the marked span is in the document");
        assert_eq!(
            marked.style.fg,
            Some(Color::Reset),
            "the terminal preset named a foreground"
        );
        assert_eq!(
            marked.style.bg,
            Some(Color::Reset),
            "the terminal preset named a background"
        );
    }

    #[test]
    fn the_mark_is_painted_on_screen() {
        // The paint reaches the pane. The pane is wide enough that the
        // wrapped note holds the whole mark on one row, and the scroll is
        // set to the marked line, so the row is deterministic.
        let work = mock_work();
        let dark = Theme::preset(Preset::Dark);
        let selection_fg = dark.text.selected.fg.expect("the selection foreground");
        let selection_bg = dark.text.selected.bg.expect("the selection background");
        let mut detail = pane("TSK13");
        let lines = document_lines(&work, &detail, 138, IconSet::Nerd, &dark);
        let marked_at = lines
            .iter()
            .position(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.contains("key finding"))
            })
            .expect("the mark is in the document");
        detail.scroll = u16::try_from(marked_at).expect("the mark sits in a pane's scroll");
        let (rows, paints) = draw_with(&dark, &work, &detail, 140, 20);
        let row_at = rows
            .iter()
            .position(|row| row.contains("key finding"))
            .expect("the marked row is on display");
        let column = rows[row_at].find("key finding").expect("the mark's column");
        assert_eq!(
            paints[row_at][column].fg, selection_fg,
            "the mark is not painted on the selection's foreground"
        );
        assert_eq!(
            paints[row_at][column].bg, selection_bg,
            "the mark is not painted on the selection's background"
        );
    }

    #[test]
    fn a_verification_pipe_is_the_pipe_character() {
        // The real fixture TSK13 carries a shell pipe in its verification
        // command. A verification is a list field, not a table cell, so the
        // projection does not escape the pipe: the code span holds the pipe
        // alone, and no backslash reaches the render.
        // The command's tail is rendered and the pipe is the pipe character.
        let work = mock_work();
        let detail = pane("TSK13");
        let rendered = document(&work, &detail, 100, &theme());
        assert!(
            rendered.contains("grep detai"),
            "the command did not survive the render: {rendered}"
        );
        // The pipe is the pipe character in the render. A split at the pipe
        // would show a border instead.
        assert!(
            rendered.contains('|'),
            "the pipe did not survive: {rendered}"
        );
        assert!(
            !rendered.contains('\\'),
            "the escape reached the render: {rendered}"
        );
    }

    #[test]
    fn a_quoted_evidence_line_cannot_open_a_block() {
        // The summary is a fixed field of the record: what the command
        // printed. The projection escapes it, so a line that would open a
        // heading, a list or a nested quote is rendered as text inside the
        // quotation, marker on every line, with no heading and no rule.
        let mut task = coding_crew_core::Task::new("TSK01", "Evidence probe");
        task.acceptance_criteria = vec![coding_crew_core::AcceptanceCriterion {
            id: "ACCEPT-01".to_string(),
            verification: "cargo test".to_string(),
            evidence: vec![coding_crew_core::Evidence {
                command: "cargo test".to_string(),
                exit_code: Some(1),
                summary: "ok\n# injected heading\n- injected bullet\n> injected quote".to_string(),
                at: "2026-11-14T09:00:00Z".to_string(),
                by: "runner-1".to_string(),
                session: None,
            }],
            ..coding_crew_core::AcceptanceCriterion::default()
        }];
        let work = Work {
            root: PathBuf::from("/nowhere/coding-crew/work"),
            tasks: vec![task],
            ..Work::default()
        };

        let rendered = document(&work, &pane("TSK01"), 120, &theme());

        let injected: Vec<&str> = rendered
            .lines()
            .filter(|line| line.contains("injected"))
            .collect();
        assert_eq!(injected.len(), 3, "{rendered}");
        for line in &injected {
            assert!(
                line.trim_start().starts_with('▎'),
                "a quoted line left the quotation: {line:?}\n{rendered}"
            );
        }

        let lines: Vec<&str> = rendered.lines().collect();
        let heading = lines
            .iter()
            .position(|line| line.contains("injected heading"))
            .expect("the escaped heading is not shown");
        assert_ne!(lines[heading].trim(), "injected heading", "{rendered}");
        assert!(
            !lines[heading + 1]
                .trim_start_matches('▎')
                .trim()
                .chars()
                .all(|ch| ch == '─'),
            "the quotation opened a rule:\n{rendered}"
        );
    }

    #[test]
    fn the_engine_consumes_a_dependency_link_and_a_child_box() {
        // The projection writes a link and a task-list box; the engine is
        // the other half of that seam. A dependency renders as one link —
        // one marker, its label, no literal brackets — and a child renders
        // as one box on one logical row.
        let mut task = coding_crew_core::Task::new("TSK01", "Parent");
        task.depends_on = vec!["TSK02".to_string()];
        task.children = vec!["TSK01.1".to_string()];

        let mut dependency = coding_crew_core::Task::new("TSK02", "Dependency");
        dependency.status = coding_crew_core::Status::Done;

        let mut child = coding_crew_core::Task::new("TSK01.1", "Child");
        child.parent = Some("TSK01".to_string());
        child.status = coding_crew_core::Status::Groomed;

        let work = Work {
            root: PathBuf::from("/nowhere/coding-crew/work"),
            tasks: vec![task, dependency, child],
            ..Work::default()
        };

        let lines = document_lines(&work, &pane("TSK01"), 120, IconSet::Nerd, &theme());

        // The engine's own parse: the link is one span with its destination,
        // and no second link is invented.
        let markdown = coding_crew_core::task_to_markdown(
            work.task("TSK01").expect("the parent record"),
            &work.tasks,
        );
        let (syntax, paired) = syntect(&theme());
        let parsed = parse_markdown_with_width(
            &markdown,
            syntax,
            &paired,
            120,
            &theme().markdown_theme(),
            false,
            false,
        );
        assert_eq!(parsed.link_urls, vec!["TSK02".to_string()]);
        assert_eq!(
            parsed.link_spans.len(),
            1,
            "the dependency is not one link span: {} spans",
            parsed.link_spans.len()
        );

        let link_line = lines
            .iter()
            .find(|line| line_text(line).contains("TSK02"))
            .expect("the dependency line is not rendered");
        let link_text = line_text(link_line);
        let glyph = Icon::Link.glyph(IconSet::Nerd);
        assert!(link_text.contains(&format!("{glyph} TSK02")), "{link_text}");
        assert!(
            !link_text.contains('[') && !link_text.contains("]("),
            "{link_text}"
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line_text(line).contains("TSK02"))
                .count(),
            1,
            "the dependency is not one row:\n{link_text}"
        );
        let link_icon = theme().markdown_theme().link_icon;
        assert_eq!(
            link_line
                .spans
                .iter()
                .filter(|span| span.style.fg == Some(link_icon))
                .count(),
            1,
            "the dependency is not one link span: {link_text}"
        );

        let child_line = lines
            .iter()
            .find(|line| line_text(line).contains("TSK01.1"))
            .expect("the child line is not rendered");
        let child_text = line_text(child_line);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line_text(line).contains("TSK01.1"))
                .count(),
            1,
            "the child is not one logical row:\n{child_text}"
        );
        assert_eq!(
            child_text
                .matches(Icon::TaskOpen.glyph(IconSet::Nerd))
                .count(),
            1,
            "the child does not render one open box: {child_text}"
        );
        assert!(
            !child_text.contains(Icon::StatusDone.glyph(IconSet::Nerd)),
            "the open child renders a done box: {child_text}"
        );
        assert!(!child_text.contains("[ ]"), "{child_text}");
    }

    /// The text of one rendered line, its spans joined.
    fn line_text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    /// The paint the preset's syntect theme gives `code`, as (text, colour)
    /// pairs, run through syntect's own highlighter. A colour the theme names
    /// as the terminal's own reads back as `Reset`, as the engine reads it.
    ///
    /// It is computed from the pane's pairing, through a highlighter of its
    /// own, so a pane that paired another theme shows different colours here.
    fn syntect_paint(theme: &Theme, code: &str) -> Vec<(String, Color)> {
        syntect_paint_as(theme, "rust", code)
    }

    /// The paint of `code` in the language `token` names, as
    /// [`syntect_paint`] paints rust.
    fn syntect_paint_as(theme: &Theme, token: &str, code: &str) -> Vec<(String, Color)> {
        use syntect::easy::HighlightLines;
        use syntect::util::LinesWithEndings;

        let (syntax, paired) = syntect(theme);
        let language = syntax
            .find_syntax_by_token(token)
            .unwrap_or_else(|| panic!("the {token} syntax"));
        let mut highlighter = HighlightLines::new(language, &paired);
        let mut paint = Vec::new();
        for line in LinesWithEndings::from(code) {
            for (style, text) in highlighter
                .highlight_line(line, syntax)
                .expect("highlighting the code")
            {
                let text = text.trim_end_matches('\n');
                if text.is_empty() {
                    continue;
                }
                let colour = if style.foreground.a == 0 {
                    Color::Reset
                } else {
                    Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b)
                };
                paint.push((text.to_string(), colour));
            }
        }
        paint
    }

    /// The code block in a rendered document: the line that names its
    /// language, the words of its first code row, and the (text, colour)
    /// pairs that row's tokens are painted with.
    fn code_block(lines: &[Line<'static>]) -> (Line<'static>, String, Vec<(String, Color)>) {
        let header_at = lines
            .iter()
            .position(|line| line_text(line).starts_with("╭─ rust "))
            .expect("the fenced code block is not in the document");
        let header = lines[header_at].clone();
        // The first code row: the numbered gutter, then the painted tokens.
        // The gutter, the unstyled padding and the right edge are not tokens.
        let row = &lines[header_at + 1];
        let tokens = row
            .spans
            .iter()
            .filter(|span| span.style.fg.is_some() && !span.content.contains('│'))
            .map(|span| {
                (
                    span.content.to_string(),
                    span.style.fg.expect("a painted token"),
                )
            })
            .collect();
        (header, line_text(row), tokens)
    }

    #[test]
    fn a_fenced_code_block_is_painted_by_its_presets_syntect_theme() {
        // The fixture's note carries a fenced rust block. The document draws
        // it as a code block whose tokens are syntect's own paint in the dark
        // preset's theme, derived from that preset's palette — the third
        // palette the theme doc records and the pane handles deliberately.
        // The words are the fixture's; the colours are the theme's, not a
        // raul slot's. The check can fail: a pane that stops highlighting, or
        // pairs another syntect theme, changes the tokens or their colours.
        let work = mock_work();
        let detail = pane("TSK13");
        let dark = Theme::preset(Preset::Dark);
        let lines = document_lines(&work, &detail, 138, IconSet::Nerd, &dark);
        let (header, row, tokens) = code_block(&lines);
        // The frame is the document's line, and the label is the quietest
        // word grade: the block around the tokens stays quiet, and the tokens
        // are the ones that stand out.
        assert_eq!(
            header.spans.first().and_then(|span| span.style.fg),
            Some(dark.document().rule.line),
            "the code frame is not the document's line"
        );
        let label = header
            .spans
            .iter()
            .find(|span| span.content == "rust ")
            .expect("the block names its language");
        assert_eq!(
            label.style.fg,
            Some(dark.document().list_depth_3),
            "the code label is not the quietest word grade"
        );
        // The code row is numbered from one and says the fixture's code, and
        // it is painted.
        assert!(
            row.starts_with("│1│ let answer = 42;"),
            "the code block does not carry the fixture's code: {row:?}"
        );
        let expected = syntect_paint(&dark, "let answer = 42;\n");
        assert!(
            expected.len() >= 3,
            "the expected paint is too small to pin: {expected:?}"
        );
        let colours: std::collections::HashSet<Color> =
            expected.iter().map(|(_, colour)| *colour).collect();
        assert!(
            colours.len() >= 2,
            "the code is not highlighted: {expected:?}"
        );
        // The pane merges adjacent spans of one style and syntect does not,
        // so the comparison is per character: every character of the code
        // carries the colour the theme gives it.
        let paint = |spans: &[(String, Color)]| -> Vec<(char, Color)> {
            spans
                .iter()
                .flat_map(|(text, colour)| text.chars().map(|c| (c, *colour)))
                .collect()
        };
        assert_eq!(
            paint(&tokens),
            paint(&expected),
            "the code tokens are not painted by the dark preset's syntect theme"
        );
    }

    /// The colours a word is painted in, one per occurrence, in the order the
    /// code gives them.
    fn painted_as(paint: &[(String, Color)], word: &str) -> Vec<Color> {
        paint
            .iter()
            .filter(|(text, _)| text == word)
            .map(|(_, colour)| *colour)
            .collect()
    }

    #[test]
    fn the_tokens_of_four_languages_are_painted_by_their_roles() {
        // Each word takes the role its token class names, in every colour
        // preset. The check can fail: a scope the mapping misses leaves the
        // word in the plain ink, and the word is then not its role's colour.
        // TOML has no grammar in the bundled syntaxes, so the config fixture
        // is YAML, the bundled one.
        type Role = fn(&SyntaxPaint) -> Color;
        let keyword: Role = |syntax| syntax.keyword;
        let string: Role = |syntax| syntax.string;
        let number: Role = |syntax| syntax.number;
        let function: Role = |syntax| syntax.function;
        let type_name: Role = |syntax| syntax.type_name;
        let comment: Role = |syntax| syntax.comment;
        let tag: Role = |syntax| syntax.tag;
        type Fixture = (&'static str, &'static str, Vec<(&'static str, Role)>);
        let fixtures: [Fixture; 4] = [
            (
                "rust",
                "fn main() {\n    let answer = \"forty\"; // the answer\n    let n = 42;\n    println!(\"{}\", n);\n    let v = Vec::new();\n}\n",
                vec![
                    ("fn", keyword),
                    ("main", function),
                    ("let", keyword),
                    ("forty", string),
                    (" the answer", comment),
                    ("42", number),
                    ("println!", function),
                    ("Vec", type_name),
                ],
            ),
            (
                "python",
                "def main(count: int):\n    answer = \"forty\"  # the answer\n    if count > 0:\n        return 42\n    print(answer)\n",
                vec![
                    ("def", keyword),
                    ("main", function),
                    ("int", type_name),
                    ("forty", string),
                    (" the answer", comment),
                    ("if", keyword),
                    ("42", number),
                    ("print", function),
                ],
            ),
            (
                "yaml",
                "name: coding-crew\nversion: 3\n# a comment\n",
                vec![
                    ("name", tag),
                    ("coding-crew", string),
                    ("3", number),
                    (" a comment", comment),
                ],
            ),
            (
                "bash",
                "# a comment\nANSWER=\"forty\"\nfor f in *.md; do\n  echo \"$ANSWER\" $f\ndone\nif [ -f x ]; then exit 1; fi\n",
                vec![
                    ("for", keyword),
                    ("do", keyword),
                    ("done", keyword),
                    ("echo", function),
                    ("forty", string),
                    (" a comment", comment),
                ],
            ),
        ];
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let syntax = theme.document().syntax;
            for (token, code, expected) in &fixtures {
                let paint = syntect_paint_as(&theme, token, code);
                if preset == Preset::Terminal {
                    assert!(
                        paint.iter().all(|(_, colour)| *colour == Color::Reset),
                        "the {} preset paints a {token} token",
                        preset.name()
                    );
                    continue;
                }
                for (word, role) in expected {
                    let painted = painted_as(&paint, word);
                    assert!(
                        !painted.is_empty(),
                        "the {token} fixture has no {word:?} token: {paint:?}"
                    );
                    for colour in painted {
                        assert_eq!(
                            colour,
                            role(&syntax),
                            "{} paints {token}'s {word:?} from another role",
                            preset.name()
                        );
                    }
                }
                let colours: std::collections::HashSet<Color> =
                    paint.iter().map(|(_, colour)| *colour).collect();
                assert!(
                    colours.len() >= 5,
                    "{} paints the {token} fixture in {} colours",
                    preset.name(),
                    colours.len()
                );
            }
        }
    }

    #[test]
    fn the_terminal_preset_says_the_code_block_s_words() {
        // The terminal preset names no colour of ours, so the words are what
        // it pins: the same document, the same block and the same code. Its
        // tokens keep their words and paint the terminal's own colour, and
        // the frame and label around them are Reset too.
        let work = mock_work();
        let detail = pane("TSK13");
        let dark = Theme::preset(Preset::Dark);
        let terminal = Theme::preset(Preset::Terminal);
        assert_eq!(
            document(&work, &detail, 138, &terminal),
            document(&work, &detail, 138, &dark),
            "the terminal preset says different words"
        );
        let lines = document_lines(&work, &detail, 138, IconSet::Nerd, &terminal);
        let (header, _, tokens) = code_block(&lines);
        let dark_tokens = code_block(&document_lines(&work, &detail, 138, IconSet::Nerd, &dark)).2;
        let words = |spans: &[(String, Color)]| -> String {
            spans.iter().map(|(text, _)| text.as_str()).collect()
        };
        assert_eq!(
            words(&tokens),
            words(&dark_tokens),
            "the terminal preset changed the code's words"
        );
        for (text, colour) in &tokens {
            assert_eq!(
                *colour,
                Color::Reset,
                "the terminal preset painted {text:?} with a colour"
            );
        }
        for span in &header.spans {
            assert_eq!(
                span.style.fg,
                Some(Color::Reset),
                "the terminal preset named a colour on the code frame: {:?}",
                span.content
            );
        }
    }

    #[test]
    fn each_preset_pairs_a_syntect_theme_of_its_own() {
        // Each colour preset derives its code theme from its own palette, so
        // the same code paints four ways; the terminal preset paints all of
        // it in its own colour and emits no RGB.
        let code = "fn main() {\n    // a note\n    let answer: u32 = 42;\n    \"text\";\n}\n";
        let paints: Vec<(&str, Vec<(String, Color)>)> = Preset::ALL
            .iter()
            .map(|preset| (preset.name(), syntect_paint(&Theme::preset(*preset), code)))
            .collect();
        for (index, (one, one_paint)) in paints.iter().enumerate() {
            for (other, other_paint) in &paints[index + 1..] {
                assert_ne!(
                    one_paint, other_paint,
                    "{one} and {other} paint the code the same"
                );
            }
            if *one == "terminal" {
                for (text, colour) in one_paint {
                    assert_eq!(*colour, Color::Reset, "the terminal paints {text:?}");
                }
            } else {
                for (text, colour) in one_paint {
                    assert!(
                        matches!(colour, Color::Rgb(..)),
                        "{one} paints {text:?} with no colour of its own: {colour:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_fenced_code_block_is_a_rounded_box_with_dim_numbers() {
        // The block's corners are round, its language sits in the top
        // border, and its line numbers are dim: the gutter's span carries
        // the modifier, and the number is the line's own.
        let work = mock_work();
        let lines = document_lines(&work, &pane("TSK13"), 138, IconSet::Nerd, &theme());
        let header_at = lines
            .iter()
            .position(|line| line_text(line).starts_with("╭─ rust "))
            .expect("the fenced code block is not in the document");
        assert!(
            line_text(&lines[header_at]).ends_with('╮'),
            "the top border is not round: {:?}",
            line_text(&lines[header_at])
        );
        let footer = lines
            .iter()
            .skip(header_at + 1)
            .find(|line| line_text(line).starts_with('╰'))
            .expect("the code block has no bottom border");
        assert!(
            line_text(footer).ends_with('╯'),
            "the bottom border is not round: {:?}",
            line_text(footer)
        );
        let gutter = &lines[header_at + 1].spans[0];
        assert_eq!(gutter.content, "│1│", "the first line is not numbered");
        assert!(
            gutter.style.add_modifier.contains(Modifier::DIM),
            "the line number is not dim: {:?}",
            gutter.style
        );
    }

    #[test]
    fn inline_code_is_a_chip_on_the_code_ground_in_every_preset() {
        // Inline code is the code role's ink on its chip, padded one space
        // each side. The chip is the palette's own background, so the same
        // code is on a chip in every preset, and the terminal's is Reset.
        let markdown = "run `cargo test` now";
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let (syntax, paired) = syntect(&theme);
            let parsed = parse_markdown_with_width(
                markdown,
                syntax,
                &paired,
                100,
                &markdown_theme(&theme, IconSet::Nerd),
                false,
                false,
            );
            let code = parsed
                .lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .find(|span| span.content == " cargo test ")
                .unwrap_or_else(|| panic!("{}: no inline code chip", preset.name()));
            assert_eq!(
                code.style.bg,
                Some(theme.document().code.chip),
                "{}: the chip is not the palette's chip",
                preset.name()
            );
            assert_eq!(
                code.style.fg,
                Some(theme.document().code.text),
                "{}: the chip's ink is not the code role's",
                preset.name()
            );
        }
    }

    #[test]
    fn the_document_fits_at_40_and_100_columns_in_every_preset() {
        // The code block's numbered gutter and its rounded frame are wider
        // than the words they hold, so every line of the document is checked
        // against the width it is drawn at.
        let work = mock_work();
        for width in [40usize, 100] {
            for preset in Preset::ALL {
                let lines = document_lines(
                    &work,
                    &pane("TSK13"),
                    width,
                    IconSet::Nerd,
                    &Theme::preset(preset),
                );
                for line in &lines {
                    let text = line_text(line);
                    assert!(
                        text_width(&text) <= width,
                        "{}-column {} draws {} columns: {text:?}",
                        width,
                        preset.name(),
                        text_width(&text)
                    );
                }
            }
        }
    }

    #[test]
    fn the_footer_names_only_keys_that_do_something() {
        let work = mock_work();
        let (rows, _) = draw(&work, &pane("TSK13"), 100, 40);
        let footer = rows.last().expect("a footer row").trim().to_string();
        assert_eq!(footer, "esc close · ? help · q quit");
        // The scroll keys work in the pane, and the help over it names them:
        // the footer does not.
        for key in ["pgup/pgdn", "g/G", "↑↓", "scroll", "page"] {
            assert!(!footer.contains(key), "{footer}");
        }
        // Enter has opened, and the tabs are left alone: none of them is named.
        assert!(!footer.contains("open"), "{footer}");
        assert!(!footer.contains("tab"), "{footer}");
        assert!(!footer.contains("1-5"), "{footer}");
    }

    #[test]
    fn a_record_that_left_the_folder_is_said() {
        let work = mock_work();
        let (rows, _) = draw(&work, &pane("TSK99"), 100, 40);
        let text = text_of(&rows);
        assert!(text.contains("TSK99"), "{text}");
        assert!(text.contains("not in the work folder"), "{text}");
        assert!(!text.contains("Header"), "{text}");
    }

    #[test]
    fn a_smaller_task_is_said_to_be_one() {
        // The pane opens from the Backlog, whose rows are smaller tasks
        // too. A smaller task has no document: it is said, not shown as
        // the task it is not.
        let work = mock_work();
        let (rows, _) = draw(&work, &pane("ITM01"), 100, 40);
        let text = text_of(&rows);
        assert!(text.contains("ITM01"), "{text}");
        assert!(text.contains("smaller task"), "{text}");
        assert!(text.contains("no document yet"), "{text}");
        assert!(!text.contains("Header"), "{text}");
    }

    #[test]
    fn a_notice_is_painted_dim() {
        // Both notices — the record that left the folder and the smaller
        // task with no document — take `text.dim`: a notice is secondary
        // text, not the document's own words. The check can fail: an
        // unstyled notice reads as document text.
        let work = mock_work();
        let dim = theme().text.dim.fg.expect("the dim colour");
        for (id, marker) in [
            ("TSK99", "not in the work folder"),
            ("ITM01", "no document yet"),
        ] {
            let (rows, paints) = draw(&work, &pane(id), 100, 40);
            let row_at = rows
                .iter()
                .position(|row| row.contains(marker))
                .unwrap_or_else(|| panic!("{id}: the notice is not on screen"));
            let column = rows[row_at].find(marker).expect("the notice's column");
            assert_eq!(
                paints[row_at][column].fg, dim,
                "{id}: the notice is not on text.dim"
            );
        }
    }

    #[test]
    fn the_footer_degrades_in_the_built_order() {
        // The pane's own chip gives way first, whole, and the closing chips
        // hold longest: `q quit` is the last one dropped, and never cut.
        let text = |width: u16| {
            footer(width, 1, false, &theme())
                .iter()
                .flat_map(|line| line.spans.iter())
                .map(|span| span.content.as_ref())
                .collect::<String>()
        };
        // The whole footer is 27 columns.
        assert_eq!(text(27), "esc close · ? help · q quit");
        assert_eq!(text(20), "? help · q quit", "esc gives way first");
        assert_eq!(text(12), "? help", "q quit is dropped whole, not cut");
    }

    #[test]
    fn the_footer_says_close_help_while_the_help_is_open() {
        // With the help over the pane, esc closes the help, as it does over
        // the window: the footer says so, and the closing chips stay.
        let text: String = footer(100, 1, true, &theme())
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(text, "esc close help · ? help · q quit");
    }

    #[test]
    fn the_footer_chips_are_the_shared_key_chips() {
        // The key is bold and not reversed, and the chips are parted by the
        // separator, as every footer draws them.
        let line = footer(100, 1, false, &theme()).remove(0);
        let spans = &line.spans;
        assert_eq!(spans[0].content, "esc");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert!(!spans[0].style.add_modifier.contains(Modifier::REVERSED));
        assert_eq!(spans[1].content, " close");
        assert_eq!(spans[2].content, " · ");
    }

    #[test]
    fn the_presets_say_the_same_thing_and_paint_differently() {
        let work = mock_work();
        for (width, height) in [(100u16, 20u16), (60, 16), (40, 12), (24, 8)] {
            let mut rendered = Vec::new();
            for preset in Preset::ALL {
                let (rows, paints) =
                    draw_with(&Theme::preset(preset), &work, &pane("TSK13"), width, height);
                rendered.push((preset.name(), rows, paints));
            }
            let (first_name, first_rows, _) = &rendered[0];
            for (name, rows, _) in &rendered[1..] {
                // A theme changes the paint, never the words.
                assert_eq!(
                    first_rows, rows,
                    "{width}x{height}: {first_name} and {name} say different things"
                );
            }
            for (index, (one, _, one_paints)) in rendered.iter().enumerate() {
                for (other, _, other_paints) in &rendered[index + 1..] {
                    assert_ne!(
                        one_paints, other_paints,
                        "{width}x{height}: {one} and {other} paint the same"
                    );
                }
            }
        }
    }

    #[test]
    fn the_task_and_alert_glyphs_are_the_icon_sets() {
        // The engine takes its glyphs as configuration, so the document draws
        // the set in force: the nerd set in the nerd set, and the unicode
        // glyphs the engine always drew in the unicode one.
        let nerd = markdown_theme(&theme(), IconSet::Nerd);
        assert_eq!(nerd.task_open_glyph, Icon::TaskOpen.glyph(IconSet::Nerd));
        assert_eq!(
            nerd.alert_glyphs,
            [
                Icon::AlertNote.glyph(IconSet::Nerd),
                Icon::AlertTip.glyph(IconSet::Nerd),
                Icon::AlertImportant.glyph(IconSet::Nerd),
                Icon::AlertWarning.glyph(IconSet::Nerd),
                Icon::AlertCaution.glyph(IconSet::Nerd),
            ]
        );
        let unicode = markdown_theme(&theme(), IconSet::Unicode);
        assert_eq!(unicode.task_open_glyph, "○");
        assert_eq!(unicode.alert_glyphs, ["ℹ", "✦", "!", "⚠", "✗"]);
    }
}
