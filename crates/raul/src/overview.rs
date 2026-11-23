//! The Overview tab: what is the state of the project.
//!
//! Not a list — a summary the human reads in one screen.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph, Wrap};

use coding_crew_core::{ActivityEvent, EventKind, Status};

use crate::data::{Missing, Reason, Work};
use crate::needs::reason_icon;
use coding_crew_theme::chrome;
use coding_crew_theme::text::{cut_to, head_to, text_width};
use coding_crew_theme::{Icon, IconSet, Theme, phase_slot, phase_style, reason_style};

/// From this width the cards take two columns; below it they stack in one.
const TWO_COLUMNS: u16 = 80;
/// The rows a block of the Overview takes for its border.
const BORDER: u16 = 2;
/// The columns a card takes beside its content: its two borders, and one
/// column of padding inside each.
const CARD_COLUMNS: u16 = 4;
/// The columns the label of a count takes, so the counts line up.
const LABEL_WIDTH: usize = 12;
/// The events the activity block shows: the last few of the journal.
const ACTIVITY_SHOWN: usize = 5;
/// What the activity block says when the journal itself cannot be read.
const JOURNAL_UNREADABLE: &str = "the journal cannot be read";

/// Draw the tab.
///
/// `now` is the frame's instant: the claimable count, the summary line and
/// the next task all answer from it, so one screen cannot draw a count and
/// a next task from two clocks.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    theme: &Theme,
    icons: IconSet,
    now: &str,
) {
    if let Some(missing) = work.missing {
        render_missing(frame, area, work, missing, icons, theme);
        return;
    }
    if work.is_empty() {
        render_empty(frame, area, work, icons, theme);
        return;
    }
    if area.width < TWO_COLUMNS {
        render_stacked(frame, area, work, icons, theme, now);
    } else {
        render_wide(frame, area, work, icons, theme, now);
    }
}

/// There is no work folder.
fn render_missing(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    missing: Missing,
    icons: IconSet,
    theme: &Theme,
) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(missing.headline(), theme.semantic.error)),
            Line::from(Span::styled(
                format!("looked in {}", work.root.display()),
                theme.text.dim,
            )),
        ])
        .wrap(Wrap { trim: true })
        .block(card("overview", Icon::TabOverview, icons, theme)),
        area,
    );
}

/// There is nothing to read — but a file that does not parse still shows.
fn render_empty(frame: &mut Frame, area: Rect, work: &Work, icons: IconSet, theme: &Theme) {
    let empty = Paragraph::new(Line::from(Span::styled("no tasks yet", theme.text.dim)))
        .block(card("overview", Icon::TabOverview, icons, theme));
    if work.errors.is_empty() {
        frame.render_widget(
            empty,
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 3.min(area.height),
            },
        );
        return;
    }
    let panel = errors_panel(work, theme);
    let width = area.width.saturating_sub(CARD_COLUMNS);
    let needed = panel.rows(width) + BORDER;
    // The empty state in its box, and the files that could not be read.
    if area.height >= 3 + needed {
        frame.render_widget(
            empty,
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 3,
            },
        );
        frame.render_widget(
            errors_block(&panel, icons, theme),
            Rect {
                x: area.x,
                y: area.y + 3,
                width: area.width,
                height: needed,
            },
        );
        return;
    }
    // Not enough room for both: the empty state gives up its box, and the
    // file that could not be read is named.
    let empty_rows = u16::from(area.height >= 2);
    if empty_rows == 1 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no tasks yet", theme.text.dim))),
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
    }
    let y = area.y + empty_rows;
    let height = area.height - empty_rows;
    if height >= needed {
        frame.render_widget(
            errors_block(&panel, icons, theme),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: needed,
            },
        );
    } else if height >= 1 {
        // The block does not fit: the file is named on one row.
        frame.render_widget(
            Paragraph::new(error_marker(work, area.width, theme)),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
        );
    }
}

/// A card of the Overview: a pane with one column of padding inside each
/// border, so its text does not sit against the frame.
fn card(title: &'static str, icon: Icon, icons: IconSet, theme: &Theme) -> Block<'static> {
    chrome::pane(title, icon, icons, false, theme).padding(Padding::horizontal(1))
}

/// The files that could not be read, in a box.
fn errors_block(panel: &Panel, icons: IconSet, theme: &Theme) -> Paragraph<'static> {
    Paragraph::new(panel.lines.clone())
        .wrap(Wrap { trim: true })
        .block(card(panel.title, panel.icon, icons, theme))
}

/// The line that names a file that could not be read, when the block does
/// not fit.
fn error_marker(work: &Work, width: u16, theme: &Theme) -> Line<'static> {
    let first = &work.errors[0];
    let mut text = format!("{}: {}", first.file, first.error);
    if work.errors.len() > 1 {
        text.push_str(&format!(" (+{} more)", work.errors.len() - 1));
    }
    Line::from(Span::styled(
        cut_to(&text, usize::from(width)),
        theme.semantic.error,
    ))
}

/// Two columns: the counts on the left, the numbers that ask for a decision
/// on the right.
fn render_wide(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    icons: IconSet,
    theme: &Theme,
    now: &str,
) {
    let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(area);
    // The unreadable block outranks the kinds: a file that could not be read
    // matters more than how many smaller tasks there are.
    let mut left = vec![counts_panel(work, columns[0].width, icons, theme)];
    if !work.errors.is_empty() {
        left.push(errors_panel(work, theme));
    }
    left.push(kinds_panel(work, columns[0].width, icons, theme));
    let right = vec![
        needs_you_panel(work, columns[1].width, icons, theme),
        claimable_panel(work, icons, theme, now),
        activity_panel(work, columns[1].width, icons, theme),
        next_panel(work, icons, theme, now),
    ];
    let fits = column_fits(
        &left,
        columns[0].width.saturating_sub(CARD_COLUMNS),
        columns[0].height,
    ) && column_fits(
        &right,
        columns[1].width.saturating_sub(CARD_COLUMNS),
        columns[1].height,
    );
    let area = summary_area(frame, area, work, fits, theme, now);
    let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(area);
    draw_panels(frame, columns[0], &left, icons, theme);
    draw_panels(frame, columns[1], &right, icons, theme);
}

/// One column, for a narrow terminal.
fn render_stacked(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    icons: IconSet,
    theme: &Theme,
    now: &str,
) {
    let mut panels = vec![
        needs_you_panel(work, area.width, icons, theme),
        claimable_panel(work, icons, theme, now),
        next_panel(work, icons, theme, now),
    ];
    panels.push(counts_panel(work, area.width, icons, theme));
    // The unreadable block outranks the kinds: a file that could not be read
    // matters more than how many smaller tasks there are.
    if !work.errors.is_empty() {
        panels.push(errors_panel(work, theme));
    }
    panels.push(kinds_panel(work, area.width, icons, theme));
    // The activity block is the first to give up its rows: it is pushed last,
    // and the layout reduces from the last block. A short pane keeps the
    // counts and the next task, and the block says how many events it left
    // out.
    panels.push(activity_panel(work, area.width, icons, theme));
    let width = area.width.saturating_sub(CARD_COLUMNS);
    let fits = column_fits(&panels, width, area.height);
    let area = summary_area(frame, area, work, fits, theme, now);
    draw_panels(frame, area, &panels, icons, theme);
}

/// The area the blocks get, and the summary line above them.
///
/// The key numbers stay visible, whatever the height and the width: when a
/// block does not fit, the numbers go on top of the blocks.
fn summary_area(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    fits: bool,
    theme: &Theme,
    now: &str,
) -> Rect {
    if fits || area.height == 0 {
        return area;
    }
    let Some(summary) = summary_line(work, area.width, theme, now) else {
        return area;
    };
    frame.render_widget(
        Paragraph::new(summary),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );
    Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height - 1,
    }
}

/// Whether every block of the column is drawn.
fn column_fits(panels: &[Panel], width: u16, height: u16) -> bool {
    layout_of(panels, width, height)
        .iter()
        .all(|rows| *rows > 0)
}

/// The key numbers on one line, for a terminal too short for the blocks.
///
/// The longest form that fits the row is the one shown, so a part of the
/// line is never cut in half. When not even the shortest form fits, there
/// is no summary line: a cut number is worse than none. Every form keeps
/// the qualifier of its number, so a number on this line is never read as
/// the number of tasks in the project.
///
/// The lane facts are painted with the lane slots, the same as in the
/// blocks: the same fact is never painted two ways on one screen.
fn summary_line(work: &Work, width: u16, theme: &Theme, now: &str) -> Option<Line<'static>> {
    let (approve, review, escalated) = work.needs_you_counts();
    let total = approve + review + escalated;
    let claimable = work.claimable_at(now).len();
    // The long forms of the two lane facts, and the short ones.
    let human_long = format!("{} {} on a human", task_word(total), wait_word(total));
    let human_brief = format!("{} a human", need_word(total));
    let human_short = wait_word(total).to_string();
    let claim_long = "claimable".to_string();
    let claim_short = "claim".to_string();
    let separator = || Span::styled(" · ", theme.text.dim);
    let mut forms: Vec<Vec<Span<'static>>> = Vec::new();
    if let Some(task) = work.next_at(now) {
        forms.push(
            [
                lane_span(total, &human_long, theme.semantic.human, theme),
                vec![separator()],
                lane_span(claimable, &claim_long, theme.semantic.agent, theme),
                vec![separator()],
                vec![Span::styled(format!("next {}", task.id), theme.text.strong)],
            ]
            .concat(),
        );
    }
    for (human, claim) in [
        (&human_long, &claim_long),
        (&human_brief, &claim_long),
        (&human_brief, &claim_short),
        (&human_short, &claim_long),
        (&human_short, &claim_short),
    ] {
        forms.push(
            [
                lane_span(total, human, theme.semantic.human, theme),
                vec![separator()],
                lane_span(claimable, claim, theme.semantic.agent, theme),
            ]
            .concat(),
        );
    }
    forms
        .into_iter()
        .find(|form| width_of(form) <= usize::from(width))
        .map(Line::from)
}

