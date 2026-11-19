//! The Needs you tab: the human lane.
//!
//! The tasks whose next action is the human's: approve, review and
//! escalated. A task carrying an overlay is not waiting on a human, and
//! drops out of the tab.
//!
//! The tab carries the three counts the Overview shows, and the rows
//! themselves: the id, the reason, the title and how long the task has
//! waited.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use coding_crew_core::{Task, parse_rfc3339};

use crate::data::{Reason, Work};
use crate::theme::{Theme, block_title_style, reason_style, selected_line};
use crate::view::View;

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between two columns.
const GAP: usize = 2;
/// The indent of a group header.
const HEADER_INDENT: usize = 2;
/// The indent of a row.
const ROW_INDENT: usize = 4;
/// The width of the id column.
const ID_WIDTH: usize = 5;
/// The width of the reason column: the longest reason.
const WHY_WIDTH: usize = 9;
/// The narrowest a title is shown in.
const MIN_TITLE: usize = 8;
/// The width a title is shown in when the reason is on the row too.
const COMFORTABLE_TITLE: usize = 12;
/// The margin at the right of a row.
const MARGIN: usize = 1;

/// A task waiting on a human.
#[derive(Debug, Clone)]
pub struct Row {
    /// The id of the task.
    pub id: String,
    /// The title of the task.
    pub title: String,
    /// Why the task waits on a human.
    pub reason: Reason,
    /// How long the task has waited, in seconds.
    pub waited: i64,
}

/// The tasks that wait for the same reason.
#[derive(Debug, Clone)]
pub struct Group {
    /// The reason the group waits for.
    pub reason: Reason,
    /// The tasks of the group, the longest wait first.
    pub rows: Vec<Row>,
}

/// Draw the tab through the window the keys move.
///
/// `selected` is the row the cursor is on: its cells take the selection's
/// paint. Every other row keeps its own. The window is settled so the cursor
/// is on a row the pane shows.
pub fn render_view(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    theme: &Theme,
    selected: Option<&str>,
    view: &mut View,
    now: &str,
) {
    let block = Block::bordered()
        .title("needs you")
        .title_style(block_title_style(theme))
        .border_style(theme.chrome.border);
    if let Some(missing) = work.missing {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(missing.headline(), theme.semantic.error)),
                Line::from(Span::styled(
                    format!("looked in {}", work.root.display()),
                    theme.text.dim,
                )),
            ])
            .wrap(Wrap { trim: true })
            .block(block),
            area,
        );
        return;
    }
    if work.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no tasks yet", theme.text.dim))).block(block),
            area,
        );
        return;
    }
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let ids = row_ids(work, now);
    let rows = view.draw(&ids, selected, |first| {
        window(work, width, height, theme, selected, first, now)
    });
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }).block(block),
        area,
    );
}

/// The ids of the tab's rows, in the order they are shown.
///
/// The detail pane opens from this tab: the rows are what it can open.
pub fn row_ids(work: &Work, now: &str) -> Vec<String> {
    groups(work, now)
        .into_iter()
        .flat_map(|group| group.rows.into_iter().map(|row| row.id))
        .collect()
}

/// The groups of the human lane: every reason, and the tasks that wait
/// for it.
///
/// The reasons are always there: a count of zero is a fact too, and the
/// Overview shows the same three.
pub fn groups(work: &Work, now: &str) -> Vec<Group> {
    let mut groups: Vec<Group> = Reason::ALL
        .iter()
        .map(|reason| Group {
            reason: *reason,
            rows: Vec::new(),
        })
        .collect();
    for (task, reason) in work.needs_you() {
        let Some(group) = groups.iter_mut().find(|group| group.reason == reason) else {
            continue;
        };
        group.rows.push(Row {
            id: task.id.clone(),
            title: task.title.clone(),
            reason,
            waited: waited_since(task, now),
        });
    }
    for group in &mut groups {
        // The longest wait comes first: the task that has waited longest is
        // the one the human should see first.
        group
            .rows
            .sort_by(|one, other| other.waited.cmp(&one.waited).then(one.id.cmp(&other.id)));
    }
    groups
}

/// How long the task has been waiting.
///
/// The wait is measured from the moment the task entered the status it
/// waits in: `status_since`. The model has the rule for a file that
/// carries no stamp — see `docs/tasks/task-model.md` §2.3.
fn waited_since(task: &Task, now: &str) -> i64 {
    let now = parse_rfc3339(now).unwrap_or(0);
    let since = parse_rfc3339(&task.status_since)
        .or_else(|| parse_rfc3339(&task.created))
        .unwrap_or(now);
    (now - since).max(0)
}

/// The lines of the tab.
///
/// What does not fit is said out loud: a count is never shown as if it
/// were the whole count, and a reason is never lost.
#[cfg(test)]
fn lines(
    work: &Work,
    width: usize,
    height: usize,
    theme: &Theme,
    selected: Option<&str>,
    now: &str,
) -> Vec<Line<'static>> {
    window(work, width, height, theme, selected, 0, now).0
}

