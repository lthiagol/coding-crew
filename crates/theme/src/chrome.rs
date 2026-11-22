//! The chrome both TUIs draw with: panes, key chips, badges, bars and the
//! spinner.
//!
//! Each piece is a pure function of its inputs and the theme, and returns the
//! ratatui value it paints with. The paint comes from slots, never from a
//! colour written here.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType};

use crate::icons::{Icon, IconSet};
use crate::{Slot, Theme, block_title_style};

/// The frames of the spinner: braille dots, in the order they turn.
pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A pane: a rounded block with its title in the top border, beside its icon.
///
/// The title follows a rule segment after the corner: `╭─ icon title ───╮`.
/// The border is `chrome.border`, or `chrome.border_focus` when the pane has
/// focus. The title is `text.strong`, as every block's title is.
pub fn pane(title: &str, icon: Icon, set: IconSet, focused: bool, theme: &Theme) -> Block<'static> {
    let border = if focused {
        theme.chrome.border_focus
    } else {
        theme.chrome.border
    };
    let heading = Line::from(vec![
        Span::styled("─", border),
        Span::styled(
            format!(" {} {title} ", icon.glyph(set)),
            block_title_style(theme),
        ),
    ]);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(heading)
}

/// A key chip: the key reversed on `chrome.key`, with a space each side, then
/// the action in `chrome.footer`.
pub fn key_chip(key: &str, action: &str, theme: &Theme) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {key} "),
            theme.chrome.key.add_modifier(Modifier::REVERSED),
        ),
        Span::styled(format!(" {action}"), theme.chrome.footer),
    ]
}

/// A status pill: the text with a space each side, reversed on the slot's
/// style, so it reads as a filled chip on the header line.
pub fn pill(text: &str, style: Style) -> Span<'static> {
    Span::styled(format!(" {text} "), style.add_modifier(Modifier::REVERSED))
}

/// A count, in `text.strong`.
pub fn badge(count: usize, theme: &Theme) -> Span<'static> {
    Span::styled(count.to_string(), theme.text.strong)
}

/// A progress bar `width` cells wide: `█` for the filled cells, in `slot`,
/// and `░` for the empty ones, in `text.dim`.
///
/// The filled cells are `done` of `total`, rounded down, and never more than
/// `width`: a `done` past `total` fills the bar. A `total` of zero draws every
/// cell empty.
pub fn progress_bar(
    done: usize,
    total: usize,
    width: usize,
    slot: Slot,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let filled = done
        .min(total)
        .checked_mul(width)
        .and_then(|cells| cells.checked_div(total))
        .unwrap_or(0);
    vec![
        Span::styled("█".repeat(filled), theme.slot(slot)),
        Span::styled("░".repeat(width - filled), theme.text.dim),
    ]
}

