//! The vertical window over a tab's rows.
//!
//! A tab with more rows than the content pane shows a window of them. The
//! window follows the cursor: a key that moves the cursor past the last
//! visible row moves the window with it. The cursor's row wins over the
//! announcement: when the pane is too short to draw the row and say what is
//! hidden, the tab draws the row and leaves the count to the footer, which
//! carries it beside the keys. The same fact is never painted twice. The
//! cursor is only on a row the pane does not draw when the pane is too short
//! for that row: the row itself is taller than the pane, or the pane is one
//! content row and no window can draw a row at all — then only the cut
//! announcement shows.
//!
//! The rows of a tab share one left edge: the column of the selection bar. The
//! cursor's row draws `▌` there in the focus slot and paints itself whole; every
//! other row draws a space, so the columns line up whether a row is selected or
//! not.

use ratatui::text::{Line, Span};

use coding_crew_theme::{Icon, IconSet, Theme, selected_line};

/// The columns the selection bar takes: the first column of every row.
pub const BAR: usize = 1;

/// The selection bar: the cursor bar in `chrome.border_focus` on the selected
/// row, a space on the rest.
pub fn bar(selected: bool, icons: IconSet, theme: &Theme) -> Span<'static> {
    if selected {
        Span::styled(Icon::CursorBar.glyph(icons), theme.chrome.border_focus)
    } else {
        Span::raw(" ")
    }
}

/// A row whose first span is its bar, painted to `width` columns.
///
/// The selected row's cells keep their own colours on the tint of
/// `text.selected_bg`; the bar keeps its own paint. Any other row is returned
/// as it is.
pub fn selected_row(
    line: Line<'static>,
    width: usize,
    selected: bool,
    theme: &Theme,
) -> Line<'static> {
    if !selected {
        return line;
    }
    let mut spans = line.spans;
    let rest = spans.split_off(1.min(spans.len()));
    spans.extend(selected_line(Line::from(rest), width.saturating_sub(BAR), theme).spans);
    Line::from(spans)
}

/// A row with its selection bar at the left edge, drawn in the `width - BAR`
/// columns after the bar's column.
pub fn barred(
    line: Line<'static>,
    width: usize,
    selected: bool,
    icons: IconSet,
    theme: &Theme,
) -> Line<'static> {
    let mut spans = vec![bar(selected, icons, theme)];
    spans.extend(line.spans);
    selected_row(Line::from(spans), width, selected, theme)
}

/// The row a drawn line belongs to, by its index in the tab's rows; `None` for
/// a heading, an announcement or any line that is not a row.
pub type Owner = Option<usize>;

/// The lines a window draws, each with the row it belongs to. A window adds
/// its lines through `push` and `extend` only, so a line and its owner cannot
/// drift apart.
#[derive(Debug, Clone, Default)]
pub struct Lines {
    pub lines: Vec<Line<'static>>,
    pub owners: Vec<Owner>,
}

impl Lines {
    /// One line, and the row it belongs to.
    pub fn push(&mut self, line: Line<'static>, owner: Owner) {
        self.lines.push(line);
        self.owners.push(owner);
    }

    /// Lines that all belong to one row, or to none, as they are.
    pub fn of(lines: Vec<Line<'static>>, owner: Owner) -> Self {
        let mut drawn = Lines::default();
        drawn.extend(lines, owner);
        drawn
    }

    /// Lines that all belong to one row, or to none.
    pub fn extend(&mut self, lines: Vec<Line<'static>>, owner: Owner) {
        self.owners.extend(lines.iter().map(|_| owner));
        self.lines.extend(lines);
    }

    /// The lines of another window, after these.
    pub fn append(&mut self, other: Lines) {
        self.lines.extend(other.lines);
        self.owners.extend(other.owners);
    }

    /// Keep the first `count` lines, and their owners.
    pub fn truncate(&mut self, count: usize) {
        self.lines.truncate(count);
        self.owners.truncate(count);
    }