/// The lines of the tab, from the row at `first`, and the rows they show.
///
/// What does not fit is said out loud: a count is never shown as if it were
/// the whole count, and a reason is never lost. The rows before the window
/// and the rows after it are counted the same way: the announcement names
/// every reason and the count of its tasks that are not in view.
///
/// A pane too short for the announcement and the cursor's row draws the row
/// alone: the selection is never on nothing.
fn window(
    work: &Work,
    width: usize,
    height: usize,
    theme: &Theme,
    selected: Option<&str>,
    first: usize,
    now: &str,
) -> (Vec<Line<'static>>, Vec<usize>, bool) {
    let groups = groups(work, now);
    let ages = age_width(&groups);
    let all: Vec<String> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.clone()))
        .collect();
    let mut out: Vec<Line<'static>> = Vec::new();
    if height == 0 {
        return (out, Vec::new(), false);
    }
    out.push(headline(work, width, theme));
    let rest = height - 1;
    if rest == 0 {
        // The pane holds one line: the cursor's row is worth more than the
        // headline, so the cursor is never on a row the pane does not draw.
        if let Some(id) = selected
            && let Some((index, line)) = cursor_row(&groups, width, ages, theme, id)
        {
            return (vec![line], vec![index], false);
        }
        return (out, Vec::new(), false);
    }
    let wanted: usize = groups.iter().map(|group| 1 + group.rows.len()).sum();
    if first == 0 && wanted <= rest {
        // Everything fits: there is nothing to say.
        for group in &groups {
            push_group(&mut out, group, width, ages, theme, selected);
        }
        return (out, (0..all.len()).collect(), true);
    }
    // Something does not fit. The announcement is whole or it is cut: the
    // longest announcement that fits whole wins, and the rows give way to
    // it. Only when even the smallest layout cannot say it whole is the
    // announcement cut, and then it says that too. The cursor's row is the
    // one row the window may not leave out: a cut layout that shows it wins
    // over the shortest one.
    let mut shortest: Option<(Vec<Line<'static>>, Vec<Line<'static>>, Vec<usize>)> = None;
    let mut with_cursor: Option<(Vec<Line<'static>>, Vec<Line<'static>>, Vec<usize>)> = None;
    for budget in (0..=rest).rev() {
        let (rows, omitted, shown) = layout(&groups, budget, width, ages, theme, selected, first);
        let room = rest.saturating_sub(rows.len());
        let (said, cut) = announcement(&omitted, width, room, theme);
        let has_cursor = selected.is_none_or(|id| {
            shown
                .iter()
                .any(|index| all.get(*index).is_some_and(|other| other == id))
        });
        if cut {
            if selected.is_some() && has_cursor && with_cursor.is_none() {
                with_cursor = Some((rows.clone(), said.clone(), shown.clone()));
            }
            shortest = Some((rows, said, shown));
            continue;
        }
        if !has_cursor {
            // Fewer rows cannot bring the cursor back into view.
            continue;
        }
        out.extend(rows);
        out.extend(said);
        return (out, shown, true);
    }
    let (rows, said, shown) = with_cursor.or(shortest).expect("a budget of zero");
    if let Some(id) = selected
        && !shown
            .iter()
            .any(|index| all.get(*index).is_some_and(|other| other == id))
        && let Some((index, line)) = cursor_row(&groups, width, ages, theme, id)
    {
        // The pane is too short for the cursor's row and the announcement:
        // the row is shown alone, so the selection is never on nothing.
        return (vec![line], vec![index], false);
    }
    out.extend(rows);
    out.extend(said);
    (out, shown, false)
}

/// The line of the cursor's row, and its place among the tab's rows.
fn cursor_row(
    groups: &[Group],
    width: usize,
    age_width: usize,
    theme: &Theme,
    id: &str,
) -> Option<(usize, Line<'static>)> {
    let mut index = 0;
    for group in groups {
        for row in &group.rows {
            if row.id == id {
                return Some((index, row_line(row, width, age_width, theme, Some(id))));
            }
            index += 1;
        }
    }
    None
}

/// The rows of the tab that fit in `capacity` rows, and how many tasks of
/// each reason are left out of them.
///
/// The groups keep their order: the header of a group, then its rows. A
/// header's count is the whole group; the omitted count is what the
/// announcement says instead, and the two are meant to differ.
fn layout(
    groups: &[Group],
    capacity: usize,
    width: usize,
    age_width: usize,
    theme: &Theme,
    selected: Option<&str>,
    first: usize,
) -> (Vec<Line<'static>>, [usize; 3], Vec<usize>) {
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut omitted = [0usize; 3];
    let mut shown_rows: Vec<usize> = Vec::new();
    let mut used = 0usize;
    let mut row_index = 0usize;
    for group in groups {
        let index = index_of(group.reason);
        let start = row_index;
        row_index += group.rows.len();
        if row_index <= first {
            // The whole group is above the window.
            omitted[index] = group.rows.len();
            continue;
        }
        if used + 1 > capacity {
            // The header does not fit: the group is not shown at all, and
            // every task of it is left out.
            omitted[index] = group.rows.len();
            continue;
        }
        out.push(header(group, width, theme));
        used += 1;
        let skip = first.saturating_sub(start).min(group.rows.len());
        let visible = group.rows.len() - skip;
        let shown = (capacity - used).min(visible);
        for (offset, row) in group.rows.iter().skip(skip).take(shown).enumerate() {
            out.push(row_line(row, width, age_width, theme, selected));
            shown_rows.push(start + skip + offset);
        }
        used += shown;
        if shown < group.rows.len() {
            // The group is on screen, and part of it is not: the rows above
            // the window count, and so do the rows below it.
            omitted[index] = group.rows.len() - shown;
        }
    }
    (out, omitted, shown_rows)
}

/// The place of a reason in the order of the tab.
fn index_of(reason: Reason) -> usize {
    Reason::ALL
        .iter()
        .position(|other| *other == reason)
        .unwrap_or(0)
}

/// The headline: how many tasks wait on a human.
///
/// The same words the Overview uses, painted the same way. The longest
/// form that fits the row is the one shown, so a part of the line is never
/// cut in half.
fn headline(work: &Work, width: usize, theme: &Theme) -> Line<'static> {
    let (approve, review, escalated) = work.needs_you_counts();
    let total = approve + review + escalated;
    let count = total.to_string();
    // The count is the fact: it is never cut, and the words give way.
    if count.chars().count() > width {
        return Line::from(Span::styled(cut_to(&count, width), theme.text.strong));
    }
    let room = width.saturating_sub(count.chars().count() + 1);
    let forms = [
        format!("{} {} on a human", task_word(total), wait_word(total)),
        format!("{} a human", need_word(total)),
        wait_word(total).to_string(),
    ];
    let words = forms
        .iter()
        .find(|words| words.chars().count() <= room)
        .cloned()
        .unwrap_or_else(|| forms[forms.len() - 1].clone());
    let mut spans = vec![Span::styled(count, theme.text.strong)];
    if room > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(cut_to(&words, room), theme.semantic.human));
    }
    Line::from(spans)
}

/// The header of a group: its reason and the total number of tasks that
/// wait for it.
///
/// The count is `text.strong`, as every other count is, and it is on the
/// row of the reason it counts: a pane too narrow for both says the reason
/// alone. What does not fit on screen is the announcement's job, in a
/// different shape: the header never counts what is hidden.
fn header(group: &Group, width: usize, theme: &Theme) -> Line<'static> {
    let count = group.rows.len().to_string();
    let label = group.reason.label();
    let style = reason_style(theme, group.reason);
    let separator = " · ";
    let label_width = WHY_WIDTH;
    let label = pad(&cut_to(label, label_width), label_width);
    let fixed = separator.chars().count() + count.chars().count();
    let indent = " ".repeat(HEADER_INDENT);
    // The reason, the count beside it, and the indent.
    if width >= HEADER_INDENT + label_width + fixed {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label, style),
            Span::styled(separator.to_string(), theme.text.dim),
            Span::styled(count, theme.text.strong),
        ]);
    }
    // The indent gives way.
    if width >= label_width + fixed {
        return Line::from(vec![
            Span::styled(label, style),
            Span::styled(separator.to_string(), theme.text.dim),
            Span::styled(count, theme.text.strong),
        ]);
    }
    // The count gives way: a count is never shown without its reason.
    if width >= HEADER_INDENT + label_width {
        return Line::from(vec![Span::raw(indent), Span::styled(label, style)]);
    }
    if width >= label_width {
        return Line::from(Span::styled(label, style));
    }
    // The pane is too narrow for the word: it is cut.
    Line::from(Span::styled(cut_to(group.reason.label(), width), style))
}

