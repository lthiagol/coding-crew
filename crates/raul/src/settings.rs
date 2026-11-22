//! The Settings tab: what the tool is doing, in one list.
//!
//! Four groups, all derived, none stored: the themes that exist, the keys
//! that work, the order each tab keeps, and the icon sets. It reads: nothing
//! here is changed from the tab.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::keys;
use crate::sort;
use crate::tabs::Tab;
use crate::tasks::Order;
use crate::view::View;
use coding_crew_theme::chrome;
use coding_crew_theme::text::{cut_to, pad, text_width};
use coding_crew_theme::{Icon, IconSet, Preset, Theme, selected_line};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between two columns.
const GAP: usize = 2;
/// The indent of a group header.
const HEADER_INDENT: usize = 2;
/// The indent of a row.
const ROW_INDENT: usize = 2;

/// A group of the tab, in the order of the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    /// The themes that exist.
    Theme,
    /// The keys that work.
    Keys,
    /// The order each tab keeps.
    Sort,
    /// The icon sets that exist.
    Icons,
}

impl GroupKind {
    /// Every group, in the order of the tab.
    pub const ALL: [GroupKind; 4] = [
        GroupKind::Theme,
        GroupKind::Keys,
        GroupKind::Sort,
        GroupKind::Icons,
    ];

    /// The name of the group.
    pub fn label(&self) -> &'static str {
        match self {
            GroupKind::Theme => "theme",
            GroupKind::Keys => "keys",
            GroupKind::Sort => "sort",
            GroupKind::Icons => "icons",
        }
    }
}

/// The number of groups: the size of every per-group count.
const GROUPS: usize = GroupKind::ALL.len();

/// A row of the tab: one fact, derived from the code that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The id the cursor and the window use.
    pub id: String,
    /// The group the row belongs to.
    pub group: GroupKind,
    /// The theme the row names, for the theme group.
    pub preset: Option<Preset>,
    /// The icon set the row names, for the icons group.
    pub set: Option<IconSet>,
    /// The fixed label: the theme's name, the key, the tab.
    pub label: String,
    /// What the row says: the theme's kind, what the key does, the order.
    pub says: String,
    /// The scopes of a key row.
    pub scopes: Option<String>,
}

/// A group and its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Which group it is.
    pub kind: GroupKind,
    /// The rows, in the order of the source they are derived from.
    pub rows: Vec<Row>,
}

/// The groups, derived: the theme crate's presets, the key table, the tabs'
/// own sorts and the icon sets, in the order `GroupKind::ALL` names. The sort
/// group names the Tasks tab's order as `order` has it in force.
pub fn groups(order: Order) -> [Group; GROUPS] {
    GroupKind::ALL.map(|kind| Group {
        kind,
        rows: rows(kind, order),
    })
}

/// The rows of one group, derived from the code that owns the fact.
fn rows(kind: GroupKind, order: Order) -> Vec<Row> {
    match kind {
        GroupKind::Theme => Preset::ALL
            .iter()
            .map(|preset| Row {
                id: format!("theme:{}", preset.name()),
                group: GroupKind::Theme,
                preset: Some(*preset),
                set: None,
                label: preset.label().to_string(),
                says: preset.kind().label().to_string(),
                scopes: None,
            })
            .collect(),
        GroupKind::Keys => keys::BINDINGS
            .iter()
            .enumerate()
            .map(|(index, binding)| Row {
                id: format!("key:{index:02}"),
                group: GroupKind::Keys,
                preset: None,
                set: None,
                label: binding.key.to_string(),
                says: binding.says.to_string(),
                scopes: Some(binding.scope_names()),
            })
            .collect(),
        GroupKind::Sort => Tab::ALL
            .iter()
            .map(|tab| Row {
                id: format!("sort:{}", tab.name()),
                group: GroupKind::Sort,
                preset: None,
                set: None,
                label: tab.label().to_string(),
                says: sort::in_force(*tab, order).words(),
                scopes: None,
            })
            .collect(),
        GroupKind::Icons => IconSet::ALL
            .iter()
            .map(|set| Row {
                id: format!("icons:{}", set.name()),
                group: GroupKind::Icons,
                preset: None,
                set: Some(*set),
                label: set.name().to_string(),
                says: match set {
                    IconSet::Nerd => "needs a Nerd Font",
                    IconSet::Unicode => "any font, one column each",
                }
                .to_string(),
                scopes: None,
            })
            .collect(),
    }
}