    /// The number of lines drawn.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether no line is drawn.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// The window over a tab's rows.
///
/// The fields are private and `settle` is private: a tab reaches the window
/// only through [`View::draw`]. A copy of the ids-to-cursor-to-settle-to-page
/// block planted in another module cannot write the window back and does not
/// build, so the block has one home.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct View {
    /// The index of the first row the content pane shows.
    first: usize,
    /// The rows the pane showed at the last draw: the distance a page key
    /// moves.
    page: usize,
    /// The row each line the pane drew at the last draw belongs to, by id, in
    /// the order the lines are drawn. A click on a line is answered from this.
    lines: Vec<Option<String>>,
}

impl View {
    /// The index of the first row the pane shows, for the tests that check
    /// the window the keys moved.
    #[cfg(test)]
    pub fn first(&self) -> usize {
        self.first
    }

    /// The rows the pane showed at the last draw.
    pub fn page(&self) -> usize {
        self.page
    }

    /// Back to the top, with no page measured yet.
    pub fn reset(&mut self) {
        *self = View::default();
    }

    /// The id of the row the pane drew on its `line`th line at the last draw,
    /// or `None` when that line is not a row.
    pub fn row_at(&self, line: usize) -> Option<&str> {
        self.lines.get(line)?.as_deref()
    }

    /// Forget the lines drawn: a frame that draws no rows leaves none to click.
    pub fn forget_lines(&mut self) {
        self.lines.clear();
    }

    /// Draw a tab's rows through the window the keys move.
    ///
    /// `ids` are the tab's rows in the order they are drawn; `window` returns
    /// the lines a window at the given first row would draw, whether the pane
    /// announced what it hides, the rows that window shows, and whether it is
    /// complete. The window is settled so the cursor is on a row the pane
    /// shows, and the lines of the settled window are returned ready to draw,
    /// with whether they announce. This is the one place a row tab finds the
    /// cursor, settles the window, re-windows at it, measures the page, and
    /// keeps which row each line is.
    pub fn draw(
        &mut self,
        ids: &[String],
        selected: Option<&str>,
        mut window: impl FnMut(usize) -> (Lines, bool, Vec<usize>, bool),
    ) -> (Vec<Line<'static>>, bool) {
        let cursor = selected
            .and_then(|id| ids.iter().position(|other| other == id))
            .unwrap_or(0);
        let first = self.settle(ids.len(), cursor, |first| {
            let (_, _, shown, whole) = window(first);
            (shown, whole)
        });
        let (drawn, announced, shown, _) = window(first);
        self.page = shown.len();
        self.lines = drawn
            .owners
            .into_iter()
            .map(|owner| owner.and_then(|index| ids.get(index).cloned()))
            .collect();
        (drawn.lines, announced)
    }

    /// The first row to show so that the cursor is on a visible row.
    ///
    /// `shown` returns the rows a window at the given first row would show,
    /// and whether that window is complete: it holds the cursor and the tab
    /// has room to say what it hides. The window moves the least it can: the
    /// cursor is brought into view a row at a time, and the window never
    /// starts past the last row. A window that would hide what it holds is
    /// passed over for the next one, so a scrolled pane still announces its
    /// hidden rows.
    ///
    /// The cursor's row is the one thing the window may not leave out, so
    /// `shown` returns it whenever the pane can draw it whole, even when that
    /// leaves no room for the announcement. A row taller than the pane cannot
    /// be drawn: the window then walks back toward the top until it finds a
    /// window that draws a row. When the pane is so short that no window
    /// draws a row — one content row and a row taller than it — it settles at
    /// the top and the tab shows what it can: the cut announcement, and no
    /// row.
    fn settle(
        &mut self,
        total: usize,
        cursor: usize,
        mut shown: impl FnMut(usize) -> (Vec<usize>, bool),
    ) -> usize {
        if total == 0 {
            self.first = 0;
            self.page = 0;
            return 0;
        }
        // A pane that holds every row and is whole has nowhere to scroll,
        // whatever the window was before. A pane that holds every row but not
        // the tab's closing line still scrolls to make room for it.
        let (all_rows, all_whole) = shown(0);
        if all_rows.len() == total && all_whole {
            self.first = 0;
            self.page = all_rows.len();
            return 0;
        }
        let cursor = cursor.min(total - 1);
        let mut first = self.first.min(total - 1);
        if cursor < first {
            first = cursor;
        }
        let (mut rows, mut whole) = shown(first);
        while !(rows.contains(&cursor) && whole) && first < cursor {
            first += 1;
            (rows, whole) = shown(first);
        }
        if !rows.contains(&cursor) {
            first = cursor;
            rows = shown(first).0;
        }
        while first > 0 && rows.is_empty() {
            first -= 1;
            rows = shown(first).0;
        }
        self.first = first;
        self.page = rows.len();
        first
    }
}