/// The header of a group and its rows.
fn push_group(
    out: &mut Vec<Line<'static>>,
    group: &Group,
    width: usize,
    age_width: usize,
    theme: &Theme,
    selected: Option<&str>,
) {
    out.push(header(group, width, theme));
    for row in &group.rows {
        out.push(row_line(row, width, age_width, theme, selected));
    }
}

/// The width of the age column: the widest age of the rows.
///
/// The ages stand in a column of their own, so the days line up.
fn age_width(groups: &[Group]) -> usize {
    groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .map(|row| age_text(row.waited).chars().count())
        .max()
        .unwrap_or(2)
        .max(2)
}

/// A row: the id, the reason, the title and how long it has waited.
///
/// The row is a table: the id, the reason and the age are never cut, and
/// the title gives way when the pane is narrow. The age stands at the right
/// of the row, in a column of its own, so the days line up.
fn row_line(
    row: &Row,
    width: usize,
    age_width: usize,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    let line = row_line_plain(row, width, age_width, theme);
    if selected == Some(row.id.as_str()) {
        return selected_line(line, width, theme);
    }
    line
}

/// A row in its own paint, before the selection.
fn row_line_plain(row: &Row, width: usize, age_width: usize, theme: &Theme) -> Line<'static> {
    let age = format!("{:>width$}", age_text(row.waited), width = age_width);
    let style = reason_style(theme, row.reason);
    let id = format!("{:<id_width$}", row.id, id_width = ID_WIDTH);
    let why = format!("{:<why_width$}", row.reason.label(), why_width = WHY_WIDTH);
    let indent = " ".repeat(ROW_INDENT);
    let gap = " ".repeat(GAP);
    let margin = " ".repeat(MARGIN);
    let tail = vec![
        Span::raw(gap.clone()),
        Span::styled(age.clone(), theme.text.dim),
        Span::raw(margin.clone()),
    ];
    let tail_width = tail
        .iter()
        .map(|span| span.content.chars().count())
        .sum::<usize>();

    // The row with every column.
    let head = format!("{indent}{id}{gap}{why}{gap}");
    let title_width = width.saturating_sub(head.chars().count() + tail_width);
    if title_width >= COMFORTABLE_TITLE {
        let mut spans = vec![
            Span::styled(head, style),
            Span::styled(pad(&cut_to(&row.title, title_width), title_width), style),
        ];
        spans.extend(tail);
        return Line::from(spans);
    }
    // The reason gives way: the group header names it, and the title is
    // what the reader is here for.
    let head = format!("{indent}{id}{gap}");
    let title_width = width.saturating_sub(head.chars().count() + tail_width);
    if title_width >= MIN_TITLE {
        let mut spans = vec![
            Span::styled(head, style),
            Span::styled(pad(&cut_to(&row.title, title_width), title_width), style),
        ];
        spans.extend(tail);
        return Line::from(spans);
    }
    // The title gives way: the age stays at the right of the row.
    let head = format!("{indent}{id}{gap}{why}");
    if head.chars().count() + tail_width <= width {
        let filler = " ".repeat(width - head.chars().count() - tail_width);
        let mut spans = vec![Span::styled(head, style), Span::raw(filler)];
        spans.extend(tail);
        return Line::from(spans);
    }
    // The reason gives way too: the id and the age are what is left.
    let head = format!("{indent}{id}");
    if head.chars().count() + tail_width <= width {
        let filler = " ".repeat(width - head.chars().count() - tail_width);
        let mut spans = vec![Span::styled(head, style), Span::raw(filler)];
        spans.extend(tail);
        return Line::from(spans);
    }
    // Nothing but the id fits.
    Line::from(Span::styled(cut_to(&format!("{indent}{id}"), width), style))
}

/// How a piece of the announcement is painted.
#[derive(Debug, Clone, Copy)]
enum Paint {
    /// A reason name: the slot of the reason, as on the header and the row.
    Reason(Reason),
    /// A count, as every count in raul.
    Strong,
    /// The bracket, the word `hidden`, the separator, the lead-in.
    Dim,
}

/// A piece of the announcement: its text and how it is painted.
#[derive(Debug, Clone)]
struct Said {
    spans: Vec<(String, Paint)>,
}

impl Said {
    fn new(text: impl Into<String>, paint: Paint) -> Self {
        Said {
            spans: vec![(text.into(), paint)],
        }
    }

    fn width(&self) -> usize {
        self.spans
            .iter()
            .map(|(text, _)| text.chars().count())
            .sum()
    }

    #[cfg(test)]
    fn text(&self) -> String {
        self.spans.iter().map(|(text, _)| text.as_str()).collect()
    }
}

/// The lines that say what is not shown, in at most `room` rows.
///
/// The count here is the count of what is *left out*, not of what the group
/// holds — the header owns that — so the entry has a shape of its own: the
/// reason, then the count in brackets with the word `hidden`. It never takes
/// the header's `reason · total` shape, and the two meanings cannot be read
/// for each other. On a narrow row the reason gives way, then the count; the
/// word `hidden` is the last thing to go.
///
/// A reason is painted with its own slot and a count with `text.strong`, as
/// the theme requires; neither is dim.
///
/// An entry is whole on its row: a count is never cut from its reason. When
/// even the first entry does not fit in `room`, the entries give way whole,
/// from the last one, and the last row is marked with `…`. A row that
/// already fills the width keeps its spans: there is no room for the marker,
/// and repainting it would throw away the reason's slot and the count's
/// strength. The flag says the announcement is cut.
fn announcement(
    omitted: &[usize; 3],
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    let entries: Vec<Said> = Reason::ALL
        .iter()
        .enumerate()
        .map(|(index, reason)| entry(*reason, omitted[index], width))
        .collect();
    let rows = pack(&entries, width);
    if rows.len() <= room {
        return (paint(rows, theme), false);
    }
    if room == 0 {
        return (Vec::new(), true);
    }
    for keep in (0..entries.len()).rev() {
        let mut rows = pack(&entries[..keep], width);
        let Some(last) = rows.last_mut() else {
            break;
        };
        if last.width() < width {
            last.spans.push(("…".to_string(), Paint::Dim));
        } else if last.width() > width {
            *last = cut_row(last, width);
        }
        if rows.len() <= room {
            return (paint(rows, theme), true);
        }
    }
    // Not even one entry fits: the cut is all that is said.
    let marker = cut_to("…", width);
    if marker.is_empty() {
        (Vec::new(), true)
    } else {
        (vec![Line::from(Span::styled(marker, theme.text.dim))], true)
    }
}

