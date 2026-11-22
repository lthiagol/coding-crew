//! The document reads like a page: each construct drawn at the pane's two
//! widths, every line within its width.

use ratatui::{style::Modifier, text::Line};

use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{display_width, line_plain_text, parse_markdown_with_width};
use crate::theme::MarkdownTheme;

const WIDTHS: [usize; 2] = [40, 100];

fn lines_at(md: &str, width: usize, theme: &MarkdownTheme) -> Vec<Line<'static>> {
    let (ss, syntax) = test_assets();
    let (lines, _, _, _) =
        parse_markdown_with_width(md, &ss, &syntax, width, theme, false, true).into();
    lines
        .into_iter()
        .filter(|line| !line_plain_text(line).is_empty())
        .collect()
}

fn render(md: &str, width: usize) -> Vec<String> {
    let lines = lines_at(md, width, &test_md_theme());
    rendered_non_empty_lines(&lines)
}

fn assert_fits(rendered: &[String], width: usize) {
    for line in rendered {
        assert!(
            display_width(line) <= width,
            "width {width}: {line:?} is {} columns",
            display_width(line)
        );
    }
}

#[test]
fn h1_leads_with_a_bar_and_closes_with_a_rule_to_the_edge() {
    for width in WIDTHS {
        let rendered = render("# Title\n", width);
        let rule = width - 2 - "Title".len() - 1;
        assert_eq!(rendered, [format!("▍ Title {}", "─".repeat(rule))]);
        assert_eq!(display_width(&rendered[0]), width);
    }
}

#[test]
fn h1_that_does_not_fit_wraps_without_a_rule() {
    let title = "A heading that is long enough to need more than one row at forty columns";
    let rendered = render(&format!("# {title}\n"), 40);
    assert!(rendered.len() > 1, "{rendered:?}");
    assert!(rendered[0].starts_with("▍ "), "{rendered:?}");
    assert!(rendered[1..].iter().all(|line| line.starts_with("  ")));
    assert!(
        !rendered.iter().any(|line| line.contains('─')),
        "{rendered:?}"
    );
    assert_fits(&rendered, 40);
}

#[test]
fn h2_leads_with_an_arrow_and_has_no_rule() {
    for width in WIDTHS {
        let rendered = render("## Section\n", width);
        assert_eq!(rendered, ["▸ Section"]);
    }
}

#[test]
fn h3_and_deeper_are_bold_text_alone() {
    for width in WIDTHS {
        for (md, title) in [("### Third\n", "Third"), ("#### Fourth\n", "Fourth")] {
            let lines = lines_at(md, width, &test_md_theme());
            assert_eq!(rendered_non_empty_lines(&lines), [title]);
            let style = lines[0].spans[0].style;
            assert!(style.add_modifier.contains(Modifier::BOLD), "{md}");
            assert!(!style.add_modifier.contains(Modifier::ITALIC), "{md}");
        }
    }
}

#[test]
fn tables_round_the_outer_corners_and_rule_only_the_header() {
    let md = "| a | b |\n| - | - |\n| 1 | 2 |\n| 3 | 4 |\n";
    for width in WIDTHS {
        let rendered = render(md, width);
        assert_fits(&rendered, width);
        let edges: Vec<char> = rendered
            .iter()
            .map(|line| line.chars().next().unwrap())
            .collect();
        assert_eq!(edges.first(), Some(&'╭'), "{rendered:?}");
        assert_eq!(edges.last(), Some(&'╰'), "{rendered:?}");
        assert_eq!(
            edges.iter().filter(|&&c| c == '├').count(),
            1,
            "{rendered:?}"
        );
        let rule = rendered.iter().find(|line| line.starts_with('├')).unwrap();
        assert!(rule.ends_with('┤'), "{rule:?}");
        assert!(rule.chars().all(|c| "├┼┤─".contains(c)), "{rule:?}");
        let joined = rendered.join("\n");
        assert!(!joined.contains('═'), "{rendered:?}");
        assert!(rendered[0].ends_with('╮'), "{rendered:?}");
        assert!(rendered.last().unwrap().ends_with('╯'), "{rendered:?}");
    }
}

#[test]
fn blockquote_is_a_bar_with_the_text_indented_two() {
    for width in WIDTHS {
        let rendered = render("> quoted text\n", width);
        assert_eq!(rendered, ["▎ quoted text"]);
        assert_fits(&rendered, width);
    }
}

