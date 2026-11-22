//! The Tasks tab: every task file, in any status.
//!
//! The index of the work. Drafts and finished tasks are here too, so a task
//! is reachable without knowing its id. A row is the Needs you row: the status
//! icon, the id and the title, with the value of the sort key in force on the
//! right. A subtask carries its parent in a suffix and is never nested.
//!
//! The order belongs to the window, not to the tab: `o` cycles the key and
//! `O` reverses it. A categorical key groups the rows under headings.

use std::cmp::Ordering;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use coding_crew_core::task::{Effort, Priority};
use coding_crew_core::{Status, Task};

use crate::announce;
use crate::data::{STATUSES, Work};
use crate::filter::Filter;
use crate::view::{self, View};
use coding_crew_theme::chrome;
use coding_crew_theme::text::{cut_to, pad, text_width};
use coding_crew_theme::{Icon, IconSet, Theme, phase_style, selected_line};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between two columns.
const GAP: usize = 2;
/// The columns before a row's id: a space, the status icon and a space.
const LEAD: usize = 3;
/// The width of the id column, when every id is shorter.
const ID_WIDTH: usize = 5;
/// The margin at the right of a row.
const MARGIN: usize = 1;
/// The narrowest a title is shown in before a column gives way.
const MIN_TITLE: usize = 8;

/// The priorities, most urgent first: the order `priority` sorts by.
pub(crate) const PRIORITIES: [Priority; 4] = [
    Priority::Urgent,
    Priority::High,
    Priority::Normal,
    Priority::Low,
];

/// The efforts, smallest first: the order `effort` sorts by.
pub(crate) const EFFORTS: [Effort; 3] = [Effort::Small, Effort::Medium, Effort::Large];

/// The key the rows are ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Key {
    /// The id: the default, and the order the work folder is read in.
    #[default]
    Id,
    /// The creation stamp.
    Created,
    /// The last update stamp.
    Updated,
    /// The lifecycle: draft first, done last.
    Status,
    /// The priority: urgent first.
    Priority,
    /// The effort: small first.
    Effort,
}

impl Key {
    /// Every key, in the order `o` cycles them.
    pub const ALL: [Key; 6] = [
        Key::Id,
        Key::Created,
        Key::Updated,
        Key::Status,
        Key::Priority,
        Key::Effort,
    ];

    /// Whether the key is a category: its rows sit under headings.
    pub fn categorical(self) -> bool {
        matches!(self, Key::Status | Key::Priority | Key::Effort)
    }

    /// The key's name, as the configuration file spells it.
    pub fn name(self) -> &'static str {
        match self {
            Key::Id => "id",
            Key::Created => "created",
            Key::Updated => "updated",
            Key::Status => "status",
            Key::Priority => "priority",
            Key::Effort => "effort",
        }
    }

    /// The key of a name, if there is one.
    pub fn from_name(name: &str) -> Option<Self> {
        Key::ALL.into_iter().find(|key| key.name() == name)
    }

    /// The key `o` moves to.
    fn next(self) -> Key {
        let index = Key::ALL
            .iter()
            .position(|key| *key == self)
            .unwrap_or_default();
        Key::ALL[(index + 1) % Key::ALL.len()]
    }
}

/// The order of the rows: the key they are sorted by, and whether they run
/// backwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Order {
    pub key: Key,
    pub reverse: bool,
}

impl Order {
    /// The order `o` moves to: the next key, in the same direction.
    pub fn cycled(self) -> Order {
        Order {
            key: self.key.next(),
            ..self
        }
    }

    /// The order `O` moves to: the same key, the other way.
    pub fn reversed(self) -> Order {
        Order {
            reverse: !self.reverse,
            ..self
        }
    }

    /// The order of two tasks: the key, in the direction in force, and the id
    /// when the key ties. The id breaks a tie either way, so the order is
    /// stable.
    pub fn compare(self, one: &Task, other: &Task) -> Ordering {
        let by_key = match self.key {
            Key::Id => one.id.cmp(&other.id),
            Key::Created => one.created.cmp(&other.created),
            Key::Updated => one.updated.cmp(&other.updated),
            Key::Status => rank(&STATUSES, one.status).cmp(&rank(&STATUSES, other.status)),
            Key::Priority => {
                rank(&PRIORITIES, one.meta.priority).cmp(&rank(&PRIORITIES, other.meta.priority))
            }
            Key::Effort => rank(&EFFORTS, one.meta.effort).cmp(&rank(&EFFORTS, other.meta.effort)),
        };
        let by_key = if self.reverse {
            by_key.reverse()
        } else {
            by_key
        };
        by_key.then_with(|| one.id.cmp(&other.id))
    }