/// A lane fact on the summary line: the count, and the words that name the
/// lane it belongs to.
///
/// The count is a count and the words are the lane, the same way the
/// blocks paint them.
fn lane_span(count: usize, words: &str, lane: Style, theme: &Theme) -> Vec<Span<'static>> {
    vec![
        Span::styled(count.to_string(), theme.text.strong),
        Span::styled(format!(" {words}"), lane),
    ]
}

/// The columns a line takes.
fn width_of(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

/// A block of the Overview: a titled box with lines in it.
struct Panel {
    title: &'static str,
    /// The icon the title carries.
    icon: Icon,
    /// The lines it shows when there is room.
    lines: Vec<Line<'static>>,
    /// The lines it shows when there is less room: complete, but denser.
    compact: Option<Vec<Line<'static>>>,
    /// Whether the dense form is only used when each of its lines fits on
    /// one row. A dense form that wraps in the middle of a value puts the
    /// value out of the reader's reach.
    compact_whole: bool,
    /// The lines at the end that may be dropped when there is no room.
    optional: usize,
    /// Whether the block is left out when it does not fit whole.
    atomic: bool,
    /// What the hidden lines are called.
    more: &'static str,
    /// What the marker says when the block is left out.
    marker: Option<Marker>,
}

/// A marker: what a left-out block says, and how it is painted.
struct Marker {
    text: String,
    style: Style,
}

impl Panel {
    /// The lines to draw in a box of `content` rows and `width` columns.
    ///
    /// A block that cannot show everything says how much is hidden, so a
    /// count is never shown as if it were the whole count.
    fn shown(&self, width: u16, content: u16, theme: &Theme) -> Vec<Line<'static>> {
        if fits(&self.lines, width, content) {
            return self.lines.clone();
        }
        let essential = self.lines.len().saturating_sub(self.optional);
        if essential > 0 && fits(&self.lines[..essential], width, content) {
            return self.lines[..essential].to_vec();
        }
        if let Some(compact) = self.dense(width)
            && fits(compact, width, content)
        {
            return compact.to_vec();
        }
        if self.atomic {
            return Vec::new();
        }
        if content < 2 {
            return Vec::new();
        }
        cut(&self.lines, width, content, self.more, theme)
    }

    /// The dense form, when it is worth using.
    ///
    /// A dense form that wraps in the middle of a value hides the value from
    /// the reader, so a block that asks for one is drawn in its full form
    /// instead.
    fn dense(&self, width: u16) -> Option<&[Line<'static>]> {
        let compact = self.compact.as_ref()?;
        if self.compact_whole
            && compact
                .iter()
                .any(|line| rows_of(std::slice::from_ref(line), width) > 1)
        {
            return None;
        }
        Some(compact)
    }

    /// The rows the block wants.
    fn rows(&self, width: u16) -> u16 {
        rows_of(&self.lines, width)
    }

    /// The rows the block needs when it gives up its last lines.
    fn essential_rows(&self, width: u16) -> u16 {
        let essential = self.lines.len().saturating_sub(self.optional);
        rows_of(&self.lines[..essential], width)
    }

    /// The rows the dense form takes, when it is worth using.
    fn dense_rows(&self, width: u16) -> Option<u16> {
        let compact = self.dense(width)?;
        Some(rows_of(compact, width))
    }
}

/// Whether the lines fit in the box.
fn fits(lines: &[Line<'static>], width: u16, content: u16) -> bool {
    rows_of(lines, width) <= content
}

/// The lines that fit, and a line saying how many do not.
fn cut(
    lines: &[Line<'static>],
    width: u16,
    content: u16,
    more: &str,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut shown: Vec<Line<'static>> = Vec::new();
    let mut used = 0u16;
    for line in lines {
        let rows = rows_of(std::slice::from_ref(line), width);
        if used + rows + 1 > content {
            break;
        }
        used += rows;
        shown.push(line.clone());
    }
    let hidden = lines.len() - shown.len();
    if hidden > 0 {
        shown.push(Line::from(Span::styled(
            format!("+{hidden} more {more}"),
            theme.text.dim,
        )));
    }
    shown
}

/// The rows each block of a column gets.
///
/// Every block gets the rows it needs. The blocks that come last give up
/// their rows first — they are the ones the reader can do without. A block
/// gives up its last lines, then takes its dense form, and is left out only
/// when there is nothing left to give. A block that is left out says so,
/// so nothing disappears without a word.
fn layout_of(panels: &[Panel], width: u16, height: u16) -> Vec<u16> {
    let mut content: Vec<u16> = panels.iter().map(|panel| panel.rows(width)).collect();
    for index in (0..panels.len()).rev() {
        while total(&content) + dropped_markers(&content) > height {
            if !reduce(&panels[index], &mut content[index], width) {
                break;
            }
        }
        if total(&content) + dropped_markers(&content) <= height {
            break;
        }
    }
    content
}

/// Give a block fewer rows: first the lines it can do without, then its
/// dense form, and last the block itself.
fn reduce(panel: &Panel, content: &mut u16, width: u16) -> bool {
    let essential = panel.essential_rows(width);
    if *content > essential {
        *content = essential;
        return true;
    }
    if let Some(dense) = panel.dense_rows(width)
        && *content > dense
    {
        *content = dense;
        return true;
    }
    if *content > 0 {
        *content = 0;
        return true;
    }
    false
}

/// Draw the blocks of a column, top to bottom.
fn draw_panels(frame: &mut Frame, area: Rect, panels: &[Panel], icons: IconSet, theme: &Theme) {
    let width = area.width.saturating_sub(CARD_COLUMNS);
    let content = layout_of(panels, width, area.height);
    let mut y = area.y;
    for (index, panel) in panels.iter().enumerate() {
        if content[index] == 0 {
            // The block was left out: say so, where it was.
            if y < area.y + area.height {
                frame.render_widget(
                    Paragraph::new(marker(panel, area.width, theme)),
                    Rect {
                        x: area.x,
                        y,
                        width: area.width,
                        height: 1,
                    },
                );
            }
            y += 1;
            continue;
        }
        let lines = panel.shown(width, content[index], theme);
        if lines.is_empty() {
            // The block has nothing that fits: say so rather than show
            // nothing at all.
            if y < area.y + area.height {
                frame.render_widget(
                    Paragraph::new(marker(panel, area.width, theme)),
                    Rect {
                        x: area.x,
                        y,
                        width: area.width,
                        height: 1,
                    },
                );
            }
            y += 1;
            continue;
        }
        let rows = rows_of(&lines, width);
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).block(card(
                panel.title,
                panel.icon,
                icons,
                theme,
            )),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: rows + BORDER,
            },
        );
        y += rows + BORDER;
    }
}

/// The rows the markers of the left out blocks take.
fn dropped_markers(content: &[u16]) -> u16 {
    content.iter().filter(|rows| **rows == 0).count() as u16
}

/// The line that says a block was left out.
fn marker(panel: &Panel, width: u16, theme: &Theme) -> Line<'static> {
    let (text, style) = match &panel.marker {
        Some(marker) => (marker.text.clone(), marker.style),
        None => (format!("{} not shown", panel.title), theme.text.dim),
    };
    Line::from(Span::styled(cut_to(&text, usize::from(width)), style))
}

/// What the marker of the unreadable block says: how many files it holds,
/// then the files and what is wrong with them.
fn hidden_files(work: &Work) -> String {
    let files: Vec<String> = work
        .errors
        .iter()
        .map(|error| format!("{}: {}", error.file, error.error))
        .collect();
    let count = files.len();
    let word = if count == 1 { "file" } else { "files" };
    format!(
        "unreadable not shown: {count} {word} ({})",
        files.join(", ")
    )
}

/// The rows the blocks take, borders included.
fn total(content: &[u16]) -> u16 {
    content
        .iter()
        .filter(|rows| **rows > 0)
        .map(|rows| rows + BORDER)
        .sum()
}

