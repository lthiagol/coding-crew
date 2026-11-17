//! Measuring and cutting text the way ratatui draws it.

use ratatui::style::Style;
use ratatui::text::Span;

/// The terminal columns a text takes.
pub fn text_width(text: &str) -> usize {
    Span::raw(text.to_string()).width()
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

/// The text, padded to `width` columns.
pub fn pad(text: &str, width: usize) -> String {
    let length = text_width(text);
    if length >= width {
        return text.to_string();
    }
    format!("{text}{}", " ".repeat(width - length))
}