/// The row, cut to `width` columns span by span.
///
/// Each kept piece keeps its paint: a reason stays in its slot and a count
/// stays `text.strong`. Only the `…` that says the row was cut is dim.
fn cut_row(said: &Said, width: usize) -> Said {
    let budget = width.saturating_sub(1);
    let mut spans: Vec<(String, Paint)> = Vec::new();
    let mut used = 0usize;
    for (text, paint) in &said.spans {
        if used >= budget {
            break;
        }
        let room = budget - used;
        let piece: String = text.chars().take(room).collect();
        used += piece.chars().count();
        spans.push((piece, *paint));
        if text.chars().count() > room {
            break;
        }
    }
    if width > 0 {
        spans.push(("…".to_string(), Paint::Dim));
    }
    Said { spans }
}

/// The entries, packed into rows of at most `width` columns.
///
/// The words that say what the rows are about lead the first row, when they
/// fit there. An entry is never broken over two rows, so a count is never
/// cut from its reason.
fn pack(entries: &[Said], width: usize) -> Vec<Said> {
    let prefix = Said::new("not shown: ", Paint::Dim);
    let mut rows: Vec<Said> = Vec::new();
    let mut current = Said { spans: Vec::new() };
    for entry in entries {
        if entry.width() == 0 {
            continue;
        }
        if current.width() == 0 {
            let lead = rows.is_empty() && prefix.width() + entry.width() <= width;
            if lead {
                current.spans.extend(prefix.spans.clone());
            }
            current.spans.extend(entry.spans.clone());
            continue;
        }
        if current.width() + 2 + entry.width() <= width {
            current.spans.push((", ".to_string(), Paint::Dim));
            current.spans.extend(entry.spans.clone());
        } else {
            rows.push(Said {
                spans: std::mem::take(&mut current.spans),
            });
            current.spans.extend(entry.spans.clone());
        }
    }
    if current.width() > 0 {
        rows.push(current);
    }
    rows
}

/// The announcement's lines, each span painted by its role.
fn paint(rows: Vec<Said>, theme: &Theme) -> Vec<Line<'static>> {
    rows.into_iter()
        .map(|said| {
            Line::from(
                said.spans
                    .into_iter()
                    .map(|(text, paint)| {
                        let style = match paint {
                            Paint::Reason(reason) => reason_style(theme, reason),
                            Paint::Strong => theme.text.strong,
                            Paint::Dim => theme.text.dim,
                        };
                        Span::styled(text, style)
                    })
                    .collect::<Vec<Span<'static>>>(),
            )
        })
        .collect()
}

/// The name of a reason and the count of its tasks that are not shown.
///
/// The count is the number of tasks left out of view. The shape says so —
/// the reason, then the count in brackets with the word `hidden` — so it
/// cannot be read as the header's `reason · total`. The word is the one
/// thing that survives every width: the reason gives way first, then the
/// count, and only a row too narrow for `hidden` itself loses it.
fn entry(reason: Reason, count: usize, width: usize) -> Said {
    let name = reason.label();
    let count = count.to_string();
    let full = format!("{name} ({count} hidden)");
    if full.chars().count() <= width {
        return Said {
            spans: vec![
                (name.to_string(), Paint::Reason(reason)),
                (" (".to_string(), Paint::Dim),
                (count, Paint::Strong),
                (" hidden)".to_string(), Paint::Dim),
            ],
        };
    }
    let counted = format!("{count} hidden");
    if counted.chars().count() <= width {
        return Said {
            spans: vec![(count, Paint::Strong), (" hidden".to_string(), Paint::Dim)],
        };
    }
    Said::new(cut_to("hidden", width), Paint::Dim)
}

/// How long a wait reads as.
fn age_text(seconds: i64) -> String {
    let seconds = seconds.max(0);
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

/// The word for a number of tasks.
fn task_word(count: usize) -> &'static str {
    if count == 1 { "task" } else { "tasks" }
}

/// The verb for a number of tasks that need a human.
fn need_word(count: usize) -> &'static str {
    if count == 1 { "needs" } else { "need" }
}

/// The verb for a number of tasks.
fn wait_word(count: usize) -> &'static str {
    if count == 1 { "waits" } else { "wait" }
}

/// The text, cut to `width` columns.
fn cut_to(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let head: String = text.chars().take(width - 1).collect();
    format!("{head}…")
}

