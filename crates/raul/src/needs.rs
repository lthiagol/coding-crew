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
use ratatui::widgets::{Paragraph, Wrap};

use coding_crew_core::{Task, age_text, parse_rfc3339};

use crate::data::{Reason, Work};
use crate::review_state::{self, Part};
use crate::tabs::Tab;
use crate::view::{self, View};
use coding_crew_theme::chrome;
use coding_crew_theme::text::{cut_to, pad};
use coding_crew_theme::{Icon, IconSet, Theme, reason_style, selected_line};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between two columns.
const GAP: usize = 2;
/// The indent of a group header.
const HEADER_INDENT: usize = 2;
/// The columns before a row's id: the selection bar, then a space, the status
/// icon and a space.
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
    /// The review state the row carries, when there is one to carry.
    pub review: Option<Vec<Part>>,
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
#[allow(clippy::too_many_arguments)]
pub fn render_view(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    theme: &Theme,
    icons: IconSet,
    selected: Option<&str>,
    view: &mut View,
    now: &str,
) -> Option<usize> {
    let block = chrome::pane("needs you", Icon::TabNeeds, icons, false, theme);
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
        return None;
    }
    if work.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no tasks yet", theme.text.dim))).block(block),
            area,
        );
        return None;
    }
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let ids = row_ids(work, now);
    let (rows, announced) = view.draw(&ids, selected, |first| {
        let (lines, announced, shown, whole) =
            window(work, width, height, theme, icons, selected, first, now);
        ((lines, announced), shown, whole)
    });
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }).block(block),
        area,
    );
    let hidden = ids.len().saturating_sub(view.page());
    (hidden > 0 && !announced).then_some(hidden)
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
            review: review_state::state(task),
        });
    }
    for group in &mut groups {
        // The longest wait comes first: the task that has waited longest is
        // the one the human should see first. The order is the tab's sort,
        // the same value the Settings tab names.
        let sort = Tab::NeedsYou.sort();
        group.rows.sort_by(|one, other| {
            sort.compare_waits((one.waited, &one.id), (other.waited, &other.id))
        });
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
    icons: IconSet,
    selected: Option<&str>,
    now: &str,
) -> Vec<Line<'static>> {
    window(work, width, height, theme, icons, selected, 0, now).0
}

/// The lines of the tab, from the row at `first`, and the rows they show.
///
/// What does not fit is said out loud: a count is never shown as if it were
/// the whole count, and a reason is never lost. The rows before the window
/// and the rows after it are counted the same way: the announcement names
/// every reason and the count of its tasks that are not in view.
///
/// A pane too short for the announcement and the cursor's row draws the row
/// alone: the selection is never on nothing. The returned flag says whether
/// the pane painted the announcement itself; when it did not, the window's
/// caller carries the hidden count to the footer.
#[allow(clippy::too_many_arguments)]
fn window(
    work: &Work,
    width: usize,
    height: usize,
    theme: &Theme,
    icons: IconSet,
    selected: Option<&str>,
    first: usize,
    now: &str,
) -> (Vec<Line<'static>>, bool, Vec<usize>, bool) {
    let groups = groups(work, now);
    let ages = age_width(&groups);
    let all: Vec<String> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.clone()))
        .collect();
    let mut out: Vec<Line<'static>> = Vec::new();
    if height == 0 {
        return (out, false, Vec::new(), false);
    }
    out.push(headline(work, width, theme));
    let rest = height - 1;
    if rest == 0 {
        // The pane holds one line: the cursor's row is worth more than the
        // headline, so the cursor is never on a row the pane does not draw.
        if let Some(id) = selected
            && let Some((index, line)) = cursor_row(&groups, width, ages, icons, theme, id)
        {
            return (vec![line], false, vec![index], false);
        }
        return (out, false, Vec::new(), false);
    }
    let wanted: usize = groups.iter().map(|group| 1 + group.rows.len()).sum();
    if first == 0 && wanted <= rest {
        // Everything fits: there is nothing to say.
        for group in &groups {
            push_group(&mut out, group, width, ages, icons, theme, selected);
        }
        return (out, false, (0..all.len()).collect(), true);
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
        let (rows, omitted, shown) =
            layout(&groups, budget, width, ages, icons, theme, selected, first);
        let room = rest.saturating_sub(rows.len());
        let (said, cut) = announcement(&omitted, width, room, theme);
        let has_cursor = selected.is_none_or(|id| {
            shown
                .iter()
                .any(|index| all.get(*index).is_some_and(|other| other == id))
        });
        if cut {
            if selected.is_some() && has_cursor {
                // The announcement comes first: a cut layout that says more
                // (and draws fewer rows) is the better cut. A pane that
                // leaves no room to say anything is the last resort.
                let score = (said.len(), usize::MAX - rows.len());
                let better = with_cursor
                    .as_ref()
                    .is_none_or(|(rows, said, _)| score > (said.len(), usize::MAX - rows.len()));
                if better {
                    with_cursor = Some((rows.clone(), said.clone(), shown.clone()));
                }
            }
            shortest = Some((rows, said, shown));
            continue;
        }
        if !has_cursor {
            // Fewer rows cannot bring the cursor back into view.
            continue;
        }
        let announced = !said.is_empty();
        out.extend(rows);
        out.extend(said);
        return (out, announced, shown, true);
    }
    if let Some((rows, said, shown)) = with_cursor.or(shortest) {
        if said.is_empty()
            && height >= 2
            && let Some(id) = selected
            && let Some((index, line)) = cursor_row(&groups, width, ages, icons, theme, id)
        {
            // The rows took every line and said nothing about what they hid.
            // The cursor's row is drawn, the headline gives way first, and the
            // rest of the pane names every reason and what it hides.
            let mut rows = Vec::new();
            if height >= 3 {
                rows.push(headline(work, width, theme));
            }
            rows.push(line);
            let room = height - rows.len();
            if room > 0 {
                let omitted = omitted_with_only(&groups, index);
                let (word, _) = announcement(&omitted, width, room, theme);
                if !word.is_empty() {
                    rows.extend(word);
                    return (rows, true, vec![index], false);
                }
            }
        }
        if let Some(id) = selected
            && !shown
                .iter()
                .any(|index| all.get(*index).is_some_and(|other| other == id))
            && let Some((index, line)) = cursor_row(&groups, width, ages, icons, theme, id)
        {
            // The pane is too short for the cursor's row and the layout: the
            // row is drawn, the headline gives way, and what is left says what
            // the pane hides. Only a pane of one line shows the row alone.
            let mut rows = Vec::new();
            if height >= 3 {
                rows.push(headline(work, width, theme));
            }
            rows.push(line);
            let room = height - rows.len();
            let mut announced = false;
            if room > 0 {
                let omitted = omitted_with_only(&groups, index);
                let (word, _) = announcement(&omitted, width, room, theme);
                if !word.is_empty() {
                    announced = true;
                    rows.extend(word);
                }
            }
            return (rows, announced, vec![index], false);
        }
        let announced = !said.is_empty();
        out.extend(rows);
        out.extend(said);
        return (out, announced, shown, false);
    }
    // No layout was chosen: the pane is too short to say anything. The
    // cursor's row is still drawn, alone.
    if let Some(id) = selected
        && let Some((index, line)) = cursor_row(&groups, width, ages, icons, theme, id)
    {
        return (vec![line], false, vec![index], false);
    }
    (out, false, Vec::new(), false)
}