#[cfg(test)]
mod tests {
    use coding_crew_theme::Preset;

    use super::*;

    /// A list of `total` rows where a window at `first` shows three rows.
    fn three_row_window(total: usize) -> impl FnMut(usize) -> (Vec<usize>, bool) {
        move |first| {
            let first = first.min(total.saturating_sub(1));
            ((first..(first + 3).min(total)).collect(), true)
        }
    }

    #[test]
    fn the_window_follows_the_cursor_down_a_row_at_a_time() {
        let mut view = View::default();
        // The cursor on the fourth row of a three-row window moves the
        // window one row, not all the way to the cursor.
        assert_eq!(view.settle(10, 3, three_row_window(10)), 1);
        assert_eq!(view.first, 1);
        assert_eq!(view.page, 3);
        // The cursor on the first row returns the window to the top.
        assert_eq!(view.settle(10, 0, three_row_window(10)), 0);
        // The window clamps at the last row.
        assert_eq!(view.settle(10, 9, three_row_window(10)), 7);
        assert_eq!(view.first, 7);
        assert_eq!(view.page, 3);
    }

    #[test]
    fn the_window_of_an_empty_list_is_at_the_top() {
        let mut view = View {
            first: 4,
            page: 3,
            ..View::default()
        };
        assert_eq!(view.settle(0, 0, |_| (Vec::new(), true)), 0);
        assert_eq!(view.first, 0);
        assert_eq!(view.page, 0);
    }

    #[test]
    fn a_window_that_cannot_say_what_it_hides_is_passed_over() {
        // The cursor is on the last of ten rows. A three-row window first
        // holds it at the seventh row, but the count of what is hidden does
        // not fit there: the window moves on to the eighth, where it does.
        let mut view = View::default();
        let shown = |first: usize| {
            let rows = (first..(first + 3).min(10)).collect();
            (rows, first >= 8)
        };
        assert_eq!(view.settle(10, 9, shown), 8);
        assert_eq!(view.first, 8);
        assert_eq!(view.page, 2);
    }

    #[test]
    fn a_pane_that_holds_every_row_scrolls_nowhere() {
        let mut view = View {
            first: 4,
            page: 3,
            ..View::default()
        };
        assert_eq!(view.settle(10, 0, |_| ((0..10).collect(), true)), 0);
        assert_eq!(view.first, 0);
        assert_eq!(view.page, 10);
    }

    #[test]
    fn a_selected_row_is_barred_and_painted_whole() {
        let theme = Theme::preset(Preset::Dark);
        let line = Line::from(Span::styled("ab", theme.text.strong));
        let row = barred(line, 5, true, IconSet::Nerd, &theme);
        assert_eq!(row.spans[0].content, "▌");
        assert_eq!(row.spans[0].style, theme.chrome.border_focus);
        assert_eq!(row.width(), 5);
        for span in &row.spans[1..] {
            assert_eq!(span.style.bg, theme.text.selected_bg.bg, "{span:?}");
        }
    }

    #[test]
    fn an_unselected_row_keeps_its_paint_behind_a_space() {
        let theme = Theme::preset(Preset::Dark);
        let line = Line::from(Span::styled("ab", theme.text.strong));
        let row = barred(line, 5, false, IconSet::Nerd, &theme);
        assert_eq!(row.spans[0].content, " ");
        assert_eq!(row.spans[1].content, "ab");
        assert_eq!(row.spans[1].style, theme.text.strong);
    }
}
