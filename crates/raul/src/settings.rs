//! The Settings tab: how the tool behaves.
//!
//! A placeholder: it holds no settings yet, and says what it will hold.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::theme::{Theme, block_title_style};

/// Draw the tab.
pub fn render(frame: &mut Frame, area: Rect, theme: &Theme) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("nothing to set yet", theme.text.dim)),
            Line::from(Span::styled(
                "will hold keybinds, sort keys, theme",
                theme.text.dim,
            )),
        ])
        .wrap(Wrap { trim: true })
        .block(
            Block::bordered()
                .title("settings")
                .title_style(block_title_style(theme))
                .border_style(theme.chrome.border),
        ),
        area,
    );
}
