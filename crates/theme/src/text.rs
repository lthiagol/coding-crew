//! Measuring and cutting text the way ratatui draws it.

use ratatui::buffer::CellWidth;
use ratatui::style::Style;
use ratatui::text::Span;

/// The terminal columns a text takes: the sum of the graphemes ratatui draws.
///
/// The whole string is not measured at once. ratatui draws a grapheme at a
/// time and drops the control characters, so a tab takes no column, and a
/// ligature script takes one column per grapheme where the whole string
/// takes one.
pub fn text_width(text: &str) -> usize {
    Span::raw(text.to_string())
        .styled_graphemes(Style::default())
        .map(|grapheme| usize::from(grapheme.symbol.cell_width()))
        .sum()
}

/// The text, cut to `width` columns.
pub fn cut_to(text: &str, width: usize) -> String {
    if text_width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let head = head_to(text, width - 1);
    format!("{head}…")
}

/// The longest head of `text` that fits in `budget` columns.
///
/// A grapheme is the smallest piece a terminal draws: cutting on scalars
/// can leave a dangling joiner or a combining mark behind, so the cut is on
/// the grapheme boundaries ratatui draws with.
pub fn head_to(text: &str, budget: usize) -> String {
    let span = Span::raw(text.to_string());
    let mut used = 0;
    let mut head = String::new();
    for grapheme in span.styled_graphemes(Style::default()) {
        let width = text_width(grapheme.symbol);
        if used + width > budget {
            break;
        }
        used += width;
        head.push_str(grapheme.symbol);
    }
    head
}

/// The first grapheme of `text`: the smallest piece a terminal draws.
pub fn first_grapheme(text: &str) -> String {
    Span::raw(text.to_string())
        .styled_graphemes(Style::default())
        .next()
        .map(|grapheme| grapheme.symbol.to_string())
        .unwrap_or_default()
}

/// The text, padded to `width` columns.
pub fn pad(text: &str, width: usize) -> String {
    let length = text_width(text);
    if length >= width {
        return text.to_string();
    }
    format!("{text}{}", " ".repeat(width - length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn the_first_grapheme_is_what_the_terminal_draws_once() {
        assert_eq!(first_grapheme("e\u{301}x"), "e\u{301}");
        assert_eq!(first_grapheme("界x"), "界");
        assert_eq!(first_grapheme(""), "");
    }

    #[test]
    fn the_width_is_the_width_ratatui_draws() {
        // The whole-string width of a tab or an Arabic ligature differs from
        // what ratatui paints, so each text is measured against the paint.
        for text in [
            "a\tb",
            "\u{644}\u{627}",
            "e\u{301}x",
            "界x",
            "\u{26a0}\u{fe0f}",
        ] {
            assert_eq!(text_width(text), drawn(text), "{text:?}");
        }
    }

    /// The columns ratatui paints a text in, from column zero.
    fn drawn(text: &str) -> usize {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 1));
        let (column, _) = buffer.set_stringn(0, 0, text, 20, Style::default());
        usize::from(column)
    }
}
