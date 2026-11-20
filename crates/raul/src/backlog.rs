//! The Backlog tab: what is not a task yet.
//!
//! The smaller tasks and the draft tasks, in one list. The groups are
//! derived from the model — ideas, then small tasks, then drafts — and are
//! never stored.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use coding_crew_core::{Item, ItemKind, ItemStatus, Status, Task};

use crate::data::Work;
use crate::view::View;
use coding_crew_theme::text::{cut_to, pad, text_width};
use coding_crew_theme::{Theme, block_title_style, phase_style, selected_line};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between two columns.
const GAP: usize = 2;

/// A group of the backlog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    /// Ideas: a hunch, not work yet.
    Ideas,
    /// Small tasks: a feature or a bug.
    SmallTasks,
    /// Draft tasks: a task that has not been groomed.
    Drafts,
}

impl GroupKind {
    /// Every group, in the order of the tab.
    pub const ALL: [GroupKind; 3] = [GroupKind::Ideas, GroupKind::SmallTasks, GroupKind::Drafts];

    /// The heading of the group.
    pub fn label(&self) -> &'static str {
        match self {
            GroupKind::Ideas => "IDEAS",
            GroupKind::SmallTasks => "SMALL TASKS",
            GroupKind::Drafts => "DRAFTS",
        }
    }
}

/// A row of the backlog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The id: `ITM01` or `TSK01`.
    pub id: String,
    /// What it is: `idea`, `feature`, `bug` or `draft`.
    pub kind: String,
    /// The title.
    pub title: String,
    /// The work, when it is known.
    pub scope: Option<String>,
    /// `open`, `dismissed`, `promoted` or `draft`.
    pub state: String,
    /// A draft or a settled item: the row may be dimmed.
    pub settled: bool,
}

/// A group of rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Which group it is.
    pub kind: GroupKind,
    /// The rows, sorted by id.
    pub rows: Vec<Row>,
}

/// The ids of the tab's rows, in the order they are shown.
///
/// The detail pane opens from this tab: the rows are what it can open.
pub fn row_ids(work: &Work) -> Vec<String> {
    groups(work)
        .into_iter()
        .flat_map(|group| group.rows.into_iter().map(|row| row.id))
        .collect()
}

/// The groups of the backlog, in the order of the tab.
///
/// The groups are always there, so a count of zero is a fact too. The rows
/// are derived from the model: the item's kind, and the task's status.
pub fn groups(work: &Work) -> [Group; 3] {
    let mut ideas = Vec::new();
    let mut small_tasks = Vec::new();
    let mut drafts = Vec::new();
    for item in &work.items {
        let row = item_row(item);
        match item.kind {
            ItemKind::Idea => ideas.push(row),
            ItemKind::Feature | ItemKind::Bug => small_tasks.push(row),
        }
    }
    for task in &work.tasks {
        if task.status == Status::Draft {
            drafts.push(task_row(task));
        }
    }
    let mut groups = [
        Group {
            kind: GroupKind::Ideas,
            rows: ideas,
        },
        Group {
            kind: GroupKind::SmallTasks,
            rows: small_tasks,
        },
        Group {
            kind: GroupKind::Drafts,
            rows: drafts,
        },
    ];
    for group in &mut groups {
        group.rows.sort_by(|one, other| one.id.cmp(&other.id));
    }
    groups
}

/// A smaller task, as the backlog holds it.
fn item_row(item: &Item) -> Row {
    Row {
        id: item.id.clone(),
        kind: item.kind.as_str().to_string(),
        title: item.title.clone(),
        scope: scope_text(&item.scope),
        state: item.status.as_str().to_string(),
        settled: item.status != ItemStatus::Open,
    }
}

/// A draft task, as the backlog holds it.
fn task_row(task: &Task) -> Row {
    Row {
        id: task.id.clone(),
        kind: "draft".to_string(),
        title: task.title.clone(),
        scope: scope_text(&task.scope.in_scope),
        state: Status::Draft.as_str().to_string(),
        settled: true,
    }
}

/// The scope, when it is known.
fn scope_text(scope: &[String]) -> Option<String> {
    let known: Vec<&str> = scope
        .iter()
        .map(String::as_str)
        .filter(|part| !part.trim().is_empty())
        .collect();
    (!known.is_empty()).then(|| known.join(", "))
}

/// Draw the tab at the top of its rows.
#[cfg(test)]
pub fn render(frame: &mut Frame, area: Rect, work: &Work, theme: &Theme, selected: Option<&str>) {
    let mut view = View::default();
    render_view(frame, area, work, theme, selected, &mut view);
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
) -> Option<usize> {
    let block = Block::bordered()
        .title("backlog")
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
        return None;
    }
    let groups = groups(work);
    if groups.iter().all(|group| group.rows.is_empty()) {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no backlog yet", theme.text.dim))).block(block),
            area,
        );
        return None;
    }
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let ids = row_ids(work);
    let (rows, announced) = view.draw(&ids, selected, |first| {
        let (lines, announced, shown, whole) =
            window(&groups, width, height, theme, selected, first);
        ((lines, announced), shown, whole)
    });
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }).block(block),
        area,
    );
    let hidden = ids.len().saturating_sub(view.page());
    (hidden > 0 && !announced).then_some(hidden)
}

/// The columns of a row: the fixed ones, and the two that give way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    id: bool,
    kind: bool,
    state: bool,
    scope: bool,
}

/// The candidates, in the order the columns give way.
///
/// The title and the scope are what the reader is here for: the id, the
/// kind and the state give way first, whole.
const CANDIDATES: [Columns; 5] = [
    Columns {
        id: true,
        kind: true,
        state: true,
        scope: true,
    },
    Columns {
        id: true,
        kind: false,
        state: true,
        scope: true,
    },
    Columns {
        id: true,
        kind: false,
        state: false,
        scope: true,
    },
    Columns {
        id: false,
        kind: false,
        state: false,
        scope: true,
    },
    Columns {
        id: false,
        kind: false,
        state: false,
        scope: false,
    },
];

/// The width of the id column: the widest id.
fn id_width(groups: &[Group]) -> usize {
    groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .map(|row| text_width(&row.id))
        .max()
        .unwrap_or(0)
}

/// The width of the kind column: the widest kind.
fn kind_width(groups: &[Group]) -> usize {
    groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .map(|row| text_width(&row.kind))
        .max()
        .unwrap_or(0)
}

/// The width of the state column: the widest state.
fn state_width(groups: &[Group]) -> usize {
    groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .map(|row| text_width(&row.state))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
fn lines(
    groups: &[Group],
    width: usize,
    height: usize,
    theme: &Theme,
    selected: Option<&str>,
) -> Vec<Line<'static>> {
    window(groups, width, height, theme, selected, 0).0
}