/// The ids of the tab's rows, in the order they are shown.
///
/// The tab is a list: the cursor and the window move through these, though
/// enter opens nothing.
pub fn row_ids() -> Vec<String> {
    // The rows are the same in every order: only the sort row's words move.
    groups(Order::default())
        .into_iter()
        .flat_map(|group| group.rows.into_iter().map(|row| row.id))
        .collect()
}

/// Draw the tab through the window the keys move.
///
/// `selected` is the row the cursor is on: its cells take the selection's
/// paint. The window is settled so the cursor is on a row the pane shows.
pub fn render_view(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    icons: IconSet,
    selected: Option<&str>,
    view: &mut View,
    order: Order,
) -> Option<usize> {
    let block = chrome::pane("settings", Icon::TabSettings, icons, false, theme);
    let groups = groups(order);
    let ids = row_ids();
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let (rows, announced) = view.draw(&ids, selected, |first| {
        let (lines, announced, shown, whole) =
            window(&groups, width, height, theme, icons, selected, first);
        ((lines, announced), shown, whole)
    });
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }).block(block),
        area,
    );
    let hidden = ids.len().saturating_sub(view.page());
    (hidden > 0 && !announced).then_some(hidden)
}

/// The widths of the label column of each group, and the icon set in force:
/// the mark of the icons group is drawn from it, as the theme group's is
/// drawn from the theme in force.
#[derive(Debug, Clone, Copy)]
struct Metrics {
    theme: usize,
    keys: usize,
    sort: usize,
    icons: usize,
    set: IconSet,
}

/// The widths of the label columns: the widest label of each group.
fn metrics(groups: &[Group], set: IconSet) -> Metrics {
    let width = |kind: GroupKind| {
        groups
            .iter()
            .filter(|group| group.kind == kind)
            .flat_map(|group| group.rows.iter())
            .map(|row| text_width(&row.label))
            .max()
            .unwrap_or(0)
    };
    Metrics {
        theme: width(GroupKind::Theme),
        keys: width(GroupKind::Keys),
        sort: width(GroupKind::Sort),
        icons: width(GroupKind::Icons),
        set,
    }
}

/// The lines of the tab from the row at `first`, what they show, whether the
/// pane announced the hidden rows, and whether it may stop there.
///
/// What does not fit is said out loud, the way the row tabs say it: the
/// announcement names every group and the count of its rows that are not in
/// view, including the rows above the window. A count never appears without
/// the group it counts. The rows give way to the announcement, so a pane that
/// hides rows says which ones it hides; the cursor's row is the one row the
/// window may not leave out, so a pane too short for both draws the row and
/// leaves the count to the footer. The returned flag says whether the pane
/// painted the announcement itself.
fn window(
    groups: &[Group],
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
    let metrics = metrics(groups, icons);
    let total: usize = groups.iter().map(|group| group.rows.len()).sum();
    let wanted: usize = groups.iter().map(|group| 1 + group.rows.len()).sum();
    if first == 0 && wanted <= height {
        let mut lines = Vec::new();
        for group in groups {
            lines.push(header(group, width, theme));
            for row in &group.rows {
                lines.push(row_line(row, width, metrics, theme, selected));
            }
        }
        return (lines, false, (0..total).collect(), true);
    }
    // Some line does not fit. The rows give way to the announcement: the
    // first layout whose whole announcement fits wins, and the rows take
    // what is left. When even the shortest announcement fits in no layout,
    // the last one says what it can and marks the cut. The cursor's row is
    // the one row the window may not leave out: a cut layout that shows it
    // wins over the shortest one.
    let mut shortest: Option<(Vec<Line<'static>>, Vec<Line<'static>>, Vec<usize>)> = None;
    let mut with_cursor: Option<(Vec<Line<'static>>, Vec<Line<'static>>, Vec<usize>)> = None;
    for capacity in (0..=height).rev() {
        let (rows, shown, omitted) =
            layout(groups, first, capacity, width, metrics, theme, selected);
        let hidden = total.saturating_sub(shown.len());
        let room = height.saturating_sub(rows.len());
        let (said, cut) = announcement(&omitted, width, room, theme);
        let has_cursor =
            selected.is_none_or(|id| shown.iter().any(|index| row_id(groups, *index) == id));
        if hidden == 0 {
            // Everything is in view: there is nothing to say.
            return (rows, false, shown, true);
        }
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
            // Fewer rows cannot bring the cursor back into view.
            continue;
        }
        let announced = !said.is_empty();
        let mut out = rows;
        out.extend(said);
        return (out, announced, shown, true);
    }
    if let Some((mut out, said, shown)) = with_cursor.or(shortest) {
        if let Some(id) = selected
            && let Some((index, line)) = cursor_row(groups, width, metrics, theme, id)
            && !shown.contains(&index)
        {
            // The pane is too short for the layout and the cursor's row: the
            // row is drawn, and what is left says what the pane hides. Only a
            // pane of one line shows the row alone.
            let mut lines = vec![line];
            let mut announced = false;
            let room = height - lines.len();
            if room > 0 {
                let omitted = omitted_with_only(groups, index);
                let (word, _) = announcement(&omitted, width, room, theme);
                announced = !word.is_empty();
                lines.extend(word);
            }
            return (lines, announced, vec![index], false);
        }
        let announced = !said.is_empty();
        out.extend(said);
        return (out, announced, shown, false);
    }
    // No layout was chosen and the cursor is nowhere: the pane is too short
    // to say anything.
    (Vec::new(), false, Vec::new(), false)
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