/// The text, padded to `width` columns.
fn pad(text: &str, width: usize) -> String {
    let length = text.chars().count();
    if length >= width {
        return text.to_string();
    }
    format!("{text}{}", " ".repeat(width - length))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;
    use crate::data::Missing;
    use crate::theme::Preset;

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// The tests' clock: one instant for the rows and the draw that shows
    /// them. Every fixture entered its status before it.
    const NOW: &str = "2026-11-19T08:00:00Z";

    fn now() -> &'static str {
        NOW
    }

    /// Draw the tab and return the rows it shows.
    fn draw(work: &Work, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let mut view = View::default();
        terminal
            .draw(|frame| render_view(frame, frame.area(), work, &theme(), None, &mut view, now()))
            .expect("drawing the needs you tab");
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

    /// Draw the tab and return every cell, with its colour.
    fn draw_cells(work: &Work, width: u16, height: u16) -> Vec<Vec<(char, Color)>> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let mut view = View::default();
        terminal
            .draw(|frame| render_view(frame, frame.area(), work, &theme(), None, &mut view, now()))
            .expect("drawing the needs you tab");
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

    /// The text of the rows, without the border.
    fn text_of(rows: &[String]) -> String {
        rows.iter()
            .map(|row| row.trim_end().to_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// The rows of the pane, without the border of the block.
    fn inner(rows: &[String]) -> Vec<String> {
        rows.iter()
            .skip(1)
            .take(rows.len().saturating_sub(2))
            .map(|row| {
                let row = row.trim_end();
                match row.strip_prefix('│').and_then(|row| row.strip_suffix('│')) {
                    Some(inner) => inner.trim_end().to_string(),
                    None => row.to_string(),
                }
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

    /// The rows of a group, by reason.
    ///
    /// The clock is read once, by the caller: two reads can land in two
    /// seconds, and the wait of a row would not be the same in both.
    fn group_of(work: &Work, reason: Reason, now: &str) -> Group {
        groups(work, now)
            .into_iter()
            .find(|group| group.reason == reason)
            .expect("a group per reason")
    }

    /// A work folder holding the tasks given.
    fn work_of(tasks: Vec<Task>) -> Work {
        Work {
            root: std::path::PathBuf::new(),
            tasks,
            items: Vec::new(),
            errors: Vec::new(),
            missing: None,
        }
    }

    /// A task that waits for approval.
    fn waiting_task(id: &str) -> Task {
        let mut task = coding_crew_core::Task::new(id, "a task");
        task.status = coding_crew_core::Status::Groomed;
        task
    }

    #[test]
    fn the_counts_of_the_fixtures_are_pinned() {
        // The three counts the Overview shows, and the rows behind them.
        let work = mock_work();
        let now = NOW;
        assert_eq!(work.needs_you_counts(), (2, 3, 3));
        assert_eq!(group_of(&work, Reason::Approve, now).rows.len(), 2);
        assert_eq!(group_of(&work, Reason::Review, now).rows.len(), 3);
        assert_eq!(group_of(&work, Reason::Escalated, now).rows.len(), 3);
    }

    #[test]
    fn a_task_with_an_overlay_does_not_wait_on_a_human() {
        // TSK05 is groomed and blocked: it is held, not waiting on a human.
        let work = mock_work();
        let ids: Vec<String> = groups(&work, now())
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.id))
            .collect();
        assert!(!ids.contains(&"TSK05".to_string()), "{ids:?}");
        assert!(ids.contains(&"TSK04".to_string()), "{ids:?}");
    }

    #[test]
    fn every_row_of_the_human_lane_is_in_the_tab() {
        let work = mock_work();
        let shown: Vec<String> = groups(&work, now())
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.id))
            .collect();
        let waiting: Vec<String> = work
            .needs_you()
            .into_iter()
            .map(|(task, _)| task.id.clone())
            .collect();
        assert_eq!(shown.len(), waiting.len());
        for id in waiting {
            assert!(shown.contains(&id), "{id} is missing");
        }
    }

    #[test]
    fn the_wait_is_measured_from_the_status_and_not_from_the_last_change() {
        // The task entered the state it waits in three hours ago, and was
        // written to a second later: the wait is three hours, not one
        // second.
        let mut task = waiting_task("TSK01");
        task.status_since = "2026-11-07T13:00:00Z".to_string();
        task.updated = "2026-11-07T13:00:01Z".to_string();
        let work = work_of(vec![task]);
        let now = "2026-11-07T16:00:00Z";
        let row = group_of(&work, Reason::Approve, now)
            .rows
            .into_iter()
            .next()
            .expect("the task waits for approval");
        assert_eq!(row.waited, 3 * 3600);
    }

    #[test]
    fn a_task_without_the_field_waits_since_it_was_created() {
        // A file written before `status_since` existed: the wait is at
        // least the time since the task was created.
        let mut task = waiting_task("TSK01");
        task.status_since = String::new();
        task.created = "2026-11-07T10:00:00Z".to_string();
        task.updated = "2026-11-07T15:59:00Z".to_string();
        let work = work_of(vec![task]);
        let now = "2026-11-07T16:00:00Z";
        let row = group_of(&work, Reason::Approve, now)
            .rows
            .into_iter()
            .next()
            .expect("the task waits for approval");
        assert_eq!(row.waited, 6 * 3600);
    }

    #[test]
    fn the_wait_of_a_fixture_is_the_time_since_it_entered_its_status() {
        // One clock for the whole test: two reads can land in two seconds.
        let work = mock_work();
        let now = NOW;
        let row = group_of(&work, Reason::Approve, now)
            .rows
            .into_iter()
            .find(|row| row.id == "TSK04")
            .expect("TSK04 waits for approval");
        let task = work.task("TSK04").expect("the task");
        let expected = parse_rfc3339(now).expect("now")
            - parse_rfc3339(&task.status_since)
                .or_else(|| parse_rfc3339(&task.created))
                .expect("the moment the task entered its status");
        assert_eq!(row.waited, expected);
    }

    #[test]
    fn a_fixture_row_waits_since_its_status_and_not_since_created_or_updated() {
        // Every fixture in the human lane carries a status_since distinct
        // from created and updated: the wait on the row comes from the
        // stamp of the status, not from the fallback to created, and not
        // from the last write.
        let work = mock_work();
        let now = NOW;
        let now_ts = parse_rfc3339(now).expect("now");
        let rows: Vec<Row> = groups(&work, now)
            .into_iter()
            .flat_map(|group| group.rows)
            .collect();
        assert!(!rows.is_empty(), "no human lane fixtures");
        for row in rows {
            let task = work.task(&row.id).expect("the task");
            let since = parse_rfc3339(&task.status_since).expect("it entered its status");
            let created = parse_rfc3339(&task.created).expect("it was created");
            let updated = parse_rfc3339(&task.updated).expect("it was last written");
            assert_eq!(row.waited, now_ts - since, "{}", row.id);
            assert_ne!(row.waited, now_ts - created, "{}", row.id);
            assert_ne!(row.waited, now_ts - updated, "{}", row.id);
        }
    }

    #[test]
    fn the_tab_shows_the_counts_and_the_rows() {
        let rows = draw(&mock_work(), 100, 40);
        let text = text_of(&rows);
        // The headline, the way the Overview words it.
        assert!(text.contains("8 tasks wait on a human"), "{text}");
        // The three counts, each on the row of its reason.
        for (reason, count) in [("approve", "2"), ("review", "3"), ("escalated", "3")] {
            let row = rows
                .iter()
                .find(|row| row.contains(reason))
                .unwrap_or_else(|| panic!("no row for {reason}: {text}"));
            assert!(row.contains(count), "{reason}: {row}");
        }
        // The rows themselves: the id, the reason and the title.
        assert!(text.contains("TSK04"), "{text}");
        assert!(text.contains("TSK13"), "{text}");
        assert!(text.contains("TSK19"), "{text}");
        let title = work_title(&mock_work(), "TSK13");
        assert!(text.contains(&title), "{text}");
    }

    /// The title of a task.
    fn work_title(work: &Work, id: &str) -> String {
        work.task(id).expect("the task").title.clone()
    }

    #[test]
    fn a_row_shows_the_age() {
        let rows = inner(&draw(&mock_work(), 100, 40));
        let row = rows
            .iter()
            .find(|row| row.contains("TSK04"))
            .unwrap_or_else(|| panic!("no row for TSK04: {rows:?}"));
        // The age is a number and a unit: days, hours or minutes.
        let age = row
            .chars()
            .rev()
            .take_while(|ch| !ch.is_whitespace())
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        assert!(
            age.ends_with('d') || age.ends_with('h') || age.ends_with('m'),
            "{row:?} has no age"
        );
    }

    #[test]
    fn the_longest_wait_comes_first() {
        let work = mock_work();
        let group = group_of(&work, Reason::Escalated, now());
        let waits: Vec<i64> = group.rows.iter().map(|row| row.waited).collect();
        let mut sorted = waits.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(waits, sorted);
    }

    #[test]
    fn the_rows_of_a_group_do_not_all_show_the_same_wait() {
        // Each fixture of a phase entered it at its own moment: the reader
        // can tell the rows apart by their wait.
        let work = mock_work();
        let now = NOW;
        for group in groups(&work, now) {
            let mut ages: Vec<String> = group.rows.iter().map(|row| age_text(row.waited)).collect();
            ages.sort();
            ages.dedup();
            assert_eq!(
                ages.len(),
                group.rows.len(),
                "two {:?} rows show the same wait",
                group.reason
            );
        }
    }

    #[test]
    fn no_line_is_longer_than_the_width() {
        let work = mock_work();
        for width in 1..=120 {
            for height in 4..=40 {
                let rows = draw(&work, width, height);
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
    fn a_reason_is_never_lost() {
        // The reason of a row is in the header of its group: however narrow
        // the pane is, the reader knows why the task waits.
        let work = mock_work();
        for width in 11..=120 {
            let text = text_of(&draw(&work, width, 40));
            for reason in ["approve", "review", "escalated"] {
                assert!(
                    text.contains(reason),
                    "{width}: {reason} is missing: {text}"
                );
            }
        }
    }

    #[test]
    fn no_line_the_tab_builds_is_longer_than_the_width() {
        // The check is on the lines the tab builds: the rendered buffer is
        // the width wide by construction, so a line that is too wide would
        // never show there.
        let work = mock_work();
        for width in 1..=120 {
            for height in 1..=40 {
                let rows = lines(&work, width, height, &theme(), None, now());
                for row in &rows {
                    assert!(
                        row.width() <= width,
                        "{width}x{height}: {row:?} is {} wide",
                        row.width()
                    );
                }
            }
        }
    }

    #[test]
    fn the_width_check_can_fail() {
        // A check that cannot fail is worse than no check.
        let wide = vec![Line::from(Span::raw("x".repeat(41)))];
        assert_eq!(too_wide(&wide, 40).len(), 1);
        let exact = vec![Line::from(Span::raw("x".repeat(40)))];
        assert!(too_wide(&exact, 40).is_empty());
    }

    /// The lines of a list that are wider than the pane.
    fn too_wide(lines: &[Line<'_>], width: usize) -> Vec<String> {
        lines
            .iter()
            .filter(|line| line.width() > width)
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn the_tab_asks_for_no_more_rows_than_the_pane_has() {
        let work = mock_work();
        for width in [20u16, 40, 60, 100] {
            for height in [4u16, 6, 8, 12, 24, 40] {
                let asked = lines(
                    &work,
                    usize::from(width) - BORDER,
                    usize::from(height) - BORDER,
                    &theme(),
                    None,
                    now(),
                );
                assert!(
                    asked.len() <= usize::from(height).saturating_sub(BORDER),
                    "{width}x{height}: {} rows",
                    asked.len()
                );
            }
        }
    }

    #[test]
    fn what_is_not_shown_is_announced() {
        // The pane is too short for the rows: what is hidden is said out
        // loud, with the count of the tasks it holds.
        let text = text_of(&draw(&mock_work(), 100, 12));
        assert!(text.contains("not shown"), "{text}");
    }

    #[test]
    fn the_announcement_names_every_reason_even_at_zero() {
        // Two tasks wait for approval and nothing else: the reasons with
        // nothing to hide are named all the same.
        let mut first = waiting_task("TSK01");
        first.status_since = "2026-11-07T13:00:00Z".to_string();
        let mut second = waiting_task("TSK02");
        second.status_since = "2026-11-07T13:00:00Z".to_string();
        let work = work_of(vec![first, second]);
        let text = text_of(&draw(&work, 60, 6));
        assert!(text.contains("not shown"), "{text}");
        for reason in ["approve", "review", "escalated"] {
            assert!(text.contains(reason), "{reason} is missing: {text}");
        }
    }

    #[test]
    fn an_entry_of_the_announcement_is_never_cut_in_half() {
        // An entry is whole on its row, in the announcement's own shape.
        // The full entry keeps the reason and the count with the word; as
        // the row narrows the reason gives way, then the count, and the
        // word `hidden` is the last thing to go.
        for width in 4..=80 {
            let (rows, cut) = announcement(&[2, 3, 3], width, usize::MAX, &theme());
            assert!(!cut, "{width}");
            for row in &rows {
                assert!(row.width() <= width, "{width}: {row:?}");
            }
            let text = said_text(&rows);
            if width >= 8 {
                for count in ["2", "3"] {
                    assert!(text.contains(count), "{width}: {count} is missing: {text}");
                }
            }
            // The word is the last thing to give way.
            if width >= 6 {
                assert!(text.contains("hidden"), "{width}: {text}");
            }
            for row in &rows {
                let row = said_text(std::slice::from_ref(row));
                if let Some(at) = row.find('(') {
                    assert!(at > 0, "{width}: a count without a reason: {row}");
                }
            }
            if width >= 20 {
                for entry in [
                    "approve (2 hidden)",
                    "review (3 hidden)",
                    "escalated (3 hidden)",
                ] {
                    assert!(text.contains(entry), "{width}: {entry} is missing: {text}");
                }
            }
        }
    }

    /// The text of the announcement's lines.
    fn said_text(rows: &[Line<'_>]) -> String {
        rows.iter()
            .map(|row| {
                row.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    #[test]
    fn the_header_counts_the_group_and_the_announcement_counts_the_hidden() {
        // Two tasks wait for approval, three for review, three are
        // escalated. A five-row budget shows the approval group whole, one
        // review row, and nothing of the escalated group.
        let work = human_lane();
        let now = "2026-11-07T16:00:00Z";
        let groups = groups(&work, now);
        let (rows, omitted, _) = layout(&groups, 5, 100, age_width(&groups), &theme(), None, 0);
        // A wholly hidden group, a partly shown one, and one with nothing
        // left out.
        assert_eq!(omitted, [0, 2, 3]);
        let headers = rows
            .iter()
            .map(|row| {
                row.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<String>>();
        // The header counts the group: review holds three tasks.
        let header_of = |reason: &str| {
            headers
                .iter()
                .find(|row| row.trim_start().starts_with(reason))
                .unwrap_or_else(|| panic!("no header for {reason}: {headers:?}"))
                .clone()
        };
        assert!(header_of("approve").contains("· 2"), "{headers:?}");
        assert!(header_of("review").contains("· 3"), "{headers:?}");
        let (said, cut) = announcement(&omitted, 100, 10, &theme());
        assert!(!cut);
        let said = said_text(&said);
        // The announcement counts what is hidden, in a shape of its own:
        // the review header says 3, the announcement says 2 are not shown.
        assert!(said.contains("review (2 hidden)"), "{said}");
        assert!(said.contains("escalated (3 hidden)"), "{said}");
        // A reason with nothing left out is named all the same.
        assert!(said.contains("approve (0 hidden)"), "{said}");
        // The header's shape is not the announcement's.
        assert!(!said.contains(" · "), "{said}");
    }

    #[test]
    fn the_announcement_paints_each_reason_with_its_slot_and_each_count_strong() {
        // The same facts the header paints: a reason takes its own slot, a
        // count is `text.strong`, and an escalation is a failure.
        let theme = theme();
        let human = theme.semantic.human.fg.expect("the human colour");
        let error = theme.semantic.error.fg.expect("the error colour");
        let strong = theme.text.strong.fg.expect("the strong colour");
        let (rows, cut) = announcement(&[2, 3, 3], 80, 10, &theme);
        assert!(!cut);
        for (reason, count, colour) in [
            ("approve", "2", human),
            ("review", "3", human),
            ("escalated", "3", error),
        ] {
            let line = rows
                .iter()
                .find(|line| line.spans.iter().any(|span| span.content == reason))
                .unwrap_or_else(|| panic!("no announcement entry for {reason}"));
            let start = line
                .spans
                .iter()
                .position(|span| span.content == reason)
                .expect("the reason's span");
            let name = &line.spans[start];
            let counted = &line.spans[start + 2];
            assert_eq!(counted.content, count, "{reason}: {line:?}");
            assert_eq!(
                name.style.fg,
                Some(colour),
                "{reason} is not painted with its slot"
            );
            assert_eq!(
                counted.style.fg,
                Some(strong),
                "the {reason} count is not strong"
            );
        }
    }

    /// The colour of `needle` on the announcement row of `reason`.
    fn announcement_colour(
        cells: &[Vec<(char, Color)>],
        reason: &str,
        needle: &str,
    ) -> Option<Color> {
        let row = cells.iter().find(|row| {
            let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
            text.contains("hidden") && text.contains(reason)
        })?;
        let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
        let chars: Vec<char> = text.chars().collect();
        let needle: Vec<char> = needle.chars().collect();
        let start = chars
            .windows(needle.len())
            .position(|window| window == needle.as_slice())?;
        row.get(start).map(|(_, colour)| *colour)
    }

    #[test]
    fn a_cut_announcement_keeps_the_reason_slot_and_the_strong_count() {
        // The reported sizes: the kept row is exactly as wide as the
        // content, so the cut has nothing to shorten. Its spans must
        // survive the cut. Read the buffer, not the model: the model was
        // right when the cut repainted the whole row dim.
        let theme = theme();
        let human = theme.semantic.human.fg.expect("the human colour");
        let strong = theme.text.strong.fg.expect("the strong colour");
        for (width, height, reason, count) in [
            (31u16, 4u16, "approve", "2"),
            (20, 4, "approve", "2"),
            (19, 5, "review", "3"),
        ] {
            let cells = draw_cells(&mock_work(), width, height);
            assert_eq!(
                announcement_colour(&cells, reason, reason),
                Some(human),
                "{width}x{height}: {reason} lost its slot"
            );
            assert_eq!(
                announcement_colour(&cells, reason, count),
                Some(strong),
                "{width}x{height}: the {reason} count is not strong"
            );
        }
    }

    #[test]
    fn a_short_pane_says_the_header_total_and_the_hidden_count_apart() {
        // The pane shows the headline, the approval group, one review row
        // and none of the escalated group: the review header still says
        // three, and the announcement says two are hidden.
        let text = text_of(&draw(&human_lane(), 100, 9));
        assert!(text.contains("8 tasks wait on a human"), "{text}");
        let header = text
            .lines()
            .find(|line| line.contains("review") && line.contains("· 3"))
            .unwrap_or_else(|| panic!("no review header: {text}"));
        assert!(header.contains("· 3"), "{text}");
        assert!(text.contains("review (2 hidden)"), "{text}");
        assert!(text.contains("escalated (3 hidden)"), "{text}");
        assert!(text.contains("approve (0 hidden)"), "{text}");
    }

    /// A task that waits on a human for the given status.
    fn waiter(id: &str, status: coding_crew_core::Status) -> Task {
        let mut task = coding_crew_core::Task::new(id, "a task");
        task.status = status;
        task.status_since = "2026-11-07T13:00:00Z".to_string();
        task
    }

    /// Two approvals, three reviews and three escalations.
    fn human_lane() -> Work {
        work_of(vec![
            waiter("TSK01", coding_crew_core::Status::Groomed),
            waiter("TSK02", coding_crew_core::Status::Groomed),
            waiter("TSK03", coding_crew_core::Status::CodeReview),
            waiter("TSK04", coding_crew_core::Status::CodeReview),
            waiter("TSK05", coding_crew_core::Status::CodeReview),
            waiter("TSK06", coding_crew_core::Status::NeedsHuman),
            waiter("TSK07", coding_crew_core::Status::NeedsHuman),
            waiter("TSK08", coding_crew_core::Status::NeedsHuman),
        ])
    }

    #[test]
    fn no_line_is_spent_on_an_empty_announcement() {
        // Everything fits: there is nothing to say, and no blank line is
        // spent saying it.
        let built = lines(&mock_work(), 58, 38, &theme(), None, now());
        for row in &built {
            assert!(row.width() > 0, "{built:?}");
            let text: String = row
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();
            assert!(!text.contains("not shown"), "{built:?}");
        }
    }

    #[test]
    fn the_count_of_a_reason_is_never_shown_without_the_reason() {
        let work = mock_work();
        for (width, height) in [(40u16, 12), (60, 12), (100, 12), (100, 6)] {
            let text = text_of(&draw(&work, width, height));
            for (reason, count) in [("approve", "2"), ("review", "3"), ("escalated", "3")] {
                let found = text
                    .lines()
                    .any(|line| line.contains(reason) && line.contains(count));
                assert!(found, "{width}x{height}: {reason} has no count: {text}");
            }
        }
    }

    #[test]
    fn a_row_is_painted_with_the_colour_of_its_reason() {
        // The human lane is one colour; an escalation is a failure, and is
        // painted like one.
        let theme = theme();
        let human = theme.semantic.human.fg.expect("the human colour");
        let error = theme.semantic.error.fg.expect("the error colour");
        let cells = draw_cells(&mock_work(), 100, 40);
        assert_eq!(colour_of(&cells, "TSK04"), Some(human), "an approval");
        assert_eq!(colour_of(&cells, "TSK13"), Some(human), "a review");
        assert_eq!(colour_of(&cells, "TSK19"), Some(error), "an escalation");
    }

    #[test]
    fn a_selected_row_takes_the_selection_slot_and_the_others_keep_their_reason() {
        let theme = theme();
        let selection = theme.text.selected;
        let groups = groups(&human_lane(), "2026-11-07T16:00:00Z");
        let approve = &groups[0].rows[0];
        let escalated = &groups[2].rows[0];
        // A row that is not the cursor's keeps the slot of its reason.
        let plain = row_line(approve, 60, 2, &theme, None);
        assert_eq!(plain.spans[0].style, theme.semantic.human, "an approval");
        let plain = row_line(escalated, 60, 2, &theme, None);
        assert_eq!(plain.spans[0].style, theme.semantic.error, "an escalation");
        // The cursor's row takes the selection's pair on every cell, and the
        // bar runs to the edge of the pane.
        let selected = row_line(approve, 60, 2, &theme, Some("TSK01"));
        assert_eq!(selected.width(), 60, "the selection bar stops short");
        for span in &selected.spans {
            assert_eq!(span.style, selection, "{:?} is not the selection", span);
        }
        assert_eq!(selected.spans[0].style.fg, selection.fg);
        assert_eq!(selected.spans[0].style.bg, selection.bg);
        // A later row is not painted as the selection.
        let other = row_line(escalated, 60, 2, &theme, Some("TSK01"));
        assert_eq!(other.spans[0].style, theme.semantic.error);
    }

    #[test]
    fn the_empty_state_is_not_a_blank_screen() {
        let rows = draw(&Work::load(Path::new("/nowhere/coding-crew/work")), 60, 12);
        let text = text_of(&rows);
        assert!(text.contains("no coding-crew folder found"), "{text}");
        assert!(
            text.contains("looked in /nowhere/coding-crew/work"),
            "{text}"
        );
        assert!(!text.contains("no work folder"), "{text}");
    }

    #[test]
    fn a_project_without_work_says_so_and_names_the_path() {
        let work = Work::load_missing(
            Path::new("/nowhere/project/coding-crew/work"),
            Missing::NoWork,
        );
        let text = text_of(&draw(&work, 100, 10));
        assert!(text.contains("this project has no work folder"), "{text}");
        assert!(
            text.contains("looked in /nowhere/project/coding-crew/work"),
            "{text}"
        );
        assert!(!text.contains("no coding-crew folder found"), "{text}");
    }

    #[test]
    fn a_lane_with_nothing_in_it_says_so() {
        let dir = std::env::temp_dir().join(format!("raul-needs-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        let mut task = coding_crew_core::Task::new("TSK01", "a task");
        task.status = coding_crew_core::Status::Approved;
        std::fs::write(
            dir.join("TSK01-a-task.json"),
            serde_json::to_string_pretty(&task).expect("writing a task"),
        )
        .expect("writing a task");
        let work = Work::load(&dir);
        let text = text_of(&draw(&work, 60, 12));
        assert!(text.contains("0 tasks wait on a human"), "{text}");
        assert!(text.contains("approve"), "{text}");
    }

    #[test]
    fn the_presets_say_the_same_thing() {
        let work = mock_work();
        for (width, height) in [(100u16, 40), (60, 16), (40, 12)] {
            let mut rendered = Vec::new();
            for preset in Preset::ALL {
                let mut terminal =
                    Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
                let mut view = View::default();
                terminal
                    .draw(|frame| {
                        render_view(
                            frame,
                            frame.area(),
                            &work,
                            &Theme::preset(preset),
                            None,
                            &mut view,
                            now(),
                        )
                    })
                    .expect("drawing");
                rendered.push((preset.name(), buffer_text(terminal.backend().buffer())));
            }
            let (first_name, first) = &rendered[0];
            for (name, text) in &rendered[1..] {
                assert_eq!(
                    first, text,
                    "{width}x{height}: {first_name} and {name} say different things"
                );
            }
        }
    }

    #[test]
    fn a_narrow_entry_keeps_the_word_hidden() {
        // The reason and the count may give way; the word that says the
        // count is a count of what is hidden may not.
        for width in 6..=40 {
            for count in [0usize, 2, 3] {
                let text = entry(Reason::Approve, count, width).text();
                assert!(text.chars().count() <= width, "{width}: {text}");
                assert!(text.contains("hidden"), "{width}: {count}: {text}");
                assert!(!text.contains(" · "), "{width}: {text}");
            }
        }
    }

    #[test]
    fn a_narrow_announcement_still_says_the_number_is_hidden() {
        // At terminal widths 13..=18 the content is 11..=16 columns: too
        // narrow for `approve (0 hidden)`, and the old shape fell back to
        // `approve (0)`, which reads like a header total. Every reason's
        // entry still says the count is hidden.
        let work = human_lane();
        for width in 13..=18u16 {
            let text = text_of(&draw(&work, width, 12));
            let hidden: Vec<&str> = text
                .lines()
                .filter(|line| line.contains("hidden"))
                .collect();
            assert_eq!(
                text.matches("hidden").count(),
                3,
                "{width}: every reason is announced: {text}"
            );
            for line in hidden {
                assert!(
                    !line.contains(" · "),
                    "{width}: a hidden count reads like a header: {line}"
                );
            }
        }
    }

    /// The text of a buffer.
    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