/// The lines of the tab, from the row at `first`, and the rows they show.
///
/// Everything fits at the default size, and there is nothing to announce.
/// On a short pane the announcement comes first: a row or a header is only
/// left out when the lines that say so are on screen. A group is never
/// silently absent — it is either shown, header and rows, or named in the
/// announcement with the number of its rows that are not in view, including
/// the rows above the window.
///
/// A pane too short for the announcement and the cursor's row draws the row
/// alone: the selection is never on nothing. The returned flag says whether
/// the pane painted the announcement itself; when it did not, the window's
/// caller carries the hidden count to the footer.
fn window(
    groups: &[Group],
    width: usize,
    height: usize,
    theme: &Theme,
    selected: Option<&str>,
    first: usize,
) -> (Vec<Line<'static>>, bool, Vec<usize>, bool) {
    if height == 0 {
        return (Vec::new(), false, Vec::new(), false);
    }
    let metrics = Metrics {
        id: id_width(groups),
        kind: kind_width(groups),
        state: state_width(groups),
    };
    let total: usize = groups.iter().map(|group| group.rows.len()).sum();
    let wanted: usize = groups.iter().map(|group| 1 + group.rows.len()).sum();
    if first == 0 && wanted <= height {
        return (
            all_lines(groups, width, metrics, theme, selected),
            false,
            (0..total).collect(),
            true,
        );
    }
    // Some line does not fit. The rows give way to the announcement: the
    // first layout whose whole announcement fits wins, and the rows take
    // what is left. When even the shortest announcement fills the pane,
    // the announcement alone is shown, so no omission goes unsaid. The
    // cursor's row is the one row the window may not leave out: a cut
    // layout that shows it wins over the shortest one.
    let mut shortest: Option<(Vec<Line<'static>>, Vec<Line<'static>>, Vec<usize>)> = None;
    let mut with_cursor: Option<(Vec<Line<'static>>, Vec<Line<'static>>, Vec<usize>)> = None;
    for capacity in (0..=height).rev() {
        let (rows, omitted, headers, shown) =
            layout_groups(groups, capacity, width, metrics, theme, selected, first);
        let room = height.saturating_sub(rows.len());
        let present = present_groups(groups, &shown);
        let (said, cut) = announcement(&omitted, &headers, &present, width, room, theme);
        let has_cursor =
            selected.is_none_or(|id| shown.iter().any(|index| row_id(groups, *index) == id));
        if cut {
            if selected.is_some() && has_cursor {
                // The announcement comes first: a cut layout that says more
                // (and draws fewer rows) is the better cut.
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
            continue;
        }
        let mut out = rows;
        let announced = !said.is_empty();
        out.extend(said);
        return (out, announced, shown, true);
    }
    if let Some((mut out, said, shown)) = with_cursor.or(shortest) {
        if let Some(id) = selected
            && !shown.iter().any(|index| row_id(groups, *index) == id)
            && let Some((index, line)) = cursor_row(groups, width, metrics, theme, id)
        {
            // The pane is too short for the cursor's row and the layout: the
            // row is drawn, and what is left says what the pane hides. Only a
            // pane of one line shows the row alone.
            let mut rows = vec![line];
            let room = height - rows.len();
            if room > 0 {
                let omitted = omitted_with_only(groups, index);
                let present = present_with_only(groups, index);
                let headers = [false; 3];
                let (word, _) = announcement(&omitted, &headers, &present, width, room, theme);
                rows.extend(word);
            }
            let announced = rows.len() > 1;
            return (rows, announced, vec![index], false);
        }
        if said.is_empty()
            && height >= 2
            && let Some(id) = selected
            && let Some(cursor) = (0..total).find(|at| row_id(groups, *at) == id)
        {
            // The rows took every line and said nothing about what they hid.
            // The header gives way: the cursor's row is drawn, and the rest of
            // the pane names the groups and what they hide.
            let omitted = omitted_with_only(groups, cursor);
            let present = present_with_only(groups, cursor);
            let headers = [false; 3];
            let (word, _) = announcement(&omitted, &headers, &present, width, height - 1, theme);
            if !word.is_empty()
                && let Some((_, line)) = cursor_row(groups, width, metrics, theme, id)
            {
                let mut rows = vec![line];
                rows.extend(word);
                return (rows, true, vec![cursor], false);
            }
        }
        let announced = !said.is_empty();
        out.extend(said);
        return (out, announced, shown, false);
    }
    // No layout was chosen: the pane is too narrow to draw a header and too
    // short to say anything. The cursor's row is still drawn, alone.
    if let Some(id) = selected
        && let Some((index, line)) = cursor_row(groups, width, metrics, theme, id)
    {
        return (vec![line], false, vec![index], false);
    }
    (Vec::new(), false, Vec::new(), false)
}

/// What each group hides when only the row at `cursor` is shown.
fn omitted_with_only(groups: &[Group], cursor: usize) -> [usize; 3] {
    let mut omitted = [0usize; 3];
    let mut at = 0usize;
    for (index, group) in groups.iter().enumerate() {
        omitted[index] = group.rows.len();
        if (at..at + group.rows.len()).contains(&cursor) {
            omitted[index] = group.rows.len().saturating_sub(1);
        }
        at += group.rows.len();
    }
    omitted
}

/// Which groups have a row on screen.
fn present_groups(groups: &[Group], shown: &[usize]) -> [bool; 3] {
    let mut present = [false; 3];
    for index in shown {
        let mut at = *index;
        for (group, rows) in groups.iter().enumerate() {
            if at < rows.rows.len() {
                present[group] = true;
                break;
            }
            at -= rows.rows.len();
        }
    }
    present
}

/// Which group is on screen when only the row at `cursor` is shown.
fn present_with_only(groups: &[Group], cursor: usize) -> [bool; 3] {
    let mut present = [false; 3];
    let mut at = 0usize;
    for (index, group) in groups.iter().enumerate() {
        if (at..at + group.rows.len()).contains(&cursor) {
            present[index] = true;
            break;
        }
        at += group.rows.len();
    }
    present
}

/// The line of the cursor's row, and its place among the tab's rows.
fn cursor_row(
    groups: &[Group],
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    id: &str,
) -> Option<(usize, Line<'static>)> {
    let mut index = 0;
    for group in groups {
        for row in &group.rows {
            if row.id == id {
                return Some((index, row_line(row, width, metrics, theme, Some(id))));
            }
            index += 1;
        }
    }
    None
}

/// The id of the row at an index of the tab's rows.
fn row_id(groups: &[Group], index: usize) -> &str {
    let mut at = index;
    for group in groups {
        if at < group.rows.len() {
            return &group.rows[at].id;
        }
        at -= group.rows.len();
    }
    ""
}

/// Every group, header and rows, in the order of the tab.
fn all_lines(
    groups: &[Group],
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    selected: Option<&str>,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for group in groups {
        out.push(header(group, width, theme));
        for row in &group.rows {
            out.push(row_line(row, width, metrics, theme, selected));
        }
    }
    out
}

/// The rows of the tab that fit in `capacity` lines, how many rows of each
/// group are left out of them, which headers are shown, and the rows drawn.
///
/// The groups keep their order: the header of a group, then its rows. A
/// header's count is the whole group; the omitted count is what the
/// announcement says instead, and the two are meant to differ. A group
/// whose header does not fit is not shown at all, every one of its rows is
/// left out of view, and the announcement says the header itself was hidden
/// even when the group holds no rows.
fn layout_groups(
    groups: &[Group],
    capacity: usize,
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    selected: Option<&str>,
    first: usize,
) -> (Vec<Line<'static>>, [usize; 3], [bool; 3], Vec<usize>) {
    let mut out = Vec::new();
    let mut omitted = [0usize; 3];
    let mut headers = [false; 3];
    let mut shown_rows = Vec::new();
    let mut used = 0usize;
    let mut row_index = 0usize;
    for (index, group) in groups.iter().enumerate() {
        let start = row_index;
        row_index += group.rows.len();
        if row_index <= first {
            // The whole group is above the window: its header is hidden too.
            omitted[index] = group.rows.len();
            continue;
        }
        if used + 1 > capacity {
            // The header does not fit. The cursor's row still may: the row
            // the enter key opens is worth more than the header above it,
            // and the announcement names the group the header would have.
            if let Some(cursor) =
                selected.and_then(|id| group.rows.iter().position(|row| row.id == id))
                && used < capacity
            {
                let row_index = start + cursor;
                if row_index >= first {
                    out.push(row_line(
                        &group.rows[cursor],
                        width,
                        metrics,
                        theme,
                        selected,
                    ));
                    shown_rows.push(row_index);
                    omitted[index] = group.rows.len() - 1;
                    used += 1;
                    continue;
                }
            }
            omitted[index] = group.rows.len();
            continue;
        }
        out.push(header(group, width, theme));
        headers[index] = true;
        used += 1;
        let skip = first.saturating_sub(start).min(group.rows.len());
        let visible = group.rows.len() - skip;
        let shown = (capacity - used).min(visible);
        for (offset, row) in group.rows.iter().skip(skip).take(shown).enumerate() {
            out.push(row_line(row, width, metrics, theme, selected));
            shown_rows.push(start + skip + offset);
        }
        used += shown;
        if shown < group.rows.len() {
            omitted[index] = group.rows.len() - shown;
        }
    }
    (out, omitted, headers, shown_rows)
}

/// The widths of the fixed columns.
#[derive(Debug, Clone, Copy)]
struct Metrics {
    id: usize,
    kind: usize,
    state: usize,
}

/// How a row is laid out in the pane.
#[derive(Debug, Clone, Copy)]
struct Layout {
    /// The columns that are shown.
    columns: Columns,
    /// The width of the title.
    title: usize,
    /// The width of the scope.
    scope: usize,
}

/// The layout of a row: the first candidate that leaves the fixed columns
/// whole and room for the title and the scope.
fn layout(row: &Row, width: usize, metrics: Metrics) -> Layout {
    let title = text_width(&row.title);
    let scope = row.scope.as_deref().map(text_width).unwrap_or(0);
    for mut columns in CANDIDATES {
        if row.scope.is_none() {
            columns.scope = false;
        }
        let (fixed, gaps) = fixed_width(columns, metrics);
        let flex = 1 + usize::from(columns.scope);
        if fixed + gaps + flex > width {
            continue;
        }
        let available = width - fixed - gaps;
        // A fixed column is kept only when the title and the scope still
        // fit whole: otherwise it gives way, so they are not cut to make
        // room for it.
        let has_fixed = columns.id || columns.kind || columns.state;
        if has_fixed && title + scope > available {
            continue;
        }
        let (title_width, scope_width) = flex_widths(title, scope, available, columns.scope);
        return Layout {
            columns,
            title: title_width,
            scope: scope_width,
        };
    }
    Layout {
        columns: Columns {
            id: false,
            kind: false,
            state: false,
            scope: false,
        },
        title: width,
        scope: 0,
    }
}

/// The width of the fixed columns, and the gaps between every column.
fn fixed_width(columns: Columns, metrics: Metrics) -> (usize, usize) {
    let mut fixed = 0;
    let mut count: usize = 1; // the title is always there
    if columns.id {
        fixed += metrics.id;
        count += 1;
    }
    if columns.kind {
        fixed += metrics.kind;
        count += 1;
    }
    if columns.state {
        fixed += metrics.state;
        count += 1;
    }
    if columns.scope {
        count += 1;
    }
    (fixed, GAP * count.saturating_sub(1))
}

/// The width of the title and the scope in the room left.
///
/// Both keep room: the scope takes at most half, and the title takes the
/// rest. A column that does not fit is cut with an ellipsis; the fixed
/// columns are never cut.
fn flex_widths(title: usize, scope: usize, available: usize, has_scope: bool) -> (usize, usize) {
    if !has_scope {
        return (available.min(title), 0);
    }
    if title + scope <= available {
        return (title, scope);
    }
    let mut scope_width = scope.min(available / 2);
    let mut title_width = title.min(available - scope_width);
    let leftover = available - title_width - scope_width;
    // The title is first in the reading order: it takes the leftover first,
    // and the scope gets what remains of its own text.
    let to_title = leftover.min(title - title_width);
    title_width += to_title;
    let leftover = leftover - to_title;
    scope_width += leftover.min(scope - scope_width);
    (title_width, scope_width)
}

/// A row: the id, the kind, the title, the scope and the state.
fn row_line(
    row: &Row,
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    let layout = layout(row, width, metrics);
    let row_style = if row.settled {
        theme.text.dim
    } else {
        theme.text.normal
    };
    let title_style = theme.text.strong;
    let mut spans: Vec<Span<'static>> = Vec::new();
    if layout.columns.id {
        spans.push(Span::styled(pad(&row.id, metrics.id), row_style));
    }
    if layout.columns.kind {
        push_gap(&mut spans);
        spans.push(Span::styled(pad(&row.kind, metrics.kind), row_style));
    }
    push_gap(&mut spans);
    spans.push(Span::styled(
        pad(&cut_to(&row.title, layout.title), layout.title),
        title_style,
    ));
    if layout.columns.scope
        && let Some(scope) = &row.scope
    {
        push_gap(&mut spans);
        spans.push(Span::styled(cut_to(scope, layout.scope), row_style));
    }
    if layout.columns.state {
        push_gap(&mut spans);
        spans.push(Span::styled(pad(&row.state, metrics.state), row_style));
    }
    // The row is padded as it is built; trim the trailing padding that
    // carries no information.
    trim_trailing(&mut spans);
    let mut line = Line::from(spans);
    // A row whose content is zero columns wide is not silently blank: the
    // marker says that something was dropped.
    if line.width() == 0 && width > 0 {
        line = Line::from(Span::styled(cut_to("…", width), theme.text.dim));
    }
    if selected == Some(row.id.as_str()) {
        return selected_line(line, width, theme);
    }
    line
}

/// Add the gap between two columns.
fn push_gap(spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        spans.push(Span::raw(" ".repeat(GAP)));
    }
}

