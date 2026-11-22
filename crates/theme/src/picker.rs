//! The picker: a short list in a popup, one entry under the cursor.
//!
//! hugo's choices and raul's Settings picks both draw it, so a list reads the
//! same in both tools: a rounded pane over the window, the entries from its
//! top, the entry under the cursor painted as the selection, and the window
//! scrolled to keep the cursor on screen. The caller says what the entries are
//! and which of them the cursor may pick.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::chrome;
use crate::icons::{Icon, IconSet};
use crate::text::cut_to;
use crate::{Theme, selected_line};

/// The columns the popup takes.
const WIDTH: u16 = 56;
/// The most entries the popup lists at once: a longer list scrolls.
pub const ROWS: usize = 18;

/// A line of the popup: its text, and the paint it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    pub style: Style,
}

/// Where a popup of `rows` lines sits: centred over `area`, and never on the
/// title row or the footer, so the keys that close it stay on screen.
pub fn popup(area: Rect, rows: usize) -> Rect {
    let room = area.height.saturating_sub(2);
    let height = u16::try_from(rows.saturating_add(2))
        .unwrap_or(u16::MAX)
        .min(ROWS as u16 + 2)
        .min(room)
        .max(3)
        .min(area.height);
    let width = WIDTH.min(area.width);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// What a picker drew: its popup, and the line each drawn entry took, with the
/// entry's index. A click is answered from these, so the mouse acts on the
/// layout on screen. A popup that was not drawn has no entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Picked {
    pub area: Rect,
    pub entries: Vec<(Rect, usize)>,
}

/// Draw a picker in `popup`, which [`popup`] sized: the `lead` lines first, then
/// the `entries`, the one at `cursor` painted as the selection. The entries
/// scroll to keep the cursor in view. A popup too small to hold a line is not
/// drawn.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    frame: &mut Frame,
    popup: Rect,
    title: &str,
    icon: Icon,
    icons: IconSet,
    theme: &Theme,
    lead: &[Row],
    entries: &[Row],
    cursor: usize,
) -> Picked {
    if popup.width < 4 || popup.height < 3 {
        return Picked::default();
    }
    frame.render_widget(Clear, popup);
    let block = chrome::pane(title, icon, icons, true, theme);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let width = usize::from(inner.width);
    let room = usize::from(inner.height);
    if width == 0 || room == 0 {
        return Picked {
            area: popup,
            entries: Vec::new(),
        };
    }
    let mut lines: Vec<Line<'static>> = lead
        .iter()
        .take(room)
        .map(|row| Line::from(Span::styled(cut_to(&row.text, width), row.style)))
        .collect();
    let height = room.saturating_sub(lines.len());
    let top = lines.len();
    let hits = window_range(entries, cursor, height)
        .enumerate()
        .map(|(line, index)| {
            let rect = Rect {
                x: inner.x,
                y: inner.y + (top + line) as u16,
                width: inner.width,
                height: 1,
            };
            (rect, index)
        })
        .collect();
    lines.extend(window(entries, cursor, width, height, theme));
    frame.render_widget(Paragraph::new(lines), inner);
    Picked {
        area: popup,
        entries: hits,
    }
}

/// The range of entries the window shows in `height` lines: the entries from the
/// top, or scrolled so the cursor is the last of them.
fn window_range(entries: &[Row], cursor: usize, height: usize) -> std::ops::Range<usize> {
    if height == 0 {
        return 0..0;
    }
    let start = cursor.saturating_sub(height - 1).min(entries.len());
    start..(start + height).min(entries.len())
}

/// The entries that fit in `height` lines, from the top or scrolled so the
/// cursor is the last of them. The entry under the cursor takes the selection's
/// paint.
fn window(
    entries: &[Row],
    cursor: usize,
    width: usize,
    height: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let shown = window_range(entries, cursor, height);
    let first = shown.start;
    entries[shown]
        .iter()
        .enumerate()
        .map(|(offset, row)| {
            let index = first + offset;
            let line = Line::from(Span::styled(cut_to(&row.text, width), row.style));
            if index == cursor {
                selected_line(line, width, theme)
            } else {
                line
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_popup_is_centred_and_keeps_the_title_and_footer_on_screen() {
        let area = Rect::new(0, 0, 100, 30);
        let popup = popup(area, 3);
        assert_eq!((popup.width, popup.height), (56, 5));
        assert_eq!(popup.x, 22);
        assert_eq!(popup.y, 12);
        // A long list is capped at the list's rows, and two rows of the
        // window stay free for the title and the footer.
        let long = self::popup(Rect::new(0, 0, 100, 12), 40);
        assert_eq!(long.height, 10);
        assert!(long.y >= 1 && long.y + long.height <= 11);
    }

    #[test]
    fn the_popup_is_narrower_than_a_narrow_window() {
        let popup = popup(Rect::new(0, 0, 40, 20), 2);
        assert_eq!(popup.width, 40);
    }

    #[test]
    fn the_window_scrolls_so_the_cursor_is_its_last_entry() {
        let theme = Theme::preset(crate::Preset::Dark);
        let entries: Vec<Row> = (0..10)
            .map(|index| Row {
                text: format!("entry {index}"),
                style: theme.text.normal,
            })
            .collect();
        let shown = window(&entries, 6, 20, 3, &theme);
        let text: Vec<String> = shown.iter().map(line_text).collect();
        assert_eq!(text[0], "entry 4");
        assert_eq!(text[2], "entry 6");
        // The top of the list shows from its first entry.
        let top = window(&entries, 0, 20, 3, &theme);
        assert_eq!(line_text(&top[0]), "entry 0");
        assert_eq!(top.len(), 3);
    }

    #[test]
    fn an_empty_list_shows_no_entry() {
        let theme = Theme::preset(crate::Preset::Dark);
        assert!(window(&[], 0, 20, 5, &theme).is_empty());
    }

    fn line_text(line: &Line<'_>) -> String {
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        text.trim_end().to_string()
    }
}
