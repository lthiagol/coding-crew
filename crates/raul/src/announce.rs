//! The announcement of what a pane does not show: each group named with the
//! count of its rows that are out of view.
//!
//! The tabs that group their rows share it, so a count never appears without
//! the group it counts. The shape is `not shown: approve (2 hidden), review
//! (3 hidden)`; a pane too narrow for that packs the groups into the compact
//! shape, and what still does not fit ends in `…`.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use coding_crew_theme::Theme;
use coding_crew_theme::text::cut_to;

/// A group the announcement names: its label, its paint, and how many of its
/// rows are not shown.
#[derive(Debug, Clone)]
pub struct Group {
    pub label: String,
    pub style: Style,
    pub count: usize,
}

/// How a piece of the announcement is painted.
#[derive(Debug, Clone, Copy)]
enum Paint {
    /// A group's label: the slot of the group, as on its header and its rows.
    Name(Style),
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

/// A group's entry: the group and its count. The label keeps a row of its
/// own when the count cannot sit beside it, so the label is never replaced
/// by a count or by the word `hidden`.
#[derive(Debug, Clone)]
struct Entry {
    rows: Vec<Said>,
}

/// The lines that say what is not shown, in at most `room` rows.
///
/// The count here is the count of what is *left out*, not of what the group
/// holds — the group's header owns that — so the entry has a shape of its
/// own: the label, then the count in brackets with the word `hidden`. It
/// never takes the header's `label · total` shape, and the two meanings
/// cannot be read for each other.
///
/// The label is never cut away: when the count does not fit beside it, the
/// label takes a row of its own and the count and the word follow. When even
/// the packed entries do not fit in `room`, the compact shape names every
/// group and its count over the rows there are; when that does not fit
/// either, the entries give way whole, from the last one, and the last row is
/// marked with `…`. The flag says the announcement is cut.
///
/// A label is painted with its group's style and a count with `text.strong`,
/// as the theme requires; neither is dim.
pub fn lines(
    groups: &[Group],
    width: usize,
    room: usize,
    theme: &Theme,
) -> (Vec<Line<'static>>, bool) {
    let entries: Vec<Entry> = groups.iter().map(|group| entry(group, width)).collect();
    let rows = pack(&entries, width);
    if rows.len() <= room {
        return (paint(rows, theme), false);
    }
    if room > 0
        && let Some((mut compact, complete)) = compact(groups, width, room)
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
/// A row is never cut: a label and its count stay together, and so do the
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
/// its label, and a label is never parted from the count below it. Only
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
                            Paint::Name(style) => style,
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

/// The label of a group and the count of its rows that are not shown.
///
/// The count is the number of rows left out of view. The shape says so —
/// the label, then the count in brackets with the word `hidden` — so it
/// cannot be read as the header's `label · total`. The label is never
/// replaced: when it cannot sit beside its count, it takes a row of its own
/// and the count follows.
fn entry(group: &Group, width: usize) -> Entry {
    let name = group.label.as_str();
    let counted = group.count.to_string();
    let full = format!("{name} ({counted} hidden)");
    if full.chars().count() <= width {
        return Entry {
            rows: vec![Said {
                spans: vec![
                    (name.to_string(), Paint::Name(group.style)),
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
                    (name.to_string(), Paint::Name(group.style)),
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
    // The label keeps a row; the count and the word stay together below.
    let name_row = Said::new(cut_to(name, width), Paint::Name(group.style));
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

/// The compact shape: every group and its count, packed over at most `room`
/// rows.
///
/// The shape is `label count, label count … hidden`: the labels and their
/// counts are named even when a row is too narrow for `label (count
/// hidden)`. The word `hidden` or the lead-in says what the counts are; the
/// flag reports whether one of them is present. What fits is preferred in
/// that order.
fn compact(groups: &[Group], width: usize, room: usize) -> Option<(Vec<Said>, bool)> {
    let tokens: Vec<Said> = groups.iter().map(|group| token(group, width)).collect();
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
    /// Not at all: the labels and counts are the whole announcement.
    None,
}

/// A group's token in the compact shape: its label and its count.
///
/// The label is cut only when the row cannot hold it whole; the count stays
/// whole while there is room for it. When there is no room for both, the
/// label takes the row, cut to the marker — the label is what the row is,
/// and a count is never shown without it.
fn token(group: &Group, width: usize) -> Said {
    let name = group.label.as_str();
    let counted = group.count.to_string();
    let full = format!("{name} {counted}");
    if full.chars().count() <= width {
        return Said {
            spans: vec![
                (name.to_string(), Paint::Name(group.style)),
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
                (cut_to(name, room), Paint::Name(group.style)),
                (" ".to_string(), Paint::Dim),
                (counted, Paint::Strong),
            ],
        };
    }
    Said::new(cut_to(name, width), Paint::Name(group.style))
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

#[cfg(test)]
mod tests {
    use coding_crew_theme::Preset;
    use ratatui::style::Style;

    use super::*;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    fn group(label: &str, count: usize) -> Group {
        Group {
            label: label.to_string(),
            style: Style::default(),
            count,
        }
    }

    #[test]
    fn a_narrow_entry_keeps_the_label_and_the_word_hidden() {
        // The count may give way; the label that names the group may not,
        // and the word that says the count is a count of what is hidden
        // stays with it.
        for width in 6..=40 {
            for count in [0usize, 2, 3] {
                let entry = entry(&group("approve", count), width);
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
                // The label is never replaced by the count: it heads the
                // entry, whole while it fits and cut only when it cannot.
                let head = entry.rows[0].text();
                assert!(
                    head.starts_with("approve")
                        || "approve".starts_with(head.trim_end_matches('…')),
                    "{width}: {head:?} is not the label"
                );
            }
        }
    }

    #[test]
    fn the_groups_are_named_in_their_order_with_their_counts() {
        let groups = [group("DRAFT", 3), group("DONE", 5)];
        let (rows, cut) = lines(&groups, 80, 3, &theme());
        assert!(!cut);
        let text: String = rows[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(text, "not shown: DRAFT (3 hidden), DONE (5 hidden)");
    }
}