#[test]
fn task_items_draw_the_done_and_open_glyphs() {
    let theme = test_md_theme();
    assert_eq!(theme.task_done_glyph, "✓");
    assert_eq!(theme.task_open_glyph, "○");
    for width in WIDTHS {
        let rendered = render("- [x] done\n- [ ] open\n", width);
        assert_eq!(rendered, ["✓ done", "○ open"]);
        assert_fits(&rendered, width);
    }
}

#[test]
fn alerts_lead_with_their_glyph_and_label_and_a_coloured_bar() {
    let theme = test_md_theme();
    let alerts = [
        ("NOTE", "ℹ", "Note", theme.alert_note),
        ("TIP", "✦", "Tip", theme.alert_tip),
        ("IMPORTANT", "!", "Important", theme.alert_important),
        ("WARNING", "⚠", "Warning", theme.alert_warning),
        ("CAUTION", "✗", "Caution", theme.alert_caution),
    ];
    for width in WIDTHS {
        for (kind, glyph, label, colour) in alerts {
            let md = format!("> [!{kind}]\n> body\n");
            let lines = lines_at(&md, width, &theme);
            let rendered = rendered_non_empty_lines(&lines);
            assert_eq!(rendered, [format!("▎ {glyph} {label}"), "▎ body".into()]);
            assert_fits(&rendered, width);
            let header = &lines[0].spans.last().unwrap();
            assert_eq!(header.style.fg, Some(colour), "{kind}");
            assert!(header.style.add_modifier.contains(Modifier::BOLD), "{kind}");
            assert_eq!(lines[1].spans[0].style.fg, Some(colour), "{kind}");
        }
    }
}

#[test]
fn alert_glyphs_are_configuration() {
    let mut theme = test_md_theme();
    theme.alert_glyphs = ["i", "t", "I", "w", "c"];
    let lines = lines_at("> [!WARNING]\n> body\n", 40, &theme);
    assert_eq!(rendered_non_empty_lines(&lines)[0], "▎ w Warning");
}

#[test]
fn links_show_their_text_underlined_and_the_url_dim_when_it_differs() {
    let theme = test_md_theme();
    for width in WIDTHS {
        let lines = lines_at("see [docs](https://example.test/docs) now\n", width, &theme);
        let rendered = rendered_non_empty_lines(&lines);
        assert_fits(&rendered, width);
        assert!(
            rendered[0].contains("docs (https://example.test/docs)"),
            "{rendered:?}"
        );
        let text = lines[0]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "docs")
            .expect("the link text is its own span");
        assert_eq!(text.style.fg, Some(theme.link_text));
        assert!(text.style.add_modifier.contains(Modifier::UNDERLINED));
        let url = lines[0]
            .spans
            .iter()
            .find(|span| span.content.contains("https://example.test/docs"))
            .expect("the URL is a span");
        assert!(url.style.add_modifier.contains(Modifier::DIM));
    }
}

#[test]
fn an_autolink_does_not_repeat_its_url() {
    for width in WIDTHS {
        let rendered = render("<https://example.test/auto>\n", width);
        assert_eq!(rendered.len(), 1, "{rendered:?}");
        assert_eq!(rendered[0].matches("https://example.test/auto").count(), 1);
        assert!(!rendered[0].contains('('), "{rendered:?}");
    }
}

#[test]
fn a_long_url_wraps_and_fits() {
    let md = "[a](https://example.test/a/very/long/path/that/keeps/going/and/going/on)\n";
    let rendered = render(md, 40);
    assert!(rendered.len() > 1, "{rendered:?}");
    assert_fits(&rendered, 40);
}

#[test]
fn a_table_link_shows_its_url_dim_in_the_cell() {
    let theme = test_md_theme();
    let md = "| Name | Link |\n|------|------|\n| Test | [example](https://example.test/t) |\n";
    let lines = lines_at(md, 100, &theme);
    let rendered = rendered_non_empty_lines(&lines);
    assert!(
        rendered
            .iter()
            .any(|line| line.contains("example (https://example.test/t)")),
        "{rendered:?}"
    );
    let url = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .find(|span| span.content.contains("https://example.test/t"))
        .expect("the URL is a span");
    assert!(url.style.add_modifier.contains(Modifier::DIM));
}
