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
use crate::text::text_width;
use crate::{Slot, Theme, block_title_style};

/// The frames of the spinner: braille dots, in the order they turn.
pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A pane: a rounded block with its title in the top border, beside its icon.
///
/// The title follows a rule segment after the corner: `╭─ icon title ───╮`.
/// The border is `chrome.border`, or `chrome.border_focus` when the pane has
/// focus. The title is `text.strong`, as every block's title is.
pub fn pane(title: &str, icon: Icon, set: IconSet, focused: bool, theme: &Theme) -> Block<'static> {
    pane_titled(title, icon, set, focused, block_title_style(theme), theme)
}

/// A pane whose title takes the given style: the accent, for the main pane
/// of hugo, and `text.strong` everywhere else, as [`pane`] paints it.
pub fn pane_titled(
    title: &str,
    icon: Icon,
    set: IconSet,
    focused: bool,
    title_style: Style,
    theme: &Theme,
) -> Block<'static> {
    let border = if focused {
        theme.chrome.border_focus
    } else {
        theme.chrome.border
    };
    let heading = Line::from(vec![
        Span::styled("─", border),
        Span::styled(format!(" {} {title} ", icon.glyph(set)), title_style),
    ]);
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(heading)
}

/// The separator between the chips of a footer.
const DOT: &str = " · ";

/// A key chip: the key bold on `chrome.key`, then its action on `text.dim`,
/// one space apart. The key keeps the slot's other attributes, reversed where
/// the preset sets it; a chip is not filled with a tint of its own.
pub fn key_chip(key: &str, action: &str, theme: &Theme) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            key.to_string(),
            theme.chrome.key.add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {action}"), theme.text.dim),
    ]
}