/// Tasks per status: the total, then a row per status with its count and a
/// bar sized to the largest count.
fn counts_panel(work: &Work, width: u16, icons: IconSet, theme: &Theme) -> Panel {
    let counts = work.status_counts();
    let total: usize = counts.iter().map(|(_, count)| count).sum();
    let heading = counts_heading(total, theme);
    let inner = width.saturating_sub(CARD_COLUMNS);
    let label_width = counts
        .iter()
        .map(|(status, _)| status.as_str().chars().count())
        .max()
        .unwrap_or(0);
    let digits = count_width(counts.iter().map(|(_, count)| *count));
    let largest = counts.iter().map(|(_, count)| *count).max().unwrap_or(0);
    // The row without its bar: the icon and the space after it, the label, a
    // space and the count. The count stands apart from its label.
    let row = 2 + label_width + 1 + digits;
    let dense = dense_counts(&counts, total, inner, icons, theme);
    let mut lines = vec![heading];
    if usize::from(inner) >= row {
        // The bar takes what is left of the card, after a space.
        let bar = usize::from(inner).saturating_sub(row + 1);
        for (status, count) in &counts {
            let paint = phase_style(theme, *status);
            let mut spans = vec![
                Span::styled(format!("{} ", status_icon(*status).glyph(icons)), paint),
                Span::styled(format!("{:<label_width$}", status.as_str()), paint),
                Span::styled(format!(" {count:>digits$}"), theme.text.strong),
            ];
            if bar > 0 {
                spans.push(Span::raw(" "));
                spans.extend(chrome::progress_bar(
                    *count,
                    largest,
                    bar,
                    phase_slot(*status),
                    theme,
                ));
            }
            lines.push(Line::from(spans));
        }
    } else {
        // The rows do not fit the card. The dense form keeps every count with
        // its status; when it does not fit either, the block is left out whole.
        lines = dense.clone().unwrap_or_default();
    }
    Panel {
        title: "counts",
        icon: Icon::TabOverview,
        lines,
        compact: dense,
        // The counts are read together: the dense form is used only when
        // every count stays with its status.
        compact_whole: true,
        optional: 0,
        atomic: false,
        more: "rows",
        marker: None,
    }
}

/// The heading of the counts block.
fn counts_heading(total: usize, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{total} "), theme.text.strong),
        Span::styled(
            format!("{} by status", task_word(total)),
            theme.semantic.info,
        ),
    ])
}

/// The icon of a status: its phase's icon.
fn status_icon(status: Status) -> Icon {
    Icon::for_status(status.as_str()).expect("every status has its icon")
}

/// The dense form of the counts: a status and its count on the same row.
///
/// The rows are built to fit the column, so a count is never parted from
/// its status. A status that does not fit on a row of its own leaves the
/// dense form out.
fn dense_counts(
    counts: &[(Status, usize)],
    total: usize,
    inner: u16,
    icons: IconSet,
    theme: &Theme,
) -> Option<Vec<Line<'static>>> {
    let inner = usize::from(inner);
    let mut rows: Vec<Vec<Span<'static>>> = Vec::new();
    let mut row: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for (status, count) in counts {
        let label = status.as_str();
        // The icon and the space after it lead the label.
        let width = 2 + label.chars().count() + 1 + count.to_string().chars().count();
        if width > inner {
            return None;
        }
        if !row.is_empty() {
            if used + 1 + width > inner {
                rows.push(std::mem::take(&mut row));
                used = 0;
            } else {
                row.push(Span::raw(" "));
                used += 1;
            }
        }
        let paint = phase_style(theme, *status);
        row.push(Span::styled(
            format!("{} ", status_icon(*status).glyph(icons)),
            paint,
        ));
        row.push(Span::styled(label.to_string(), paint));
        row.push(Span::raw(" "));
        row.push(Span::styled(count.to_string(), theme.text.strong));
        used += width;
    }
    if !row.is_empty() {
        rows.push(row);
    }
    let mut lines = vec![counts_heading(total, theme)];
    lines.extend(rows.into_iter().map(Line::from));
    Some(lines)
}

/// Smaller tasks per kind.
///
/// The label is padded to the width of the column, so a count stays on the
/// row of its label. A kind whose label and count do not fit on one row
/// leaves the block out whole.
fn kinds_panel(work: &Work, width: u16, icons: IconSet, theme: &Theme) -> Panel {
    let kinds = work.kind_counts();
    let inner = usize::from(width.saturating_sub(CARD_COLUMNS));
    let digits = count_width(kinds.iter().map(|(_, count)| *count));
    let label_width = kinds
        .iter()
        .map(|(kind, _)| kind.as_str().chars().count())
        .max()
        .unwrap_or(0);
    // The label is padded as far as the column allows, after the icon and
    // the space that follows it.
    let pad = LABEL_WIDTH.min(inner.saturating_sub(2 + digits + 1));
    let mut lines = vec![Line::from(Span::styled(
        "smaller tasks by kind",
        theme.semantic.info,
    ))];
    if label_width < pad {
        for (kind, count) in &kinds {
            let icon = Icon::for_kind(kind.as_str()).expect("every kind has its icon");
            lines.push(Line::from(vec![
                Span::styled(format!("{} ", icon.glyph(icons)), theme.text.normal),
                Span::styled(format!("{:<pad$}", kind.as_str()), theme.text.normal),
                Span::styled(count.to_string(), theme.text.strong),
            ]));
        }
    } else {
        // A kind and its count do not keep a gap on one row: the block is left
        // out whole rather than showing a count on a row of its own, or against
        // its label.
        lines.clear();
    }
    Panel {
        title: "smaller tasks",
        icon: Icon::TabBacklog,
        lines,
        compact: None,
        compact_whole: false,
        optional: 0,
        atomic: false,
        more: "kinds",
        marker: None,
    }
}

/// What waits on a human.
fn needs_you_panel(work: &Work, width: u16, icons: IconSet, theme: &Theme) -> Panel {
    let (approve, review, escalated) = work.needs_you_counts();
    let total = approve + review + escalated;
    let headline = Line::from(vec![
        Span::styled(format!("{total} "), theme.text.strong),
        Span::styled(
            format!("{} {} on a human", task_word(total), wait_word(total)),
            theme.semantic.human,
        ),
    ]);
    let inner = width.saturating_sub(CARD_COLUMNS);
    let counts = [approve, review, escalated];
    // The counts sit in a column when the column holds one. In a narrow
    // terminal they sit right after their reason.
    let column = usize::from(inner) >= 2 + LABEL_WIDTH + count_width(counts);
    let mut lines = vec![headline.clone()];
    let mut hidden = 0;
    for (reason, count) in [
        (Reason::Approve, approve),
        (Reason::Review, review),
        (Reason::Escalated, escalated),
    ] {
        let line = reason_line(reason, count, column, icons, theme);
        // The reason and its count share a row: when they do not fit, the
        // reason is left out rather than parted from its count.
        if rows_of(std::slice::from_ref(&line), inner) == 1 {
            lines.push(line);
        } else {
            hidden += 1;
        }
    }
    if hidden > 0 {
        lines.push(Line::from(Span::styled(
            format!("+{hidden} more reasons"),
            theme.text.dim,
        )));
    }
    // The dense form: the three reasons on one row.
    let mut spans = Vec::new();
    for (index, (reason, count)) in [
        (Reason::Approve, approve),
        (Reason::Review, review),
        (Reason::Escalated, escalated),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        // The reason and its count are painted apart, as the wide form
        // paints them: the count is a count, and every other count on the
        // screen is `text.strong`.
        spans.push(Span::styled(
            format!("{} {}", reason_icon(reason).glyph(icons), reason.label()),
            reason_style(theme, reason),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(count.to_string(), theme.text.strong));
    }
    Panel {
        title: "needs you",
        icon: Icon::TabNeeds,
        lines,
        compact: Some(vec![headline, Line::from(spans)]),
        // The three reasons are read together: the dense form is used only
        // when it keeps every count with its reason.
        compact_whole: true,
        optional: 0,
        atomic: true,
        more: "reasons",
        marker: None,
    }
}

/// One reason and its count, led by the reason's icon.
fn reason_line(
    reason: Reason,
    count: usize,
    column: bool,
    icons: IconSet,
    theme: &Theme,
) -> Line<'static> {
    let icon = reason_icon(reason).glyph(icons);
    let label = if column {
        format!("{icon} {:<LABEL_WIDTH$}", reason.label())
    } else {
        format!("{icon} {} ", reason.label())
    };
    Line::from(vec![
        Span::styled(label, reason_style(theme, reason)),
        Span::styled(count.to_string(), theme.text.strong),
    ])
}

/// The columns the widest of the counts takes.
fn count_width(counts: impl IntoIterator<Item = usize>) -> usize {
    counts
        .into_iter()
        .map(|count| count.to_string().chars().count())
        .max()
        .unwrap_or(1)
}