/// The spinner's frame at `tick`. Tick zero is the first frame, so a plain
/// render, which has no tick, shows the same frame on every run.
pub fn spinner_frame(tick: usize) -> &'static str {
    SPINNER[tick % SPINNER.len()]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::Preset;
    use crate::text::text_width;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;

    fn theme() -> Theme {
        Theme::preset(Preset::Dark)
    }

    /// The text of a row of a buffer.
    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    /// The text of the spans, joined.
    fn text(spans: &[Span<'static>]) -> String {
        spans.iter().map(|span| span.content.as_ref()).collect()
    }

    #[test]
    fn a_pane_is_rounded_with_its_title_and_icon_in_the_top_border() {
        let theme = theme();
        let area = Rect::new(0, 0, 20, 3);
        let mut buffer = Buffer::empty(area);
        pane("raul", Icon::AppRaul, IconSet::Unicode, false, &theme).render(area, &mut buffer);
        let top = row(&buffer, 0);
        assert!(top.starts_with('╭'), "{top:?}");
        assert!(top.ends_with('╮'), "{top:?}");
        assert!(top.contains(" ◆ raul "), "{top:?}");
        assert_eq!(buffer[(0, 2)].symbol(), "╰");
        assert_eq!(Some(buffer[(0, 0)].fg), theme.chrome.border.fg);
    }

    #[test]
    fn a_pane_title_follows_a_rule_after_the_corner() {
        let theme = theme();
        let area = Rect::new(0, 0, 20, 3);
        let mut buffer = Buffer::empty(area);
        pane("raul", Icon::AppRaul, IconSet::Unicode, false, &theme).render(area, &mut buffer);
        let top = row(&buffer, 0);
        assert!(top.starts_with("╭─ ◆ raul "), "{top:?}");
        assert_eq!(Some(buffer[(1, 0)].fg), theme.chrome.border.fg);
    }

    #[test]
    fn a_focused_pane_takes_the_focus_border() {
        let theme = theme();
        let area = Rect::new(0, 0, 20, 3);
        let mut buffer = Buffer::empty(area);
        pane("raul", Icon::AppRaul, IconSet::Nerd, true, &theme).render(area, &mut buffer);
        assert_eq!(Some(buffer[(0, 0)].fg), theme.chrome.border_focus.fg);
        assert!(row(&buffer, 0).contains(Icon::AppRaul.glyph(IconSet::Nerd)));
    }

    #[test]
    fn a_key_chip_is_the_key_reversed_then_the_action() {
        let theme = theme();
        let spans = key_chip("q", "quit", &theme);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, " q ");
        assert_eq!(
            spans[0].style,
            theme.chrome.key.add_modifier(Modifier::REVERSED)
        );
        assert_eq!(spans[1].content, " quit");
        assert_eq!(spans[1].style, theme.chrome.footer);
    }

    #[test]
    fn a_pill_is_the_text_padded_and_reversed_on_its_slot() {
        let theme = theme();
        let span = pill("8 need you", theme.semantic.warn);
        assert_eq!(span.content, " 8 need you ");
        assert_eq!(
            span.style,
            theme.semantic.warn.add_modifier(Modifier::REVERSED)
        );
    }

    #[test]
    fn a_badge_is_the_count_in_text_strong() {
        let theme = theme();
        let span = badge(8, &theme);
        assert_eq!(span.content, "8");
        assert_eq!(span.style, theme.text.strong);
    }

    #[test]
    fn a_progress_bar_fills_by_the_share_rounded_down() {
        let theme = theme();
        let bar = progress_bar(18, 45, 10, Slot::SemanticOk, &theme);
        assert_eq!(text(&bar), "████░░░░░░");
        assert_eq!(bar[0].style, theme.semantic.ok);
        assert_eq!(bar[1].style, theme.text.dim);
    }

    #[test]
    fn a_progress_bar_with_no_total_is_empty() {
        let theme = theme();
        assert_eq!(
            text(&progress_bar(0, 0, 4, Slot::SemanticOk, &theme)),
            "░░░░"
        );
        assert_eq!(
            text(&progress_bar(3, 0, 4, Slot::SemanticOk, &theme)),
            "░░░░"
        );
    }

    #[test]
    fn a_progress_bar_never_overflows_its_width() {
        let theme = theme();
        for width in 0..=12 {
            for total in 0..=12 {
                for done in 0..=20 {
                    let bar = text(&progress_bar(done, total, width, Slot::SemanticOk, &theme));
                    assert_eq!(text_width(&bar), width, "{done}/{total} in {width}");
                    let filled = bar.matches('█').count();
                    assert!(filled <= width, "{done}/{total} in {width}");
                }
            }
        }
        assert_eq!(
            text(&progress_bar(50, 45, 10, Slot::SemanticOk, &theme)),
            "██████████",
            "a done past the total fills the bar"
        );
    }

    #[test]
    fn the_spinner_frames_are_braille_and_one_column_each() {
        assert_eq!(SPINNER.len(), 10);
        let distinct: HashSet<&str> = SPINNER.into_iter().collect();
        assert_eq!(distinct.len(), SPINNER.len());
        for frame in SPINNER {
            assert_eq!(text_width(frame), 1, "{frame}");
            assert!(
                frame
                    .chars()
                    .all(|c| ('\u{2800}'..='\u{28FF}').contains(&c))
            );
        }
    }

    #[test]
    fn the_spinner_turns_on_the_tick_and_starts_at_the_first_frame() {
        assert_eq!(spinner_frame(0), "⠋");
        assert_eq!(spinner_frame(3), "⠸");
        assert_eq!(spinner_frame(10), "⠋");
        assert_eq!(spinner_frame(11), "⠙");
    }
}