/// The footer: the context's chips, then the closing chips, centred.
///
/// One line when the whole fits. Otherwise, with two rows, the context's
/// chips take the first line and the closing chips the second. With one row
/// the context's chips give way from the right first, then the closing chips
/// from the right. A chip that does not fit ends its line, so only whole chips
/// drop: a chip too wide for the pane drops too, rather than being cut.
///
/// Every chip a line leaves out is counted: the line ends in `+N keys`, where
/// N is the number of chips dropped, so a drop is never silent. The count
/// takes room of its own, so a chip gives way to it when the line is too
/// narrow for both.
pub fn footer(
    actions: &[Vec<Span<'static>>],
    closing: &[Vec<Span<'static>>],
    width: u16,
    rows: u16,
    theme: &Theme,
) -> Vec<Line<'static>> {
    footer_led(&[], actions, closing, width, rows, theme)
}

/// The footer with a lead: chips that come first on the line and hold while
/// anything else can, such as a pane's count of the rows it hides. The lead
/// gives way last of all.
pub fn footer_led(
    lead: &[Vec<Span<'static>>],
    actions: &[Vec<Span<'static>>],
    closing: &[Vec<Span<'static>>],
    width: u16,
    rows: u16,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let width = usize::from(width);
    let whole = line_of(lead, actions, actions.len(), closing, closing.len(), theme);
    if rows < 2 || closing.is_empty() || columns(&whole) <= width {
        let spans = (0..=closing.len())
            .rev()
            .flat_map(|shown_closing| {
                (0..=actions.len())
                    .rev()
                    .map(move |shown_actions| (shown_actions, shown_closing))
            })
            .map(|(shown_actions, shown_closing)| {
                line_of(lead, actions, shown_actions, closing, shown_closing, theme)
            })
            .find(|spans| columns(spans) <= width)
            .unwrap_or_else(|| groups_line(vec![join(lead, theme)], theme));
        return vec![Line::from(spans).centered()];
    }
    // Two rows: the closing chips keep the second whole if they can, and the
    // count rides the first line with the context's chips.
    let shown_closing = (0..=closing.len())
        .rev()
        .find(|&shown| columns(&join(&closing[..shown], theme)) <= width)
        .unwrap_or(0);
    let dropped_closing = closing.len() - shown_closing;
    let first = (0..=actions.len())
        .rev()
        .map(|shown| {
            let dropped = actions.len() - shown + dropped_closing;
            groups_line(
                vec![
                    join(lead, theme),
                    join(&actions[..shown], theme),
                    marker(dropped, theme),
                ],
                theme,
            )
        })
        .find(|spans| columns(spans) <= width)
        .unwrap_or_default();
    let second = join(&closing[..shown_closing], theme);
    if second.is_empty() {
        return vec![Line::from(first).centered()];
    }
    vec![Line::from(first).centered(), Line::from(second).centered()]
}

/// One footer line: the lead, the first `shown_actions` context chips, the
/// first `shown_closing` closing chips, and the count of the rest.
fn line_of(
    lead: &[Vec<Span<'static>>],
    actions: &[Vec<Span<'static>>],
    shown_actions: usize,
    closing: &[Vec<Span<'static>>],
    shown_closing: usize,
    theme: &Theme,
) -> Vec<Span<'static>> {
    let dropped = (actions.len() - shown_actions) + (closing.len() - shown_closing);
    groups_line(
        vec![
            join(lead, theme),
            join(&actions[..shown_actions], theme),
            join(&closing[..shown_closing], theme),
            marker(dropped, theme),
        ],
        theme,
    )
}

/// The groups of a footer line, joined by the separator. A group with no
/// chips is left out, so no separator stands beside nothing.
fn groups_line(groups: Vec<Vec<Span<'static>>>, theme: &Theme) -> Vec<Span<'static>> {
    let groups: Vec<Vec<Span<'static>>> = groups
        .into_iter()
        .filter(|group| !group.is_empty())
        .collect();
    join(&groups, theme)
}

/// The count of the chips a line leaves out: `+N keys`, or nothing when none
/// is left out.
fn marker(dropped: usize, theme: &Theme) -> Vec<Span<'static>> {
    if dropped == 0 {
        return Vec::new();
    }
    vec![Span::styled(
        format!("+{dropped} keys"),
        theme.chrome.footer,
    )]
}

/// The thin rule over the footer: `─` across the width, in `chrome.border`.
pub fn rule(width: u16, theme: &Theme) -> Span<'static> {
    Span::styled("─".repeat(usize::from(width)), theme.chrome.border)
}

/// The chips joined by the separator.
fn join(chips: &[Vec<Span<'static>>], theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for chip in chips {
        if !spans.is_empty() {
            spans.push(Span::styled(DOT, theme.chrome.border));
        }
        spans.extend(chip.iter().cloned());
    }
    spans
}

/// The columns the spans take.
fn columns(spans: &[Span]) -> usize {
    spans.iter().map(|span| text_width(&span.content)).sum()
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
    fn a_key_chip_is_the_key_bold_then_the_action_dim() {
        let theme = theme();
        let spans = key_chip("q", "quit", &theme);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "q");
        assert_eq!(spans[0].style.fg, theme.chrome.key.fg);
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, " quit");
        assert_eq!(spans[1].style, theme.text.dim);
    }

    #[test]
    fn a_key_chip_in_the_terminal_preset_is_bold_and_reversed() {
        // theme.md: every key label holds BOLD and REVERSED together.
        let theme = Theme::preset(Preset::Terminal);
        let spans = key_chip("q", "quit", &theme);
        assert_eq!(spans[0].style, theme.chrome.key);
        assert!(
            spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD | Modifier::REVERSED)
        );
    }

    /// The footer's lines as text.
    fn footer_text(
        actions: &[Vec<Span<'static>>],
        closing: &[Vec<Span<'static>>],
        width: u16,
        rows: u16,
    ) -> Vec<String> {
        footer(actions, closing, width, rows, &theme())
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect()
    }

    /// The chips of a footer, in the order given.
    fn chips(keys: &[(&str, &str)]) -> Vec<Vec<Span<'static>>> {
        keys.iter()
            .map(|(key, action)| key_chip(key, action, &theme()))
            .collect()
    }

    #[test]
    fn a_footer_is_one_centred_line_when_it_fits() {
        let actions = chips(&[("enter", "open"), ("o/O", "sort")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        let lines = footer(&actions, &closing, 60, 2, &theme());
        assert_eq!(lines.len(), 1);
        assert_eq!(
            footer_text(&actions, &closing, 60, 2),
            ["enter open · o/O sort · ? help · q quit"]
        );
        assert_eq!(lines[0].alignment, Some(ratatui::layout::Alignment::Center));
    }

    #[test]
    fn a_footer_takes_a_second_line_for_the_closing_chips_when_two_rows_allow() {
        let actions = chips(&[("enter", "open"), ("o/O", "sort"), ("/", "filter")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        assert_eq!(
            footer_text(&actions, &closing, 24, 2),
            ["enter open · +2 keys", "? help · q quit"],
            "the count rides the context's line, the closing chips keep their own"
        );
        assert_eq!(
            footer_text(&actions, &closing, 50, 2),
            ["enter open · o/O sort · / filter · ? help · q quit"],
            "a footer that fits takes one line"
        );
        assert_eq!(footer(&actions, &closing, 24, 2, &theme()).len(), 2);
    }

    #[test]
    fn a_footer_with_one_row_drops_whole_chips_and_counts_them() {
        let actions = chips(&[("enter", "open"), ("o/O", "sort"), ("/", "filter")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        assert_eq!(
            footer_text(&actions, &closing, 50, 1),
            ["enter open · o/O sort · / filter · ? help · q quit"]
        );
        assert_eq!(
            footer_text(&actions, &closing, 30, 1),
            ["? help · q quit · +3 keys"],
            "the closing chips stay; the count takes the room the actions gave"
        );
        assert_eq!(
            footer_text(&actions, &closing, 19, 1),
            ["? help · +4 keys"],
            "a closing chip gives way when the count cannot sit beside both"
        );
    }

    #[test]
    fn a_closing_chip_too_wide_drops_whole_and_is_never_cut() {
        let actions = chips(&[("enter", "open")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        assert_eq!(footer_text(&actions, &closing, 10, 1), ["+3 keys"]);
        assert_eq!(footer_text(&actions, &closing, 4, 2), [""]);
    }

    #[test]
    fn a_lead_holds_while_the_other_chips_give_way() {
        let lead = vec![vec![Span::raw("8 hidden")]];
        let actions = chips(&[("enter", "open")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        let text = |width: u16| -> Vec<String> {
            footer_led(&lead, &actions, &closing, width, 1, &theme())
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect()
                })
                .collect()
        };
        assert_eq!(text(40), ["8 hidden · enter open · ? help · q quit"]);
        assert_eq!(text(27), ["8 hidden · ? help · +2 keys"]);
        assert_eq!(text(10), ["8 hidden"], "the lead is the last to go");
    }

    #[test]
    fn a_footer_that_drops_nothing_has_no_count() {
        let actions = chips(&[("enter", "open")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        let text = footer_text(&actions, &closing, 40, 1);
        assert!(!text.join("").contains("keys"), "{text:?}");
    }

    #[test]
    fn every_footer_line_fits_the_width_at_every_width() {
        // The count takes room, so no line runs past the pane at any width.
        let actions = chips(&[("enter", "open"), ("o/O", "sort"), ("/", "filter")]);
        let closing = chips(&[("?", "help"), ("q", "quit")]);
        for width in 1..=60u16 {
            for rows in 1..=2u16 {
                for line in footer_text(&actions, &closing, width, rows) {
                    assert!(
                        text_width(&line) <= usize::from(width),
                        "{width}x{rows}: {line:?} is wider than the footer"
                    );
                }
            }
        }
    }

    #[test]
    fn a_footer_with_no_closing_chips_is_the_actions_alone() {
        let actions = chips(&[("enter", "open")]);
        assert_eq!(footer_text(&actions, &[], 40, 2), ["enter open"]);
        assert_eq!(footer_text(&[], &[], 40, 2), [""]);
    }

    #[test]
    fn the_rule_is_one_thin_line_across_the_width() {
        let theme = theme();
        let rule = rule(7, &theme);
        assert_eq!(rule.content, "───────");
        assert_eq!(rule.style, theme.chrome.border);
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