/// What each group hides when only the row at `cursor` is shown.
fn omitted_with_only(groups: &[Group], cursor: usize) -> [usize; GROUPS] {
    let mut omitted = [0usize; GROUPS];
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

/// The lines of a window that starts at the row `first`, the rows they show,
/// and, for each group, the count of its rows that are not in view.
///
/// A group's header is drawn wherever its rows are in the window, even when
/// the window starts inside the group: rows are never shown without the name
/// of the group above them, as on the other row tabs. A group with no room
/// for its header is not shown at all, and every one of its rows is left out
/// of view.
fn layout(
    groups: &[Group],
    first: usize,
    capacity: usize,
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    selected: Option<&str>,
) -> (Vec<Line<'static>>, Vec<usize>, [usize; GROUPS]) {
    let mut lines = Vec::new();
    let mut shown = Vec::new();
    let mut omitted = [0usize; GROUPS];
    let mut used = 0usize;
    let mut row_index = 0usize;
    for (index, group) in groups.iter().enumerate() {
        let start = row_index;
        row_index += group.rows.len();
        if row_index <= first {
            // The whole group is above the window.
            omitted[index] = group.rows.len();
            continue;
        }
        if used + 1 > capacity {
            // The header does not fit: the group is not shown at all.
            omitted[index] = group.rows.len();
            continue;
        }
        lines.push(header(group, width, theme));
        used += 1;
        let skip = first.saturating_sub(start).min(group.rows.len());
        let visible = group.rows.len() - skip;
        let shown_count = (capacity - used).min(visible);
        for (offset, row) in group.rows.iter().skip(skip).take(shown_count).enumerate() {
            lines.push(row_line(row, width, metrics, theme, selected));
            shown.push(start + skip + offset);
        }
        used += shown_count;
        omitted[index] = group.rows.len() - shown_count;
    }
    (lines, shown, omitted)
}

/// The header of a group: its name and the count of its rows.
fn header(group: &Group, width: usize, theme: &Theme) -> Line<'static> {
    let label = group.kind.label();
    let count = group.rows.len().to_string();
    let separator = " · ";
    let indent = " ".repeat(HEADER_INDENT);
    let full = HEADER_INDENT + text_width(label) + text_width(separator) + text_width(&count);
    if width >= full {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label.to_string(), theme.text.strong),
            Span::styled(separator.to_string(), theme.text.dim),
            Span::styled(count, theme.text.strong),
        ]);
    }
    Line::from(Span::styled(
        cut_to(&format!("{indent}{label}{separator}{count}"), width),
        theme.text.strong,
    ))
}

