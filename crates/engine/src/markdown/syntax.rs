use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet, util::LinesWithEndings,
};
use unicode_width::UnicodeWidthStr;

use super::width::{display_width, expand_tabs};
use ratatui::style::Color;

/// A syntect colour as a terminal colour. Alpha 0 is the terminal's own
/// colour, so a theme that paints no colour emits no RGB.
pub(super) fn syntect_to_color(c: syntect::highlighting::Color) -> Color {
    if c.a == 0 {
        Color::Reset
    } else {
        Color::Rgb(c.r, c.g, c.b)
    }
}

pub fn resolve_syntax<'a>(lang: &str, ss: &'a SyntaxSet) -> &'a syntect::parsing::SyntaxReference {
    let raw = lang.trim();
    let normalized = raw
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let aliases: &[&str] = match normalized.as_str() {
        "ts" | "typescript" => &[
            "JavaScript",
            "js",
            "javascript",
            "TypeScript",
            "ts",
            "typescript",
        ],
        "tsx" => &["JSX", "jsx", "JavaScript", "js", "typescriptreact", "tsx"],
        "js" | "javascript" => &["JavaScript", "js", "javascript"],
        "jsx" => &["JSX", "jsx", "JavaScript React", "JavaScript", "js"],
        "shell" | "bash" | "sh" | "zsh" => &["Bourne Again Shell (bash)", "bash", "sh"],
        "py" | "python" => &["Python", "py", "python"],
        "c" => &["C", "c"],
        "cpp" | "cxx" | "cc" | "c++" => &["C++", "cpp", "cxx", "cc"],
        "json" | "json5" => &["JSON", "json"],
        "toml" => &["TOML", "toml", "YAML", "yml", "yaml"],
        "java" => &["Java", "java"],
        "kt" | "kotlin" => &["Kotlin", "kt", "kotlin", "Java", "java"],
        "ps1" | "powershell" | "pwsh" => &["PowerShell", "ps1", "powershell", "bash", "sh"],
        "docker" | "dockerfile" => &["Dockerfile", "dockerfile", "bash", "sh"],
        "yml" | "yaml" => &["YAML", "yml", "yaml"],
        "rs" | "rust" => &["Rust", "rs", "rust"],
        _ if normalized.is_empty() => &[],
        _ => &[],
    };

    // "PHP Source" highlights php even without "<?php".
    if matches!(
        normalized.as_str(),
        "php" | "php3" | "php4" | "php5" | "php7" | "phtml"
    ) && let Some(s) = ss.find_syntax_by_name("PHP Source")
    {
        return s;
    }

    ss.find_syntax_by_token(raw)
        .or_else(|| ss.find_syntax_by_extension(raw))
        .or_else(|| ss.find_syntax_by_token(&normalized))
        .or_else(|| ss.find_syntax_by_extension(&normalized))
        .or_else(|| {
            aliases.iter().find_map(|alias| {
                ss.find_syntax_by_token(alias)
                    .or_else(|| ss.find_syntax_by_extension(alias))
                    .or_else(|| ss.find_syntax_by_name(alias))
            })
        })
        .unwrap_or_else(|| ss.find_syntax_plain_text())
}

pub(super) struct CodeLine {
    pub(super) content_spans: Vec<Span<'static>>,
}