    /// The value of a task on the key: the right column of its row, and the
    /// heading of its group.
    fn value(self, task: &Task) -> String {
        match self.key {
            Key::Id | Key::Status => task.status.as_str().to_string(),
            Key::Created => date(&task.created),
            Key::Updated => date(&task.updated),
            Key::Priority => task.meta.priority.as_str().to_string(),
            Key::Effort => task.meta.effort.as_str().to_string(),
        }
    }
}

/// The place of a value in the order it sorts by.
fn rank<T: PartialEq>(order: &[T], value: T) -> usize {
    order
        .iter()
        .position(|other| *other == value)
        .unwrap_or(usize::MAX)
}

/// The day a stamp was made: its first ten characters.
fn date(stamp: &str) -> String {
    stamp.chars().take(10).collect()
}

/// A task row: what the tab shows of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The id of the task.
    pub id: String,
    /// The title of the task.
    pub title: String,
    /// The status the task is in: its icon and its paint.
    pub status: Status,
    /// The parent, when the task is a subtask.
    pub parent: Option<String>,
    /// The value of the sort key: the right column.
    pub value: String,
}

/// The rows under one heading: a value of a categorical key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The value the rows share. Empty when the order has no headings.
    pub value: String,
    /// The status the heading names, when the order is by status.
    pub status: Option<Status>,
    /// The rows, in the order of the tab.
    pub rows: Vec<Row>,
}

/// The groups of the tab, in the order of its rows.
///
/// A categorical order holds one group per value, the headings in the order
/// the key runs. A flat order holds one group with no heading. Only the tasks
/// the filter text lets through are rows.
pub fn groups(work: &Work, order: Order, filter: &str) -> Vec<Group> {
    let filter = Filter::parse(filter);
    let mut tasks: Vec<&Task> = work
        .tasks
        .iter()
        .filter(|task| filter.matches(task))
        .collect();
    tasks.sort_by(|one, other| order.compare(one, other));
    let mut groups: Vec<Group> = Vec::new();
    for task in tasks {
        let value = if order.key.categorical() {
            order.value(task)
        } else {
            String::new()
        };
        let row = Row {
            id: task.id.clone(),
            title: task.title.clone(),
            status: task.status,
            parent: task.parent.clone(),
            value: order.value(task),
        };
        match groups.last_mut() {
            Some(group) if group.value == value => group.rows.push(row),
            _ => groups.push(Group {
                value,
                status: (order.key == Key::Status).then_some(task.status),
                rows: vec![row],
            }),
        }
    }
    groups
}

/// The ids of the tab's rows, in the order they are shown.
///
/// The detail pane opens from this tab: the rows are what it can open.
pub fn row_ids(work: &Work, order: Order, filter: &str) -> Vec<String> {
    groups(work, order, filter)
        .into_iter()
        .flat_map(|group| group.rows.into_iter().map(|row| row.id))
        .collect()
}

/// The widths the columns of every row share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    /// The widest id, and never narrower than the id column.
    id: usize,
    /// The widest value of the sort key.
    value: usize,
}

impl Columns {
    fn of(groups: &[Group]) -> Self {
        let rows = || groups.iter().flat_map(|group| group.rows.iter());
        Columns {
            id: rows()
                .map(|row| text_width(&row.id))
                .max()
                .unwrap_or(0)
                .max(ID_WIDTH),
            value: rows().map(|row| text_width(&row.value)).max().unwrap_or(0),
        }
    }
}

/// One line of the tab before the window: a heading, or the row at an index
/// of the tab's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Heading(usize),
    Row(usize),
}