/// How a piece of the announcement is painted.
#[derive(Debug, Clone, Copy)]
enum Paint {
    /// A group name: the same paint its header uses.
    Group,
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
/// The count here is the number of rows *left out of view*, not the group's
/// total — the header owns that — so an entry has a shape of its own: the
/// group, then the count in brackets with the word `hidden`. It never takes
/// the header's `group · total` shape, and every group is named, including a
/// group that is wholly in view. When the packed entries do not fit in
/// `room`, the compact shape names every group and count over the rows there
/// are; when even that does not fit, the entries give way whole from the
/// last one and the last row is marked with `…`. An entry is never cut in
/// half: a group and its count stay together.
fn announcement(
    omitted: &[usize; GROUPS],
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    let entries: Vec<Said> = GroupKind::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| entry(*kind, omitted[index], width))
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
        // every group is named, so no entry is dropped for the marker.
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
/// The full entry is `label (count hidden)`. The count is never shown without
/// the group it counts: a row too narrow for both keeps the group, cut, and
/// the count gives way.
fn entry(kind: GroupKind, count: usize, width: usize) -> Said {
    let label = kind.label();
    let counted = count.to_string();
    let full = format!("{label} ({counted} hidden)");
    if text_width(&full) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group),
                (" (".to_string(), Paint::Dim),
                (counted, Paint::Strong),
                (" hidden)".to_string(), Paint::Dim),
            ],
        };
    }
    let bare = format!("{label} {counted} hidden");
    if text_width(&bare) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
                (" hidden".to_string(), Paint::Dim),
            ],
        };
    }
    let tail = format!(" {counted} hidden");
    if text_width(&tail) < width {
        let room = width - text_width(&tail);
        return Said {
            spans: vec![
                (cut_to(label, room), Paint::Group),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
                (" hidden".to_string(), Paint::Dim),
            ],
        };
    }
    Said::new(cut_to(label, width), Paint::Group)
}

/// The compact shape: every group and its count, packed over at most `room`
/// rows.
///
/// The shape is `group count, group count … hidden`: distinct from the
/// header's `group · total`, and it names the groups even when a row is too
/// narrow for `group (count hidden)`. The lead-in and the word are dropped
/// only when they do not fit; the flag says whether one of them is present.
fn compact(omitted: &[usize; GROUPS], width: usize, room: usize) -> Option<(Vec<Said>, bool)> {
    let tokens: Vec<Said> = GroupKind::ALL
        .iter()
        .enumerate()
        .map(|(index, kind)| token(*kind, omitted[index], width))
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
    let counted = count.to_string();
    let full = format!("{label} {counted}");
    if text_width(&full) <= width {
        return Said {
            spans: vec![
                (label.to_string(), Paint::Group),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
            ],
        };
    }
    let tail = format!(" {counted}");
    if text_width(&tail) < width {
        let room = width - text_width(&tail);
        return Said {
            spans: vec![
                (cut_to(label, room), Paint::Group),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
            ],
        };
    }
    Said::new(cut_to(label, width), Paint::Group)
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
/// count. The lead-in sits on the first row when it fits there.
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
                            Paint::Group => theme.text.strong,
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

/// A row of the tab, in its paint, and in the selection's when it is the
/// cursor's row.
fn row_line(
    row: &Row,
    width: usize,
    metrics: Metrics,
    theme: &Theme,
    selected: Option<&str>,
) -> Line<'static> {
    let line = match row.group {
        GroupKind::Theme => theme_line(row, width, metrics, theme),
        GroupKind::Keys => key_line(row, width, metrics, theme),
        GroupKind::Sort => sort_line(row, width, metrics, theme),
        GroupKind::Icons => icons_line(row, width, metrics, theme),
    };
    if selected == Some(row.id.as_str()) {
        return selected_line(line, width, theme);
    }
    line
}

/// A theme row: the mark of the theme in use, the name, and its kind.
fn theme_line(row: &Row, width: usize, metrics: Metrics, theme: &Theme) -> Line<'static> {
    let in_use = row.preset == Some(theme.preset);
    marked_line(row, in_use, metrics.theme, metrics.set, width, theme)
}

/// An icons row: the mark of the set in force, the name, and what it needs.
fn icons_line(row: &Row, width: usize, metrics: Metrics, theme: &Theme) -> Line<'static> {
    let in_use = row.set == Some(metrics.set);
    marked_line(row, in_use, metrics.icons, metrics.set, width, theme)
}

/// A row that names one of a group's values: the mark of the value in use,
/// the name in a column `label_width` wide, and what the value says.
fn marked_line(
    row: &Row,
    in_use: bool,
    label_width: usize,
    icons: IconSet,
    width: usize,
    theme: &Theme,
) -> Line<'static> {
    let mark = if in_use {
        Icon::InUse.glyph(icons)
    } else {
        " "
    };
    let indent = " ".repeat(ROW_INDENT);
    let name = pad(&row.label, label_width);
    let head = ROW_INDENT + 1 + 1 + label_width;
    let full = head + GAP + text_width(&row.says);
    let name_style = if in_use {
        theme.text.strong
    } else {
        theme.text.normal
    };
    if width >= full {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(format!("{mark} "), theme.text.strong),
            Span::styled(name, name_style),
            Span::raw(" ".repeat(GAP)),
            Span::styled(row.says.clone(), theme.text.dim),
        ]);
    }
    // The kind gives way: the mark and the name say which value it is.
    if width >= head {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(format!("{mark} "), theme.text.strong),
            Span::styled(name, name_style),
        ]);
    }
    Line::from(Span::styled(
        cut_to(&format!("{indent}{mark} {}", row.label), width),
        name_style,
    ))
}

