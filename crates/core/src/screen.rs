//! The terminal screen: the alternate screen both TUIs open, and the plain
//! render that needs no terminal.
//!
//! The two binaries share the screen contract, so they share its code: one
//! conversion from a drawn buffer to text, one way in and out of the
//! alternate screen.

use std::io::Stdout;

use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::{Buffer, CellWidth};

/// The buffer as plain text: one row per line, no paint, and no trailing
/// spaces that carry no information.
///
/// A symbol that takes more than one column is emitted once, and the
/// continuation cells ratatui reset behind it are skipped: the terminal gives
/// the symbol its columns already.
///
/// Every row ends with a newline, so the output has a stable final newline.
pub fn plain_text(buffer: &Buffer) -> String {
    let area = *buffer.area();
    let mut text = String::new();
    for y in 0..area.height {
        let mut row = String::new();
        let mut skip = 0u16;
        for x in 0..area.width {
            let Some(cell) = buffer.cell((x, y)) else {
                continue;
            };
            if skip == 0 {
                row.push_str(cell.symbol());
            }
            skip = skip.max(cell.cell_width()).saturating_sub(1);
        }
        text.push_str(row.trim_end_matches(' '));
        text.push('\n');
    }
    text
}

/// Enter raw mode, the alternate screen and mouse capture.
pub fn setup() -> std::io::Result<Terminal<CrosstermBackend<Stdout>>> {
    chain_hook(leave_terminal);
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

/// Chain a panic hook that runs `leave` before the hook already set prints the
/// panic, so the message lands on the shell's screen rather than behind the
/// alternate one.
pub fn chain_hook(leave: fn()) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &std::panic::PanicHookInfo<'_>| {
        leave();
        previous(info);
    }));
}

/// Leave mouse capture, the alternate screen and raw mode, and show the cursor
/// again.
fn leave_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        std::io::stdout(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        Show
    );
}

/// Leave mouse capture, the alternate screen and raw mode, and show the cursor
/// again.
pub fn restore(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    /// The text of a buffer drawn with `draw`.
    fn rendered(width: u16, height: u16, draw: impl FnOnce(&mut ratatui::Frame)) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a terminal");
        terminal.draw(draw).expect("a draw");
        plain_text(terminal.backend().buffer())
    }

    #[test]
    fn a_row_takes_one_line_and_drops_its_trailing_spaces() {
        let text = rendered(10, 2, |frame| {
            let area = frame.area();
            frame.render_widget(ratatui::widgets::Paragraph::new("ab"), area);
        });
        assert_eq!(text, "ab\n\n");
    }

    #[test]
    fn every_row_ends_with_a_newline() {
        let text = rendered(4, 3, |_| {});
        assert_eq!(text.matches('\n').count(), 3, "{text:?}");
        assert!(text.ends_with('\n'), "{text:?}");
    }

    #[test]
    fn a_wide_symbol_is_emitted_once_and_its_continuation_skipped() {
        // `世` takes two columns: the cell behind it is a continuation, and
        // the text carries the symbol once, not twice.
        let text = rendered(6, 1, |frame| {
            let area = frame.area();
            frame.render_widget(ratatui::widgets::Paragraph::new("世ab"), area);
        });
        assert_eq!(text, "世ab\n");
    }
}