/// Remove the trailing spaces of the last span.
fn trim_trailing(spans: &mut Vec<Span<'static>>) {
    let Some(last) = spans.last_mut() else {
        return;
    };
    let trimmed = last.content.trim_end_matches(' ').to_string();
    if trimmed != last.content.as_ref() {
        last.content = trimmed.into();
    }
    while spans.last().is_some_and(|span| span.content.is_empty()) {
        spans.pop();
    }
}

/// The style of a group's label.
///
/// A label carries the same fact as the rows under it: the drafts are the
/// `draft` phase, whose slot is `text.dim`, so the heading is dim too.
fn group_style(kind: GroupKind, theme: &Theme) -> Style {
    match kind {
        GroupKind::Ideas | GroupKind::SmallTasks => theme.text.normal,
        GroupKind::Drafts => phase_style(theme, Status::Draft),
    }
}

/// The header of a group: its name and the count of its rows.
///
/// The count is `text.strong`, as every count is. It is never cut; when the
/// pane is too narrow for the whole header, the name gives way.
fn header(group: &Group, width: usize, theme: &Theme) -> Line<'static> {
    let count = group.rows.len().to_string();
    let label = group.kind.label();
    let style = group_style(group.kind, theme);
    let separator = " · ";
    let fixed = text_width(separator) + text_width(&count);
    if width >= text_width(label) + fixed {
        return Line::from(vec![
            Span::styled(label.to_string(), style),
            Span::styled(separator.to_string(), theme.text.dim),
            Span::styled(count, theme.text.strong),
        ]);
    }
    if width > fixed {
        let room = width - fixed;
        return Line::from(vec![
            Span::styled(cut_to(label, room), style),
            Span::styled(separator.to_string(), theme.text.dim),
            Span::styled(count, theme.text.strong),
        ]);
    }
    if width >= text_width(&count) {
        return Line::from(Span::styled(count, theme.text.strong));
    }
    Line::from(Span::styled(cut_to(label, width), style))
}

