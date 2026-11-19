//! The vertical window over a tab's rows.
//!
//! A tab with more rows than the content pane shows a window of them. The
//! window follows the cursor: a key that moves the cursor past the last
//! visible row moves the window with it. The cursor's row wins over the
//! announcement: when the pane is too short to draw the row and say what is
//! hidden, the tab draws the row and leaves the count out. The cursor is only
//! on a row the pane does not draw when the pane is too short for that row:
//! the row itself is taller than the pane, or the pane is one content row
//! and no window can draw a row at all — then only the cut announcement
//! shows.

/// The window over a tab's rows.
///
/// The fields are private and `settle` is private: a tab reaches the window
/// only through [`View::draw`]. A copy of the ids-to-cursor-to-settle-to-page
/// block planted in another module cannot write the window back and does not
/// build, so the block has one home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct View {
    /// The index of the first row the content pane shows.
    first: usize,
    /// The rows the pane showed at the last draw: the distance a page key
    /// moves.
    page: usize,
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

    /// Draw a tab's rows through the window the keys move.
    ///
    /// `ids` are the tab's rows in the order they are drawn; `window` returns
    /// the lines a window at the given first row would draw, the rows that
    /// window shows, and whether it is complete. The window is settled so the
    /// cursor is on a row the pane shows, and the lines of the settled window
    /// are returned ready to draw. This is the one place a row tab finds the
    /// cursor, settles the window, re-windows at it, and measures the page.
    pub fn draw<T>(
        &mut self,
        ids: &[String],
        selected: Option<&str>,
        mut window: impl FnMut(usize) -> (T, Vec<usize>, bool),
    ) -> T {
        let cursor = selected
            .and_then(|id| ids.iter().position(|other| other == id))
            .unwrap_or(0);
        let first = self.settle(ids.len(), cursor, |first| {
            let (_, shown, whole) = window(first);
            (shown, whole)
        });
        let (rows, shown, _) = window(first);
        self.page = shown.len();
        rows
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
        // A pane that holds every row has nowhere to scroll, whatever the
        // window was before.
        let (all_rows, _) = shown(0);
        if all_rows.len() == total {
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
        let mut view = View { first: 4, page: 3 };
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
        let mut view = View { first: 4, page: 3 };
        assert_eq!(view.settle(10, 0, |_| ((0..10).collect(), true)), 0);
        assert_eq!(view.first, 0);
        assert_eq!(view.page, 10);
    }
}
