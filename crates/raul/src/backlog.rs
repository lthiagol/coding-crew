//! The Backlog tab: what is not a task yet.
//!
//! The smaller tasks and the draft tasks, in one list. The groups are
//! derived from the model — ideas, then small tasks, then drafts — and are
//! never stored.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use coding_crew_core::{Item, ItemKind, ItemStatus, Status, Task};

use crate::announce;
use crate::data::Work;
use crate::tabs::Tab;
use crate::view::{self, Lines, View};
use coding_crew_theme::chrome;
use coding_crew_theme::text::{cut_to, pad, text_width};
use coding_crew_theme::{Icon, IconSet, Theme, phase_style, selected_line};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between two columns.
const GAP: usize = 2;
/// The columns of a row's icon: a space, the icon and a space.
const ICON_LEAD: usize = 3;

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
    /// What the item is about: the body it carries, as one line.
    pub body: Option<String>,
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
        // The id ascending: the order is the tab's sort, the same value the
        // Settings tab names.
        let sort = Tab::Backlog.sort();
        group
            .rows
            .sort_by(|one, other| sort.compare_ids(&one.id, &other.id));
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
        body: body_text(&item.body),
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
        body: None,
        settled: true,
    }
}

/// The item's body as one line of text, when it has one.
///
/// The body is the item's own sentence: only its whitespace is normalised, so
/// a multi-line body still reads as the one body the item carries.
fn body_text(body: &str) -> Option<String> {
    let words: Vec<&str> = body.split_whitespace().collect();
    (!words.is_empty()).then(|| words.join(" "))
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
    render_view(frame, area, work, theme, IconSet::Nerd, selected, &mut view);
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
    icons: IconSet,
    selected: Option<&str>,
    view: &mut View,
) -> Option<usize> {
    let block = chrome::pane("backlog", Icon::TabBacklog, icons, false, theme);
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
        window(&groups, width, height, theme, icons, selected, first)
    });
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }).block(block),
        area,
    );
    let hidden = ids.len().saturating_sub(view.page());
    (hidden > 0 && !announced).then_some(hidden)
}

/// The columns of a row: the fixed ones, and the ones that give way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    icon: bool,
    id: bool,
    kind: bool,
    state: bool,
    scope: bool,
}