/// How a piece of the announcement is painted.
#[derive(Debug, Clone, Copy)]
enum Paint {
    /// A group name: the same slot its header uses.
    Group(GroupKind),
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
        self.spans.iter().map(|(text, _)| text_width(text)).sum()
    }
}

/// The lines that say what is not shown.
///
/// The count here is the number of *rows left out of view*, not the group's
/// total — the header owns that — so an entry has a shape of its own: the
/// group, then the count in brackets with the word `hidden`. It never takes
/// the header's `group · total` shape, and every group is named, including
/// a group with nothing hidden.
///
/// A group whose header was dropped for room is announced as hidden even
/// when it holds no rows: its `0 hidden` shape would say the group was on
/// screen. A group that has a row on screen is not dropped: its entry says
/// zero are hidden rather than claiming the whole group is gone. When the
/// packed entries do not fit in `room`, the compact shape names every group
/// and count over the rows there are; when even that does not fit, the
/// entries give way whole from the last one and the last row is marked with
/// `…`. An entry is never cut in half: a name and its count stay together.
fn announcement(
    omitted: &[usize; 3],
    headers: &[bool; 3],
    present: &[bool; 3],
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    let entries: Vec<Said> = GroupKind::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            if omitted[index] == 0 && !headers[index] && !present[index] {
                dropped_header(*kind, width)
            } else {
                entry(*kind, omitted[index], width)
            }
        })
        .collect();
    let rows = pack(&entries, width);
    if rows.len() <= room {
        return (paint(rows, theme), false);
    }
    if let Some((mut rows, complete)) = compact(omitted, headers, present, width, room) {
        if complete {
            return (paint(rows, theme), false);
        }
        // The word or the lead-in did not fit: the row is not whole. The
        // marker says so when there is room for it beside the last entry;
        // every group is named, so no entry is dropped for the marker.
        if let Some(last) = rows.last_mut()
            && last.width() < width
        {
            last.spans.push(("…".to_string(), Paint::Dim));
        }
        return (paint(rows, theme), true);
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
/// A row is never cut: a group name and its count stay together, and so do
/// the rows of the compact shape. The flag says a row was marked.
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

/// A group and the number of its rows that are hidden.
///
/// The full entry is `label (count hidden)`. On a narrow row the label gives
/// way first, but the count and the word stay with it; only a row narrower
/// than `count hidden` loses the label, and only a row narrower than
/// `hidden` itself loses the count.
fn entry(kind: GroupKind, count: usize, width: usize) -> Said {
    let label = kind.label();
    let count = count.to_string();
    let full = format!("{label} ({count} hidden)");
    if text_width(&full) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group(kind)),
                (" (".to_string(), Paint::Dim),
                (count, Paint::Strong),
                (" hidden)".to_string(), Paint::Dim),
            ],
        };
    }
    // The parentheses are the first thing to give way: the whole label is
    // kept when it can be.
    let bare = format!("{label} {count} hidden");
    if text_width(&bare) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group(kind)),
                (" ".to_string(), Paint::Dim),
                (count, Paint::Strong),
                (" hidden".to_string(), Paint::Dim),
            ],
        };
    }
    let tail = format!(" {count} hidden");
    if text_width(&tail) < width {
        let room = width - text_width(&tail);
        return Said {
            spans: vec![
                (cut_to(label, room), Paint::Group(kind)),
                (" ".to_string(), Paint::Dim),
                (count, Paint::Strong),
                (" hidden".to_string(), Paint::Dim),
            ],
        };
    }
    let counted = format!("{count} hidden");
    if text_width(&counted) <= width {
        return Said {
            spans: vec![(count, Paint::Strong), (" hidden".to_string(), Paint::Dim)],
        };
    }
    Said::new(cut_to("hidden", width), Paint::Dim)
}

/// A group whose header was dropped for room.
///
/// It says the group is hidden rather than `0 hidden`: the group is not on
/// screen, even when it holds no rows.
fn dropped_header(kind: GroupKind, width: usize) -> Said {
    let label = kind.label();
    let full = format!("{label} (hidden)");
    if text_width(&full) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group(kind)),
                (" (hidden)".to_string(), Paint::Dim),
            ],
        };
    }
    let bare = format!("{label} hidden");
    if text_width(&bare) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group(kind)),
                (" hidden".to_string(), Paint::Dim),
            ],
        };
    }
    let tail = " hidden";
    if text_width(tail) < width {
        let room = width - text_width(tail);
        return Said {
            spans: vec![
                (cut_to(label, room), Paint::Group(kind)),
                (" hidden".to_string(), Paint::Dim),
            ],
        };
    }
    Said::new(cut_to("hidden", width), Paint::Dim)
}