/// A key row: the key, what it does, and where it applies.
fn key_line(row: &Row, width: usize, metrics: Metrics, theme: &Theme) -> Line<'static> {
    let indent = " ".repeat(ROW_INDENT);
    let label = pad(&row.label, metrics.keys);
    let scopes = row.scopes.as_deref().unwrap_or("");
    let tail = format!(" ({scopes})");
    let fixed = ROW_INDENT + metrics.keys + GAP + text_width(&row.says);
    if width >= fixed + text_width(&tail) {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label, theme.text.strong),
            Span::raw(" ".repeat(GAP)),
            Span::styled(row.says.clone(), theme.text.normal),
            Span::styled(tail, theme.text.dim),
        ]);
    }
    // The scope gives way before the word that says what the key does.
    if width >= fixed {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label, theme.text.strong),
            Span::raw(" ".repeat(GAP)),
            Span::styled(row.says.clone(), theme.text.normal),
        ]);
    }
    // The description gives way: the key is still named.
    if width >= ROW_INDENT + metrics.keys {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label, theme.text.strong),
        ]);
    }
    Line::from(Span::styled(
        cut_to(&format!("{indent}{}", row.label), width),
        theme.text.strong,
    ))
}

/// A sort row: the tab and the order it keeps.
fn sort_line(row: &Row, width: usize, metrics: Metrics, theme: &Theme) -> Line<'static> {
    let indent = " ".repeat(ROW_INDENT);
    let label = pad(&row.label, metrics.sort);
    let fixed = ROW_INDENT + metrics.sort + GAP;
    if width >= fixed + text_width(&row.says) {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label, theme.text.strong),
            Span::raw(" ".repeat(GAP)),
            Span::styled(row.says.clone(), theme.text.normal),
        ]);
    }
    if width > fixed {
        return Line::from(vec![
            Span::raw(indent),
            Span::styled(label, theme.text.strong),
            Span::raw(" ".repeat(GAP)),
            Span::styled(cut_to(&row.says, width - fixed), theme.text.normal),
        ]);
    }
    Line::from(Span::styled(
        cut_to(&format!("{indent}{}", row.label), width),
        theme.text.strong,
    ))
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

    /// Draw the tab and return the rows it shows.
    fn draw(theme: &Theme, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let mut view = View::default();
        terminal
            .draw(|frame| {
                render_view(
                    frame,
                    frame.area(),
                    theme,
                    IconSet::Nerd,
                    None,
                    &mut view,
                    Order::default(),
                );
            })
            .expect("drawing the settings tab");
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

    /// The rows of the pane without the border.
    fn inner(rows: &[String]) -> Vec<String> {
        rows.iter()
            .skip(1)
            .take(rows.len().saturating_sub(2))
            .map(|row| row.trim_end().to_string())
            .collect()
    }

    #[test]
    fn the_four_groups_are_derived_in_the_order_of_the_spec() {
        let groups = groups(Order::default());
        assert_eq!(
            [
                groups[0].kind,
                groups[1].kind,
                groups[2].kind,
                groups[3].kind
            ],
            GroupKind::ALL
        );
        assert_eq!(
            GroupKind::ALL.map(|kind| kind.label()),
            ["theme", "keys", "sort", "icons"]
        );
    }

    #[test]
    fn the_icon_rows_are_every_set_and_say_what_it_needs() {
        let groups = groups(Order::default());
        let rows = &groups[3].rows;
        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<&str>>(),
            ["nerd", "unicode"]
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.set.expect("an icon row names its set"))
                .collect::<Vec<IconSet>>(),
            IconSet::ALL.to_vec()
        );
    }

    #[test]
    fn the_theme_rows_are_every_preset_and_say_what_it_is() {
        let groups = groups(Order::default());
        let rows = &groups[0].rows;
        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<&str>>(),
            Preset::ALL.map(|preset| preset.label()),
            "a new preset is missing from the theme group"
        );
        assert_eq!(
            rows.iter()
                .map(|row| row.preset.expect("a theme row names its preset"))
                .collect::<Vec<Preset>>(),
            Preset::ALL.to_vec()
        );
        for (row, preset) in rows.iter().zip(Preset::ALL) {
            assert_eq!(row.says, preset.kind().label(), "{}", preset.name());
            assert!(
                ["palette", "terminal colours"].contains(&row.says.as_str()),
                "{}: {}",
                preset.name(),
                row.says
            );
        }
    }

    #[test]
    fn the_key_rows_are_the_table_the_input_reads() {
        let groups = groups(Order::default());
        let rows = &groups[1].rows;
        assert_eq!(rows.len(), keys::BINDINGS.len());
        for (row, binding) in rows.iter().zip(keys::BINDINGS) {
            assert_eq!(row.label, binding.key);
            assert_eq!(row.says, binding.says);
            assert_eq!(row.scopes.as_deref(), Some(binding.scope_names().as_str()));
        }
    }

    #[test]
    fn the_sort_rows_are_every_tab_and_its_own_words() {
        let groups = groups(Order::default());
        let rows = &groups[2].rows;
        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<&str>>(),
            Tab::ALL.map(|tab| tab.label())
        );
        for (row, tab) in rows.iter().zip(Tab::ALL) {
            assert_eq!(row.says, tab.sort().words(), "{:?}", tab);
        }
    }

    #[test]
    fn the_tab_always_has_rows_and_its_groups_count_them() {
        let groups = groups(Order::default());
        assert_eq!(
            groups
                .iter()
                .map(|group| group.rows.len())
                .collect::<Vec<_>>(),
            [
                Preset::ALL.len(),
                keys::BINDINGS.len(),
                Tab::ALL.len(),
                IconSet::ALL.len()
            ]
        );
        assert_eq!(
            row_ids().len(),
            Preset::ALL.len() + keys::BINDINGS.len() + Tab::ALL.len() + IconSet::ALL.len()
        );
        // The empty state is gone: the tab has no state to be empty.
        let text = inner(&draw(&theme(), 100, 60)).join("\n");
        assert!(!text.contains("nothing to set yet"), "{text}");
    }

    #[test]
    fn the_groups_are_named_with_their_counts() {
        let text = inner(&draw(&theme(), 100, 60)).join("\n");
        for (name, count) in [
            ("theme", Preset::ALL.len()),
            ("keys", keys::BINDINGS.len()),
            ("sort", Tab::ALL.len()),
            ("icons", IconSet::ALL.len()),
        ] {
            assert!(
                text.contains(&format!("{name} · {count}")),
                "{name} is not named with {count}: {text}"
            );
        }
    }

    #[test]
    fn every_preset_marks_its_own_row_exactly_once() {
        for preset in Preset::ALL {
            let theme = Theme::preset(preset);
            let text = inner(&draw(&theme, 100, 46)).join("\n");
            let mark = Icon::InUse.glyph(IconSet::Nerd);
            // The theme's rows say what kind of theme they are; the icon set
            // in force is marked on its own rows.
            let marked: Vec<&str> = text
                .lines()
                .filter(|line| {
                    line.contains(mark)
                        && (line.contains("palette") || line.contains("terminal colours"))
                })
                .collect();
            assert_eq!(marked.len(), 1, "{}: {text}", preset.name());
            assert!(
                marked[0].contains(preset.label()),
                "{}: {}",
                preset.name(),
                marked[0]
            );
            // The other presets are listed, unmarked.
            for other in Preset::ALL {
                assert!(text.contains(other.label()), "{}", other.name());
            }
        }
    }

    #[test]
    fn every_icon_set_marks_its_own_row_exactly_once() {
        let groups = groups(Order::default());
        for set in IconSet::ALL {
            let says = groups[3]
                .rows
                .iter()
                .find(|row| row.set == Some(set))
                .map(|row| row.says.as_str())
                .expect("a row for the set");
            let (lines, _, _, _) = window(&groups, 98, 60, &theme(), set, None, 0);
            let text = text_of(&lines);
            let mark = Icon::InUse.glyph(set);
            let marked: Vec<&str> = text
                .lines()
                .filter(|line| line.contains(mark) && line.contains(says))
                .collect();
            assert_eq!(marked.len(), 1, "{}: {text}", set.name());
            assert!(
                marked[0].contains(set.name()),
                "{}: {}",
                set.name(),
                marked[0]
            );
        }
    }

    #[test]
    fn the_tab_renders_at_40_60_and_100_columns() {
        for width in [40u16, 60, 100] {
            let text = inner(&draw(&theme(), width, 60)).join("\n");
            assert!(text.contains("theme · 5"), "{width}: {text}");
            assert!(
                text.contains(&format!("keys · {}", keys::BINDINGS.len())),
                "{width}: {text}"
            );
            assert!(text.contains("sort · 6"), "{width}: {text}");
            assert!(text.contains("Dracula"), "{width}: {text}");
            assert!(text.contains("palette"), "{width}: {text}");
            assert!(text.contains("terminal colours"), "{width}: {text}");
            assert!(text.contains("quit"), "{width}: {text}");
            assert!(text.contains("open"), "{width}: {text}");
            assert!(text.contains("ideas first"), "{width}: {text}");
            // The whole order is named where the columns are there for it.
            if width >= 60 {
                assert!(text.contains("ids ascending"), "{width}: {text}");
            }
        }
    }

    #[test]
    fn every_line_fits_the_pane_and_is_drawn_as_one_line() {
        // A line wider than the pane wraps, and the rows below it move: the
        // layout counts one line per row, so each line must be drawn as one.
        let groups = groups(Order::default());
        let theme = theme();
        for width in 10..=40usize {
            for height in [2usize, 5, 8, 12, 30] {
                for first in [0usize, 3, 10, 26, 32] {
                    let (lines, _, _, _) =
                        window(&groups, width, height, &theme, IconSet::Nerd, None, first);
                    let texts: Vec<String> = lines
                        .iter()
                        .map(|line| {
                            line.spans
                                .iter()
                                .map(|span| span.content.as_ref())
                                .collect::<String>()
                        })
                        .collect();
                    for (line, text) in lines.iter().zip(&texts) {
                        assert!(
                            line.width() <= width,
                            "{width}x{height} from {first}: {text:?} is wider than the pane"
                        );
                    }
                    let mut terminal =
                        Terminal::new(TestBackend::new(width as u16, lines.len() as u16 + 4))
                            .expect("a test terminal");
                    terminal
                        .draw(|frame| {
                            let paragraph =
                                Paragraph::new(lines.clone()).wrap(Wrap { trim: false });
                            frame.render_widget(paragraph, frame.area());
                        })
                        .expect("drawing the lines");
                    let buffer = terminal.backend().buffer();
                    let drawn: Vec<String> = (0..buffer.area().height)
                        .map(|y| {
                            (0..buffer.area().width)
                                .map(|x| buffer.cell((x, y)).map_or(" ", |cell| cell.symbol()))
                                .collect::<String>()
                                .trim_end()
                                .to_string()
                        })
                        .collect();
                    for (index, text) in texts.iter().enumerate() {
                        assert_eq!(
                            drawn[index],
                            text.trim_end(),
                            "{width}x{height} from {first}: line {index} wraps"
                        );
                    }
                    assert!(
                        drawn[lines.len()..].iter().all(|row| row.is_empty()),
                        "{width}x{height} from {first}: lines past the layout"
                    );
                }
            }
        }
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

    #[test]
    fn a_short_pane_names_the_groups_it_hides() {
        // A hidden count is never a bare count: the announcement names every
        // group and the rows of it that are not in view. A check that only
        // looks for "not shown:" passes on `not shown: 27`, so this one looks
        // for the groups.
        for (width, height) in [(100u16, 12u16), (60, 12), (80, 8)] {
            let text = inner(&draw(&theme(), width, height)).join("\n");
            let hidden = text
                .lines()
                .skip_while(|line| !line.contains("not shown:"))
                .collect::<Vec<&str>>()
                .join(" ");
            assert!(!hidden.is_empty(), "{width}x{height}: {text}");
            for kind in GroupKind::ALL {
                assert!(
                    hidden.contains(kind.label()),
                    "{width}x{height}: {} is not named: {hidden}",
                    kind.label()
                );
            }
            // The count sits with its group, and with the word that says it
            // is a hidden count rather than the group's total.
            assert!(hidden.contains("hidden"), "{width}x{height}: {hidden}");
            assert!(
                !hidden.contains("not shown: 2"),
                "{width}x{height}: a bare count: {hidden}"
            );
        }
    }

    #[test]
    fn the_announcement_never_shows_a_count_without_its_group() {
        let theme = theme();
        let (rows, cut) = announcement(&[5, 18, 5, 2], 60, 2, &theme);
        assert!(!cut, "the announcement does not fit two rows");
        let text = said_text(&rows);
        for kind in GroupKind::ALL {
            assert!(text.contains(kind.label()), "{text}");
        }
        assert!(text.contains("theme (5 hidden)"), "{text}");
        assert!(text.contains("keys (18 hidden)"), "{text}");
        assert!(text.contains("sort (5 hidden)"), "{text}");
        assert!(text.contains("icons (2 hidden)"), "{text}");
        assert!(!text.contains("not shown: 28"), "{text}");
        // A pane with a row to spare still names the groups: the compact
        // shape is not a bare total either.
        let (rows, cut) = announcement(&[5, 18, 5, 2], 60, 1, &theme);
        assert!(!cut, "the compact announcement does not fit one row");
        let text = said_text(&rows);
        for kind in GroupKind::ALL {
            assert!(text.contains(kind.label()), "{text}");
        }
        assert!(!text.contains("not shown: 28"), "{text}");
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
    fn a_window_inside_a_group_keeps_its_header() {
        // The window starts in the middle of the keys group: the rows are
        // not shown without the name of the group above them.
        let groups = groups(Order::default());
        let (rows, shown, _) = layout(
            &groups,
            10,
            6,
            98,
            metrics(&groups, IconSet::Nerd),
            &theme(),
            None,
        );
        let text = text_of(&rows);
        assert!(
            text.starts_with(&format!("  keys · {}", keys::BINDINGS.len())),
            "{text}"
        );
        assert!(!text.contains("theme · 5"), "{text}");
        assert_eq!(shown.first(), Some(&10), "{shown:?}");
    }

    #[test]
    fn a_scrolled_pane_names_the_rows_above_it() {
        // The window starts at the end of the keys group. What is above it
        // is counted too, by the group it belongs to.
        let groups = groups(Order::default());
        let keys = keys::BINDINGS.len();
        let end_of_keys = Preset::ALL.len() + keys;
        let (rows, _, _, _) = window(&groups, 98, 10, &theme(), IconSet::Nerd, None, end_of_keys);
        let text = text_of(&rows);
        assert!(text.contains("theme (5 hidden)"), "{text}");
        assert!(text.contains(&format!("keys ({keys} hidden)")), "{text}");
        assert!(text.contains("sort (0 hidden)"), "{text}");
    }

    #[test]
    fn the_value_in_use_is_marked_with_the_set_glyph() {
        // The mark of a value in use is the icon table's `in_use` in the set
        // in force: the nerd set draws its own glyph, not the unicode dot.
        let groups = groups(Order::default());
        let (rows, _, _, _) = window(&groups, 98, 200, &theme(), IconSet::Nerd, None, 0);
        let text = text_of(&rows);
        assert!(text.contains(Icon::InUse.glyph(IconSet::Nerd)), "{text}");
        assert!(
            !text.contains(Icon::InUse.glyph(IconSet::Unicode)),
            "{text}"
        );
    }

    #[test]
    fn the_settings_section_of_the_screens_doc_names_the_file_and_the_precedence() {
        // The section is the configuration file, drawn as a file. The view
        // that draws it is the next unit; the doc names the file and the order
        // its values are read in.
        let doc = include_str!("../../../docs/tui/screens.md");
        let section = doc
            .split("### Settings")
            .nth(1)
            .and_then(|rest| rest.split("## The detail pane").next())
            .expect("the Settings section");
        assert!(
            section.contains("`~/.config/coding-crew/config.toml`"),
            "the doc does not name the file"
        );
        assert!(
            section.contains("**The order of precedence**"),
            "the doc does not say the order of precedence"
        );
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

    /// Draw the tab and return every cell, with its colour.
    fn draw_cells(
        theme: &Theme,
        width: u16,
        height: u16,
        selected: Option<&str>,
    ) -> Vec<Vec<(char, Color)>> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let mut view = View::default();
        terminal
            .draw(|frame| {
                render_view(
                    frame,
                    frame.area(),
                    theme,
                    IconSet::Nerd,
                    selected,
                    &mut view,
                    Order::default(),
                );
            })
            .expect("drawing the settings tab");
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

    #[test]
    fn the_cursor_takes_the_selection_paint() {
        let theme = theme();
        let id = row_ids()[0].clone();
        let cells = draw_cells(&theme, 100, 40, Some(&id));
        let colour = colour_of(&cells, "Dark").expect("the first row");
        assert_eq!(
            colour,
            theme.text.selected.fg.expect("the selection colour")
        );
    }
}
