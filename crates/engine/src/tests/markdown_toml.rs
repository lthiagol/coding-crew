use super::{test_assets, test_md_theme};
use crate::markdown::{TokenClass, parse_markdown, resolve_syntax};
use ratatui::style::Color;

/// The colour of the first span whose text is `needle`, if it has one.
fn colour_of(lines: &[ratatui::text::Line<'static>], needle: &str) -> Option<Color> {
    lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .find(|span| span.content.trim() == needle)
        .and_then(|span| span.style.fg)
}

#[test]
fn toml_resolves_to_the_bundled_grammar_not_yaml() {
    let ss = crate::syntax_set();
    for tag in ["toml", "TOML", "toml {1-3}"] {
        assert_eq!(resolve_syntax(tag, &ss).name, "TOML", "{tag}");
    }
}

#[test]
fn a_toml_fence_is_painted_with_the_syntax_roles() {
    let ss = crate::syntax_set();
    let (_, theme) = test_assets();
    let md = "```toml\n[package]\n# the crate\nname = \"coding\"\nversion = 3\nready = true\n```\n";
    let (lines, _, _, _) = parse_markdown(md, &ss, &theme, &test_md_theme(), false, false).into();

    // The table, key, string, number and comment are five colours of their
    // own; a fence that fell back to plain text would paint them alike. A
    // boolean takes the number role, as the markdown code themes map it.
    let roles = [
        ("[package]", "table"),
        ("name", "key"),
        ("coding", "string"),
        ("3", "number"),
        ("# the crate", "comment"),
    ];
    let colours: Vec<(&str, Color)> = roles
        .iter()
        .map(|(text, role)| {
            let colour = colour_of(&lines, text)
                .unwrap_or_else(|| panic!("no painted {role} span for {text:?}"));
            (*role, colour)
        })
        .collect();
    for (index, (role, colour)) in colours.iter().enumerate() {
        for (other, other_colour) in &colours[index + 1..] {
            assert_ne!(colour, other_colour, "{role} and {other} share a colour");
        }
    }
    assert!(
        colour_of(&lines, "true").is_some(),
        "the boolean is not painted"
    );
}

#[test]
fn toml_tokens_class_a_key_a_string_a_number_and_a_comment() {
    let ss = crate::syntax_set();
    // The class of the token that holds `needle`.
    let class_of = |tokens: &[(String, TokenClass)], needle: &str| {
        tokens
            .iter()
            .find(|(piece, _)| piece.contains(needle))
            .map(|(_, class)| *class)
    };
    let tokens = crate::toml_tokens("round_limit = 5  # the rounds", &ss);
    assert_eq!(class_of(&tokens, "round_limit"), Some(TokenClass::Tag));
    assert_eq!(class_of(&tokens, " = "), Some(TokenClass::Plain));
    assert_eq!(class_of(&tokens, "5"), Some(TokenClass::Number));
    assert_eq!(class_of(&tokens, "# the rounds"), Some(TokenClass::Comment));
    let tokens = crate::toml_tokens("theme = \"dark\" # \"not a string\"", &ss);
    assert_eq!(
        class_of(&tokens, "dark"),
        Some(TokenClass::String),
        "{tokens:?}"
    );
    assert_eq!(
        class_of(&tokens, "# \"not"),
        Some(TokenClass::Comment),
        "{tokens:?}"
    );
    let tokens = crate::toml_tokens("[raul]", &ss);
    assert_eq!(class_of(&tokens, "raul"), Some(TokenClass::Type));
    // The tokens cover the line, byte for byte.
    let line = "gates = [{ from = \"a\", to = \"b\" }]";
    let joined: String = crate::toml_tokens(line, &ss)
        .iter()
        .map(|(piece, _)| piece.as_str())
        .collect();
    assert_eq!(joined, line);
}