/// The lines of the tab, one cell each, in the order they are drawn.
fn cells(groups: &[Group]) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut index = 0;
    for (group_index, group) in groups.iter().enumerate() {
        if !group.value.is_empty() {
            cells.push(Cell::Heading(group_index));
        }
        for _ in &group.rows {
            cells.push(Cell::Row(index));
            index += 1;
        }
    }
    cells
}

/// Draw the tab through the window the keys move.
///
/// `selected` is the row the cursor is on: its cells take the selection's
/// paint. The window is settled so the cursor is on a row the pane shows.
/// Returns the hidden count when the pane has no room to announce it: the
/// footer carries the count then.
#[allow(clippy::too_many_arguments)]
pub fn render_view(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    theme: &Theme,
    icons: IconSet,
    selected: Option<&str>,
    view: &mut View,
    order: Order,
    filter: &str,
) -> Option<usize> {
    let block = chrome::pane("tasks", Icon::TabTasks, icons, false, theme);
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
    if work.tasks.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no tasks yet", theme.text.dim))).block(block),
            area,
        );
        return None;
    }
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let groups = groups(work, order, filter);
    if groups.is_empty() {
        // Only the filter empties the tab: a folder with no tasks was named above.
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("no task matches", theme.text.dim)),
                Line::from(Span::styled(format!("/ {filter}"), theme.text.normal)),
            ])
            .wrap(Wrap { trim: true })
            .block(block),
            area,
        );
        return None;
    }
    let columns = Columns::of(&groups);
    let rows: Vec<&Row> = groups.iter().flat_map(|group| group.rows.iter()).collect();
    let line_cells = cells(&groups);
    let ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
    let (rows_drawn, announced) = view.draw(&ids, selected, |first| {
        let (lines_drawn, announced, shown, whole) = window(
            &groups,
            &rows,
            &line_cells,
            columns,
            width,
            height,
            theme,
            icons,
            selected,
            first,
        );
        ((lines_drawn, announced), shown, whole)
    });
    frame.render_widget(
        Paragraph::new(rows_drawn)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
    let hidden = ids.len().saturating_sub(view.page());
    (hidden > 0 && !announced).then_some(hidden)
}

/// The lines of the tab from the row at `first`, the rows they show, whether
/// the pane announced what it hides, and whether the window may stop there.
///
/// The announcement sits at the foot of the pane and names what the window
/// leaves out, group by group under a categorical order and as one count
/// under a flat one. The rows give way to it: the window keeps the most rows
/// that leave the announcement whole, and a pane too short for that cuts it
/// with `…`. The window starts with the heading of the first row's group when
/// the row opens that group.
#[allow(clippy::too_many_arguments)]
fn window(
    groups: &[Group],
    rows: &[&Row],
    lines: &[Cell],
    columns: Columns,
    width: usize,
    height: usize,
    theme: &Theme,
    icons: IconSet,
    selected: Option<&str>,
    first: usize,
) -> (Vec<Line<'static>>, bool, Vec<usize>, bool) {
    if height == 0 {
        return (Vec::new(), false, Vec::new(), false);
    }
    let total = rows.len();
    let start = lines
        .iter()
        .position(|cell| *cell == Cell::Row(first))
        .map_or(0, |at| {
            match at.checked_sub(1).map(|before| lines[before]) {
                Some(Cell::Heading(_)) => at - 1,
                _ => at,
            }
        });
    let draw = |capacity: usize| {
        let mut taken: Vec<Cell> = lines[start..].iter().take(capacity).copied().collect();
        // A heading is drawn with a row under it, or not at all.
        if matches!(taken.last(), Some(Cell::Heading(_))) {
            taken.pop();
        }
        let mut out = Vec::new();
        let mut shown = Vec::new();
        for cell in taken {
            match cell {
                Cell::Heading(group) => out.push(heading(&groups[group], width, theme)),
                Cell::Row(index) => {
                    out.push(row_line(
                        rows[index],
                        width,
                        columns,
                        icons,
                        theme,
                        selected,
                    ));
                    shown.push(index);
                }
            }
        }
        (out, shown)
    };
    let (out, shown) = draw(height);
    if shown.len() == total {
        return (out, false, shown, true);
    }
    if height == 1 {
        return (out, false, shown, false);
    }
    // The most rows, from the top, that leave the announcement whole, and at
    // least one row: a window of headings alone draws no row to keep. When no
    // such window is whole, the rows give the announcement one line and it is
    // cut.
    let announce_within = |capacity: usize| {
        let (mut out, shown) = draw(capacity);
        let room = height - out.len();
        let (said, cut) = foot(groups, &shown, total - shown.len(), width, room, theme);
        let announced = !said.is_empty();
        out.extend(said);
        (out, announced, shown, cut)
    };
    let (out, announced, shown, _) = (1..height)
        .rev()
        .map(&announce_within)
        .find(|(_, _, shown, cut)| !cut && !shown.is_empty())
        .unwrap_or_else(|| announce_within(height - 1));
    (out, announced, shown, true)
}