/// The candidates, in the order the columns give way.
///
/// The title and the scope are what the reader is here for: the icon gives
/// way first, then the id, the kind and the state, whole.
const CANDIDATES: [Columns; 6] = [
    Columns {
        icon: true,
        id: true,
        kind: true,
        state: true,
        scope: true,
    },
    Columns {
        icon: false,
        id: true,
        kind: true,
        state: true,
        scope: true,
    },
    Columns {
        icon: false,
        id: true,
        kind: false,
        state: true,
        scope: true,
    },
    Columns {
        icon: false,
        id: true,
        kind: false,
        state: false,
        scope: true,
    },
    Columns {
        icon: false,
        id: false,
        kind: false,
        state: false,
        scope: true,
    },
    Columns {
        icon: false,
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
    icons: IconSet,
    selected: Option<&str>,
) -> Vec<Line<'static>> {
    window(groups, width, height, theme, icons, selected, 0)
        .0
        .lines
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
    icons: IconSet,
    selected: Option<&str>,
    first: usize,
) -> (Lines, bool, Vec<usize>, bool) {
    if height == 0 {
        return (Lines::default(), false, Vec::new(), false);
    }
    let total: usize = groups.iter().map(|group| group.rows.len()).sum();
    // The pane counts lines, and a row with a body is two of them.
    let wanted: usize = groups
        .iter()
        .map(|group| 1 + group.rows.iter().map(row_height).sum::<usize>())
        .sum();
    if first == 0 && wanted <= height {
        return (
            all_lines(groups, width, icons, theme, selected),
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
    let mut shortest: Option<(Lines, Lines, Vec<usize>)> = None;
    let mut with_cursor: Option<(Lines, Lines, Vec<usize>)> = None;
    for capacity in (0..=height).rev() {
        let (rows, omitted, headers, shown) =
            layout_groups(groups, capacity, width, icons, theme, selected, first);
        let room = height.saturating_sub(rows.len());
        let present = present_groups(groups, &shown);
        let (word, cut) = announcement(&omitted, &headers, &present, width, room, theme);
        let said = Lines::of(word, None);
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
        out.append(said);
        return (out, announced, shown, true);
    }
    if let Some((mut out, said, shown)) = with_cursor.or(shortest) {
        if let Some(id) = selected
            && !shown.iter().any(|index| row_id(groups, *index) == id)
            && let Some((index, mut rows)) = cursor_lines(groups, width, icons, theme, id)
        {
            // The pane is too short for the cursor's row and the layout: the
            // row is drawn, and what is left says what the pane hides. A pane
            // of two lines shows a two-line row whole, with nothing left to
            // say. A pane of one line shows the row alone, and there its body
            // gives way with the pane: the row is drawn in the one line it is
            // counted as.
            rows.truncate(height);
            let room = height - rows.len();
            let mut announced = false;
            if room > 0 {
                let omitted = omitted_with_only(groups, index);
                let present = present_with_only(groups, index);
                let headers = [false; 3];
                let (word, _) = announcement(&omitted, &headers, &present, width, room, theme);
                announced = !word.is_empty();
                rows.append(Lines::of(word, None));
            }
            return (rows, announced, vec![index], false);
        }
        if said.is_empty()
            && height >= 2
            && let Some(id) = selected
            && let Some(cursor) = (0..total).find(|at| row_id(groups, *at) == id)
            && let Some((_, mut rows)) = cursor_lines(groups, width, icons, theme, id)
            && rows.len() < height
        {
            // The rows took every line and said nothing about what they hid.
            // The header gives way: the cursor's row is drawn, and the rest of
            // the pane names the groups and what they hide.
            let omitted = omitted_with_only(groups, cursor);
            let present = present_with_only(groups, cursor);
            let headers = [false; 3];
            let room = height - rows.len();
            let (word, _) = announcement(&omitted, &headers, &present, width, room, theme);
            if !word.is_empty() {
                rows.append(Lines::of(word, None));
                return (rows, true, vec![cursor], false);
            }
        }
        let announced = !said.is_empty();
        out.append(said);
        return (out, announced, shown, false);
    }
    // No layout was chosen: the pane is too narrow to draw a header and too
    // short to say anything. The cursor's row is still drawn, alone.
    if let Some(id) = selected
        && let Some((index, mut rows)) = cursor_lines(groups, width, icons, theme, id)
    {
        rows.truncate(height);
        return (rows, false, vec![index], false);
    }
    (Lines::default(), false, Vec::new(), false)
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

/// The lines of the cursor's row, and its place among the tab's rows.
fn cursor_lines(
    groups: &[Group],
    width: usize,
    icons: IconSet,
    theme: &Theme,
    id: &str,
) -> Option<(usize, Lines)> {
    let mut index = 0;
    for group in groups {
        let metrics = group_metrics(groups, group, width);
        for row in &group.rows {
            if row.id == id {
                return Some((
                    index,
                    Lines::of(
                        row_lines(row, width, metrics, icons, theme, Some(id)),
                        Some(index),
                    ),
                ));
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
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) -> Lines {
    let mut out = Lines::default();
    let mut index = 0;
    for group in groups {
        let metrics = group_metrics(groups, group, width);
        out.push(header(group, width, theme), None);
        for row in &group.rows {
            let lines = row_lines(row, width, metrics, icons, theme, selected);
            out.extend(lines, Some(index));
            index += 1;
        }
    }
    out
}

/// The rows of the tab that fit in `capacity` lines, how many rows of each
/// group are left out of them, which headers are shown, and the rows drawn.
///
/// The capacity is in lines, and a row is atomic: its columns and its body
/// are drawn together or not at all, so the row the window counts is the row
/// it draws. The groups keep their order: the header of a group, then its
/// rows. A header's count is the whole group; the omitted count is what the
/// announcement says instead, and the two are meant to differ. A group
/// whose header does not fit is not shown at all, every one of its rows is
/// left out of view, and the announcement says the header itself was hidden
/// even when the group holds no rows.
#[allow(clippy::too_many_arguments)]
fn layout_groups(
    groups: &[Group],
    capacity: usize,
    width: usize,
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
    first: usize,
) -> (Lines, [usize; 3], [bool; 3], Vec<usize>) {
    let mut out = Lines::default();
    let mut omitted = [0usize; 3];
    let mut headers = [false; 3];
    let mut shown_rows = Vec::new();
    let mut used = 0usize;
    let mut row_index = 0usize;
    for (index, group) in groups.iter().enumerate() {
        let start = row_index;
        row_index += group.rows.len();
        // An empty group has no rows above the window, so its header is
        // shown wherever the window starts.
        if !group.rows.is_empty() && row_index <= first {
            // The whole group is above the window: its header is hidden too.
            omitted[index] = group.rows.len();
            continue;
        }
        if used + 1 > capacity {
            omitted[index] = group.rows.len();
            continue;
        }
        let metrics = group_metrics(groups, group, width);
        out.push(header(group, width, theme), None);
        headers[index] = true;
        used += 1;
        let skip = first.saturating_sub(start).min(group.rows.len());
        let mut shown = 0;
        for (offset, row) in group.rows.iter().skip(skip).enumerate() {
            let height = row_height(row);
            if used + height > capacity {
                break;
            }
            let at = start + skip + offset;
            out.extend(
                row_lines(row, width, metrics, icons, theme, selected),
                Some(at),
            );
            shown_rows.push(at);
            used += height;
            shown += 1;
        }
        if shown < group.rows.len() {
            omitted[index] = group.rows.len() - shown;
        }
    }
    (out, omitted, headers, shown_rows)
}

/// The widths of the fixed columns, and the candidate the group shows.
#[derive(Debug, Clone, Copy)]
struct Metrics {
    id: usize,
    kind: usize,
    state: usize,
    columns: Columns,
}

/// The metrics of a group of a pane `width` wide.
///
/// The columns are decided for the group, not the row: the group shows the
/// first candidate that every one of its rows fits, so its icons, ids, kinds
/// and states share their columns. The group headers separate the groups, so
/// a group that gives its columns up does not take them from the others.
fn group_metrics(groups: &[Group], group: &Group, width: usize) -> Metrics {
    let metrics = Metrics {
        id: id_width(groups),
        kind: kind_width(groups),
        state: state_width(groups),
        columns: CANDIDATES[0],
    };
    let width = width.saturating_sub(view::BAR);
    let columns = CANDIDATES
        .into_iter()
        .find(|candidate| {
            group.rows.iter().all(|row| {
                (!candidate.icon || row_icon(row).is_some())
                    && fit(row, width, metrics, *candidate).is_some()
            })
        })
        .unwrap_or(CANDIDATES[CANDIDATES.len() - 1]);
    Metrics { columns, ..metrics }
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

/// The layout of a row: the group's candidate, or the first candidate after it
/// that leaves the fixed columns whole and room for the title and the scope.
fn layout(row: &Row, width: usize, metrics: Metrics) -> Layout {
    CANDIDATES
        .iter()
        .skip_while(|candidate| **candidate != metrics.columns)
        .find_map(|candidate| fit(row, width, metrics, *candidate))
        .unwrap_or(Layout {
            columns: Columns {
                icon: false,
                id: false,
                kind: false,
                state: false,
                scope: false,
            },
            title: width,
            scope: 0,
        })
}

/// The layout of a row in a candidate, if the candidate's fixed columns fit
/// the row with room for its title and its scope.
fn fit(row: &Row, width: usize, metrics: Metrics, candidate: Columns) -> Option<Layout> {
    let title = text_width(&row.title);
    let scope = row.scope.as_deref().map(text_width).unwrap_or(0);
    let mut columns = candidate;
    if row.scope.is_none() {
        columns.scope = false;
    }
    if row_icon(row).is_none() {
        columns.icon = false;
    }
    let (fixed, gaps) = fixed_width(columns, metrics);
    let flex = 1 + usize::from(columns.scope);
    if fixed + gaps + flex > width {
        return None;
    }
    let available = width - fixed - gaps;
    // A fixed column is kept only when the title and the scope still fit
    // whole: otherwise it gives way, so they are not cut to make room for it.
    let has_fixed = columns.id || columns.kind || columns.state;
    if has_fixed && title + scope > available {
        return None;
    }
    let (title_width, scope_width) = flex_widths(title, scope, available, columns.scope);
    Some(Layout {
        columns,
        title: title_width,
        scope: scope_width,
    })
}

/// The icon of a row: its kind's, or the draft phase's for a draft task.
fn row_icon(row: &Row) -> Option<Icon> {
    if row.kind == "draft" {
        Some(Icon::StatusDraft)
    } else {
        Icon::for_kind(&row.kind)
    }
}

/// The width of the fixed columns, and the gaps between every column.
///
/// The icon's lead is fixed but takes no gap: it stands before the id.
fn fixed_width(columns: Columns, metrics: Metrics) -> (usize, usize) {
    let mut fixed = 0;
    let mut count: usize = 1; // the title is always there
    if columns.icon {
        fixed += ICON_LEAD;
    }
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

/// The lines a row takes in the pane.
///
/// A row with a body is two lines: its columns, then the body. Everything the
/// tab counts — the window, the announcement and the hidden count — counts
/// these lines, so a row drawn in two lines is never counted as one.
fn row_height(row: &Row) -> usize {
    1 + usize::from(row.body.is_some())
}

/// The lines of a row: its columns, and its body under them.
fn row_lines(
    row: &Row,
    width: usize,
    metrics: Metrics,
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) -> Vec<Line<'static>> {
    // The selection bar takes the row's first column: the columns are laid
    // out in the rest.
    let layout = layout(row, width.saturating_sub(view::BAR), metrics);
    let mut lines = vec![columns_line(
        row, &layout, width, metrics, icons, theme, selected,
    )];
    if let Some(line) = body_line(row, &layout, width, metrics, theme, selected) {
        lines.push(line);
    }
    lines
}

/// The row's columns line: the bar, the icon, the id, the kind, the title, the
/// scope and the state.
///
/// The tests read a row's first line through this; the tab draws a row
/// through [`row_lines`].
#[cfg(test)]
fn row_line(
    row: &Row,
    width: usize,
    metrics: Metrics,
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    let layout = layout(row, width.saturating_sub(view::BAR), metrics);
    columns_line(row, &layout, width, metrics, icons, theme, selected)
}

/// The row's own line: the bar, the icon, the id, the kind, the title, the
/// scope and the state.
///
/// The bar is the row's first column, drawn whether the row is selected or not,
/// so the rows line up. The icon is painted in the row's slot: a draft is
/// settled, so its icon is the dim slot the draft phase has.
fn columns_line(
    row: &Row,
    layout: &Layout,
    width: usize,
    metrics: Metrics,
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    let inner = width.saturating_sub(view::BAR);
    let row_style = if row.settled {
        theme.text.dim
    } else {
        theme.text.normal
    };
    let id_style = if row.settled {
        theme.text.dim
    } else {
        theme.text.strong
    };
    let mut lead: Vec<Span<'static>> = Vec::new();
    if layout.columns.icon
        && let Some(icon) = row_icon(row)
    {
        lead.push(Span::raw(" "));
        lead.push(Span::styled(icon.glyph(icons), row_style));
        lead.push(Span::raw(" "));
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    if layout.columns.id {
        spans.push(Span::styled(pad(&row.id, metrics.id), id_style));
    }
    if layout.columns.kind {
        push_gap(&mut spans);
        spans.push(Span::styled(pad(&row.kind, metrics.kind), row_style));
    }
    push_gap(&mut spans);
    spans.push(Span::styled(
        pad(&cut_to(&row.title, layout.title), layout.title),
        row_style,
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
    // marker says that something was dropped. A pane one column wide has no
    // room for the bar beside the marker, so the marker takes the bar's column.
    if line.width() == 0 && width > 0 {
        if inner == 0 {
            return Line::from(Span::styled(cut_to("…", width), theme.text.dim));
        }
        line = Line::from(Span::styled(cut_to("…", inner), theme.text.dim));
    }
    let mut spans = lead;
    spans.extend(line.spans);
    view::barred(
        Line::from(spans),
        width,
        selected == Some(row.id.as_str()),
        icons,
        theme,
    )
}

/// The column the row's title starts at: the columns before it, their gaps,
/// and the icon's lead. The selection bar's column is not counted here.
fn title_start(layout: &Layout, metrics: Metrics) -> usize {
    let mut start = 0;
    if layout.columns.icon {
        start += ICON_LEAD;
    }
    if layout.columns.id {
        start += metrics.id + GAP;
    }
    if layout.columns.kind {
        start += metrics.kind + GAP;
    }
    start
}

/// The column the row's body starts at: where the title starts, the selection
/// bar's column included.
///
/// The indent gives way before the body does: a row with no room beside its
/// indent starts the body at the edge, so the body's marker is drawn.
fn body_indent(layout: &Layout, metrics: Metrics, width: usize) -> usize {
    let indent = view::BAR + title_start(layout, metrics);
    if indent < width { indent } else { 0 }
}

/// The row's second line: the item's own body, cut, under the row's text.
///
/// The body is indented to where the row's title starts, the selection bar's
/// column included, so it reads as the row's own and not as a row of its own.
/// It is painted with the dim slot, the grade the tab gives a hint or a
/// marker, so it never competes with the title. It is the body, cut by the
/// shared grapheme helper: never a summary, and a cut body always ends in the
/// tab's own marker.
fn body_line(
    row: &Row,
    layout: &Layout,
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    selected: Option<&str>,
) -> Option<Line<'static>> {
    let body = row.body.as_deref()?;
    // A pane with no content columns draws the body as a blank line, as the
    // columns line does: the row is still counted as two lines.
    if width == 0 {
        return Some(Line::from(""));
    }
    let indent = body_indent(layout, metrics, width);
    let room = width - indent;
    let mut spans: Vec<Span<'static>> = Vec::new();
    if indent > 0 {
        spans.push(Span::raw(" ".repeat(indent)));
    }
    spans.push(Span::styled(cut_to(body, room), theme.text.dim));
    let line = Line::from(spans);
    if selected == Some(row.id.as_str()) {
        return Some(selected_line(line, width, theme));
    }
    Some(line)
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

/// The header of a group: its name, and the count of its rows as a badge.
///
/// The count is `text.strong`, as every count is. It is never cut; when the
/// pane is too narrow for the whole header, the name gives way.
fn header(group: &Group, width: usize, theme: &Theme) -> Line<'static> {
    let count = group.rows.len();
    let digits = count.to_string();
    let label = group.kind.label();
    let style = group_style(group.kind, theme);
    let separator = "  ";
    let fixed = text_width(separator) + text_width(&digits);
    if width >= text_width(label) + fixed {
        return Line::from(vec![
            Span::styled(label.to_string(), style),
            Span::raw(separator),
            chrome::badge(count, theme),
        ]);
    }
    if width > fixed {
        let room = width - fixed;
        return Line::from(vec![
            Span::styled(cut_to(label, room), style),
            Span::raw(separator),
            chrome::badge(count, theme),
        ]);
    }
    if width >= text_width(&digits) {
        return Line::from(chrome::badge(count, theme));
    }
    Line::from(Span::styled(cut_to(label, width), style))
}

/// What the tab does not show, announced through the shared rule: each group
/// with the rows of it that are left out. A group whose header was dropped for
/// room is named as hidden, not as zero.
fn announcement(
    omitted: &[usize; 3],
    headers: &[bool; 3],
    present: &[bool; 3],
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    let groups: Vec<announce::Group> = GroupKind::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| announce::Group {
            label: kind.label().to_string(),
            style: group_style(*kind, theme),
            count: omitted[index],
            dropped: omitted[index] == 0 && !headers[index] && !present[index],
        })
        .collect();
    announce::lines(&groups, width, room, theme)
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

    /// The text of one built line.
    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    /// The widths of the fixed columns, with every column shown.
    fn metrics(groups: &[Group]) -> Metrics {
        Metrics {
            id: id_width(groups),
            kind: kind_width(groups),
            state: state_width(groups),
            columns: CANDIDATES[0],
        }
    }

    /// The metrics of the group that holds the row `id`, `width` wide.
    fn metrics_of(groups: &[Group], id: &str, width: usize) -> Metrics {
        let group = groups
            .iter()
            .find(|group| group.rows.iter().any(|row| row.id == id))
            .expect("a row of the tab");
        group_metrics(groups, group, width)
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
        body: Option<&str>,
        settled: bool,
    ) -> Row {
        Row {
            id: id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            scope: scope.map(str::to_string),
            state: state.to_string(),
            body: body.map(str::to_string),
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
        assert_eq!(groups[2].rows.len(), 6, "drafts");
        assert_eq!(
            groups[0].rows,
            vec![row(
                "ITM01",
                "idea",
                "The Overview says how long the run has been going",
                Some("the Overview's activity block"),
                "open",
                Some("The activity block says what happened; it does not say when the run began."),
                false,
            )]
        );
        assert_eq!(
            groups[1].rows,
            vec![
                row(
                    "ITM02",
                    "feature",
                    "The Path row names the round it waits on",
                    Some("the Path rows, the review state"),
                    "open",
                    Some("A task in review waits on a round number the Path row does not show."),
                    false,
                ),
                row(
                    "ITM03",
                    "bug",
                    "hugo says its configuration home could not be made",
                    Some("hugo's sidebar, the configuration home"),
                    "open",
                    Some(
                        "A home that cannot be created blanks the sidebar instead of naming the path."
                    ),
                    false,
                ),
                row(
                    "ITM04",
                    "feature",
                    "The light theme families",
                    Some("Catppuccin Latte, Nord Light, Dracula Alucard"),
                    "dismissed",
                    Some(
                        "Catppuccin Latte, Nord Light and Dracula Alucard would finish the families."
                    ),
                    true,
                ),
                row(
                    "ITM05",
                    "bug",
                    "The candidate row cuts a wide title",
                    Some("the candidate row"),
                    "promoted",
                    Some("The candidate row overflows when the title is wider than the row."),
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
                    "The Settings tab remembers the chosen theme",
                    None,
                    "draft",
                    None,
                    true,
                ),
                row(
                    "TSK02",
                    "draft",
                    "The footer says how many rows it hides",
                    None,
                    "draft",
                    None,
                    true,
                ),
                row(
                    "TSK03",
                    "draft",
                    "hugo opens the herdr tab and starts its agent",
                    None,
                    "draft",
                    None,
                    true,
                ),
                row(
                    "TSK27",
                    "draft",
                    "The candidate row cuts a wide title",
                    Some("the candidate row"),
                    "draft",
                    None,
                    true,
                ),
                row(
                    "TSK28",
                    "draft",
                    "The review nests a round with no findings",
                    None,
                    "draft",
                    None,
                    true,
                ),
                row(
                    "TSK30",
                    "draft",
                    "The review shows a finding that belongs to no round",
                    None,
                    "draft",
                    None,
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
            let text = text_of(&lines(&groups, 38, height, &theme(), IconSet::Nerd, None));
            for group in &groups {
                let name = group.kind.label();
                assert!(text.contains(name), "{height}: {name} is missing: {text}");
                assert!(
                    text.contains(&group.rows.len().to_string()),
                    "{height}: the count of {name} is missing: {text}"
                );
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
    fn an_empty_first_group_keeps_its_header_in_a_short_pane() {
        // No ideas and many small tasks: the empty IDEAS group leads the tab.
        // Its header is drawn, and the announcement does not call it hidden.
        let groups = [
            Group {
                kind: GroupKind::Ideas,
                rows: Vec::new(),
            },
            Group {
                kind: GroupKind::SmallTasks,
                rows: (1..=8)
                    .map(|n| {
                        row(
                            &format!("ITM{n:02}"),
                            "bug",
                            "a small task",
                            None,
                            "open",
                            None,
                            true,
                        )
                    })
                    .collect(),
            },
        ];
        for height in [3usize, 4, 5, 6] {
            let text = text_of(&lines(&groups, 60, height, &theme(), IconSet::Nerd, None));
            assert!(text.contains("IDEAS  0"), "{height}: {text}");
            assert!(!text.contains("IDEAS (hidden)"), "{height}: {text}");
        }
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
        let (rows, announced, shown, _) =
            window(&groups, 0, 1, &theme(), IconSet::Nerd, Some("ITM01"), 0);
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
            let text = text_of(&lines(&groups, width, 1, &theme(), IconSet::Nerd, None));
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
            columns: CANDIDATES[0],
        };
        let line = row_line(
            &groups[0].rows[0],
            1,
            metrics,
            IconSet::Nerd,
            &theme(),
            None,
        );
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
        // The group keeps its states once every one of its rows fits them: at
        // 110 inside the border every row of SMALL TASKS does.
        let rows = draw(&mock_work(), 112, 40).join("\n");
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
        let work = mock_work();
        let groups = groups(&work);
        let text = draw(&work, 100, 40).join("\n");
        for group in &groups {
            let heading = format!("{}  {}", group.kind.label(), group.rows.len());
            assert!(text.contains(&heading), "{heading} is missing: {text}");
        }
        let ideas = text.find("IDEAS").expect("the ideas heading");
        let small = text.find("SMALL TASKS").expect("the small tasks heading");
        let drafts = text.find("DRAFTS").expect("the drafts heading");
        assert!(ideas < small && small < drafts, "{text}");
    }

    #[test]
    fn a_wide_row_shows_every_column() {
        // A group shows its kind and state only when every one of its rows
        // fits them beside its title and scope: at 118 inside the border every
        // row of SMALL TASKS does, and the selection bar and the icon's lead
        // take their columns.
        let text = draw(&mock_work(), 120, 40).join("\n");
        let row = text
            .lines()
            .find(|line| line.contains("ITM04"))
            .unwrap_or_else(|| panic!("no ITM04 row: {text}"));
        assert!(row.contains("feature"), "{row}");
        assert!(row.contains("The light theme families"), "{row}");
        assert!(row.contains("dismissed"), "{row}");
        let row = text
            .lines()
            .find(|line| line.contains("TSK27"))
            .unwrap_or_else(|| panic!("no TSK27 row: {text}"));
        assert!(row.contains("the candidate row"), "{row}");
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
                let lines = lines(&groups, width, height, &theme(), IconSet::Nerd, None);
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
        for width in 1..=120 {
            let built = lines(&groups, width, 40, &theme(), IconSet::Nerd, None);
            let mut at = 0;
            for group in &groups {
                if group.rows.is_empty() {
                    continue;
                }
                at += 1; // the header
                let metrics = group_metrics(&groups, group, width);
                for row in &group.rows {
                    let text = built[at]
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>();
                    // The row's next line is its body, when it has one.
                    at += row_height(row);
                    let layout = layout(row, width.saturating_sub(view::BAR), metrics);
                    // The selection bar and the icon's lead come before the
                    // columns: the columns are what the test reads.
                    let lead = view::BAR + if layout.columns.icon { ICON_LEAD } else { 0 };
                    let text: String = text.chars().skip(lead).collect();
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
    fn a_row_shows_the_item_body_on_its_second_line() {
        // The body is the row's own second line, indented to where the row's
        // title starts and painted dim, so it reads as the row's and never
        // competes with the title.
        let work = mock_work();
        let groups = groups(&work);
        let metrics = metrics(&groups);
        let theme = theme();
        let row = &groups[0].rows[0];
        let body = row.body.clone().expect("the idea carries a body");
        let built = row_lines(row, 98, metrics, IconSet::Nerd, &theme, None);
        assert_eq!(built.len(), 2, "the row is not two lines");
        let layout = layout(row, 98 - view::BAR, metrics);
        let indent = " ".repeat(body_indent(&layout, metrics, 98));
        assert!(!indent.is_empty(), "the body is not indented");
        assert_eq!(line_text(&built[1]), format!("{indent}{body}"));
        let body_span = built[1]
            .spans
            .iter()
            .find(|span| !span.content.trim().is_empty())
            .expect("the body span");
        assert_eq!(body_span.style, theme.text.dim, "the body is not dim");
        let title_span = built[0]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == row.title)
            .expect("the title span");
        assert_eq!(title_span.style, theme.text.normal);
        // The screen draws the same two lines.
        let screen = draw(&work, 100, 40);
        let at = screen
            .iter()
            .position(|line| line.contains("ITM01"))
            .unwrap_or_else(|| panic!("no ITM01 row: {screen:#?}"));
        assert!(screen[at + 1].contains(&body), "{:#?}", &screen[at..at + 3]);
    }

    #[test]
    fn a_body_that_does_not_fit_is_cut_with_the_tab_marker() {
        // Only the body gives way: the columns keep their widths, and a cut
        // body always ends in the tab's own marker. The cut is on a grapheme
        // boundary, so half a grapheme is never drawn.
        let body = "é👩‍💻e\u{0301}界abc ".repeat(4);
        let mut item = Item::new("ITM01", "a title");
        item.body = body.clone();
        let work = work_of(Vec::new(), vec![item]);
        let groups = groups(&work);
        let metrics = metrics(&groups);
        let row = &groups[0].rows[0];
        let body = row.body.clone().expect("the body the row carries");
        assert_eq!(row_height(row), 2);
        for width in 1..=120 {
            let built = row_lines(row, width, metrics, IconSet::Nerd, &theme(), None);
            assert_eq!(built.len(), 2, "{width}: the row is not two lines");
            assert!(built[1].width() <= width, "{width}: a wider body");
            let layout = layout(row, width.saturating_sub(view::BAR), metrics);
            let indent = " ".repeat(body_indent(&layout, metrics, width));
            let text = line_text(&built[1]);
            let shown = text
                .strip_prefix(&indent)
                .unwrap_or_else(|| panic!("{width}: the body is not indented: {text:?}"));
            let head = shown.strip_suffix('…').unwrap_or(shown);
            assert!(
                is_grapheme_prefix(head, &body),
                "{width}: {shown:?} cuts a grapheme"
            );
            if text_width(&body) + indent.len() > width {
                assert!(
                    shown.ends_with('…'),
                    "{width}: a cut body reads as the whole one: {shown:?}"
                );
            }
        }
    }

    #[test]
    fn a_row_with_a_body_is_drawn_in_both_lines_or_in_neither() {
        // The row is atomic: its columns and its body are drawn together, so
        // the row the window counts is the row it draws. A two-line row is
        // never counted as one.
        let work = mock_work();
        let groups = groups(&work);
        let theme = theme();
        let rows: Vec<&Row> = groups.iter().flat_map(|group| group.rows.iter()).collect();
        for width in [40usize, 60, 98] {
            for height in 1..=14 {
                let (built, _, shown, _) =
                    window(&groups, width, height, &theme, IconSet::Nerd, None, 0);
                assert!(
                    built.len() <= height,
                    "{width}x{height}: {} lines",
                    built.len()
                );
                let texts: Vec<String> = built.lines.iter().map(line_text).collect();
                for index in &shown {
                    let row = rows[*index];
                    let metrics = metrics_of(&groups, &row.id, width);
                    for line in row_lines(row, width, metrics, IconSet::Nerd, &theme, None) {
                        let text = line_text(&line);
                        assert!(
                            texts.contains(&text),
                            "{width}x{height}: {} is drawn in half: {texts:#?}",
                            row.id
                        );
                    }
                }
                // A row with no body draws no second line at all.
                assert!(
                    !texts.iter().any(|text| text.trim().is_empty()),
                    "{width}x{height}: a blank second line: {texts:#?}"
                );
            }
        }
    }

    #[test]
    fn a_two_line_row_is_counted_as_two_in_the_announcement() {
        // At five content rows the idea's row fits only when its body line is
        // counted: the header, the row and its body fill three, the second
        // header one, and the announcement one. The next row is left out
        // whole rather than squeezed in as its columns alone.
        let work = mock_work();
        let groups = groups(&work);
        let (built, announced, shown, whole) =
            window(&groups, 98, 5, &theme(), IconSet::Nerd, None, 0);
        assert!(announced && whole, "the window did not choose to announce");
        assert_eq!(shown, vec![0], "the window shows one row");
        let text = text_of(&built.lines);
        assert!(text.contains("ITM01"), "{text}");
        assert!(text.contains("The activity block"), "{text}");
        assert!(!text.contains("ITM02"), "{text}");
        let small = groups[1].rows.len();
        assert!(
            text.contains(&format!("SMALL TASKS ({small} hidden)")),
            "{text}"
        );
    }

    #[test]
    fn a_two_line_row_is_drawn_whole_from_any_first_row() {
        // Scrolled to any first row, a shown two-line row is drawn with its
        // body under its columns, and the lines drawn fit the pane.
        let work = mock_work();
        let groups = groups(&work);
        let theme = theme();
        let rows: Vec<&Row> = groups.iter().flat_map(|group| group.rows.iter()).collect();
        assert!(
            rows.iter().any(|row| row.body.is_some()),
            "the mock holds no two-line row"
        );
        for first in 0..rows.len() {
            for height in 2..=12 {
                let (built, _, shown, _) =
                    window(&groups, 98, height, &theme, IconSet::Nerd, None, first);
                assert!(
                    built.len() <= height,
                    "{first}/{height}: {} lines in the pane",
                    built.len()
                );
                let drawn: Vec<String> = built.lines.iter().map(line_text).collect();
                for index in &shown {
                    let row = rows[*index];
                    let metrics = metrics_of(&groups, &row.id, 98);
                    let mut from = 0;
                    for line in row_lines(row, 98, metrics, IconSet::Nerd, &theme, None) {
                        let text = line_text(&line);
                        let at = drawn[from..].iter().position(|other| *other == text);
                        assert!(
                            at.is_some(),
                            "{first}/{height}: {} is drawn in half: {drawn:?}",
                            row.id
                        );
                        from += at.unwrap_or(0) + 1;
                    }
                }
            }
        }
    }

    #[test]
    fn a_zero_width_body_is_a_blank_line_as_the_columns_are() {
        // A pane with no content columns draws both lines of a two-line row
        // blank, so the row is counted as the two lines it takes.
        let work = mock_work();
        let groups = groups(&work);
        let row = groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .find(|row| row.body.is_some())
            .expect("a two-line row");
        let metrics = metrics_of(&groups, &row.id, 98);
        let lines = row_lines(row, 0, metrics, IconSet::Nerd, &theme(), None);
        assert_eq!(lines.len(), 2, "the row is not counted as two lines");
        assert!(
            lines.iter().all(|line| line_text(line).is_empty()),
            "{lines:?}"
        );
    }

    #[test]
    fn an_item_with_no_body_draws_the_row_it_draws_today() {
        // An item with no body is one line, byte for byte the row of today.
        let mut item = Item::new("ITM01", "only an idea");
        item.body = String::new();
        item.scope = Vec::new();
        let work = work_of(Vec::new(), vec![item]);
        let groups = groups(&work);
        let metrics = metrics(&groups);
        let row = &groups[0].rows[0];
        assert_eq!(row.body, None, "an empty body is not a body");
        assert_eq!(row_height(row), 1);
        assert_eq!(
            row_lines(row, 98, metrics, IconSet::Nerd, &theme(), None),
            vec![row_line(row, 98, metrics, IconSet::Nerd, &theme(), None)]
        );
        // Three headers and the row: no second line is drawn for it.
        let built = lines(&groups, 98, 40, &theme(), IconSet::Nerd, None);
        assert_eq!(built.len(), 4, "{built:#?}");
        assert!(text_of(&built).contains("only an idea"));
    }

    #[test]
    fn a_multi_line_body_is_drawn_as_the_one_body_it_is() {
        // The body is one sentence: its whitespace is normalised, never cut at
        // the first line, so the row never shows a summary of it.
        let mut item = Item::new("ITM01", "a title");
        item.body = "first line\nsecond   line".to_string();
        let work = work_of(Vec::new(), vec![item]);
        let groups = groups(&work);
        assert_eq!(
            groups[0].rows[0].body,
            Some("first line second line".to_string())
        );
    }

    #[test]
    fn the_cursor_is_never_on_a_row_drawn_in_half() {
        // At every height from two up, the cursor's row is drawn whole: its
        // columns and its body. Only a one-line pane cannot hold the body,
        // and there the row is drawn in the one line it is counted as.
        let work = mock_work();
        let groups = groups(&work);
        let theme = theme();
        let ids = row_ids(&work);
        let rows: Vec<&Row> = groups.iter().flat_map(|group| group.rows.iter()).collect();
        for id in &ids {
            let index = ids.iter().position(|other| other == id).expect("an id");
            let row = rows[index];
            let metrics = metrics_of(&groups, id, 98);
            for height in 2..=10 {
                let (built, _, _, _) =
                    window(&groups, 98, height, &theme, IconSet::Nerd, Some(id), 0);
                for line in row_lines(row, 98, metrics, IconSet::Nerd, &theme, Some(id)) {
                    let text = line_text(&line);
                    assert!(
                        built
                            .lines
                            .iter()
                            .any(|built_line| line_text(built_line) == text),
                        "{id} at {height}: the cursor's row is drawn in half: {built:#?}"
                    );
                }
            }
            // A one-line pane draws the columns line, and counts one row.
            let (built, announced, shown, _) =
                window(&groups, 98, 1, &theme, IconSet::Nerd, Some(id), 0);
            assert_eq!(built.len(), 1, "{id}: a one-line pane drew more");
            assert!(!announced, "{id}: a one-line pane announced");
            assert_eq!(shown, vec![index], "{id}: the cursor's row is not counted");
            assert_eq!(
                line_text(&built.lines[0]),
                line_text(&row_line(row, 98, metrics, IconSet::Nerd, &theme, Some(id))),
                "{id}: a one-line pane did not draw the row"
            );
        }
    }

    #[test]
    fn a_selected_row_paints_its_body_with_the_selection_too() {
        let theme = theme();
        let groups = groups(&mock_work());
        let metrics = metrics(&groups);
        let row = &groups[0].rows[0];
        let selection = theme.text.selected_bg;
        let built = row_lines(row, 98, metrics, IconSet::Nerd, &theme, Some("ITM01"));
        assert_eq!(built.len(), 2);
        // The bar is on the first line only, in its own paint: the rest of
        // both lines is the selection's.
        assert_eq!(built[0].spans[0].content, "▌");
        for (index, line) in built.iter().enumerate() {
            assert_eq!(line.width(), 98, "the selection bar stops short");
            let painted = if index == 0 {
                &line.spans[1..]
            } else {
                &line.spans[..]
            };
            for span in painted {
                assert_eq!(span.style.bg, selection.bg, "{span:?} is not the selection");
            }
        }
    }

    #[test]
    fn the_title_and_the_scope_keep_their_room() {
        // The icon, the kind, the state and then the id give way, whole,
        // before the title and the scope are cut: at 81 the title and the scope
        // of ITM02 are still whole, and its group's ids have given way with the
        // rest of the group's fixed columns.
        let groups = groups(&mock_work());
        let text = text_of(&lines(&groups, 81, 40, &theme(), IconSet::Nerd, None));
        let row = text
            .lines()
            .find(|line| line.contains("The Path row"))
            .unwrap_or_else(|| panic!("no 'The Path row' row: {text}"));
        assert!(
            row.contains("The Path row names the round it waits on"),
            "{row}"
        );
        assert!(row.contains("the Path rows"), "{row}");
        assert!(!row.contains("feature"), "{row}");
        // Narrower still: the id gives way before the title and the scope,
        // which keep their room.
        let text = text_of(&lines(&groups, 58, 40, &theme(), IconSet::Nerd, None));
        assert!(!text.contains("ITM02"), "{text}");
        assert!(
            text.contains("The Path row names the roun…"),
            "the title is cut without its marker: {text}"
        );
        assert!(text.contains("the Path rows"), "{text}");
        let row = text
            .lines()
            .find(|line| line.contains("The Path row"))
            .expect("the smallest row");
        assert!(!row.contains("feature"), "{row}");
    }

    #[test]
    fn a_narrow_row_cuts_the_title_and_the_scope_and_nothing_else() {
        let groups = groups(&mock_work());
        for width in 4..=40 {
            let lines = lines(&groups, width, 40, &theme(), IconSet::Nerd, None);
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
    fn the_counts_are_strong_the_ids_strong_and_the_titles_normal() {
        let theme = theme();
        let groups = groups(&mock_work());
        let lines = lines(&groups, 100, 40, &theme, IconSet::Nerd, None);
        for (index, heading) in ["IDEAS", "SMALL TASKS", "DRAFTS"].into_iter().enumerate() {
            let count = groups[index].rows.len().to_string();
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
        assert_eq!(title_span.style, theme.text.normal);
        let id = &groups[0].rows[0].id;
        let id_span = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.trim() == id)
            .unwrap_or_else(|| panic!("no id span"));
        assert_eq!(id_span.style, theme.text.strong);
    }

    #[test]
    fn a_draft_or_settled_row_is_dim() {
        let theme = theme();
        let groups = groups(&mock_work());
        let lines = lines(&groups, 100, 40, &theme, IconSet::Nerd, None);
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
        let selection = theme.text.selected_bg;
        let groups = groups(&mock_work());
        let metrics = Metrics {
            id: id_width(&groups),
            kind: kind_width(&groups),
            state: state_width(&groups),
            columns: CANDIDATES[0],
        };
        let open = &groups[0].rows[0];
        let settled = &groups[1].rows[2];
        // A row that is not the cursor's keeps its own paint.
        let plain = row_line(open, 100, metrics, IconSet::Nerd, &theme, None);
        assert_eq!(span_style(&plain, "ITM01"), theme.text.strong, "an open id");
        let plain = row_line(settled, 100, metrics, IconSet::Nerd, &theme, None);
        assert_eq!(span_style(&plain, "ITM04"), theme.text.dim, "a settled id");
        // The cursor's row takes the selection's pair on every cell after its
        // bar, and the bar runs to the edge of the pane. The rule is a
        // narrow-pane rule as much as a wide one: at 60 the title and the
        // scope are cut, and the bar still reaches the edge.
        for width in [60, 100] {
            let selected = row_line(open, width, metrics, IconSet::Nerd, &theme, Some("ITM01"));
            assert_eq!(
                selected.width(),
                width,
                "{width}: the selection bar stops short"
            );
            assert_eq!(selected.spans[0].content, "▌");
            assert_eq!(selected.spans[0].style, theme.chrome.border_focus);
            for span in &selected.spans[1..] {
                assert_eq!(
                    span.style.bg, selection.bg,
                    "{width}: {span:?} is not the selection"
                );
            }
        }
        // Another row is not painted as the selection, and keeps its bar's
        // column blank.
        let other = row_line(settled, 100, metrics, IconSet::Nerd, &theme, Some("ITM01"));
        assert_eq!(other.spans[0].content, " ");
        assert_eq!(span_style(&other, "ITM04"), theme.text.dim);
    }

    #[test]
    fn a_row_leads_with_its_kind_icon_and_a_draft_with_the_draft_icon() {
        // At 120 the lead fits beside the whole row: after the bar's column,
        // an idea leads with its kind's icon and a draft with the draft
        // phase's.
        let groups = groups(&mock_work());
        let metrics = metrics(&groups);
        let theme = theme();
        for (row, icon) in [
            (&groups[0].rows[0], Icon::KindIdea),
            (&groups[2].rows[0], Icon::StatusDraft),
        ] {
            let line = row_line(row, 120, metrics, IconSet::Unicode, &theme, None);
            assert_eq!(line.spans[0].content, " ", "{}: the bar's column", row.id);
            assert_eq!(line.spans[1].content, " ", "{}", row.id);
            assert_eq!(
                line.spans[2].content,
                icon.glyph(IconSet::Unicode),
                "{}",
                row.id
            );
            assert_eq!(line.spans[3].content, " ", "{}", row.id);
            assert_eq!(line.spans[4].content, row.id, "{}", row.id);
        }
    }

    #[test]
    fn the_ids_of_a_group_share_a_column() {
        // The columns are decided for the group: at 100 and 120 every row
        // keeps its id; at 80 the group gives its ids way together. Either way
        // the ids of a group share a column, or none of its rows shows one.
        let groups = groups(&mock_work());
        let theme = theme();
        for width in [80, 100, 120] {
            for icons in [IconSet::Nerd, IconSet::Unicode] {
                for group in &groups {
                    let metrics = group_metrics(&groups, group, width);
                    let starts: Vec<Option<usize>> = group
                        .rows
                        .iter()
                        .map(|row| {
                            let text =
                                line_text(&row_line(row, width, metrics, icons, &theme, None));
                            text.find(&row.id).map(|at| text_width(&text[..at]))
                        })
                        .collect();
                    if width >= 100 {
                        assert!(
                            starts.iter().all(Option::is_some),
                            "width {width}: a row lost its id"
                        );
                    }
                    assert!(
                        starts.windows(2).all(|pair| pair[0] == pair[1]),
                        "width {width}: the ids start at {starts:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_group_shows_the_same_icon_id_kind_and_state_on_every_row() {
        // The icon, the id, the kind and the state are decided for the group: a
        // row does not drop one of them while the row beside it keeps it, so
        // the titles of a group start in one column. The scope stays the row's
        // own: a row without one has nothing to show.
        let groups = groups(&mock_work());
        for width in 1..=120 {
            for group in &groups {
                let metrics = group_metrics(&groups, group, width);
                let shown: Vec<(bool, bool, bool, bool)> = group
                    .rows
                    .iter()
                    .map(|row| {
                        let columns = layout(row, width.saturating_sub(view::BAR), metrics).columns;
                        (columns.icon, columns.id, columns.kind, columns.state)
                    })
                    .collect();
                assert!(
                    shown.windows(2).all(|pair| pair[0] == pair[1]),
                    "width {width}: the columns of a group differ: {shown:?}"
                );
            }
        }
    }

    #[test]
    fn a_group_keeps_its_icons_when_another_group_gives_them_up() {
        // SMALL TASKS holds the long row that cannot keep its icon, and only
        // that group gives its icons up: at 100 the drafts keep theirs, and at
        // 120 the ideas and the drafts do.
        let groups = groups(&mock_work());
        let theme = theme();
        assert!(!group_metrics(&groups, &groups[1], 100).columns.icon);
        for (width, index, icon) in [
            (100, 2, Icon::StatusDraft),
            (120, 0, Icon::KindIdea),
            (120, 2, Icon::StatusDraft),
        ] {
            let group = &groups[index];
            let metrics = group_metrics(&groups, group, width);
            for row in &group.rows {
                let line = row_line(row, width, metrics, IconSet::Unicode, &theme, None);
                assert_eq!(
                    line.spans[2].content,
                    icon.glyph(IconSet::Unicode),
                    "{} at {width}",
                    row.id
                );
            }
        }
    }

    /// The style of the span that holds `text`.
    fn span_style(line: &Line<'_>, text: &str) -> Style {
        line.spans
            .iter()
            .find(|span| span.content.trim() == text)
            .unwrap_or_else(|| panic!("no span {text:?} in {line:?}"))
            .style
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
        assert!(text.contains("IDEAS  1"), "{text}");
        assert!(text.contains("SMALL TASKS  0"), "{text}");
        assert!(text.contains("DRAFTS  0"), "{text}");
        let ideas = text.find("IDEAS  1").expect("the ideas header");
        let small = text.find("SMALL TASKS  0").expect("the small tasks header");
        let drafts = text.find("DRAFTS  0").expect("the drafts header");
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
        // At 100x10 the SMALL TASKS header holds the whole group's count, and
        // the announcement says what is left out of view instead.
        let work = mock_work();
        let small = groups(&work)[1].rows.len();
        let text = draw(&work, 100, 10).join("\n");
        assert!(text.contains(&format!("SMALL TASKS  {small}")), "{text}");
        assert!(
            text.contains(&format!("SMALL TASKS ({} hidden)", small - 1)),
            "{text}"
        );
        assert!(
            !text.contains(&format!("SMALL TASKS  {}", small - 1)),
            "{text}"
        );
    }

    #[test]
    fn every_height_that_omits_a_row_says_so() {
        let work = mock_work();
        let groups = groups(&work);
        // The pane counts lines: a row with a body is two of them.
        let wanted: usize = groups
            .iter()
            .map(|group| 1 + group.rows.iter().map(row_height).sum::<usize>())
            .sum();
        for height in 1..=wanted + 2 {
            let built = lines(&groups, 100, height, &theme(), IconSet::Nerd, None);
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
        let work = mock_work();
        let drafts = groups(&work)[2].rows.len();
        let cells = draw_cells(&work, 100, 40);
        let row = cells
            .iter()
            .find(|row| {
                let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
                text.contains("DRAFTS") && text.contains(&format!("DRAFTS  {drafts}"))
            })
            .unwrap_or_else(|| panic!("no DRAFTS header"));
        assert_eq!(
            colour_at(row, "DRAFTS"),
            Some(&dim),
            "the draft heading is not dim"
        );
        let count = row.iter().map(|(symbol, _)| *symbol).collect::<Vec<char>>();
        let at = count
            .windows(2)
            .position(|window| window == [' ', ' '])
            .expect("the DRAFTS count");
        let colour = row
            .get(at + 2)
            .map(|(_, colour)| *colour)
            .expect("the DRAFTS count");
        assert_eq!(colour, strong, "the draft count is not strong");
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
            let built = lines(&groups, width, 40, &theme(), IconSet::Nerd, None);
            assert!(
                too_wide(&built, width).is_empty(),
                "{width}: {:?}",
                too_wide(&built, width)
            );
        }
    }
}