/// The compact shape: every group and its count, packed over at most `room`
/// rows.
///
/// The shape is `group count, group count … hidden`: distinct from the
/// header's `group · total`, and it names the groups even when a row is too
/// narrow for `group (count hidden)`. The lead-in and the word are dropped
/// only when they do not fit; the flag says whether one of them is present.
fn compact(
    omitted: &[usize; 3],
    headers: &[bool; 3],
    present: &[bool; 3],
    width: usize,
    room: usize,
) -> Option<(Vec<Said>, bool)> {
    let tokens: Vec<Said> = GroupKind::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            if omitted[index] == 0 && !headers[index] && !present[index] {
                Said::new(cut_to(kind.label(), width), Paint::Group(*kind))
            } else {
                token(*kind, omitted[index], width)
            }
        })
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
    /// Not at all: the groups and counts are the whole announcement.
    None,
}

/// A group's token in the compact shape: its name and its hidden count.
///
/// The name is cut only when the row cannot hold it whole; the count stays
/// whole while there is room for it. When there is no room for both, the
/// name takes the row, cut to the marker — the count is never shown without
/// the group it counts.
fn token(kind: GroupKind, count: usize, width: usize) -> Said {
    let label = kind.label();
    let count = count.to_string();
    let full = format!("{label} {count}");
    if text_width(&full) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group(kind)),
                (" ".to_string(), Paint::Dim),
                (count, Paint::Strong),
            ],
        };
    }
    let tail = format!(" {count}");
    if text_width(&tail) < width {
        let room = width - text_width(&tail);
        return Said {
            spans: vec![
                (cut_to(label, room), Paint::Group(kind)),
                (" ".to_string(), Paint::Dim),
                (count, Paint::Strong),
            ],
        };
    }
    Said::new(cut_to(label, width), Paint::Group(kind))
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
        let gap = if has_token { text_width(separator) } else { 0 };
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