/// The announcement at the foot of the window, in at most `room` lines: the
/// groups the window leaves out with their counts, or under a flat order the
/// count of hidden rows. Returns the lines and whether they are cut.
fn foot(
    groups: &[Group],
    shown: &[usize],
    hidden: usize,
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    if groups.iter().all(|group| group.value.is_empty()) {
        return (vec![announcement(hidden, width, theme)], false);
    }
    let mut start = 0;
    let omitted: Vec<announce::Group> = groups
        .iter()
        .map(|group| {
            let end = start + group.rows.len();
            let shown_here = shown
                .iter()
                .filter(|index| (start..end).contains(index))
                .count();
            start = end;
            announce::Group {
                label: group.value.to_uppercase(),
                style: group_style(group, theme),
                count: group.rows.len() - shown_here,
            }
        })
        .collect();
    announce::lines(&omitted, width, room, theme)
}

/// The paint of a group's label: a status group takes its phase's colour, any
/// other group the plain text.
fn group_style(group: &Group, theme: &Theme) -> Style {
    match group.status {
        Some(status) => phase_style(theme, status),
        None => theme.text.normal,
    }
}

/// The heading of a group: its value in capitals, and the count of its rows
/// as a badge.
///
/// The count is never cut. When the pane is too narrow for the whole heading
/// the value gives way first, and a count never shows without its value.
fn heading(group: &Group, width: usize, theme: &Theme) -> Line<'static> {
    let label = group.value.to_uppercase();
    let count = group.rows.len();
    let digits = count.to_string();
    let style = group_style(group, theme);
    let separator = "  ";
    let fixed = text_width(separator) + text_width(&digits);
    if width >= text_width(&label) + fixed {
        return Line::from(vec![
            Span::styled(label, style),
            Span::raw(separator),
            chrome::badge(count, theme),
        ]);
    }
    if width > fixed {
        return Line::from(vec![
            Span::styled(cut_to(&label, width - fixed), style),
            Span::raw(separator),
            chrome::badge(count, theme),
        ]);
    }
    if width >= text_width(&digits) {
        return Line::from(chrome::badge(count, theme));
    }
    Line::from(Span::styled(cut_to(&label, width), style))
}

/// The announcement: how many rows the window does not show.
fn announcement(hidden: usize, width: usize, theme: &Theme) -> Line<'static> {
    let count = hidden.to_string();
    if text_width(&count) + text_width(" hidden") <= width {
        return Line::from(vec![
            Span::styled(count, theme.text.strong),
            Span::styled(" hidden".to_string(), theme.text.dim),
        ]);
    }
    Line::from(Span::styled(cut_to(&count, width), theme.text.strong))
}

/// A row, barred: the selection bar, then the row in the columns after it.
fn row_line(
    row: &Row,
    width: usize,
    columns: Columns,
    icons: IconSet,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    let selected = selected == Some(row.id.as_str());
    // The bar gives way before the id is cut, as the Needs you row does.
    if width < view::BAR + columns.id {
        let line = row_line_plain(row, width, columns, icons, theme);
        return if selected {
            selected_line(line, width, theme)
        } else {
            line
        };
    }
    let line = row_line_plain(row, width - view::BAR, columns, icons, theme);
    view::barred(line, width, selected, icons, theme)
}

