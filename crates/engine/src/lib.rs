//! The markdown engine from [leaf](https://github.com/rivolink/leaf), vendored.
//!
//! Leaf is a terminal markdown viewer. Its `src/markdown` modules turn markdown
//! into ratatui lines; this crate keeps that engine and leaves the leaf
//! application behind. See `VENDORED.md` for the pinned upstream commit and
//! every deliberate deviation.

pub mod markdown;
pub mod theme;

#[cfg(test)]
mod tests;

pub use markdown::{
    CodeBlockInfo, LinkId, LinkSpan, ParseResult, TocEntry, parse_markdown,
    parse_markdown_with_width,
};
pub use theme::{MarkdownTheme, ThemePreset};

use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSet};

/// The TOML grammar, which syntect's default set lacks.
const TOML_SYNTAX: &str = include_str!("../assets/toml.sublime-syntax");

/// Leaf's default syntaxes, plus the bundled TOML grammar.
///
/// This is an expensive startup asset: it parses and compiles the whole default
/// syntax set (hundreds of milliseconds and several megabytes). Load it once at
/// startup, keep the result, and pass it to `parse_markdown`; never call it
/// from a draw loop.
pub fn syntax_set() -> SyntaxSet {
    let toml = SyntaxDefinition::load_from_str(TOML_SYNTAX, true, None)
        .expect("the bundled TOML grammar parses");
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
    builder.add(toml);
    builder.build()
}

/// Leaf's default syntect themes.
///
/// This is an expensive startup asset, like [`syntax_set`]. Load it once and
/// keep the result; never call it from a draw loop.
pub fn theme_set() -> ThemeSet {
    ThemeSet::load_defaults()
}

/// The syntect theme paired with a markdown preset.
///
/// Falls back to the default preset's theme, then to any theme, matching
/// leaf's `current_syntect_theme`.
pub fn syntect_theme(preset: ThemePreset, themes: &ThemeSet) -> &Theme {
    themes
        .themes
        .get(preset.syntect_theme_name())
        .or_else(|| {
            themes
                .themes
                .get(ThemePreset::default().syntect_theme_name())
        })
        .or_else(|| themes.themes.values().next())
        .expect("syntect theme set is empty")
}