pub(super) fn highlight_code(
    code: &str,
    lang: &str,
    ss: &SyntaxSet,
    theme: &Theme,
    render_width: usize,
    full_width: bool,
    show_line_numbers: bool,
) -> (Vec<CodeLine>, usize, usize) {
    let syntax = resolve_syntax(lang, ss);
    let mut hl = HighlightLines::new(syntax, theme);

    let mut raw: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    for line_str in LinesWithEndings::from(code) {
        let regions = hl.highlight_line(line_str, ss).unwrap_or_default();
        let mut spans = Vec::new();
        let mut text_width: usize = 0;
        for (st, text) in &regions {
            let t = expand_tabs(text.trim_end_matches('\n'), text_width);
            if t.is_empty() {
                continue;
            }
            text_width += display_width(&t);
            let mut rs = Style::default().fg(syntect_to_color(st.foreground));
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::BOLD)
            {
                rs = rs.add_modifier(Modifier::BOLD);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::ITALIC)
            {
                rs = rs.add_modifier(Modifier::ITALIC);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::UNDERLINE)
            {
                rs = rs.add_modifier(Modifier::UNDERLINED);
            }
            spans.push(Span::styled(t, rs));
        }
        raw.push((spans, text_width));
    }

    let label = if lang.is_empty() { "text" } else { lang };
    let total_lines = raw.len();
    let digit_width = if show_line_numbers {
        total_lines.max(1).to_string().len()
    } else {
        0
    };
    let gutter_width = if show_line_numbers {
        digit_width + 2
    } else {
        1
    };
    let max_inner_width = render_width
        .saturating_sub(2)
        .max(UnicodeWidthStr::width(label) + 3);
    let inner_width = if full_width {
        max_inner_width
    } else {
        let max_text = raw.iter().map(|(_, w)| *w).max().unwrap_or(0);
        let min_inner = (UnicodeWidthStr::width(label) + 3)
            .max(44)
            .min(max_inner_width);
        (max_text + 2 + gutter_width)
            .max(min_inner)
            .min(max_inner_width)
    };

    let mut out = Vec::new();
    for (spans, _text_width) in raw {
        out.push(CodeLine {
            content_spans: spans,
        });
    }
    (out, inner_width, digit_width)
}

/// What a token of a TOML line is, as the TOML grammar scopes it. A caller
/// paints each class with its own syntax role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenClass {
    /// Body text: an operator, punctuation, a space.
    Plain,
    /// A `#` comment, to the end of the line.
    Comment,
    /// A quoted string, and an escape inside one.
    String,
    /// An integer, float, datetime, boolean.
    Number,
    /// A key, or a dotted part of one.
    Tag,
    /// A `[table]` or `[[array]]` name.
    Type,
    /// Text the grammar does not accept.
    Invalid,
}

/// The TOML grammar's scopes and the class each one takes. The outermost scope
/// that names a class decides the token, so a quote inside a string is the
/// string's, and an `=` is plain although the grammar scopes it as an operator.
const TOML_CLASSES: [(&str, TokenClass); 10] = [
    ("comment", TokenClass::Comment),
    ("keyword.operator", TokenClass::Plain),
    ("string", TokenClass::String),
    ("constant.numeric", TokenClass::Number),
    ("constant.language", TokenClass::Number),
    ("entity.name.tag", TokenClass::Tag),
    ("entity.name.type", TokenClass::Type),
    ("invalid", TokenClass::Invalid),
    ("punctuation.definition.string", TokenClass::String),
    ("constant.character.escape", TokenClass::String),
];

/// A line of TOML, as tokens with their classes, from the bundled TOML grammar.
///
/// The line is parsed alone, so a token that continues from an earlier line
/// (a multi-line string) is classed as if it began here. The tokens cover the
/// line, without its end.
pub fn toml_tokens(line: &str, ss: &SyntaxSet) -> Vec<(String, TokenClass)> {
    let text = format!("{line}\n");
    let mut state = syntect::parsing::ParseState::new(resolve_syntax("toml", ss));
    let mut stack = syntect::parsing::ScopeStack::new();
    let ops = state.parse_line(&text, ss).unwrap_or_default();
    let mut tokens: Vec<(String, TokenClass)> = Vec::new();
    let mut push = |piece: &str, stack: &syntect::parsing::ScopeStack| {
        let piece = piece.trim_end_matches('\n');
        if piece.is_empty() {
            return;
        }
        let class = toml_class(stack);
        match tokens.last_mut() {
            Some((last, kind)) if *kind == class => last.push_str(piece),
            _ => tokens.push((piece.to_string(), class)),
        }
    };
    let mut at = 0;
    for (index, op) in &ops {
        push(&text[at..*index], &stack);
        let _ = stack.apply(op);
        at = *index;
    }
    push(&text[at..], &stack);
    tokens
}

/// The class of a token under `stack`: the outermost scope that names one.
fn toml_class(stack: &syntect::parsing::ScopeStack) -> TokenClass {
    for scope in stack.as_slice() {
        let name = scope.build_string();
        let named = TOML_CLASSES.iter().find(|(prefix, _)| {
            name.strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
        });
        if let Some((_, class)) = named {
            return *class;
        }
    }
    TokenClass::Plain
}