/// A row in its own paint, before the selection.
///
/// The columns give way whole, in order: the parent suffix, then the value of
/// the sort key. The title keeps `MIN_TITLE` columns before either gives way.
/// The id is never cut while a row can show it whole.
fn row_line_plain(
    row: &Row,
    width: usize,
    columns: Columns,
    icons: IconSet,
    theme: &Theme,
) -> Line<'static> {
    let icon = Icon::for_status(row.status.as_str()).expect("every status has a phase icon");
    let head = LEAD + columns.id + GAP;
    let suffix = row
        .parent
        .as_ref()
        .map(|parent| format!("{} {parent}", Icon::Parent.glyph(icons)));
    let suffix_width = suffix.as_ref().map_or(0, |suffix| GAP + text_width(suffix));
    let value_width = GAP + columns.value + MARGIN;
    for (with_suffix, with_value) in [(true, true), (false, true), (false, false)] {
        let tail =
            if with_suffix { suffix_width } else { 0 } + if with_value { value_width } else { 0 };
        let Some(title) = width.checked_sub(head + tail) else {
            continue;
        };
        if title < MIN_TITLE && (with_suffix || with_value) {
            continue;
        }
        let mut spans = vec![
            Span::raw(" "),
            Span::styled(icon.glyph(icons), phase_style(theme, row.status)),
            Span::raw(" "),
            Span::styled(
                pad(&cut_to(&row.id, columns.id), columns.id),
                theme.text.strong,
            ),
            Span::raw(" ".repeat(GAP)),
            Span::styled(pad(&cut_to(&row.title, title), title), theme.text.normal),
        ];
        if let Some(suffix) = suffix.as_ref().filter(|_| with_suffix) {
            spans.push(Span::raw(" ".repeat(GAP)));
            spans.push(Span::styled(suffix.clone(), theme.text.dim));
        }
        if with_value {
            spans.push(Span::raw(" ".repeat(GAP)));
            spans.push(Span::styled(
                format!("{:>width$}", row.value, width = columns.value),
                theme.text.dim,
            ));
            spans.push(Span::raw(" ".repeat(MARGIN)));
        }
        return Line::from(spans);
    }
    // Nothing but the id fits.
    Line::from(Span::styled(cut_to(&row.id, width), theme.text.strong))
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_crew_theme::Preset;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// A task in a status, with a priority and an effort.
    fn task(id: &str, status: Status, priority: Priority, effort: Effort) -> Task {
        let mut task = Task::new(id, format!("title of {id}"));
        task.status = status;
        task.meta.priority = priority;
        task.meta.effort = effort;
        task.created = format!("2026-11-{:02}T09:00:00Z", id.len() + 10);
        task.updated = "2026-11-21T10:00:00Z".to_string();
        task
    }

    /// Every status, once, with the ids TSK01 to TSK08 in order.
    fn every_status() -> Work {
        let tasks = [
            Status::Draft,
            Status::Groomed,
            Status::Approved,
            Status::Executing,
            Status::CodeReview,
            Status::Remediating,
            Status::NeedsHuman,
            Status::Done,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, status)| {
            task(
                &format!("TSK{:02}", index + 1),
                status,
                Priority::Normal,
                Effort::Medium,
            )
        })
        .collect();
        Work {
            tasks,
            ..Work::default()
        }
    }

    fn by(key: Key) -> Order {
        Order {
            key,
            reverse: false,
        }
    }

    fn ids(work: &Work, order: Order) -> Vec<String> {
        row_ids(work, order, "")
    }

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// The tab drawn at a size, as plain text.
    fn draw(
        work: &Work,
        order: Order,
        width: u16,
        height: u16,
        selected: Option<&str>,
    ) -> Vec<String> {
        draw_filtered(work, order, width, height, selected, "")
    }

    /// The tab drawn at a size through a filter, as plain text.
    fn draw_filtered(
        work: &Work,
        order: Order,
        width: u16,
        height: u16,
        selected: Option<&str>,
        filter: &str,
    ) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("a terminal");
        let mut view = View::default();
        let theme = theme();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_view(
                    frame,
                    area,
                    work,
                    &theme,
                    IconSet::Unicode,
                    selected,
                    &mut view,
                    order,
                    filter,
                );
            })
            .expect("a frame");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect()
    }

    /// The rows of a flat list of lines drawn with no heading and no bar, for
    /// the width checks.
    fn plain_lines(work: &Work, order: Order, width: usize, icons: IconSet) -> Vec<Line<'static>> {
        let groups = groups(work, order, "");
        let columns = Columns::of(&groups);
        groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .map(|row| row_line(row, width, columns, icons, &theme(), None))
            .collect()
    }

    fn line_width(line: &Line<'_>) -> usize {
        line.spans
            .iter()
            .map(|span| text_width(&span.content))
            .sum()
    }

    #[test]
    fn every_task_is_a_row_in_every_status() {
        // Drafts and finished tasks are rows too: the tab is the index.
        let work = every_status();
        let shown = ids(&work, by(Key::Id));
        assert_eq!(shown.len(), 8, "{shown:?}");
        assert_eq!(shown.first().map(String::as_str), Some("TSK01"));
        assert!(shown.contains(&"TSK08".to_string()));
    }

    #[test]
    fn the_default_order_is_the_id_ascending() {
        let order = Order::default();
        assert_eq!(order.key, Key::Id);
        assert!(!order.reverse);
    }

    #[test]
    fn o_cycles_every_key_and_returns_to_the_first() {
        let mut order = Order::default();
        let mut seen = Vec::new();
        for _ in 0..Key::ALL.len() {
            seen.push(order.key);
            order = order.cycled();
        }
        assert_eq!(seen, Key::ALL);
        assert_eq!(order.key, Key::Id, "six presses come back to the id");
    }

    #[test]
    fn o_keeps_the_direction_and_big_o_keeps_the_key() {
        let order = Order {
            key: Key::Priority,
            reverse: true,
        };
        assert_eq!(order.cycled().key, Key::Effort);
        assert!(order.cycled().reverse, "o changed the direction");
        let reversed = Order::default().reversed();
        assert_eq!(reversed.key, Key::Id);
        assert!(reversed.reverse);
        assert!(!reversed.reversed().reverse, "O is its own undo");
    }

    #[test]
    fn ties_break_by_id_whatever_the_direction() {
        // Two tasks in one status: the id decides their order, forwards and
        // back, so the order is stable.
        let work = Work {
            tasks: vec![
                task("TSK02", Status::Groomed, Priority::High, Effort::Small),
                task("TSK01", Status::Groomed, Priority::High, Effort::Small),
            ],
            ..Work::default()
        };
        let forward = ids(
            &work,
            Order {
                key: Key::Priority,
                reverse: false,
            },
        );
        let back = ids(
            &work,
            Order {
                key: Key::Priority,
                reverse: true,
            },
        );
        assert_eq!(forward, ["TSK01", "TSK02"]);
        assert_eq!(back, ["TSK01", "TSK02"]);
    }

    #[test]
    fn status_runs_in_the_lifecycle_order() {
        let work = every_status();
        let by_status = ids(&work, by(Key::Status));
        assert_eq!(
            by_status,
            [
                "TSK01", "TSK02", "TSK03", "TSK04", "TSK05", "TSK06", "TSK07", "TSK08"
            ],
            "draft first, done last"
        );
        let back = ids(
            &work,
            Order {
                key: Key::Status,
                reverse: true,
            },
        );
        assert_eq!(back.last().map(String::as_str), Some("TSK01"));
    }

    #[test]
    fn priority_and_effort_run_in_their_own_order() {
        let work = Work {
            tasks: vec![
                task("TSK01", Status::Draft, Priority::Low, Effort::Large),
                task("TSK02", Status::Draft, Priority::Urgent, Effort::Small),
                task("TSK03", Status::Draft, Priority::Normal, Effort::Medium),
            ],
            ..Work::default()
        };
        assert_eq!(ids(&work, by(Key::Priority)), ["TSK02", "TSK03", "TSK01"]);
        assert_eq!(ids(&work, by(Key::Effort)), ["TSK02", "TSK03", "TSK01"]);
    }

    #[test]
    fn a_categorical_order_groups_under_headings_and_a_flat_one_does_not() {
        let mut work = every_status();
        work.tasks.push(task(
            "TSK09",
            Status::Done,
            Priority::Normal,
            Effort::Medium,
        ));
        let grouped = groups(&work, by(Key::Status), "");
        assert_eq!(grouped.len(), 8, "one group per status present");
        let done = grouped
            .iter()
            .find(|group| group.value == "done")
            .expect("done");
        assert_eq!(done.rows.len(), 2);
        assert_eq!(done.status, Some(Status::Done));
        let flat = groups(&work, by(Key::Id), "");
        assert_eq!(flat.len(), 1, "a flat order has one group");
        assert!(flat[0].value.is_empty(), "and no heading");
        assert_eq!(cells(&flat).len(), 9, "nine rows, no headings");
        assert_eq!(cells(&grouped).len(), 9 + 8, "nine rows and eight headings");
    }

    #[test]
    fn the_right_column_is_the_value_of_the_key_in_force() {
        let work = every_status();
        let value_of = |key: Key, id: &str| {
            groups(&work, by(key), "")
                .into_iter()
                .flat_map(|group| group.rows)
                .find(|row| row.id == id)
                .map(|row| row.value)
                .expect("the row")
        };
        assert_eq!(value_of(Key::Id, "TSK04"), "executing", "by id: the status");
        assert_eq!(value_of(Key::Status, "TSK04"), "executing");
        assert_eq!(value_of(Key::Created, "TSK04"), "2026-11-15");
        assert_eq!(value_of(Key::Updated, "TSK04"), "2026-11-21");
        assert_eq!(value_of(Key::Priority, "TSK04"), "normal");
        assert_eq!(value_of(Key::Effort, "TSK04"), "M");
    }

    #[test]
    fn a_subtask_is_a_flat_row_with_its_parent_in_the_suffix() {
        let mut child = task("TSK05", Status::Approved, Priority::Normal, Effort::Small);
        child.parent = Some("TSK03".to_string());
        let work = Work {
            tasks: vec![
                task("TSK03", Status::Approved, Priority::Normal, Effort::Large),
                child,
            ],
            ..Work::default()
        };
        let text = draw(&work, by(Key::Id), 100, 8, None).join("\n");
        let row = text
            .lines()
            .find(|line| line.contains("TSK05"))
            .expect("the subtask row");
        assert!(row.contains("↑ TSK03"), "no parent suffix: {row}");
        // Flat: the subtask is not indented under its parent.
        let parent = text
            .lines()
            .find(|line| line.contains("TSK03"))
            .expect("the parent row");
        assert!(
            !parent.contains("→"),
            "the parent carries a suffix: {parent}"
        );
    }

    #[test]
    fn every_row_fits_its_width_at_every_column_and_icon_set() {
        // The sweep's rule: no row is wider than the pane, at any width, for
        // any sort key, in either icon set.
        let mut work = every_status();
        let mut child = task("TSK09", Status::Done, Priority::Urgent, Effort::Large);
        child.parent = Some("TSK01".to_string());
        child.title = "a much longer title that will not fit in the narrow columns at all".into();
        work.tasks.push(child);
        let mut order = Order::default();
        for _ in 0..Key::ALL.len() {
            for reverse in [false, true] {
                let at = Order { reverse, ..order };
                for icons in IconSet::ALL {
                    for width in 1..=120usize {
                        for line in plain_lines(&work, at, width, icons) {
                            assert!(
                                line_width(&line) <= width,
                                "{at:?} at {width}: {} columns",
                                line_width(&line)
                            );
                        }
                    }
                }
            }
            order = order.cycled();
        }
    }

    #[test]
    fn a_wide_row_shows_the_id_the_title_the_value_and_the_parent() {
        let mut child = task("TSK09", Status::Done, Priority::Urgent, Effort::Large);
        child.parent = Some("TSK01".to_string());
        let work = Work {
            tasks: vec![child],
            ..Work::default()
        };
        let line = plain_lines(&work, by(Key::Priority), 100, IconSet::Unicode)
            .into_iter()
            .next()
            .expect("the row");
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect();
        assert!(text.contains("TSK09"), "{text}");
        assert!(text.contains("title of TSK09"), "{text}");
        assert!(text.contains("urgent"), "{text}");
        assert!(text.contains("↑ TSK01"), "{text}");
    }

    #[test]
    fn a_heading_names_its_value_and_counts_its_rows() {
        let work = every_status();
        let grouped = groups(&work, by(Key::Status), "");
        let done = grouped
            .iter()
            .find(|group| group.value == "done")
            .expect("done");
        let text: String = heading(done, 40, &theme())
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect();
        assert_eq!(text, "DONE  1");
        let narrow: String = heading(done, 4, &theme())
            .spans
            .iter()
            .map(|span| span.content.to_string())
            .collect();
        assert!(text_width(&narrow) <= 4, "{narrow:?}");
        assert!(
            narrow.ends_with('1'),
            "a narrow heading loses its count: {narrow:?}"
        );
    }

    #[test]
    fn the_window_follows_the_cursor_and_announces_what_it_hides() {
        let work = every_status();
        // Eight rows in a pane with room for three: the cursor's row is
        // drawn, and the foot says how many rows are hidden.
        let text = draw(&work, by(Key::Id), 40, 5, Some("TSK07"));
        let joined = text.join("\n");
        assert!(
            joined.contains("TSK07"),
            "the cursor's row is not drawn:\n{joined}"
        );
        assert!(joined.contains("hidden"), "nothing is announced:\n{joined}");
        for line in &text {
            assert!(text_width(line) <= 40, "{line:?}");
        }
    }

    #[test]
    fn a_categorical_sort_names_the_groups_it_hides() {
        // Eight groups of one row each, sorted by status, in a pane with room
        // for three rows: the cursor's row is drawn, and the foot names the
        // group it leaves out with its count, not the flat count of the rows.
        let work = every_status();
        let text = draw(&work, by(Key::Status), 40, 5, Some("TSK04"));
        let joined = text.join("\n");
        assert!(
            joined.contains("TSK04"),
            "the cursor's row is not drawn:\n{joined}"
        );
        assert!(joined.contains("not shown: DRAFT (1 hidden)"), "{joined}");
        assert!(!joined.contains("7 hidden"), "a flat count:\n{joined}");
        for line in &text {
            assert!(text_width(line) <= 40, "{line:?}");
        }
    }

    #[test]
    fn a_pane_with_room_for_every_row_says_nothing_about_hiding() {
        let work = every_status();
        let joined = draw(&work, by(Key::Id), 60, 14, None).join("\n");
        assert!(!joined.contains("hidden"), "{joined}");
        assert!(joined.contains("TSK08"), "{joined}");
    }

    #[test]
    fn a_filter_narrows_the_rows_and_its_words_and_tokens_combine() {
        let work = every_status();
        assert_eq!(row_ids(&work, by(Key::Id), "status:done"), ["TSK08"]);
        assert_eq!(
            row_ids(&work, by(Key::Id), "status:draft status:done"),
            ["TSK01", "TSK08"],
            "the same key is OR'd"
        );
        assert_eq!(
            row_ids(&work, by(Key::Id), "status:draft priority:high"),
            Vec::<String>::new(),
            "different keys are AND'd"
        );
        assert_eq!(
            row_ids(&work, by(Key::Id), "TSK03 TSK04"),
            Vec::<String>::new()
        );
        // The groups of a categorical order hold the rows the filter lets
        // through, and only those.
        let grouped = groups(&work, by(Key::Status), "status:draft status:done");
        assert_eq!(grouped.len(), 2, "{grouped:?}");
    }

    #[test]
    fn a_filter_that_matches_nothing_says_so_at_forty_and_one_hundred_columns() {
        let work = every_status();
        for width in [40u16, 100] {
            let text = draw_filtered(&work, by(Key::Id), width, 6, None, "zzz").join("\n");
            assert!(text.contains("no task matches"), "{width}: {text}");
            assert!(text.contains("/ zzz"), "{width}: {text}");
            assert!(!text.contains("TSK01"), "{width}: a row is drawn: {text}");
        }
    }

    #[test]
    fn the_empty_tasks_tab_says_so() {
        let text = draw(&Work::default(), by(Key::Id), 60, 6, None).join("\n");
        assert!(text.contains("no tasks yet"), "{text}");
    }

    #[test]
    fn a_missing_work_folder_is_named() {
        let work = Work {
            missing: Some(crate::data::Missing::NoWork),
            ..Work::default()
        };
        let text = draw(&work, by(Key::Id), 60, 6, None).join("\n");
        assert!(text.contains("this project has no work folder"), "{text}");
    }
}