/// What each reason hides when only the row at `cursor` is shown.
fn omitted_with_only(groups: &[Group], cursor: usize) -> [usize; 3] {
    let mut omitted = [0usize; 3];
    let mut at = 0usize;
    for group in groups {
        let index = index_of(group.reason);
        omitted[index] = group.rows.len();
        if (at..at + group.rows.len()).contains(&cursor) {
            omitted[index] = group.rows.len().saturating_sub(1);
        }
        at += group.rows.len();
    }
    omitted
}

/// The line of the cursor's row, and its place among the tab's rows.
fn cursor_row(
    groups: &[Group],
    width: usize,
    age_width: usize,
    icons: IconSet,
    theme: &Theme,
    id: &str,
) -> Option<(usize, Line<'static>)> {
    let mut index = 0;
    for group in groups {
        for row in &group.rows {
            if row.id == id {
                let line = row_line(row, width, age_width, icons, theme, Some(id));
                return Some((index, line));
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
#[allow(clippy::too_many_arguments)]
fn layout(
    groups: &[Group],
    capacity: usize,
    width: usize,
    age_width: usize,
    icons: IconSet,
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
        // An empty group has no rows above the window, so its header is
        // shown wherever the window starts.
        if !group.rows.is_empty() && row_index <= first {
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
            out.push(row_line(row, width, age_width, icons, theme, selected));
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
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) {
    out.push(header(group, width, theme));
    for row in &group.rows {
        out.push(row_line(row, width, age_width, icons, theme, selected));
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
/// The selection bar takes the row's first column, so the row is drawn in the
/// columns after it, whether it is selected or not.
fn row_line(
    row: &Row,
    width: usize,
    age_width: usize,
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    let selected = selected == Some(row.id.as_str());
    // The bar gives way before the id is cut: a row too narrow for both draws
    // its id whole, and a selected row keeps its paint without the bar.
    if width < view::BAR + ID_WIDTH {
        let line = row_line_plain(row, width, age_width, icons, theme);
        return if selected {
            selected_line(line, width, theme)
        } else {
            line
        };
    }
    let line = row_line_plain(row, width - view::BAR, age_width, icons, theme);
    view::barred(line, width, selected, theme)
}

/// The icon of a reason: the status the task waits in.
///
/// A reason is a phase in words, so its icon is the phase's, on the reason's
/// slot. `approve` is `groomed`, `review` is `code-review`, `escalated` is
/// `needs-human`.
fn reason_icon(reason: Reason) -> Icon {
    match reason {
        Reason::Approve => Icon::StatusGroomed,
        Reason::Review => Icon::StatusReview,
        Reason::Escalated => Icon::StatusNeedsHuman,
    }
}

/// A row in its own paint, before the selection.
///
/// The row is a table: the id, the reason and the age are never cut, and
/// the title gives way when the pane is narrow. The age stands at the right
/// of the row, in a column of its own, so the days line up. When the id and
/// the age are all that is left, the status icon goes before the id is cut.
fn row_line_plain(
    row: &Row,
    width: usize,
    age_width: usize,
    icons: IconSet,
    theme: &Theme,
) -> Line<'static> {
    let age = format!("{:>width$}", age_text(row.waited), width = age_width);
    let style = reason_style(theme, row.reason);
    let id = format!("{:<id_width$}", row.id, id_width = ID_WIDTH);
    let why = format!("{:<why_width$}", row.reason.label(), why_width = WHY_WIDTH);
    let gap = " ".repeat(GAP);
    let margin = " ".repeat(MARGIN);
    let clock = format!("{} ", Icon::Clock.glyph(icons));
    let clock_width = clock.chars().count();
    // The age's clock is a decoration: a review state that needs its columns
    // takes them without it.
    let tail = |clock_shown: bool| {
        let mut spans = vec![Span::raw(gap.clone())];
        if clock_shown {
            spans.push(Span::styled(clock.clone(), theme.text.dim));
        }
        spans.push(Span::styled(age.clone(), theme.text.dim));
        spans.push(Span::raw(margin.clone()));
        spans
    };
    let tail_width = tail(true)
        .iter()
        .map(|span| span.content.chars().count())
        .sum::<usize>();
    let lead_width = ROW_INDENT - view::BAR;
    let lead = vec![
        Span::raw(" "),
        Span::styled(reason_icon(row.reason).glyph(icons), style),
        Span::raw(" "),
    ];
    let id_cell = Span::styled(id.clone(), theme.text.strong);
    let gap_cell = Span::raw(gap.clone());
    let why_cell = Span::styled(why.clone(), style);
    let title_cell = |title_width: usize| {
        Span::styled(
            pad(&cut_to(&row.title, title_width), title_width),
            theme.text.normal,
        )
    };

    // The row with every column.
    let head_width = lead_width + ID_WIDTH + GAP + WHY_WIDTH + GAP;
    let title_width = width.saturating_sub(head_width + tail_width);
    // The review state sits before the age, in the room the title leaves.
    // The title keeps its comfortable width, so the state gives way first —
    // whole parts at a time, and a state that cannot say anything gives way
    // to nothing. A row without a state is the row it was.
    if let Some(parts) = row.review.as_deref() {
        let parts_width = |parts: &[Part]| parts.iter().map(Part::width).sum::<usize>();
        let room = title_width.saturating_sub(COMFORTABLE_TITLE + GAP);
        let unclocked = (title_width + clock_width).saturating_sub(COMFORTABLE_TITLE + GAP);
        // The decorations give way before any word: the clock first, then the
        // finding icon. Only a state that fits no other way loses a part.
        let bare = review_state::unmarked(parts);
        let cut = if parts_width(parts) <= room {
            Some((true, parts.to_vec()))
        } else if parts_width(parts) <= unclocked {
            Some((false, parts.to_vec()))
        } else if parts_width(&bare) <= unclocked {
            Some((false, bare))
        } else {
            review_state::fit(&bare, unclocked).map(|state| (false, state))
        };
        if let Some((clock_shown, state)) = cut {
            let title_width = if clock_shown {
                title_width
            } else {
                title_width + clock_width
            };
            let state_width = parts_width(&state);
            let title_width = title_width - GAP - state_width;
            let mut spans = [lead.clone(), vec![id_cell.clone(), gap_cell.clone()]].concat();
            spans.extend([
                why_cell.clone(),
                gap_cell.clone(),
                title_cell(title_width),
                gap_cell.clone(),
            ]);
            spans.extend(review_state::paint(&state, theme, icons));
            spans.extend(tail(clock_shown));
            return Line::from(spans);
        }
    }
    if title_width >= COMFORTABLE_TITLE {
        let mut spans = [lead.clone(), vec![id_cell.clone(), gap_cell.clone()]].concat();
        spans.extend([why_cell.clone(), gap_cell.clone(), title_cell(title_width)]);
        spans.extend(tail(true));
        return Line::from(spans);
    }
    // The reason gives way: the group header names it, and the title is
    // what the reader is here for.
    let head_width = lead_width + ID_WIDTH + GAP;
    let title_width = width.saturating_sub(head_width + tail_width);
    if title_width >= MIN_TITLE {
        let mut spans = [lead.clone(), vec![id_cell.clone(), gap_cell.clone()]].concat();
        spans.push(title_cell(title_width));
        spans.extend(tail(true));
        return Line::from(spans);
    }
    // The title gives way: the age stays at the right of the row.
    let head_width = lead_width + ID_WIDTH + GAP + WHY_WIDTH;
    if head_width + tail_width <= width {
        let mut spans = [lead.clone(), vec![id_cell.clone(), gap_cell.clone()]].concat();
        spans.push(why_cell.clone());
        spans.push(Span::raw(" ".repeat(width - head_width - tail_width)));
        spans.extend(tail(true));
        return Line::from(spans);
    }
    // The reason gives way too: the id and the age are what is left.
    let head_width = lead_width + ID_WIDTH;
    if head_width + tail_width <= width {
        let mut spans = [lead, vec![id_cell.clone()]].concat();
        spans.push(Span::raw(" ".repeat(width - head_width - tail_width)));
        spans.extend(tail(true));
        return Line::from(spans);
    }
    // The icon gives way before the id does: the lead is a space.
    let head_width = 1 + ID_WIDTH;
    if head_width + tail_width <= width {
        let mut spans = vec![Span::raw(" "), id_cell];
        spans.push(Span::raw(" ".repeat(width - head_width - tail_width)));
        spans.extend(tail(true));
        return Line::from(spans);
    }
    // Nothing but the id fits: the indent gives way first.
    Line::from(Span::styled(cut_to(&id, width), theme.text.strong))
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

/// A group's entry: the reason and its count. The reason keeps a row of its
/// own when the count cannot sit beside it, so the reason is never replaced
/// by a count or by the word `hidden`.
#[derive(Debug, Clone)]
struct Entry {
    rows: Vec<Said>,
}

/// The lines that say what is not shown, in at most `room` rows.
///
/// The count here is the count of what is *left out*, not of what the group
/// holds — the header owns that — so the entry has a shape of its own: the
/// reason, then the count in brackets with the word `hidden`. It never takes
/// the header's `reason · total` shape, and the two meanings cannot be read
/// for each other.
///
/// The reason is never cut away: when the count does not fit beside it, the
/// reason takes a row of its own and the count and the word follow. When even
/// the packed entries do not fit in `room`, the compact shape names every
/// reason and its count over the rows there are; when that does not fit
/// either, the entries give way whole, from the last one, and the last row is
/// marked with `…`. The flag says the announcement is cut.
///
/// A reason is painted with its own slot and a count with `text.strong`, as
/// the theme requires; neither is dim.
fn announcement(
    omitted: &[usize; 3],
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    let entries: Vec<Entry> = Reason::ALL
        .iter()
        .enumerate()
        .map(|(index, reason)| entry(*reason, omitted[index], width))
        .collect();
    let rows = pack(&entries, width);
    if rows.len() <= room {
        return (paint(rows, theme), false);
    }
    if room > 0
        && let Some((mut compact, complete)) = compact(omitted, width, room)
    {
        if complete {
            return (paint(compact, theme), false);
        }
        // The word or the lead-in did not fit: the row is not whole. The
        // marker says so when there is room for it beside the last entry;
        // every reason is named, so no entry is dropped for the marker.
        if let Some(last) = compact.last_mut()
            && last.width() < width
        {
            last.spans.push(("…".to_string(), Paint::Dim));
        }
        return (paint(compact, theme), true);
    }
    if room == 0 {
        return (Vec::new(), true);
    }
    for keep in (0..entries.len()).rev() {
        let mut rows = pack(&entries[..keep], width);
        if !mark_last(&mut rows, width) {
            break;
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

/// Mark the last row that has room for the marker, dropping whole rows to
/// make room.
///
/// A row is never cut: a reason and its count stay together, and so do the
/// rows of the compact shape. The flag says a row was marked.
fn mark_last(rows: &mut Vec<Said>, width: usize) -> bool {
    while let Some(last) = rows.last_mut() {
        if last.width() < width {
            last.spans.push(("…".to_string(), Paint::Dim));
            return true;
        }
        rows.pop();
    }
    false
}

/// The entries, packed into rows of at most `width` columns.
///
/// The words that say what the rows are about lead the first row, when they
/// fit there. An entry's own rows stay together: a count is never cut from
/// its reason, and a reason is never parted from the count below it. Only
/// single-row entries share a row with the entry after them.
fn pack(entries: &[Entry], width: usize) -> Vec<Said> {
    let prefix = Said::new("not shown: ", Paint::Dim);
    let mut rows: Vec<Said> = Vec::new();
    let mut current: Option<Said> = None;
    for entry in entries {
        if entry.rows.is_empty() {
            continue;
        }
        if entry.rows.len() == 1 {
            let line = &entry.rows[0];
            if let Some(cur) = current.as_mut()
                && cur.width() + 2 + line.width() <= width
            {
                cur.spans.push((", ".to_string(), Paint::Dim));
                cur.spans.extend(line.spans.clone());
                continue;
            }
            if let Some(cur) = current.take() {
                rows.push(cur);
            }
            let mut cur = Said { spans: Vec::new() };
            if rows.is_empty() && prefix.width() + line.width() <= width {
                cur.spans.extend(prefix.spans.clone());
            }
            cur.spans.extend(line.spans.clone());
            current = Some(cur);
            continue;
        }
        // A multi-row entry: its rows are consecutive, and nothing shares
        // them.
        if let Some(cur) = current.take() {
            rows.push(cur);
        }
        for (index, line) in entry.rows.iter().enumerate() {
            let mut line = line.clone();
            if index == 0 && rows.is_empty() && prefix.width() + line.width() <= width {
                let mut spans = prefix.spans.clone();
                spans.append(&mut line.spans);
                line.spans = spans;
            }
            rows.push(line);
        }
    }
    if let Some(cur) = current {
        rows.push(cur);
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
/// cannot be read as the header's `reason · total`. The reason is never
/// replaced: when it cannot sit beside its count, it takes a row of its own
/// and the count follows.
fn entry(reason: Reason, count: usize, width: usize) -> Entry {
    let name = reason.label();
    let counted = count.to_string();
    let full = format!("{name} ({counted} hidden)");
    if full.chars().count() <= width {
        return Entry {
            rows: vec![Said {
                spans: vec![
                    (name.to_string(), Paint::Reason(reason)),
                    (" (".to_string(), Paint::Dim),
                    (counted, Paint::Strong),
                    (" hidden)".to_string(), Paint::Dim),
                ],
            }],
        };
    }
    let bare = format!("{name} {counted} hidden");
    if bare.chars().count() <= width {
        return Entry {
            rows: vec![Said {
                spans: vec![
                    (name.to_string(), Paint::Reason(reason)),
                    (" ".to_string(), Paint::Dim),
                    (counted, Paint::Strong),
                    (" hidden".to_string(), Paint::Dim),
                ],
            }],
        };
    }
    if width == 0 {
        return Entry { rows: Vec::new() };
    }
    // The reason keeps a row; the count and the word stay together below.
    let name_row = Said::new(cut_to(name, width), Paint::Reason(reason));
    let counted_row = Said {
        spans: vec![
            (counted, Paint::Strong),
            (" hidden".to_string(), Paint::Dim),
        ],
    };
    let count_row = if counted_row.width() <= width {
        counted_row
    } else {
        Said::new(cut_to("hidden", width), Paint::Dim)
    };
    Entry {
        rows: vec![name_row, count_row],
    }
}

/// The compact shape: every reason and its count, packed over at most `room`
/// rows.
///
/// The shape is `reason count, reason count … hidden`: the reasons and their
/// counts are named even when a row is too narrow for `reason (count
/// hidden)`. The word `hidden` or the lead-in says what the counts are; the
/// flag reports whether one of them is present. What fits is preferred in
/// that order.
fn compact(omitted: &[usize; 3], width: usize, room: usize) -> Option<(Vec<Said>, bool)> {
    let tokens: Vec<Said> = Reason::ALL
        .iter()
        .enumerate()
        .map(|(index, reason)| token(*reason, omitted[index], width))
        .collect();
    let mut fallback: Option<(Vec<Said>, bool)> = None;
    for prefix in [true, false] {
        for separator in [", ", ","] {
            for suffix in [Suffix::Last, Suffix::Own, Suffix::None] {
                let (rows, complete) = compact_rows(&tokens, width, prefix, separator, suffix);
                if rows.len() > room {
                    continue;
                }
                if complete {
                    return Some((rows, true));
                }
                if fallback.is_none() {
                    fallback = Some((rows, false));
                }
            }
        }
    }
    fallback
}

/// Where the word `hidden` sits in a compact block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suffix {
    /// At the end of the last row, when it fits there.
    Last,
    /// On a row of its own.
    Own,
    /// Not at all: the reasons and counts are the whole announcement.
    None,
}

/// A group's token in the compact shape: its reason and its count.
///
/// The reason is cut only when the row cannot hold it whole; the count stays
/// whole while there is room for it. When there is no room for both, the
/// reason takes the row, cut to the marker — the reason is what the row is,
/// and a count is never shown without it.
fn token(reason: Reason, count: usize, width: usize) -> Said {
    let name = reason.label();
    let counted = count.to_string();
    let full = format!("{name} {counted}");
    if full.chars().count() <= width {
        return Said {
            spans: vec![
                (name.to_string(), Paint::Reason(reason)),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
            ],
        };
    }
    let tail = format!(" {counted}");
    if tail.chars().count() < width {
        let room = width - tail.chars().count();
        return Said {
            spans: vec![
                (cut_to(name, room), Paint::Reason(reason)),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
            ],
        };
    }
    Said::new(cut_to(name, width), Paint::Reason(reason))
}

/// The compact tokens in rows, with the lead-in and the word placed.
///
/// The flag reports whether the rows say the counts are hidden: the lead-in
/// or the word `hidden` is present.
fn compact_rows(
    tokens: &[Said],
    width: usize,
    prefix: bool,
    separator: &str,
    suffix: Suffix,
) -> (Vec<Said>, bool) {
    let lead = Said::new("not shown: ", Paint::Dim);
    let leading = prefix && lead.width() <= width;
    let mut rows: Vec<Said> = Vec::new();
    let mut current = Said { spans: Vec::new() };
    if leading {
        current.spans.extend(lead.spans.clone());
    }
    let mut has_token = false;
    for token in tokens {
        let gap = if has_token {
            separator.chars().count()
        } else {
            0
        };
        if current.width() + gap + token.width() <= width {
            if has_token {
                current.spans.push((separator.to_string(), Paint::Dim));
            }
            current.spans.extend(token.spans.clone());
            has_token = true;
        } else {
            if current.width() > 0 {
                rows.push(std::mem::replace(&mut current, Said { spans: Vec::new() }));
            }
            current.spans.extend(token.spans.clone());
            has_token = true;
        }
    }
    if current.width() > 0 {
        rows.push(current);
    }
    let word = Said::new(" hidden", Paint::Dim);
    let mut said_it = false;
    match suffix {
        Suffix::Last => {
            if let Some(last) = rows.last_mut()
                && last.width() + word.width() <= width
            {
                last.spans.extend(word.spans);
                said_it = true;
            }
        }
        Suffix::Own => {
            if word.width() <= width {
                rows.push(Said::new("hidden", Paint::Dim));
                said_it = true;
            }
        }
        Suffix::None => {}
    }
    (rows, leading || said_it)
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Style};

    use super::*;
    use crate::data::{Missing, has_overlay};
    use coding_crew_core::Status;
    use coding_crew_core::{Finding, FindingStatus, Round, RoundKind, Severity};
    use coding_crew_theme::Preset;

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
            .draw(|frame| {
                render_view(
                    frame,
                    frame.area(),
                    work,
                    &theme(),
                    IconSet::Nerd,
                    None,
                    &mut view,
                    now(),
                );
            })
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
            .draw(|frame| {
                render_view(
                    frame,
                    frame.area(),
                    work,
                    &theme(),
                    IconSet::Nerd,
                    None,
                    &mut view,
                    now(),
                );
            })
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
            if let Some(byte) = text.find(needle) {
                // The index of a string is a byte index; a cell is a character.
                let start = text[..byte].chars().count();
                return row.get(start).map(|(_, colour)| *colour);
            }
        }
        None
    }

    /// The colour of `needle` on the row that holds `id`.
    fn colour_in(cells: &[Vec<(char, Color)>], id: &str, needle: &str) -> Option<Color> {
        let text_of = |row: &Vec<(char, Color)>| -> String {
            row.iter().map(|(symbol, _)| *symbol).collect()
        };
        let row = cells.iter().find(|row| text_of(row).contains(id))?;
        let byte = text_of(row).find(needle)?;
        // The index of a string is a byte index; a cell is a character.
        let start = text_of(row)[..byte].chars().count();
        row.get(start).map(|(_, colour)| *colour)
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
            activity: None,
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
        // The three counts the Overview shows, and the rows behind them:
        // both follow the fixture tree, blocked and deferred tasks excluded.
        let work = mock_work();
        let now = NOW;
        let count = |status| {
            work.tasks
                .iter()
                .filter(|task| task.status == status && !has_overlay(task))
                .count()
        };
        let expected = (
            count(Status::Groomed),
            count(Status::CodeReview),
            count(Status::NeedsHuman),
        );
        assert_eq!(work.needs_you_counts(), expected);
        assert_eq!(group_of(&work, Reason::Approve, now).rows.len(), expected.0);
        assert_eq!(group_of(&work, Reason::Review, now).rows.len(), expected.1);
        assert_eq!(
            group_of(&work, Reason::Escalated, now).rows.len(),
            expected.2
        );
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
        let work = mock_work();
        let rows = draw(&work, 100, 40);
        let text = text_of(&rows);
        let counts = work.needs_you_counts();
        // The headline, the way the Overview words it.
        let total = counts.0 + counts.1 + counts.2;
        assert!(
            text.contains(&format!("{total} tasks wait on a human")),
            "{text}"
        );
        // The three counts, each on the row of its reason.
        for (reason, count) in [
            ("approve", counts.0),
            ("review", counts.1),
            ("escalated", counts.2),
        ] {
            let row = rows
                .iter()
                .find(|row| row.contains(reason) && row.contains('·'))
                .unwrap_or_else(|| panic!("no header row for {reason}: {text}"));
            assert!(
                row.contains(&format!("· {count}")),
                "{reason}: expected {count} on {row}"
            );
        }
        // The rows themselves: the id, the reason and the title.
        assert!(text.contains("TSK04"), "{text}");
        assert!(text.contains("TSK13"), "{text}");
        assert!(text.contains("TSK19"), "{text}");
        let title = work_title(&work, "TSK13");
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
        // The age is the shared vocabulary: days, hours, minutes, or the
        // words for a wait under a minute.
        let age = row
            .chars()
            .rev()
            .take_while(|ch| !ch.is_whitespace())
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        assert!(
            age.ends_with('d')
                || age.ends_with('h')
                || age.ends_with('m')
                || row.trim_end().ends_with("under a minute"),
            "{row:?} has no age"
        );
    }

    #[test]
    fn a_wait_under_a_minute_says_so() {
        // The shared vocabulary names the sub-minute wait; the row is not
        // zero minutes, and a fixture that reaches the state must not fail
        // on correct behaviour.
        let mut task = waiting_task("TSK01");
        task.status_since = "2026-11-19T07:59:40Z".to_string();
        let work = work_of(vec![task]);
        let rows = draw(&work, 100, 12);
        let text = text_of(&rows);
        assert!(text.contains("under a minute"), "{text}");
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
                let rows = lines(&work, width, height, &theme(), IconSet::Nerd, None, now());
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
                    IconSet::Nerd,
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
        let (rows, omitted, _) = layout(
            &groups,
            5,
            100,
            age_width(&groups),
            IconSet::Nerd,
            &theme(),
            None,
            0,
        );
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

    #[test]
    fn a_narrow_announcement_keeps_the_reason_slot_and_the_strong_count() {
        // A narrow announcement keeps each piece's paint: the reason stays in
        // its slot and the count stays `text.strong`. Read the buffer, not the
        // model: the model was right when a cut repainted the whole row dim.
        let theme = theme();
        let human = theme.semantic.human.fg.expect("the human colour");
        let strong = theme.text.strong.fg.expect("the strong colour");
        for (width, height) in [(13u16, 12u16), (16, 12), (18, 12), (19, 7), (24, 7)] {
            let cells = draw_cells(&mock_work(), width, height);
            let row = cells
                .iter()
                .find(|row| {
                    let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
                    // The announcement's shape, not the header's: a reason
                    // beside a count and no `·`.
                    text.contains("approve")
                        && !text.contains('·')
                        && text.chars().any(|symbol| symbol.is_ascii_digit())
                })
                .unwrap_or_else(|| panic!("{width}x{height}: no approve announcement"));
            let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
            let at = text.find("approve").expect("the reason");
            let start = text[..at].chars().count();
            assert_eq!(
                row[start].1, human,
                "{width}x{height}: approve lost its slot: {text:?}"
            );
            let digit = text
                .char_indices()
                .rev()
                .find(|(_, symbol)| symbol.is_ascii_digit())
                .map(|(byte, _)| text[..byte].chars().count())
                .expect("a count");
            assert_eq!(
                row[digit].1, strong,
                "{width}x{height}: the approve count is not strong: {text:?}"
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

    #[test]
    fn an_empty_first_group_keeps_its_header_in_a_short_pane() {
        // No task waits for approval, so the approval group leads the tab
        // empty. Its header is drawn, and it is not said to be hidden.
        let work = work_of(
            (1..=8)
                .map(|n| waiter(&format!("TSK{n:02}"), coding_crew_core::Status::CodeReview))
                .collect(),
        );
        for height in [5u16, 6, 7, 8] {
            let rows = draw(&work, 60, height);
            let text = rows.join("\n");
            assert!(
                rows.iter()
                    .any(|row| row.contains("approve") && row.contains("· 0")),
                "{height}: {text}"
            );
            // The announcement names the group with none of its tasks hidden.
            assert!(text.contains("not shown: approve 0,"), "{height}: {text}");
        }
    }

    #[test]
    fn the_narrowest_row_shows_its_id_and_not_the_indent() {
        // From five to eight columns of content nothing but the id fits. The
        // indent gives way first, so the row is its id, not a run of blanks:
        // the selection bar's column is the one blank that stays, and it goes
        // when the id needs it. The pane's border takes two columns of the
        // width.
        for width in 7..=10u16 {
            let rows = inner(&draw(&mock_work(), width, 40));
            let row = rows
                .iter()
                .find(|row| row.contains("TSK21"))
                .unwrap_or_else(|| panic!("no row for TSK21: {rows:?}"));
            assert_eq!(row.trim(), "TSK21", "{width}: {row:?}");
            let blanks = row.len() - row.trim_start().len();
            assert!(blanks <= 1, "{width}: {row:?}");
        }
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
        let built = lines(&mock_work(), 58, 38, &theme(), IconSet::Nerd, None, now());
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
        let counts = work.needs_you_counts();
        for (width, height) in [(40u16, 12), (60, 12), (100, 12), (100, 6)] {
            let text = text_of(&draw(&work, width, height));
            for (reason, count) in [
                ("approve", counts.0),
                ("review", counts.1),
                ("escalated", counts.2),
            ] {
                let found = text
                    .lines()
                    .any(|line| line.contains(reason) && line.contains(&count.to_string()));
                assert!(found, "{width}x{height}: {reason} has no count: {text}");
            }
        }
    }

    #[test]
    fn a_row_is_painted_with_the_colour_of_its_reason() {
        // The human lane is one colour; an escalation is a failure, and is
        // painted like one. The reason's word takes that colour, and the id is
        // strong.
        let theme = theme();
        let human = theme.semantic.human.fg.expect("the human colour");
        let error = theme.semantic.error.fg.expect("the error colour");
        let cells = draw_cells(&mock_work(), 100, 40);
        assert_eq!(
            colour_in(&cells, "TSK04", "approve"),
            Some(human),
            "an approval"
        );
        assert_eq!(
            colour_in(&cells, "TSK13", "review"),
            Some(human),
            "a review"
        );
        assert_eq!(
            colour_in(&cells, "TSK19", "escalated"),
            Some(error),
            "an escalation"
        );
        assert_eq!(
            colour_in(&cells, "TSK19", "TSK19"),
            theme.text.strong.fg,
            "the id"
        );
    }

    #[test]
    fn a_selected_row_takes_the_selection_slot_and_the_others_keep_their_reason() {
        let theme = theme();
        let selection = theme.text.selected;
        let groups = groups(&human_lane(), "2026-11-07T16:00:00Z");
        let approve = &groups[0].rows[0];
        let escalated = &groups[2].rows[0];
        // A row that is not the cursor's keeps the slot of its reason on its
        // icon, after the bar's column and the space that leads the row.
        let plain = row_line(approve, 60, 2, IconSet::Nerd, &theme, None);
        assert_eq!(plain.spans[0].content, " ", "the bar's column is blank");
        assert_eq!(plain.spans[2].style, theme.semantic.human, "an approval");
        let plain = row_line(escalated, 60, 2, IconSet::Nerd, &theme, None);
        assert_eq!(plain.spans[2].style, theme.semantic.error, "an escalation");
        // The cursor's row takes the selection's pair on every cell after its
        // bar, and the bar runs to the edge of the pane.
        let selected = row_line(approve, 60, 2, IconSet::Nerd, &theme, Some("TSK01"));
        assert_eq!(selected.width(), 60, "the selection bar stops short");
        assert_eq!(selected.spans[0].content, "▌");
        assert_eq!(
            selected.spans[0].style, theme.chrome.border_focus,
            "the bar"
        );
        for span in &selected.spans[1..] {
            assert_eq!(span.style, selection, "{:?} is not the selection", span);
        }
        // A later row is not painted as the selection.
        let other = row_line(escalated, 60, 2, IconSet::Nerd, &theme, Some("TSK01"));
        assert_eq!(other.spans[2].style, theme.semantic.error);
    }

    #[test]
    fn a_row_leads_with_the_icon_of_the_status_it_waits_in() {
        let groups = groups(&human_lane(), "2026-11-07T16:00:00Z");
        let theme = theme();
        let cases = [
            (&groups[0].rows[0], Icon::StatusGroomed),
            (&groups[1].rows[0], Icon::StatusReview),
            (&groups[2].rows[0], Icon::StatusNeedsHuman),
        ];
        for (row, icon) in cases {
            let line = row_line(row, 60, 2, IconSet::Nerd, &theme, None);
            // The bar, the space that leads the row, the icon, a space, the id.
            assert_eq!(
                line.spans[2].content,
                icon.glyph(IconSet::Nerd),
                "{}",
                row.id
            );
            assert_eq!(line.spans[3].content, " ", "{}", row.id);
            assert_eq!(
                line.spans[4].content,
                format!("{:<5}", row.id),
                "{}",
                row.id
            );
        }
    }

    #[test]
    fn an_age_is_led_by_the_clock() {
        let groups = groups(&human_lane(), "2026-11-07T16:00:00Z");
        let line = row_line(&groups[0].rows[0], 60, 2, IconSet::Nerd, &theme(), None);
        let text = line_text(&line);
        let clock = Icon::Clock.glyph(IconSet::Nerd);
        assert!(text.ends_with(&format!("{clock} 3h ")), "{text:?}");
    }

    #[test]
    fn the_status_icon_gives_way_before_the_age_does() {
        // Fourteen columns hold the id, the age and its clock, but not the
        // icon beside them: the icon is the first thing the row gives up.
        let groups = groups(&human_lane(), "2026-11-07T16:00:00Z");
        let row = &groups[0].rows[0];
        let theme = theme();
        let icon = Icon::StatusGroomed.glyph(IconSet::Nerd);
        let narrow = line_text(&row_line_plain(row, 14, 2, IconSet::Nerd, &theme));
        assert_eq!(narrow.chars().count(), 14, "{narrow:?}");
        let clock = Icon::Clock.glyph(IconSet::Nerd);
        assert!(
            narrow.contains("TSK01") && narrow.ends_with(&format!("{clock} 3h ")),
            "{narrow:?}"
        );
        assert!(!narrow.contains(icon), "{narrow:?}");
        let wide = line_text(&row_line_plain(row, 15, 2, IconSet::Nerd, &theme));
        assert!(wide.contains(icon), "{wide:?}");
    }

    #[test]
    fn the_selection_bar_gives_way_before_the_id_is_cut() {
        // Five columns hold the id and nothing else: the bar's column would
        // cut the id, so the row is drawn without it, still selected.
        let groups = groups(&human_lane(), "2026-11-07T16:00:00Z");
        let theme = theme();
        let line = row_line(
            &groups[0].rows[0],
            5,
            2,
            IconSet::Nerd,
            &theme,
            Some("TSK01"),
        );
        assert_eq!(line_text(&line), "TSK01");
        assert_eq!(line.spans[0].style, theme.text.selected);
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
                            IconSet::Nerd,
                            None,
                            &mut view,
                            now(),
                        );
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
    fn a_narrow_entry_keeps_the_reason_and_the_word_hidden() {
        // The count may give way; the reason that names the group may not,
        // and the word that says the count is a count of what is hidden
        // stays with it.
        for width in 6..=40 {
            for count in [0usize, 2, 3] {
                let entry = entry(Reason::Approve, count, width);
                for row in &entry.rows {
                    assert!(row.width() <= width, "{width}: {:?}", row.text());
                }
                let text = entry
                    .rows
                    .iter()
                    .map(Said::text)
                    .collect::<Vec<String>>()
                    .join(" ");
                assert!(text.contains("hidden"), "{width}: {count}: {text}");
                assert!(!text.contains(" · "), "{width}: {text}");
                // The reason is never replaced by the count: it heads the
                // entry, whole while it fits and cut only when it cannot.
                let head = entry.rows[0].text();
                assert!(
                    head.starts_with("approve")
                        || "approve".starts_with(head.trim_end_matches('…')),
                    "{width}: {head:?} is not the reason"
                );
            }
        }
    }

    #[test]
    fn a_reason_is_never_replaced_by_a_bare_count() {
        // At the narrowest content widths the compact shape cuts a reason to
        // the marker. A row that shows a count without its reason is a bare
        // count: the announcement never says one.
        for width in 1..=11 {
            for room in [1usize, 2, 3, 6, 9] {
                let (rows, _) = announcement(&[2, 3, 3], width, room, &theme());
                for row in &rows {
                    let text = row
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>();
                    assert!(
                        !text.trim().chars().all(|symbol| symbol.is_ascii_digit()),
                        "{width}/{room}: a bare count: {text:?}"
                    );
                }
                if width == 1 {
                    let text = said_text(&rows);
                    assert!(
                        text.contains('…'),
                        "{width}/{room}: the marker is missing: {text}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_narrow_announcement_names_every_reason() {
        // At content widths 11..=16 the full `reason (count hidden)` does not
        // fit. The announcement keeps every reason, whole, with its count:
        // the reason is what the row is and is never cut away.
        for width in 11..=16 {
            for room in [3usize, 6, 9] {
                let (rows, _) = announcement(&[2, 3, 3], width, room, &theme());
                let text = said_text(&rows);
                for reason in ["approve", "review", "escalated"] {
                    assert!(
                        text.contains(reason),
                        "{width}/{room}: {reason} is missing: {text}"
                    );
                }
                for count in ["2", "3"] {
                    assert!(text.contains(count), "{width}/{room}: {count}: {text}");
                }
            }
        }
        // And on the rendered tab: every reason is named in the trailing
        // announcement block, not only in the headers above it.
        let work = human_lane();
        for width in 13..=18u16 {
            let text = text_of(&draw(&work, width, 12));
            let lines: Vec<&str> = text.lines().collect();
            let start = lines
                .iter()
                .position(|line| line.contains("not shown:") || line.contains("hidden"))
                .unwrap_or_else(|| panic!("{width}: no announcement: {text}"));
            let announcement = lines[start..].join("\n");
            for reason in ["approve", "review", "escalated"] {
                assert!(
                    announcement.contains(reason),
                    "{width}: the announcement drops {reason}: {text}"
                );
            }
        }
    }

    /// Whether `prefix` is a whole number of graphemes at the start of
    /// `text`, by the same grapheme iterator ratatui draws with.
    fn is_grapheme_prefix(prefix: &str, text: &str) -> bool {
        let span = Span::raw(text.to_string());
        let mut built = String::new();
        if built == prefix {
            return true;
        }
        for grapheme in span.styled_graphemes(Style::default()) {
            built.push_str(grapheme.symbol);
            if built == prefix {
                return true;
            }
        }
        built == prefix
    }

    #[test]
    fn a_title_with_a_combining_mark_is_cut_on_a_grapheme_boundary() {
        // A scalar cut can keep the `e` and drop the combining mark; the
        // shared helper cuts the grapheme whole, as the detail pane does.
        let title = "e\u{0301}".repeat(20);
        let row = Row {
            id: "TSK01".to_string(),
            title: title.clone(),
            reason: Reason::Approve,
            // A two-day wait: the age column below is two columns wide.
            waited: 2 * 86_400,
            review: None,
        };
        // At this width the title column is eighteen columns wide.
        let line = row_line_plain(&row, 35, 2, IconSet::Nerd, &theme());
        let cut = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .find(|text| text.contains('\u{0301}'))
            .expect("the title");
        assert!(cut.ends_with('…'), "{line:?}");
        let head = cut.strip_suffix('…').expect("a cut title");
        assert!(is_grapheme_prefix(head, &title), "{head:?} cuts a grapheme");
        assert_eq!(head, "e\u{0301}".repeat(17));
    }

    /// A task with the review facts given: external rounds, and findings of a
    /// severity and a status.
    fn reviewed_task(rounds: usize, findings: &[(Severity, FindingStatus)]) -> Task {
        let mut task = Task::new("TSK01", "a task");
        task.status = coding_crew_core::Status::CodeReview;
        task.status_since = "2026-11-07T13:00:00Z".to_string();
        for n in 1..=rounds {
            task.review.rounds.push(Round {
                n: n as u32,
                kind: RoundKind::External,
                ..Round::default()
            });
        }
        for (index, (severity, status)) in findings.iter().enumerate() {
            task.review.findings.push(Finding {
                id: format!("FIND-{index:02}"),
                severity: *severity,
                status: *status,
                ..Finding::default()
            });
        }
        task
    }

    /// The text of a line.
    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn a_row_without_a_review_state_is_the_row_it_was() {
        // The pin the unit must not move: a row with no state is drawn with
        // the columns and the cut it had before the state existed.
        let row = Row {
            id: "TSK01".to_string(),
            title: "a task".to_string(),
            reason: Reason::Approve,
            waited: 2 * 86_400,
            review: None,
        };
        let line = row_line_plain(&row, 60, 2, IconSet::Nerd, &theme());
        let expected = format!(
            " {} {:<5}  {:<9}  {:<32}  {} {:<2} ",
            Icon::StatusGroomed.glyph(IconSet::Nerd),
            "TSK01",
            "approve",
            "a task",
            Icon::Clock.glyph(IconSet::Nerd),
            "2d"
        );
        assert_eq!(line_text(&line), expected);
    }

    #[test]
    fn the_rows_carry_the_review_state_from_the_record() {
        let work = mock_work();
        let all: Vec<Row> = groups(&work, now())
            .into_iter()
            .flat_map(|group| group.rows)
            .collect();
        let row = |id: &str| {
            all.iter()
                .find(|row| row.id == id)
                .unwrap_or_else(|| panic!("no row for {id}"))
        };
        assert!(row("TSK04").review.is_none(), "an approval has no state");
        assert_eq!(
            review_state::text(row("TSK21").review.as_deref().expect("a state")),
            "5 open findings (2 high, 2 medium, 1 low) · 5/5 rounds"
        );
        assert_eq!(
            review_state::text(row("TSK13").review.as_deref().expect("a state")),
            "1 open finding (1 medium)"
        );
    }

    #[test]
    fn a_row_says_the_review_state_before_the_age() {
        let work = mock_work();
        let rows = inner(&draw(&work, 100, 40));
        let row = rows
            .iter()
            .find(|row| row.contains("TSK21"))
            .unwrap_or_else(|| panic!("no row for TSK21: {rows:?}"));
        let icon = Icon::StatusNeedsHuman.glyph(IconSet::Nerd);
        assert!(
            row.starts_with(&format!("  {icon} TSK21  escalated  ")),
            "{row}"
        );
        assert!(
            row.contains("5 open findings (2 high, 2 medium, 1 low) · 5/5 rounds"),
            "{row}"
        );
        // The age stays right-aligned at the end of the row.
        let task = work.task("TSK21").expect("the task");
        let age = age_text(waited_since(task, now()));
        assert!(row.ends_with(&age), "{row} does not end in {age:?}");
    }

    #[test]
    fn the_clock_gives_way_before_the_state_does() {
        // A hundred columns hold the escalated row's whole state only when its
        // age drops the clock; a row with room to spare keeps the clock.
        let rows = inner(&draw(&mock_work(), 100, 40));
        let clock = Icon::Clock.glyph(IconSet::Nerd);
        let row = rows
            .iter()
            .find(|row| row.contains("TSK21"))
            .unwrap_or_else(|| panic!("no row for TSK21: {rows:?}"));
        assert!(row.contains("5/5 rounds"), "{row}");
        assert!(!row.contains(clock), "{row}");
        let row = rows
            .iter()
            .find(|row| row.contains("TSK13"))
            .unwrap_or_else(|| panic!("no row for TSK13: {rows:?}"));
        assert!(row.contains(clock), "{row}");
    }

    #[test]
    fn a_row_with_nothing_to_report_carries_no_suffix() {
        let work = mock_work();
        let rows = inner(&draw(&work, 100, 40));
        let row = rows
            .iter()
            .find(|row| row.contains("TSK04"))
            .unwrap_or_else(|| panic!("no row for TSK04: {rows:?}"));
        for word in ["rounds", "finding", "open"] {
            assert!(!row.contains(word), "TSK04 carries {word}: {row}");
        }
        let task = work.task("TSK04").expect("the task");
        let age = age_text(waited_since(task, now()));
        assert!(row.ends_with(&age), "{row} does not end in {age:?}");
    }

    #[test]
    fn a_narrow_row_gives_the_state_up_before_the_title() {
        // Forty columns leave the title its comfortable width and no more:
        // the state is dropped, and the row is the row it was.
        let rows = inner(&draw(&mock_work(), 40, 40));
        let row = rows
            .iter()
            .find(|row| row.contains("TSK21"))
            .unwrap_or_else(|| panic!("no row for TSK21: {rows:?}"));
        assert!(!row.contains("rounds"), "{row}");
        assert!(!row.contains("finding"), "{row}");
        assert!(row.contains("TSK21"), "{row}");
    }

    #[test]
    fn a_cut_state_keeps_every_count_whole_and_says_it_is_cut() {
        // Sixty columns fit the findings count and no more: the state is cut
        // between whole parts and marked, never inside a count.
        let rows = inner(&draw(&mock_work(), 60, 40));
        let row = rows
            .iter()
            .find(|row| row.contains("TSK21"))
            .unwrap_or_else(|| panic!("no row for TSK21: {rows:?}"));
        assert!(row.contains("5 open findings…"), "{row}");
        assert!(!row.contains("(2 high"), "{row}");
        // A findings-only state is cut the same way.
        let row = rows
            .iter()
            .find(|row| row.contains("TSK13"))
            .unwrap_or_else(|| panic!("no row for TSK13: {rows:?}"));
        assert!(row.contains("1 open finding…"), "{row}");
        assert!(!row.contains("1 medium"), "{row}");
    }

    #[test]
    fn the_state_counts_are_strong_and_its_words_dim() {
        let theme = theme();
        let strong = theme.text.strong.fg.expect("the strong colour");
        let dim = theme.text.dim.fg.expect("the dim colour");
        let cells = draw_cells(&mock_work(), 100, 40);
        assert_eq!(colour_of(&cells, "5/5 rounds"), Some(strong), "a count");
        assert_eq!(colour_of(&cells, "rounds"), Some(dim), "a word");
    }

    #[test]
    fn a_selected_row_takes_the_selection_paint_over_its_state() {
        let theme = theme();
        let selection = theme.text.selected;
        let task = reviewed_task(
            5,
            &[
                (Severity::High, FindingStatus::Open),
                (Severity::Medium, FindingStatus::Open),
            ],
        );
        let row = Row {
            id: task.id.clone(),
            title: task.title.clone(),
            reason: Reason::Review,
            waited: 2 * 86_400,
            review: review_state::state(&task),
        };
        let line = row_line(&row, 60, 2, IconSet::Nerd, &theme, Some("TSK01"));
        assert_eq!(line.width(), 60, "the selection bar stops short");
        assert_eq!(line.spans[0].style, theme.chrome.border_focus, "the bar");
        for span in &line.spans[1..] {
            assert_eq!(span.style, selection, "{:?} is not the selection", span);
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