/// How many tasks an agent could claim at `now`.
fn claimable_panel(work: &Work, icons: IconSet, theme: &Theme, now: &str) -> Panel {
    let count = work.claimable_at(now).len();
    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", Icon::RoleRunner.glyph(icons)),
                theme.semantic.agent,
            ),
            Span::styled(format!("{count} "), theme.text.strong),
            Span::styled(
                format!("{} an agent could claim right now", task_word(count)),
                theme.semantic.agent,
            ),
        ]),
        Line::from(Span::styled(
            "  approved, free, dependencies done",
            theme.text.dim,
        )),
    ];
    Panel {
        title: "claimable",
        icon: Icon::StatusApproved,
        lines,
        compact: None,
        compact_whole: false,
        optional: 1,
        atomic: false,
        more: "lines",
        marker: None,
    }
}

/// The last few events of the journal, newest last.
///
/// The block is the record's own: it takes the last events in the file's
/// order, and never adds, sorts or invents one. An optional journal that is
/// not there says so in one line, and says nothing else.
fn activity_panel(work: &Work, width: u16, icons: IconSet, theme: &Theme) -> Panel {
    // A journal that is there but cannot be read is named. It is not the
    // same state as a project that has no journal, and it must not read as
    // one: the errors panel names the file, and this block names the state.
    if work.activity_unreadable() {
        return Panel {
            title: "activity",
            icon: Icon::Clock,
            lines: vec![Line::from(Span::styled(
                JOURNAL_UNREADABLE,
                theme.text.normal,
            ))],
            compact: None,
            compact_whole: false,
            optional: 0,
            atomic: true,
            more: "events",
            marker: Some(Marker {
                text: JOURNAL_UNREADABLE.to_string(),
                style: theme.text.normal,
            }),
        };
    }
    let activity = work.activity.as_ref();
    let events = activity
        .map(|activity| activity.events.as_slice())
        .unwrap_or(&[]);
    let unreadable = activity.map(|activity| activity.unreadable).unwrap_or(0);
    if events.is_empty() && unreadable == 0 {
        return Panel {
            title: "activity",
            icon: Icon::Clock,
            lines: vec![Line::from(Span::styled("no activity yet", theme.text.dim))],
            compact: None,
            compact_whole: false,
            optional: 0,
            atomic: true,
            more: "events",
            marker: Some(Marker {
                text: "no activity yet".to_string(),
                style: theme.text.dim,
            }),
        };
    }
    let shown = events.len().min(ACTIVITY_SHOWN);
    let hidden = events.len() - shown;
    let mut lines = Vec::new();
    if hidden > 0 {
        lines.push(Line::from(Span::styled(
            format!("+{hidden} earlier {}", event_word(hidden)),
            theme.text.dim,
        )));
    }
    // The date is drawn once a day, as a divider above the day's first event.
    let mut day = None;
    for event in &events[events.len() - shown..] {
        let (date, _) = day_and_time(&event.at);
        if day != Some(date) {
            lines.push(Line::from(Span::styled(
                format!("── {date} ──"),
                theme.text.dim,
            )));
            day = Some(date);
        }
        lines.push(event_line(
            event,
            icons,
            width.saturating_sub(CARD_COLUMNS),
            theme,
        ));
    }
    if unreadable > 0 {
        lines.push(Line::from(Span::styled(
            format!("{unreadable} {} could not be read", event_word(unreadable)),
            theme.text.dim,
        )));
    }
    let count = events.len();
    Panel {
        title: "activity",
        icon: Icon::Clock,
        lines,
        compact: None,
        compact_whole: false,
        optional: 0,
        // The block shows its last few events whole or not at all: a cut
        // would keep the oldest and hide the newest, which is not the tail.
        atomic: true,
        more: "events",
        marker: Some(Marker {
            text: if events.is_empty() {
                format!("{unreadable} {} could not be read", event_word(unreadable))
            } else {
                format!("activity not shown: {count} {}", event_word(count))
            },
            style: theme.text.dim,
        }),
    }
}

/// One event: its time, the icon of its kind, the task, then the kind's word
/// and the note when there is one. The event is one row: a line that does not
/// fit is cut, never wrapped into the rows of the event below it.
fn event_line(event: &ActivityEvent, icons: IconSet, width: u16, theme: &Theme) -> Line<'static> {
    let (_, time) = day_and_time(&event.at);
    let mut spans = vec![
        Span::styled(format!("{time:<5} "), theme.text.dim),
        Span::styled(
            format!("{} ", event_icon(event.kind).glyph(icons)),
            theme.text.strong,
        ),
        Span::styled(event.task.clone(), theme.text.strong),
        Span::styled(format!(" {}", event.kind.as_str()), theme.semantic.info),
    ];
    if let Some(note) = event.note.as_deref().filter(|note| !note.is_empty()) {
        spans.push(Span::styled(format!(" {note}"), theme.text.normal));
    }
    fit_line(spans, width)
}

/// The day and the time of a journal stamp: `2026-11-16T09:30Z` is the day
/// `2026-11-16` at `09:30`. A stamp with no time is a day with no time.
fn day_and_time(at: &str) -> (&str, &str) {
    match at.split_once('T') {
        Some((day, rest)) => (day, rest.get(..5).unwrap_or(rest)),
        None => (at, ""),
    }
}

/// The icon of an event's kind.
fn event_icon(kind: EventKind) -> Icon {
    match kind {
        EventKind::Created => Icon::StatusDraft,
        EventKind::Groomed => Icon::StatusGroomed,
        EventKind::Approved => Icon::StatusApproved,
        EventKind::Claimed => Icon::RoleRunner,
        EventKind::Released => Icon::StatusApproved,
        EventKind::Started => Icon::StatusExecuting,
        EventKind::RoundFiled => Icon::StatusReview,
        EventKind::ReviewRequested => Icon::StatusReview,
        EventKind::FindingRaised => Icon::Finding,
        EventKind::FindingResolved => Icon::Finding,
        EventKind::ChangesRequested => Icon::RoleReviewer,
        EventKind::Remediated => Icon::StatusRemediating,
        EventKind::Escalated => Icon::StatusNeedsHuman,
        EventKind::Completed => Icon::StatusDone,
        EventKind::Resumed => Icon::StatusExecuting,
        EventKind::Reopened => Icon::StatusExecuting,
        EventKind::Blocked => Icon::Blocked,
        EventKind::Unblocked => Icon::Next,
        EventKind::Deferred => Icon::Clock,
        EventKind::Undeferred => Icon::Next,
        EventKind::Cancelled => Icon::Stopped,
        EventKind::Commented => Icon::RoleHuman,
    }
}

/// The spans, cut to one row of `width` columns.
///
/// The styles are kept up to the cut; the cut is on a grapheme boundary, with
/// an ellipsis where the text continues.
fn fit_line(spans: Vec<Span<'static>>, width: u16) -> Line<'static> {
    let width = usize::from(width);
    let total: usize = spans
        .iter()
        .map(|span| text_width(span.content.as_ref()))
        .sum();
    if total <= width {
        return Line::from(spans);
    }
    if width == 0 {
        return Line::from(Span::raw(""));
    }
    let mut out = Vec::new();
    let mut used = 0usize;
    for span in spans {
        if used >= width {
            break;
        }
        let style = span.style;
        let text = span.content.into_owned();
        let span_width = text_width(&text);
        if used + span_width <= width {
            used += span_width;
            out.push(Span::styled(text, style));
        } else {
            let head = head_to(&text, width - used - 1);
            out.push(Span::styled(format!("{head}…"), style));
            break;
        }
    }
    Line::from(out)
}

/// The word for a number of events.
fn event_word(count: usize) -> &'static str {
    if count == 1 { "event" } else { "events" }
}

/// What happens next: the first claimable task at `now`.
fn next_panel(work: &Work, icons: IconSet, theme: &Theme, now: &str) -> Panel {
    let lines = match work.next_at(now) {
        Some(task) => vec![
            Line::from(vec![
                Span::styled(format!("{} ", Icon::Link.glyph(icons)), theme.text.strong),
                Span::styled(format!("{} ", task.id), theme.text.strong),
                Span::styled(task.title.clone(), theme.text.normal),
            ]),
            Line::from(Span::styled("  the first task of the path", theme.text.dim)),
        ],
        None => vec![
            Line::from(Span::styled(
                "nothing is claimable right now",
                theme.text.normal,
            )),
            Line::from(Span::styled(
                "  no approved task has its dependencies done",
                theme.text.dim,
            )),
        ],
    };
    Panel {
        title: "next",
        icon: Icon::TabPath,
        lines,
        compact: None,
        compact_whole: false,
        optional: 1,
        atomic: false,
        more: "lines",
        marker: None,
    }
}

/// The files that could not be read.
fn errors_panel(work: &Work, theme: &Theme) -> Panel {
    let lines = work
        .errors
        .iter()
        .map(|error| {
            Line::from(vec![
                Span::styled(format!("{}: ", error.file), theme.text.strong),
                // The message of a file that could not be read is a
                // failure, not secondary text.
                Span::styled(error.error.clone(), theme.semantic.error),
            ])
        })
        .collect();
    Panel {
        title: "unreadable",
        icon: Icon::StatusNeedsHuman,
        lines,
        compact: None,
        compact_whole: false,
        optional: 0,
        atomic: false,
        more: "files",
        marker: Some(Marker {
            text: hidden_files(work),
            // The block is left out, but what it holds is still a failure.
            style: theme.semantic.error,
        }),
    }
}

