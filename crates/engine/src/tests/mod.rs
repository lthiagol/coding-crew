use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::markdown::line_plain_text;
use crate::theme::{MarkdownTheme, ThemePreset, theme_by_preset};

mod markdown_blocks;
mod markdown_embedded;
mod markdown_footnotes;
mod markdown_html_tag;
mod markdown_links;
mod markdown_list_blocks;
mod markdown_lists;
mod markdown_page;
mod markdown_rendering;
mod markdown_tables;
mod markdown_toml;
mod wrapping;

pub(super) fn test_assets() -> (SyntaxSet, Theme) {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ts.themes["base16-ocean.dark"].clone();
    (ss, theme)
}

pub(super) fn test_md_theme() -> MarkdownTheme {
    *theme_by_preset(ThemePreset::OceanDark)
}

pub(super) fn render_buffer(lines: &[Line<'static>]) -> ratatui::buffer::Buffer {
    let width = lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(1)
        .max(1) as u16;
    let height = lines.len().max(1) as u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            f.render_widget(Paragraph::new(lines.to_vec()), f.area());
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

pub(super) fn find_symbol(buffer: &ratatui::buffer::Buffer, symbol: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer
                .cell((x, y))
                .is_some_and(|cell| cell.symbol() == symbol)
            {
                return Some((x, y));
            }
        }
    }
    None
}

pub(super) fn rendered_non_empty_lines(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(line_plain_text)
        .filter(|line| !line.is_empty())
        .collect()
}

/// Leaf's `fence_wrap`, kept for the file-mode rendering tests.
pub(super) fn fence_wrap(src: &str, ext: &str) -> String {
    format!("````{ext}\n{src}\n````")
}