/// The entries, packed into rows of at most `width` columns.
///
/// An entry is never broken over two rows, so a group is never cut from its
/// count.
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
                            Paint::Group(kind) => group_style(kind, theme),
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;
    use coding_crew_theme::Preset;

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// Draw the tab and return the rows it shows.
    fn draw(work: &Work, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), work, &theme(), None))
            .expect("drawing the backlog");
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
        terminal
            .draw(|frame| render(frame, frame.area(), work, &theme(), None))
            .expect("drawing the backlog");
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

    /// The text of some lines.
    fn text_of(lines: &[Line<'_>]) -> String {
        lines
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

    /// The lines of the tab that are wider than the pane.
    ///
    /// The check the width test is made of: it must be able to fail.
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

    /// A work folder with the tasks and items given.
    fn work_of(tasks: Vec<Task>, items: Vec<Item>) -> Work {
        Work {
            root: PathBuf::new(),
            tasks,
            items,
            errors: Vec::new(),
            activity: None,
            missing: None,
        }
    }

    /// A row as the test expects it.
    fn row(
        id: &str,
        kind: &str,
        title: &str,
        scope: Option<&str>,
        state: &str,
        settled: bool,
    ) -> Row {
        Row {
            id: id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            scope: scope.map(str::to_string),
            state: state.to_string(),
            settled,
        }
    }

    #[test]
    fn the_groups_and_the_fixture_rows_are_pinned() {
        let work = mock_work();
        let groups = groups(&work);
        assert_eq!(
            [groups[0].kind, groups[1].kind, groups[2].kind],
            GroupKind::ALL,
            "the three groups are in the order of the tab"
        );
        assert_eq!(groups[0].rows.len(), 1, "ideas");
        assert_eq!(groups[1].rows.len(), 4, "small tasks");
        assert_eq!(groups[2].rows.len(), 5, "drafts");
        assert_eq!(
            groups[0].rows,
            vec![row(
                "ITM01",
                "idea",
                "Lorem ipsum dolor sit amet",
                None,
                "open",
                false,
            )]
        );
        assert_eq!(
            groups[1].rows,
            vec![
                row(
                    "ITM02",
                    "feature",
                    "Tempor incididunt ut labore et dolore",
                    Some("the lorem pane"),
                    "open",
                    false,
                ),
                row(
                    "ITM03",
                    "bug",
                    "Quis nostrud exercitation ullamco laboris",
                    Some("the lorem list"),
                    "open",
                    false,
                ),
                row(
                    "ITM04",
                    "feature",
                    "Sed do eiusmod tempor incididunt",
                    None,
                    "dismissed",
                    true,
                ),
                row(
                    "ITM05",
                    "bug",
                    "Duis aute irure dolor in reprehenderit",
                    Some("the lorem button"),
                    "promoted",
                    true,
                ),
            ]
        );
        assert_eq!(
            groups[2].rows,
            vec![
                row(
                    "TSK01",
                    "draft",
                    "At vero eos et accusamus et iusto odio",
                    None,
                    "draft",
                    true,
                ),
                row(
                    "TSK02",
                    "draft",
                    "Totam rem aperiam eaque ipsa quae ab illo",
                    None,
                    "draft",
                    true,
                ),
                row(
                    "TSK03",
                    "draft",
                    "Ut enim ad minim veniam quis nostrud",
                    None,
                    "draft",
                    true,
                ),
                row(
                    "TSK27",
                    "draft",
                    "Duis aute irure dolor in reprehenderit",
                    Some("the lorem button"),
                    "draft",
                    true,
                ),
                row(
                    "TSK28",
                    "draft",
                    "Temporibus autem quibusdam et aut officiis",
                    None,
                    "draft",
                    true,
                ),
            ]
        );
    }

    #[test]
    fn a_one_row_pane_names_every_group_it_hides() {
        // At content width 38 the packed `group (count hidden)` rows do not
        // fit a one- or two-row pane. The compact shape names every group
        // and count instead of falling back to a bare total.
        let groups = groups(&mock_work());
        for height in [1usize, 2] {
            let text = text_of(&lines(&groups, 38, height, &theme(), None));
            for name in ["IDEAS", "SMALL TASKS", "DRAFTS"] {
                assert!(text.contains(name), "{height}: {name} is missing: {text}");
            }
            for count in ["1", "4", "5"] {
                assert!(text.contains(count), "{height}: {count} is missing: {text}");
            }
        }
    }

    #[test]
    fn a_dropped_header_is_announced_as_hidden_not_as_zero() {
        // The two empty groups' headers are dropped for room. Their entries
        // say the groups are hidden, not that they hide zero rows.
        let theme = theme();
        let (rows, _) = announcement(
            &[0, 0, 0],
            &[true, false, false],
            &[false; 3],
            40,
            10,
            &theme,
        );
        let text = said_text(&rows);
        assert!(text.contains("SMALL TASKS (hidden)"), "{text}");
        assert!(text.contains("DRAFTS (hidden)"), "{text}");
        assert!(!text.contains("SMALL TASKS (0 hidden)"), "{text}");
        assert!(!text.contains("DRAFTS (0 hidden)"), "{text}");
    }

    /// The text of some announcement lines.
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
    fn a_shown_group_is_not_announced_as_wholly_hidden() {
        // A rescue draws the cursor's only row of IDEAS. The header is gone,
        // but the group is on screen: the announcement says none of it is
        // hidden rather than claiming the whole group is.
        let theme = theme();
        let (rows, _) = announcement(
            &[0, 4, 5],
            &[false, false, false],
            &[true, false, false],
            40,
            10,
            &theme,
        );
        let text = said_text(&rows);
        assert!(text.contains("IDEAS (0 hidden)"), "{text}");
        assert!(!text.contains("IDEAS (hidden)"), "{text}");
    }

    #[test]
    fn a_compact_announcement_that_cannot_say_hidden_is_marked() {
        // The compact shape names every group and count, but the word does
        // not fit beside them. It is not dropped silently: the row is marked
        // with `…`.
        let theme = theme();
        let (rows, cut) = announcement(
            &[0, 4, 5],
            &[true, false, false],
            &[true, false, false],
            36,
            1,
            &theme,
        );
        assert!(cut);
        let text = said_text(&rows);
        for name in ["IDEAS", "SMALL TASKS", "DRAFTS"] {
            assert!(text.contains(name), "{text}");
        }
        assert!(text.contains('…'), "{text}");
    }

    #[test]
    fn a_two_column_pane_does_not_panic() {
        // A pane narrower than its border has no content columns. It draws
        // nothing rather than panicking, and the footer carries the count.
        let work = mock_work();
        let groups = groups(&work);
        let (rows, announced, shown, _) = window(&groups, 0, 1, &theme(), Some("ITM01"), 0);
        assert!(rows.len() <= 1, "{rows:?}");
        assert!(!announced);
        assert!(shown.len() <= 1);
        // And the rendered tab at the reported size.
        let _ = draw(&work, 2, 5);
    }

    #[test]
    fn a_one_row_announcement_is_cut_with_a_word() {
        // However narrow the pane, an announcement that cannot name every
        // group says what it is: the word, or the marker that says it was
        // cut. (Above 29 columns every group and count fits on the row.)
        let groups = groups(&mock_work());
        for width in 10..=29 {
            let text = text_of(&lines(&groups, width, 1, &theme(), None));
            let last = text
                .lines()
                .last()
                .unwrap_or_else(|| panic!("{width}: no announcement"));
            assert!(
                last.contains("hidden") || last.ends_with('…'),
                "{width}: {last:?} is cut without a word"
            );
        }
    }

    #[test]
    fn a_row_whose_content_is_zero_width_says_so() {
        // A row with an empty title and no scope has nothing to draw at a
        // one-column pane: it says the content was dropped rather than
        // drawing a blank line.
        let mut item = Item::new("ITM01", "");
        item.scope = Vec::new();
        let work = work_of(Vec::new(), vec![item]);
        let groups = groups(&work);
        let metrics = Metrics {
            id: id_width(&groups),
            kind: kind_width(&groups),
            state: state_width(&groups),
        };
        let line = row_line(&groups[0].rows[0], 1, metrics, &theme(), None);
        assert!(line.width() > 0, "the row is blank");
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(text, "…", "the dropped row is not announced");
    }

    #[test]
    fn a_dismissed_and_a_promoted_item_are_visible_as_such() {
        let rows = draw(&mock_work(), 100, 40).join("\n");
        let dismissed = rows
            .lines()
            .find(|line| line.contains("ITM04"))
            .unwrap_or_else(|| panic!("no dismissed row: {rows}"));
        assert!(dismissed.contains("dismissed"), "{dismissed}");
        let promoted = rows
            .lines()
            .find(|line| line.contains("ITM05"))
            .unwrap_or_else(|| panic!("no promoted row: {rows}"));
        assert!(promoted.contains("promoted"), "{promoted}");
    }

    #[test]
    fn every_draft_task_is_in_the_backlog_and_no_other_task_is() {
        let work = mock_work();
        let groups = groups(&work);
        let shown: Vec<&str> = groups[2].rows.iter().map(|row| row.id.as_str()).collect();
        let drafts: Vec<&str> = work
            .tasks
            .iter()
            .filter(|task| task.status == Status::Draft)
            .map(|task| task.id.as_str())
            .collect();
        assert_eq!(shown, drafts);
        for task in &work.tasks {
            if task.status != Status::Draft {
                assert!(
                    !shown.contains(&task.id.as_str()),
                    "{} is not a draft",
                    task.id
                );
            }
        }
    }

    #[test]
    fn the_rows_are_sorted_by_id_within_their_group() {
        let groups = groups(&mock_work());
        for group in groups {
            let ids: Vec<&str> = group.rows.iter().map(|row| row.id.as_str()).collect();
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            assert_eq!(ids, sorted, "{:?}", group.kind);
        }
    }

    #[test]
    fn the_groups_have_the_shape_of_the_spec() {
        let text = draw(&mock_work(), 100, 40).join("\n");
        assert!(text.contains("IDEAS · 1"), "{text}");
        assert!(text.contains("SMALL TASKS · 4"), "{text}");
        assert!(text.contains("DRAFTS · 5"), "{text}");
        let ideas = text.find("IDEAS").expect("the ideas heading");
        let small = text.find("SMALL TASKS").expect("the small tasks heading");
        let drafts = text.find("DRAFTS").expect("the drafts heading");
        assert!(ideas < small && small < drafts, "{text}");
    }

    #[test]
    fn a_wide_row_shows_every_column() {
        let text = draw(&mock_work(), 100, 40).join("\n");
        let row = text
            .lines()
            .find(|line| line.contains("ITM04"))
            .unwrap_or_else(|| panic!("no ITM04 row: {text}"));
        assert!(row.contains("feature"), "{row}");
        assert!(row.contains("Sed do eiusmod tempor incididunt"), "{row}");
        assert!(row.contains("dismissed"), "{row}");
        let row = text
            .lines()
            .find(|line| line.contains("TSK27"))
            .unwrap_or_else(|| panic!("no TSK27 row: {text}"));
        assert!(row.contains("the lorem button"), "{row}");
        assert!(row.contains("draft"), "{row}");
    }

    #[test]
    fn the_empty_backlog_says_so() {
        let mut task = Task::new("TSK01", "a task");
        task.status = Status::Approved;
        for work in [Work::default(), work_of(vec![task], Vec::new())] {
            let text = draw(&work, 40, 10).join("\n");
            assert!(text.contains("no backlog yet"), "{text}");
            assert!(!text.contains("IDEAS"), "{text}");
        }
    }

    #[test]
    fn no_built_line_is_wider_than_the_pane() {
        let work = mock_work();
        let groups = groups(&work);
        for width in 1..=120 {
            for height in 1..=40 {
                let lines = lines(&groups, width, height, &theme(), None);
                assert!(
                    too_wide(&lines, width).is_empty(),
                    "{width}x{height}: {:?}",
                    too_wide(&lines, width)
                );
                assert!(
                    lines.len() <= height,
                    "{width}x{height}: {} lines",
                    lines.len()
                );
            }
        }
    }

    #[test]
    fn the_width_check_can_fail() {
        let wide = vec![Line::from(Span::raw("x".repeat(41)))];
        assert_eq!(too_wide(&wide, 40).len(), 1);
        let exact = vec![Line::from(Span::raw("x".repeat(40)))];
        assert!(too_wide(&exact, 40).is_empty());
    }

    #[test]
    fn a_narrow_row_drops_whole_columns() {
        let work = mock_work();
        let groups = groups(&work);
        let metrics = Metrics {
            id: id_width(&groups),
            kind: kind_width(&groups),
            state: state_width(&groups),
        };
        for width in 1..=120 {
            let built = lines(&groups, width, 40, &theme(), None);
            let mut at = 0;
            for group in &groups {
                if group.rows.is_empty() {
                    continue;
                }
                at += 1; // the header
                for row in &group.rows {
                    let text = built[at]
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>();
                    at += 1;
                    let layout = layout(row, width, metrics);
                    // A fixed column is shown whole, or not at all.
                    if layout.columns.id {
                        assert!(text.starts_with(&row.id), "{width}: {text:?}");
                    }
                    if layout.columns.kind {
                        let start = if layout.columns.id {
                            metrics.id + GAP
                        } else {
                            0
                        };
                        assert!(
                            text[start..].starts_with(&row.kind),
                            "{width}: a cut kind: {text:?}"
                        );
                    }
                    if layout.columns.state {
                        assert!(
                            text.trim_end().ends_with(&row.state),
                            "{width}: a cut state: {text:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_title_and_the_scope_keep_their_room() {
        // The kind and the state give way, whole, before the title and the
        // scope are cut: at this width the id, the title and the scope are
        // still whole.
        let groups = groups(&mock_work());
        let text = text_of(&lines(&groups, 70, 40, &theme(), None));
        let row = text
            .lines()
            .find(|line| line.contains("ITM02"))
            .unwrap_or_else(|| panic!("no ITM02 row: {text}"));
        assert!(
            row.contains("Tempor incididunt ut labore et dolore"),
            "{row}"
        );
        assert!(row.contains("the lorem pane"), "{row}");
        assert!(!row.contains("feature"), "{row}");
        // Narrower still: the id gives way before the title and the scope,
        // which keep their room.
        let text = text_of(&lines(&groups, 58, 40, &theme(), None));
        assert!(!text.contains("ITM02"), "{text}");
        assert!(text.contains("Tempor incidid"), "{text}");
        assert!(text.contains("the lorem pane"), "{text}");
        let row = text
            .lines()
            .find(|line| line.contains("Tempor incidid"))
            .expect("the smallest row");
        assert!(!row.contains("feature"), "{row}");
    }

    #[test]
    fn a_narrow_row_cuts_the_title_and_the_scope_and_nothing_else() {
        let groups = groups(&mock_work());
        for width in 4..=40 {
            let lines = lines(&groups, width, 40, &theme(), None);
            for line in &lines {
                let text = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>();
                for part in ["ITM", "TSK", "feature", "dismissed", "promoted"] {
                    if text.contains(part) {
                        assert!(!text.contains(&format!("{part}…")), "{width}: {text:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_counts_are_strong_and_the_titles_use_the_title_treatment() {
        let theme = theme();
        let groups = groups(&mock_work());
        let lines = lines(&groups, 100, 40, &theme, None);
        for (heading, count) in [("IDEAS", "1"), ("SMALL TASKS", "4"), ("DRAFTS", "5")] {
            let header = lines
                .iter()
                .find(|line| {
                    line.spans
                        .iter()
                        .any(|span| span.content.as_ref() == heading)
                })
                .unwrap_or_else(|| panic!("no {heading} header"));
            let counted = header
                .spans
                .iter()
                .find(|span| span.content.as_ref() == count)
                .unwrap_or_else(|| panic!("no {heading} count"));
            assert_eq!(counted.style, theme.text.strong, "{heading}");
        }
        let title = groups[0].rows[0].title.clone();
        let title_span = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.as_ref() == title)
            .unwrap_or_else(|| panic!("no title span"));
        assert_eq!(title_span.style, theme.text.strong);
    }

    #[test]
    fn a_draft_or_settled_row_is_dim() {
        let theme = theme();
        let groups = groups(&mock_work());
        let lines = lines(&groups, 100, 40, &theme, None);
        let text_of_line = |line: &Line<'_>| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        };
        let settled = lines
            .iter()
            .find(|line| text_of_line(line).contains("ITM05"))
            .expect("the promoted row");
        assert!(
            settled
                .spans
                .iter()
                .any(|span| span.style == theme.text.dim),
            "the settled row is not dim"
        );
    }

    #[test]
    fn a_selected_row_takes_the_selection_slot_and_the_others_keep_their_own() {
        let theme = theme();
        let selection = theme.text.selected;
        let groups = groups(&mock_work());
        let metrics = Metrics {
            id: id_width(&groups),
            kind: kind_width(&groups),
            state: state_width(&groups),
        };
        let open = &groups[0].rows[0];
        let settled = &groups[1].rows[2];
        // A row that is not the cursor's keeps its own paint.
        let plain = row_line(open, 60, metrics, &theme, None);
        assert_eq!(plain.spans[0].style, theme.text.normal, "an open row's id");
        let plain = row_line(settled, 60, metrics, &theme, None);
        assert_eq!(plain.spans[0].style, theme.text.dim, "a settled row's id");
        // The cursor's row takes the selection's pair on every cell, and the
        // bar runs to the edge of the pane.
        let selected = row_line(open, 60, metrics, &theme, Some("ITM01"));
        assert_eq!(selected.width(), 60, "the selection bar stops short");
        for span in &selected.spans {
            assert_eq!(span.style, selection, "{:?} is not the selection", span);
        }
        assert_eq!(selected.spans[0].style.fg, selection.fg);
        assert_eq!(selected.spans[0].style.bg, selection.bg);
        // Another row is not painted as the selection.
        let other = row_line(settled, 60, metrics, &theme, Some("ITM01"));
        assert_eq!(other.spans[0].style, theme.text.dim);
    }

    #[test]
    fn a_missing_folder_says_so_and_names_the_path() {
        let work = Work::load(Path::new("/nowhere/coding-crew/work"));
        let text = draw(&work, 60, 12).join("\n");
        assert!(text.contains("no coding-crew folder found"), "{text}");
        assert!(
            text.contains("looked in /nowhere/coding-crew/work"),
            "{text}"
        );
    }

    #[test]
    fn every_row_is_on_screen_at_the_default_size() {
        let work = mock_work();
        let text = draw(&work, 100, 40).join("\n");
        for group in groups(&work) {
            for row in group.rows {
                assert!(text.contains(&row.id), "{} is missing", row.id);
            }
        }
    }

    #[test]
    fn a_zero_count_group_keeps_its_header() {
        // The three groups are derived facts: an empty one is still a fact,
        // and its header is shown whenever the pane has the room.
        let work = work_of(Vec::new(), vec![Item::new("ITM01", "only an idea")]);
        let text = draw(&work, 100, 40).join("\n");
        assert!(text.contains("IDEAS · 1"), "{text}");
        assert!(text.contains("SMALL TASKS · 0"), "{text}");
        assert!(text.contains("DRAFTS · 0"), "{text}");
        let ideas = text.find("IDEAS · 1").expect("the ideas header");
        let small = text
            .find("SMALL TASKS · 0")
            .expect("the small tasks header");
        let drafts = text.find("DRAFTS · 0").expect("the drafts header");
        assert!(ideas < small && small < drafts, "{text}");
    }

    #[test]
    fn the_default_size_announces_nothing() {
        let text = draw(&mock_work(), 100, 40).join("\n");
        assert!(!text.contains("hidden"), "{text}");
        assert!(!text.contains("not shown"), "{text}");
    }

    #[test]
    fn a_short_pane_announces_every_group_and_its_hidden_rows() {
        // At 30x8 the pane cannot hold the packed `group (count hidden)`
        // rows. The compact shape names the groups and their counts, and the
        // lead-in says the counts are the counts of what is hidden.
        let text = draw(&mock_work(), 30, 8).join("\n");
        assert!(text.contains("not shown"), "{text}");
        for name in ["IDEAS", "SMALL TASKS", "DRAFTS"] {
            let line = text
                .lines()
                .find(|line| line.contains(name) && !line.contains('·'))
                .unwrap_or_else(|| panic!("{name} is not named in the announcement: {text}"));
            assert!(
                line.chars().any(|ch| ch.is_ascii_digit()),
                "{name} has no hidden count: {line}"
            );
        }
        // The announcement is not the header's shape.
        let announced = text
            .lines()
            .find(|line| line.contains("not shown"))
            .unwrap_or_else(|| panic!("no announcement: {text}"));
        assert!(!announced.contains(" · "), "{announced}");
        assert!(!announced.contains("· "), "{announced}");
    }

    #[test]
    fn the_announcement_counts_the_hidden_rows_and_not_the_group_total() {
        // At 100x10 the DRAFTS header holds the whole group's count, and
        // the announcement says what is left out of view instead.
        let text = draw(&mock_work(), 100, 10).join("\n");
        assert!(text.contains("DRAFTS (5 hidden)"), "{text}");
        assert!(!text.contains("DRAFTS · 5"), "{text}");
    }

    #[test]
    fn every_height_that_omits_a_row_says_so() {
        let work = mock_work();
        let groups = groups(&work);
        let wanted: usize = groups.iter().map(|group| 1 + group.rows.len()).sum();
        for height in 1..=wanted + 2 {
            let built = lines(&groups, 100, height, &theme(), None);
            let text = text_of(&built);
            if wanted > height {
                assert!(text.contains("hidden"), "{height}: {text}");
                for group in &groups {
                    assert!(
                        text.contains(group.kind.label()),
                        "{height}: {} is missing: {text}",
                        group.kind.label()
                    );
                }
            } else {
                assert!(!text.contains("hidden"), "{height}: {text}");
            }
        }
    }

    /// The colour of the first cell of `needle` on a row.
    fn colour_at<'a>(row: &'a [(char, Color)], needle: &str) -> Option<&'a Color> {
        let chars: Vec<char> = row.iter().map(|(symbol, _)| *symbol).collect();
        let needle: Vec<char> = needle.chars().collect();
        let start = chars
            .windows(needle.len())
            .position(|window| window == needle.as_slice())?;
        row.get(start).map(|(_, colour)| colour)
    }

    #[test]
    fn the_draft_heading_is_painted_with_the_draft_phase_slot() {
        // The heading and the rows under it are the same fact: the draft
        // phase, which is `text.dim`. The count stays `text.strong`.
        let theme = theme();
        let dim = theme.text.dim.fg.expect("the dim colour");
        let strong = theme.text.strong.fg.expect("the strong colour");
        assert_ne!(
            theme.text.normal.fg,
            Some(dim),
            "the test cannot tell normal from dim"
        );
        let cells = draw_cells(&mock_work(), 100, 40);
        let row = cells
            .iter()
            .find(|row| {
                let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
                text.contains("DRAFTS") && text.contains("· 5")
            })
            .unwrap_or_else(|| panic!("no DRAFTS header"));
        assert_eq!(
            colour_at(row, "DRAFTS"),
            Some(&dim),
            "the draft heading is not dim"
        );
        let count = row
            .iter()
            .map(|(symbol, _)| *symbol)
            .collect::<Vec<char>>()
            .windows(3)
            .position(|window| window == ['·', ' ', '5'].as_slice())
            .map(|at| row[at + 2].1)
            .expect("the DRAFTS count");
        assert_eq!(count, strong, "the draft count is not strong");
    }

    /// The title fixture: one wide, one combining and one joined grapheme.
    const GRAPHEME_TITLE: &str = "é👩‍💻e\u{0301}界abc";

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
    fn a_wide_combining_and_joined_title_is_cut_on_grapheme_boundaries() {
        // The measured width is pinned, so a change in the width table is
        // the test's problem rather than a silent one.
        assert_eq!(text_width(GRAPHEME_TITLE), 9, "the fixture measures");
        // The scalar cut kept the woman and the joiner and dropped the
        // laptop; the grapheme cut keeps the emoji whole.
        assert_eq!(cut_to(GRAPHEME_TITLE, 4), "é👩‍💻…");
        assert_eq!(cut_to(GRAPHEME_TITLE, 3), "é…");
        for width in 1..=text_width(GRAPHEME_TITLE) {
            let cut = cut_to(GRAPHEME_TITLE, width);
            let head = cut.strip_suffix('…').unwrap_or(&cut);
            assert!(
                is_grapheme_prefix(head, GRAPHEME_TITLE),
                "{width}: {cut:?} cuts a grapheme"
            );
        }
        let mut item = Item::new("ITM01", GRAPHEME_TITLE);
        item.scope = vec![GRAPHEME_TITLE.to_string()];
        let work = work_of(Vec::new(), vec![item]);
        let groups = groups(&work);
        for width in 1..=120 {
            let built = lines(&groups, width, 40, &theme(), None);
            assert!(
                too_wide(&built, width).is_empty(),
                "{width}: {:?}",
                too_wide(&built, width)
            );
        }
    }
}