/// The rows the lines take when they are wrapped into `width` columns.
///
/// The height is ratatui's own: the paragraph that will be drawn is the one
/// that is measured, so a padded line is counted the way it is drawn.
fn rows_of(lines: &[Line<'static>], width: u16) -> u16 {
    if width == 0 {
        return 0;
    }
    Paragraph::new(lines.to_vec())
        .wrap(Wrap { trim: true })
        .line_count(width) as u16
}

/// The word for a number of tasks.
fn task_word(count: usize) -> &'static str {
    if count == 1 { "task" } else { "tasks" }
}

/// The verb for a number of tasks.
fn wait_word(count: usize) -> &'static str {
    if count == 1 { "waits" } else { "wait" }
}

/// The verb for a number of tasks that need a human.
fn need_word(count: usize) -> &'static str {
    if count == 1 { "needs" } else { "need" }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;
    use coding_crew_theme::Preset;

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    /// The theme the screen is drawn with.
    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// The tests' clock: every fixture claim has expired by it.
    const NOW: &str = "2026-11-19T08:00:00Z";

    fn now() -> &'static str {
        NOW
    }

    /// Draw the Overview and return the rows it shows.
    fn draw(work: &Work, width: u16, height: u16) -> Vec<String> {
        draw_at(work, width, height, now())
    }

    /// Draw the Overview at a given instant.
    fn draw_at(work: &Work, width: u16, height: u16, now: &str) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), work, &theme(), IconSet::Nerd, now))
            .expect("drawing the overview");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect()
    }

    /// Draw the Overview and return every cell, with its colour.
    fn draw_cells(work: &Work, width: u16, height: u16) -> Vec<Vec<(char, Color)>> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), work, &theme(), IconSet::Nerd, now()))
            .expect("drawing the overview");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| {
                        let cell = buffer.cell((x, y)).expect("a cell");
                        (cell.symbol().chars().next().unwrap_or(' '), cell.fg)
                    })
                    .collect()
            })
            .collect()
    }

    /// The colour of the first cell of `needle`, on the row that holds it.
    fn colour_of(cells: &[Vec<(char, Color)>], needle: &str) -> Option<Color> {
        for row in cells {
            let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
            if let Some(start) = text.find(needle) {
                return row.get(start).map(|(_, colour)| *colour);
            }
        }
        None
    }

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("raul-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    /// A work folder that is not there, for the given reason.
    fn missing_work(reason: Missing) -> Work {
        Work {
            root: std::path::PathBuf::from("/nowhere/project/coding-crew/work"),
            tasks: Vec::new(),
            items: Vec::new(),
            errors: Vec::new(),
            activity: None,
            missing: Some(reason),
        }
    }

    #[test]
    fn without_a_project_the_screen_says_none_was_found() {
        let text = draw(&missing_work(Missing::NoFolder), 100, 10).join("\n");
        assert!(text.contains("no coding-crew folder found"), "{text}");
        assert!(
            text.contains("looked in /nowhere/project/coding-crew/work"),
            "{text}"
        );
        assert!(!text.contains("no work folder"), "{text}");
    }

    #[test]
    fn a_project_without_work_says_so_and_names_the_path() {
        let text = draw(&missing_work(Missing::NoWork), 100, 10).join("\n");
        assert!(text.contains("this project has no work folder"), "{text}");
        assert!(
            text.contains("looked in /nowhere/project/coding-crew/work"),
            "{text}"
        );
        assert!(!text.contains("no coding-crew folder found"), "{text}");
    }

    #[test]
    fn the_overview_shows_the_numbers() {
        let work = mock_work();
        let text = draw(&work, 100, 40).join("\n");
        let counts = work.needs_you_counts();
        let waiting = counts.0 + counts.1 + counts.2;
        assert!(
            text.contains(&format!("{waiting} tasks wait on a human")),
            "{text}"
        );
        let claimable = work.claimable_at(NOW).len();
        let word = if claimable == 1 { "task" } else { "tasks" };
        assert!(
            text.contains(&format!(
                "{claimable} {word} an agent could claim right now"
            )),
            "{text}"
        );
        assert!(text.contains("approve"), "{text}");
        assert!(text.contains("escalated"), "{text}");
    }

    #[test]
    fn the_overview_shows_the_counts_and_the_next_task() {
        let work = mock_work();
        let text = draw(&work, 100, 40).join("\n");
        assert!(
            text.contains(&format!("{} tasks by status", work.tasks.len())),
            "{text}"
        );
        assert!(text.contains("draft"), "{text}");
        assert!(text.contains("needs-human"), "{text}");
        assert!(text.contains("idea"), "{text}");
        let next = work.next_at(NOW).expect("a claimable task");
        assert!(text.contains(&next.id), "{text}");
    }

    /// A work folder whose journal holds the events given, newest last as
    /// the record keeps them.
    fn with_activity(events: Vec<coding_crew_core::ActivityEvent>) -> Work {
        let mut work = mock_work();
        work.activity = Some(coding_crew_core::Activity {
            events,
            ..Default::default()
        });
        work
    }

    /// One event, with a note that names it.
    fn journal_event(n: usize) -> coding_crew_core::ActivityEvent {
        coding_crew_core::ActivityEvent {
            at: format!("2026-11-{:02}T{:02}:00Z", 1 + n % 10, 9 + n % 8),
            kind: coding_crew_core::EventKind::Commented,
            task: "TSK01".to_string(),
            by: "human".to_string(),
            note: Some(format!("note {n}")),
        }
    }

    #[test]
    fn the_overview_shows_the_last_events_and_how_many_it_left_out() {
        let work = with_activity((1..=8).map(journal_event).collect());
        let text = draw(&work, 100, 60).join("\n");
        assert!(text.contains("activity"), "{text}");
        // The last five are shown, oldest first, so the newest is last.
        assert!(text.contains("+3 earlier events"), "{text}");
        let positions: Vec<usize> = (4..=8)
            .map(|n| {
                text.find(&format!("note {n}"))
                    .unwrap_or_else(|| panic!("note {n} is not drawn: {text}"))
            })
            .collect();
        assert!(
            positions.windows(2).all(|pair| pair[0] < pair[1]),
            "the events are not in the record's order: {text}"
        );
        assert!(!text.contains("note 3"), "an older event is drawn: {text}");
    }

    #[test]
    fn the_activity_block_keeps_the_record_order_and_shows_no_invented_event() {
        let mut events: Vec<coding_crew_core::ActivityEvent> = (1..=8).map(journal_event).collect();
        // The record is not sorted here: the block must not sort it either.
        events.swap(4, 5);
        let work = with_activity(events);
        let text = draw(&work, 100, 60).join("\n");
        let first = text.find("note 6").expect("note 6");
        let second = text.find("note 5").expect("note 5");
        assert!(first < second, "the block reordered the record: {text}");
        // Every drawn event is one the file holds.
        for n in 4..=8 {
            assert!(text.contains(&format!("note {n}")), "note {n}: {text}");
        }
    }

    #[test]
    fn without_a_journal_the_activity_block_says_no_activity_yet() {
        for activity in [None, Some(coding_crew_core::Activity::default())] {
            let mut work = mock_work();
            work.activity = activity;
            let text = draw(&work, 100, 40).join("\n");
            assert!(text.contains("no activity yet"), "{text}");
            assert!(!text.contains("earlier"), "{text}");
            assert!(!text.contains("not shown: "), "{text}");
            assert!(!text.contains("created "), "an event was invented: {text}");
        }
    }

    #[test]
    fn a_bad_event_is_counted_and_the_readable_history_stays() {
        let text = r#"{
          "events": [
            {"at": "2026-11-01T09:00Z", "kind": "created", "task": "TSK01", "by": "human", "note": "kept"},
            {"at": "2026-11-02T09:00Z", "kind": "not_a_kind", "task": "TSK01", "by": "human"},
            {"at": "2026-11-03T09:00Z", "kind": "claimed", "by": "runner"},
            {"at": "2026-11-04T09:00Z", "kind": "completed", "task": "TSK01", "by": "reviewer", "note": "also kept"}
          ]
        }"#;
        let mut work = mock_work();
        work.activity = Some(coding_crew_core::Activity::from_json(text).expect("a journal"));
        let text = draw(&work, 100, 60).join("\n");
        assert!(text.contains("kept"), "the readable event is gone: {text}");
        assert!(text.contains("also kept"), "{text}");
        assert!(text.contains("2 events could not be read"), "{text}");
        assert!(!text.contains("no activity yet"), "{text}");
    }

    #[test]
    fn a_journal_that_does_not_parse_is_named_not_called_empty() {
        let dir = scratch_dir("journal-unreadable");
        let work_dir = dir.join("work");
        fs::create_dir_all(&work_dir).expect("a work folder");
        let mut task = coding_crew_core::Task::new("TSK01", "a task");
        task.status = coding_crew_core::Status::Approved;
        fs::write(
            work_dir.join("TSK01-a-task.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        fs::write(dir.join("activity.json"), "{ not json").expect("a broken journal");
        let work = Work::load(&work_dir);
        assert!(work.activity_unreadable());
        let text = draw(&work, 100, 40).join("\n");
        assert!(text.contains(JOURNAL_UNREADABLE), "{text}");
        assert!(text.contains("activity.json"), "{text}");
        assert!(!text.contains("no activity yet"), "{text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_activity_block_reads_the_record_on_every_draw() {
        // The block keeps no state: the same frame drawn twice shows what
        // the record holds at that moment.
        let mut work = with_activity(vec![journal_event(1)]);
        let first = draw(&work, 100, 60).join("\n");
        assert!(first.contains("note 1"), "{first}");
        assert!(!first.contains("note 2"), "{first}");
        work.activity
            .as_mut()
            .expect("a journal")
            .events
            .push(journal_event(2));
        let second = draw(&work, 100, 60).join("\n");
        assert!(
            second.contains("note 2"),
            "the block kept a state: {second}"
        );
    }

    #[test]
    fn a_dropped_activity_block_says_how_many_events_it_left_out() {
        let work = with_activity((1..=8).map(journal_event).collect());
        let panel = activity_panel(&work, 50, IconSet::Nerd, &theme());
        let marker = panel.marker.as_ref().expect("a marker");
        assert!(
            marker.text.contains("activity not shown"),
            "{}",
            marker.text
        );
        assert!(marker.text.contains('8'), "{}", marker.text);
        // A project without a journal says the same one thing when the block
        // does not fit.
        let mut work = mock_work();
        work.activity = None;
        let panel = activity_panel(&work, 50, IconSet::Nerd, &theme());
        let marker = panel.marker.as_ref().expect("a marker");
        assert_eq!(marker.text, "no activity yet");
    }

    #[test]
    fn the_overview_draws_from_one_instant() {
        // A lease expiring at a boundary: before it the task is held, at it
        // the task is claimable. The claimable count and the next task
        // answer from the instant the frame is given, so one screen cannot
        // draw a count of zero beside a next task the count says is free,
        // or a count of one beside "nothing is claimable".
        let mut task = coding_crew_core::Task::new("TSK01", "the claimed task");
        task.status = coding_crew_core::Status::Approved;
        task.claim = Some(coding_crew_core::Claim {
            by: "runner-01".into(),
            session: Some("session-runner-01".into()),
            at: "2029-12-31T23:00:00Z".into(),
            lease_expires_at: "2030-01-01T00:00:00Z".into(),
        });
        let work = Work {
            root: std::path::PathBuf::new(),
            tasks: vec![task],
            items: Vec::new(),
            errors: Vec::new(),
            activity: None,
            missing: None,
        };
        let held = "2029-12-31T23:59:59Z";
        let free = "2030-01-01T00:00:00Z";
        let before = draw_at(&work, 100, 40, held).join("\n");
        assert!(
            before.contains("0 tasks an agent could claim right now"),
            "the live claim is not held: {before}"
        );
        assert!(
            before.contains("nothing is claimable right now"),
            "a held task is still next: {before}"
        );
        assert!(!before.contains("TSK01"), "a held task is drawn: {before}");
        // The expired side is the one a fresh clock in the tab fails: the
        // real clock is years before the fixture's lease.
        let after = draw_at(&work, 100, 40, free).join("\n");
        assert!(
            after.contains("1 task an agent could claim right now"),
            "the expired claim still holds the task: {after}"
        );
        assert!(
            after.contains("TSK01"),
            "the count says one and the next panel names none: {after}"
        );
        assert!(
            !after.contains("nothing is claimable right now"),
            "the count says one beside no next task: {after}"
        );
    }

    #[test]
    fn a_short_terminal_does_not_cut_a_count() {
        let work = mock_work();
        let text = draw(&work, 60, 16).join("\n");
        // What is shown is shown whole, and what is hidden is announced: the
        // stacked column has no room for the counts block, so it is named.
        let counts = work.needs_you_counts();
        let waiting = counts.0 + counts.1 + counts.2;
        assert!(!text.contains("draft"), "{text}");
        assert!(text.contains("counts not shown"), "{text}");
        assert!(
            text.contains(&format!("{waiting} tasks wait on a human")),
            "{text}"
        );
    }

    #[test]
    fn a_narrow_terminal_shows_the_key_numbers() {
        let work = mock_work();
        let text = draw(&work, 40, 12).join("\n");
        let counts = work.needs_you_counts();
        let waiting = counts.0 + counts.1 + counts.2;
        assert!(
            text.contains(&format!("{waiting} tasks wait on a human")),
            "{text}"
        );
        assert!(
            text.contains(&format!("{} claimable", work.claimable_at(NOW).len())),
            "{text}"
        );
    }

    #[test]
    fn an_unreadable_file_shows_its_error() {
        let dir = scratch_dir("unreadable");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        assert!(work.is_empty());
        let text = draw(&work, 80, 20).join("\n");
        assert!(text.contains("no tasks yet"), "{text}");
        assert!(text.contains("unreadable"), "{text}");
        assert!(text.contains("TSK98-broken.json"), "{text}");
    }

    #[test]
    fn a_long_text_is_wrapped_and_not_cut() {
        let dir = scratch_dir("wrapped");
        let mut task = coding_crew_core::Task::new("TSK01", "a long title");
        task.title = "a long title that has to be wrapped over more than one line".to_string();
        task.status = coding_crew_core::Status::Approved;
        fs::write(
            dir.join("TSK01-a-long-title.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        let work = Work::load(&dir);
        let text = draw(&work, 40, 20).join("\n");
        assert!(text.contains("more than one line"), "{text}");
    }

    #[test]
    fn a_word_longer_than_the_block_takes_the_rows_it_needs() {
        let dir = scratch_dir("long-word");
        let mut task = coding_crew_core::Task::new("TSK01", "a long title");
        task.title = "X".repeat(90);
        task.status = coding_crew_core::Status::Approved;
        fs::write(
            dir.join("TSK01-a-long-title.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        let work = Work::load(&dir);
        let text = draw(&work, 40, 24).join("\n");
        // The word is broken over rows; every letter of it is shown.
        assert_eq!(text.chars().filter(|ch| *ch == 'X').count(), 90, "{text}");
    }

    #[test]
    fn an_unreadable_file_outranks_the_smaller_tasks() {
        let dir = scratch_dir("unreadable-rank");
        let mut task = coding_crew_core::Task::new("TSK01", "a good task");
        task.status = coding_crew_core::Status::Approved;
        fs::write(
            dir.join("TSK01-a-good-task.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        let text = draw(&work, 100, 12).join("\n");
        // The file that could not be read is shown; the block that is left
        // out says so.
        assert!(text.contains("unreadable"), "{text}");
        assert!(text.contains("TSK98-broken.json"), "{text}");
        assert!(text.contains("not shown"), "{text}");
    }

    #[test]
    fn a_narrow_summary_is_never_cut() {
        let work = mock_work();
        let text = draw(&work, 38, 12).join("\n");
        assert!(
            text.contains(&format!("{} claimable", work.claimable_at(NOW).len())),
            "{text}"
        );
        // The next task is either shown whole or not at all.
        assert!(!text.contains("next TS"), "{text}");
    }

    #[test]
    fn a_count_stays_with_its_reason() {
        let work = mock_work();
        let escalated = work.needs_you_counts().2.to_string();
        for (width, height) in [(64, 12), (66, 12), (100, 12), (100, 8)] {
            let rows = draw(&work, width, height);
            let row = rows
                .iter()
                .find(|row| row.contains("escalated"))
                .unwrap_or_else(|| panic!("no escalated row at {width}x{height}: {rows:?}"));
            // The count of the escalated tasks is on the row of its reason.
            let text = row.replace('│', " ");
            assert!(
                text.trim().ends_with(escalated.as_str()),
                "{width}x{height}: {row}"
            );
        }
    }

    #[test]
    fn a_short_wide_terminal_shows_the_key_numbers() {
        let work = mock_work();
        let text = draw(&work, 100, 8).join("\n");
        let counts = work.needs_you_counts();
        let waiting = counts.0 + counts.1 + counts.2;
        // The blocks do not fit, so the numbers go on top of them.
        assert!(
            text.contains(&format!("{waiting} tasks wait on a human")),
            "{text}"
        );
        assert!(
            text.contains(&format!("{} claimable", work.claimable_at(NOW).len())),
            "{text}"
        );
    }

    #[test]
    fn a_dropped_block_leaves_a_whole_marker() {
        let work = mock_work();
        let text = draw(&work, 20, 6).join("\n");
        // The marker is cut to the width of the column, not to the width of
        // the block inside it.
        assert!(text.contains("needs you not shown"), "{text}");
    }

    #[test]
    fn the_dense_counts_keep_a_count_with_its_status() {
        let work = mock_work();
        let counts = work.status_counts();
        let dense = dense_counts(&counts, work.tasks.len(), 38, IconSet::Nerd, &theme())
            .expect("a dense form");
        for line in &dense {
            let text = text_of(std::slice::from_ref(line));
            assert!(text.chars().count() <= 38, "{text}");
        }
        for (status, count) in &counts {
            // The status and its count are on the same row.
            let found = dense.iter().any(|line| {
                text_of(std::slice::from_ref(line))
                    .contains(&format!("{} {count}", status.as_str()))
            });
            assert!(found, "{status:?} is not with its count");
        }
    }

    #[test]
    fn a_block_is_dropped_before_another_one_is_made_dense() {
        let work = mock_work();
        let rows = draw(&work, 80, 17);
        // The counts keep their full form: the labels are padded, so the
        // counts stand in a column. The column is counted in characters, one
        // per cell: the icons and the bars are several bytes each.
        let column = |label: &str| -> usize {
            let row = rows
                .iter()
                .find(|row| row.contains(label))
                .unwrap_or_else(|| panic!("no row with {label}: {rows:?}"));
            let start = row.find(label).expect("the label");
            let rest = &row[start + label.len()..];
            row[..start].chars().count()
                + label.chars().count()
                + rest
                    .chars()
                    .position(|ch| ch.is_ascii_digit())
                    .expect("a count")
        };
        assert_eq!(column("draft"), column("code-review"));
        // The block after the counts is the one that is left out.
        assert!(
            rows.iter()
                .any(|row| row.contains("smaller tasks not shown")),
            "{rows:?}"
        );
    }

    #[test]
    fn a_three_digit_count_stands_apart_from_its_label() {
        let dir = scratch_dir("many-tasks");
        for index in 1..=120 {
            let mut task = coding_crew_core::Task::new(format!("TSK{index:03}"), "a task");
            task.status = coding_crew_core::Status::CodeReview;
            fs::write(
                dir.join(format!("TSK{index:03}-a-task.json")),
                serde_json::to_string_pretty(&task).expect("writing a task"),
            )
            .expect("writing a task");
        }
        let work = Work::load(&dir);
        let text = draw(&work, 100, 40).join("\n");
        assert!(text.contains("code-review 120"), "{text}");
    }

    #[test]
    fn a_short_summary_keeps_the_qualifier_of_its_number() {
        let work = mock_work();
        for width in 8..=60 {
            let Some(line) = summary_line(&work, width, &theme(), now()) else {
                continue;
            };
            let text = text_of(std::slice::from_ref(&line));
            // The number is the count of the tasks that wait on a human:
            // it says so, so it is never read as the count of the tasks.
            assert!(
                text.contains("wait") || text.contains("human"),
                "{width}: {text}"
            );
        }
    }

    #[test]
    fn a_narrow_terminal_does_not_part_a_reason_from_its_count() {
        let work = mock_work();
        let counts = work.needs_you_counts();
        let expected = [
            ("approve", counts.0),
            ("review", counts.1),
            ("escalated", counts.2),
        ];
        for width in 1..=40 {
            let panel = needs_you_panel(&work, width, IconSet::Nerd, &theme());
            for line in &panel.lines {
                let text = text_of(std::slice::from_ref(line));
                for (reason, count) in expected {
                    if text.contains(reason) {
                        assert!(text.contains(&count.to_string()), "{width}: {text}");
                    }
                }
            }
        }
        // In a terminal too narrow for a reason and its count, the reason
        // is left out rather than parted from its count.
        let text = draw(&work, 11, 21).join("\n");
        assert!(!text.contains("approve"), "{text}");
        assert!(text.contains("+3 more"), "{text}");
    }

    #[test]
    fn the_dense_reason_line_paints_its_count_as_a_count() {
        // The wide form splits the reason from its count; the dense form
        // paints them on one row, and the count is still a count: it takes
        // `text.strong`, as every other count on the screen does.
        let work = mock_work();
        let theme = theme();
        let panel = needs_you_panel(&work, 100, IconSet::Nerd, &theme);
        let compact = panel.compact.as_deref().expect("a dense form");
        let line = compact.last().expect("the dense reasons");
        let counts: Vec<&str> = line
            .spans
            .iter()
            .filter(|span| span.style == theme.text.strong)
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(counts.len(), 3, "three reasons, three counts: {line:?}");
        for count in counts {
            assert!(
                count.chars().all(|c| c.is_ascii_digit()),
                "{count} is not a count"
            );
        }
    }

    #[test]
    fn an_unreadable_marker_carries_the_error() {
        let dir = scratch_dir("unreadable-marker");
        let mut task = coding_crew_core::Task::new("TSK01", "a good task");
        task.status = coding_crew_core::Status::Approved;
        fs::write(
            dir.join("TSK01-a-good-task.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        // The marker names the count as well, and at 100 columns it gets half
        // the pane: a wider pane keeps the error in view.
        let text = draw(&work, 140, 12).join("\n");
        let marker = text
            .lines()
            .find(|line| line.contains("unreadable not shown"))
            .unwrap_or_else(|| panic!("no marker: {text}"));
        // The name of the file and the error, not only the name.
        assert!(marker.contains("TSK98-broken.json"), "{marker}");
        assert!(marker.contains("key must"), "{marker}");
    }

    #[test]
    fn the_unreadable_marker_counts_its_files() {
        // The marker names the files it holds and says how many there are,
        // as every block that is left out does.
        let dir = scratch_dir("unreadable-count");
        fs::write(dir.join("TSK97-broken.json"), "{ not json").expect("writing a broken file");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        assert_eq!(work.errors.len(), 2, "{:?}", work.errors);
        let marker = hidden_files(&work);
        assert!(
            marker.starts_with("unreadable not shown: 2 files ("),
            "{marker}"
        );
        assert!(marker.contains("TSK97-broken.json"), "{marker}");
        assert!(marker.contains("TSK98-broken.json"), "{marker}");

        let single = scratch_dir("unreadable-count-one");
        fs::write(single.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&single);
        let marker = hidden_files(&work);
        assert!(
            marker.starts_with("unreadable not shown: 1 file ("),
            "{marker}"
        );
    }

    #[test]
    fn an_unreadable_file_is_painted_as_a_failure() {
        let dir = scratch_dir("unreadable-paint");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        let message = work.errors[0].error.clone();
        let error = theme().semantic.error.fg.expect("the error colour");
        let cells = draw_cells(&work, 100, 20);
        // The message of a file that could not be read is a failure, not
        // secondary text.
        assert_eq!(
            colour_of(&cells, &message),
            Some(error),
            "the message of the unreadable file is not painted as a failure"
        );
    }

    #[test]
    fn the_marker_of_an_unreadable_file_is_painted_as_a_failure() {
        let dir = scratch_dir("unreadable-marker-paint");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        let error = theme().semantic.error.fg.expect("the error colour");
        // The block does not fit: the file is named on one row, and the row
        // is painted as the failure it is.
        let cells = draw_cells(&work, 80, 2);
        assert_eq!(
            colour_of(&cells, "TSK98-broken.json"),
            Some(error),
            "the marker of the unreadable file is not painted as a failure"
        );
    }

    /// The text of some lines.
    fn text_of(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[test]
    fn an_unreadable_file_is_named_when_the_block_does_not_fit() {
        let dir = scratch_dir("unreadable-short");
        fs::write(dir.join("TSK98-broken.json"), "{ not json").expect("writing a broken file");
        let work = Work::load(&dir);
        let text = draw(&work, 80, 2).join("\n");
        // The block does not fit, but the file is named all the same.
        assert!(text.contains("TSK98-broken.json"), "{text}");
    }

    #[test]
    fn a_count_stays_with_its_status_at_every_width() {
        let work = mock_work();
        let counts = work.status_counts();
        for width in 1..=60 {
            let panel = counts_panel(&work, width, IconSet::Nerd, &theme());
            if panel.lines.is_empty() {
                // The block is left out whole: the marker says so.
                continue;
            }
            for (status, count) in &counts {
                let label = status.as_str();
                let row = panel
                    .lines
                    .iter()
                    .find(|line| text_of(std::slice::from_ref(line)).contains(label))
                    .unwrap_or_else(|| panic!("no row for {label} at {width}"));
                let text = text_of(std::slice::from_ref(row));
                // The count is on the row of its status, and the row is one
                // row: a count is never on a row of its own.
                assert!(text.contains(&count.to_string()), "{width}: {text}");
                assert_eq!(
                    rows_of(
                        std::slice::from_ref(row),
                        width.saturating_sub(CARD_COLUMNS)
                    ),
                    1,
                    "{width}: {text}"
                );
            }
        }
    }

    #[test]
    fn a_count_stays_with_its_kind_at_every_width() {
        let work = mock_work();
        for width in 1..=60 {
            let panel = kinds_panel(&work, width, IconSet::Nerd, &theme());
            if panel.lines.is_empty() {
                // The block is left out whole: the marker says so.
                continue;
            }
            for (kind, count) in work.kind_counts() {
                let label = kind.as_str();
                let row = panel
                    .lines
                    .iter()
                    .find(|line| text_of(std::slice::from_ref(line)).contains(label))
                    .unwrap_or_else(|| panic!("no row for {label} at {width}"));
                let text = text_of(std::slice::from_ref(row));
                assert!(text.contains(&count.to_string()), "{width}: {text}");
                // The count is never on the label's last column.
                assert!(text.ends_with(&format!(" {count}")), "{width}: {text}");
                assert_eq!(
                    rows_of(
                        std::slice::from_ref(row),
                        width.saturating_sub(CARD_COLUMNS)
                    ),
                    1,
                    "{width}: {text}"
                );
            }
        }
    }

    #[test]
    fn the_kinds_counts_share_a_column_with_the_other_panels() {
        // At an ordinary width the counts sit where the other panels' do: the
        // label starts after its icon and a space, and is padded to the column.
        let panel = kinds_panel(&mock_work(), 40, IconSet::Nerd, &theme());
        assert!(panel.lines.len() > 1, "the block is left out");
        for line in &panel.lines[1..] {
            let text = text_of(std::slice::from_ref(line));
            assert_eq!(
                text.chars().position(|symbol| symbol.is_ascii_digit()),
                Some(2 + LABEL_WIDTH),
                "{text}"
            );
        }
    }

    #[test]
    fn the_cards_take_two_columns_from_eighty_columns() {
        let work = mock_work();
        let shares_a_row = |rows: &[String]| {
            rows.iter()
                .any(|row| row.contains("counts") && row.contains("needs you"))
        };
        assert!(shares_a_row(&draw(&work, TWO_COLUMNS, 40)));
        assert!(!shares_a_row(&draw(&work, TWO_COLUMNS - 1, 40)));
    }

    #[test]
    fn the_cards_pad_one_column_inside_their_borders_and_part_their_columns() {
        let rows = draw(&mock_work(), 100, 40);
        assert!(
            rows.iter().any(|row| row.contains("│ 30 tasks by status")),
            "{}",
            rows.join("\n")
        );
        assert!(
            rows.iter().any(|row| row.contains("╮ ╭")),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn the_status_bars_are_proportional_to_the_largest_count() {
        let work = mock_work();
        let panel = counts_panel(&work, 40, IconSet::Nerd, &theme());
        // Draft holds the largest count, six: its bar is full. Groomed holds
        // three, half of it, so its bar is half full.
        let bar = |label: &str| -> (usize, usize) {
            let row = panel
                .lines
                .iter()
                .map(|line| text_of(std::slice::from_ref(line)))
                .find(|text| text.contains(label))
                .unwrap_or_else(|| panic!("no row for {label}"));
            (row.matches('█').count(), row.matches('░').count())
        };
        let (draft, draft_empty) = bar("draft");
        assert_eq!(draft_empty, 0, "the largest count does not fill its bar");
        let (groomed, _) = bar("groomed");
        assert_eq!(groomed * 2, draft, "groomed is not half of draft");
    }

    /// The first of some lines that holds `needle`.
    fn line_with<'a>(lines: &'a [Line<'static>], needle: &str) -> &'a Line<'static> {
        lines
            .iter()
            .find(|line| text_of(std::slice::from_ref(line)).contains(needle))
            .unwrap_or_else(|| panic!("no line with {needle}"))
    }

    #[test]
    fn approve_has_the_same_icon_on_the_card_as_on_the_needs_tab() {
        let work = mock_work();
        let panel = needs_you_panel(&work, 100, IconSet::Nerd, &theme());
        let approve = line_with(&panel.lines, "approve");
        assert!(
            approve.spans[0]
                .content
                .starts_with(Icon::StatusGroomed.glyph(IconSet::Nerd)),
            "{approve:?}"
        );
    }

    #[test]
    fn the_needs_you_card_paints_each_reason_on_its_slot_and_each_count_strong() {
        let work = mock_work();
        let theme = theme();
        let panel = needs_you_panel(&work, 100, IconSet::Nerd, &theme);
        for (reason, label) in [
            (Reason::Approve, "approve"),
            (Reason::Review, "review"),
            (Reason::Escalated, "escalated"),
        ] {
            let line = line_with(&panel.lines, label);
            let [reason_span, count_span] = line.spans.as_slice() else {
                panic!("a reason and a count: {line:?}");
            };
            assert!(
                reason_span
                    .content
                    .starts_with(reason_icon(reason).glyph(IconSet::Nerd)),
                "{label}: {line:?}"
            );
            assert_eq!(reason_span.style, reason_style(&theme, reason), "{label}");
            assert_eq!(count_span.style, theme.text.strong, "{label}");
        }
    }

    #[test]
    fn the_claimable_card_paints_its_icon_on_the_agent_slot_and_its_count_strong() {
        let work = mock_work();
        let theme = theme();
        let panel = claimable_panel(&work, IconSet::Nerd, &theme, now());
        let line = &panel.lines[0];
        let [icon, count, _words] = line.spans.as_slice() else {
            panic!("an icon, a count and the words: {line:?}");
        };
        assert_eq!(icon.style, theme.semantic.agent);
        assert_eq!(count.style, theme.text.strong);
        assert_eq!(
            count.content.trim(),
            work.claimable_at(now()).len().to_string()
        );
    }

    #[test]
    fn the_smaller_tasks_card_paints_each_count_strong() {
        let work = mock_work();
        let theme = theme();
        let panel = kinds_panel(&work, 100, IconSet::Nerd, &theme);
        let kinds = work.kind_counts();
        // The first line is the heading; each kind has a row after it.
        let rows = &panel.lines[1..];
        assert_eq!(rows.len(), kinds.len(), "{rows:?}");
        for (line, (kind, count)) in rows.iter().zip(kinds) {
            let [icon, label, number] = line.spans.as_slice() else {
                panic!("an icon, a kind and a count: {line:?}");
            };
            assert_eq!(icon.style, theme.text.normal);
            assert_eq!(label.content.trim(), kind.as_str());
            assert_eq!(number.style, theme.text.strong);
            assert_eq!(number.content, count.to_string());
        }
    }

    #[test]
    fn the_next_card_paints_its_icon_and_its_id_strong() {
        let work = mock_work();
        let theme = theme();
        let next = work.next_at(now()).expect("a claimable task");
        let panel = next_panel(&work, IconSet::Nerd, &theme, now());
        let line = &panel.lines[0];
        let [icon, id, _title] = line.spans.as_slice() else {
            panic!("an icon, an id and a title: {line:?}");
        };
        assert!(
            icon.content.starts_with(Icon::Link.glyph(IconSet::Nerd)),
            "{line:?}"
        );
        assert_eq!(icon.style, theme.text.strong);
        assert_eq!(id.content.trim(), next.id);
        assert_eq!(id.style, theme.text.strong);
    }

    #[test]
    fn the_activity_block_draws_each_days_date_once_and_the_time_of_each_event() {
        let event = |at: &str, n: usize| coding_crew_core::ActivityEvent {
            at: at.to_string(),
            kind: EventKind::Commented,
            task: "TSK01".to_string(),
            by: "human".to_string(),
            note: Some(format!("note {n}")),
        };
        let work = with_activity(vec![
            event("2026-11-16T09:30Z", 1),
            event("2026-11-16T10:00Z", 2),
            event("2026-11-17T08:15Z", 3),
        ]);
        let text = draw(&work, 100, 60).join("\n");
        assert_eq!(text.matches("── 2026-11-16 ──").count(), 1, "{text}");
        assert_eq!(text.matches("── 2026-11-17 ──").count(), 1, "{text}");
        assert!(text.contains("09:30 "), "{text}");
        assert!(text.contains("08:15 "), "{text}");
        assert!(!text.contains("T09:30Z"), "a stamp is drawn whole: {text}");
    }

    #[test]
    fn a_block_that_cannot_hold_a_count_is_left_out_whole() {
        let work = mock_work();
        // The counts do not fit a card of eighteen columns, which holds
        // sixteen inside its border and padding: the block is left out, not
        // shown with a count on a row of its own.
        let text = draw(&work, 18, 60).join("\n");
        assert!(text.contains("counts not sh"), "{text}");
        assert!(!text.contains("code-review"), "{text}");
        // At nineteen columns they fit, and every count is with its status.
        let text = draw(&work, 19, 60).join("\n");
        let code_review = work
            .status_counts()
            .into_iter()
            .find(|(status, _)| *status == Status::CodeReview)
            .map(|(_, count)| count)
            .expect("a code-review count");
        assert!(
            text.contains(&format!("code-review {code_review}")),
            "{text}"
        );
        assert!(!text.contains("counts not shown"), "{text}");
    }

    #[test]
    fn the_short_summary_agrees_with_its_count() {
        let dir = scratch_dir("one-human");
        let mut task = coding_crew_core::Task::new("TSK01", "a task");
        task.status = coding_crew_core::Status::Groomed;
        fs::write(
            dir.join("TSK01-a-task.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        let work = Work::load(&dir);
        let mut seen = false;
        for width in 1..=60 {
            let Some(line) = summary_line(&work, width, &theme(), now()) else {
                continue;
            };
            let text = text_of(std::slice::from_ref(&line));
            assert!(!text.contains("1 need a human"), "{width}: {text}");
            if text.contains("needs a human") {
                seen = true;
            }
        }
        assert!(seen, "the short form never said '1 needs a human'");
    }
}
