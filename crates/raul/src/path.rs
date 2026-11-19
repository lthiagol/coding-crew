//! The Path tab: what happens next, in order.
//!
//! A spine of what can run now, with branches for what is waiting, and the
//! blocked branch forked under what blocks it.
//!
//! The tree holds the live work: the tasks that are neither `draft` nor
//! `done`. The status of a task is the colour of its row; the branch says
//! who the next action belongs to.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use coding_crew_core::{Status, Task};

use crate::data::{Work, has_overlay, live_claim};
use crate::view::View;
use coding_crew_theme::text::{cut_to, first_grapheme, head_to, text_width};
use coding_crew_theme::{Theme, block_title_style, phase_style, selected_line};

/// The columns the border of the block takes.
const BORDER: usize = 2;
/// The gap between the id of a row and its title.
const GAP: usize = 3;
/// The marker of the task that happens next.
const NEXT: &str = "●";
/// The name of the spine, for the announcement.
const SPINE: &str = "EXECUTION";

/// The branches of the tree, in the order they are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// `groomed` — the human approves it.
    AwaitingApproval,
    /// `approved`, but not claimable.
    Blocked,
    /// `executing` and `remediating`.
    Running,
    /// `code-review`.
    Review,
    /// `needs-human` — the review budget ran out.
    Escalated,
}

impl Branch {
    /// Every branch, in the order of the tree.
    pub const ALL: [Branch; 5] = [
        Branch::AwaitingApproval,
        Branch::Blocked,
        Branch::Running,
        Branch::Review,
        Branch::Escalated,
    ];

    /// The name of the branch.
    pub fn label(&self) -> &'static str {
        match self {
            Branch::AwaitingApproval => "AWAITING APPROVAL",
            Branch::Blocked => "BLOCKED",
            Branch::Running => "RUNNING",
            Branch::Review => "REVIEW",
            Branch::Escalated => "ESCALATED",
        }
    }

    /// The phase the branch names — the slot its label takes.
    pub fn phase(&self) -> Status {
        match self {
            Branch::AwaitingApproval => Status::Groomed,
            Branch::Blocked => Status::Approved,
            Branch::Running => Status::Executing,
            Branch::Review => Status::CodeReview,
            Branch::Escalated => Status::NeedsHuman,
        }
    }
}

/// What holds a task that is not waiting on a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// An overlay: the task is held on purpose.
    Overlay,
    /// A claim another session holds.
    Claim,
}

impl Hold {
    /// The words that say what holds the task.
    pub fn label(&self) -> &'static str {
        match self {
            Hold::Overlay => "overlay",
            Hold::Claim => "claim held elsewhere",
        }
    }
}

/// A task in the tree.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The id of the task.
    pub id: String,
    /// The title of the task.
    pub title: String,
    /// The status of the task — the colour of its row.
    pub status: Status,
    /// The dependencies it still waits on.
    pub waits_on: Vec<String>,
    /// What holds it, when it is not only the dependencies.
    pub hold: Option<Hold>,
}

/// A group of the blocked branch: the tasks that wait on the same
/// dependency.
#[derive(Debug, Clone)]
pub struct Group {
    /// The dependency the group is forked under. The tasks that are held by
    /// an overlay or a claim alone have none.
    pub blocker: Option<String>,
    /// The tasks of the group.
    pub rows: Vec<Entry>,
}

/// A branch of the tree.
#[derive(Debug, Clone)]
pub struct BranchView {
    /// The branch.
    pub branch: Branch,
    /// The tasks in the branch.
    pub count: usize,
    /// The groups of the branch.
    pub groups: Vec<Group>,
}

/// The tree of the live work.
#[derive(Debug, Clone)]
pub struct Tree {
    /// The claimable tasks, in dependency order. The first is what happens
    /// next.
    pub spine: Vec<Entry>,
    /// The branches, in the order they are drawn.
    pub branches: Vec<BranchView>,
}

impl Tree {
    /// Whether the tree holds nothing.
    pub fn is_empty(&self) -> bool {
        self.spine.is_empty() && self.branches.is_empty()
    }

    /// The ids of the tree's rows, in the order they are drawn: the spine,
    /// then the branches.
    ///
    /// The detail pane opens from this tab: the rows are what it can open.
    /// This is the one assembly of the drawn order: the cursor's row list
    /// and the tree's window both read it.
    pub fn row_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.spine.iter().map(|entry| entry.id.clone()).collect();
        ids.extend(
            self.branches
                .iter()
                .flat_map(|view| view.groups.iter())
                .flat_map(|group| group.rows.iter())
                .map(|entry| entry.id.clone()),
        );
        ids
    }

    /// The tasks of a branch.
    #[cfg(test)]
    pub fn rows(&self, branch: Branch) -> usize {
        self.branches
            .iter()
            .find(|view| view.branch == branch)
            .map_or(0, |view| view.count)
    }
}

/// The tree of the live work in the folder, at `now`.
///
/// `now` is the frame's instant, read once by the caller: the spine's claim
/// test, the BLOCKED branch's held test and the cursor's row list all answer
/// from the same clock, so a lease cannot expire between them.
pub fn tree(work: &Work, now: &str) -> Tree {
    let spine = work
        .claimable_at(now)
        .into_iter()
        .map(|task| entry(task, work))
        .collect();
    let branches = Branch::ALL
        .iter()
        .map(|branch| branch_view(work, *branch, now))
        .filter(|view| view.count > 0)
        .collect();
    Tree { spine, branches }
}

/// A task, as the tree holds it.
fn entry(task: &Task, work: &Work) -> Entry {
    Entry {
        id: task.id.clone(),
        title: task.title.clone(),
        status: task.status,
        waits_on: unfinished(task, work),
        hold: None,
    }
}

/// The dependencies of a task that are not done.
///
/// A dependency the folder does not hold is one that is not done either:
/// nothing says it is finished.
fn unfinished(task: &Task, work: &Work) -> Vec<String> {
    task.depends_on
        .iter()
        .filter(|dependency| {
            work.task(dependency)
                .is_none_or(|dependency| dependency.status != Status::Done)
        })
        .cloned()
        .collect()
}

/// The branch of the tree.
fn branch_view(work: &Work, branch: Branch, now: &str) -> BranchView {
    match branch {
        Branch::AwaitingApproval => {
            // A task with an overlay is not waiting on a human: it is held.
            let rows = work
                .tasks
                .iter()
                .filter(|task| task.status == Status::Groomed && !has_overlay(task))
                .map(|task| entry(task, work))
                .collect();
            simple(branch, rows)
        }
        Branch::Blocked => blocked(work, now),
        Branch::Running => {
            let rows = work
                .tasks
                .iter()
                .filter(|task| matches!(task.status, Status::Executing | Status::Remediating))
                .map(|task| entry(task, work))
                .collect();
            simple(branch, rows)
        }
        Branch::Review => {
            let rows = work
                .tasks
                .iter()
                .filter(|task| task.status == Status::CodeReview)
                .map(|task| entry(task, work))
                .collect();
            simple(branch, rows)
        }
        Branch::Escalated => {
            let rows = work
                .tasks
                .iter()
                .filter(|task| task.status == Status::NeedsHuman)
                .map(|task| entry(task, work))
                .collect();
            simple(branch, rows)
        }
    }
}

/// A branch whose tasks are not grouped under a blocker.
fn simple(branch: Branch, rows: Vec<Entry>) -> BranchView {
    let count = rows.len();
    BranchView {
        branch,
        count,
        groups: vec![Group {
            blocker: None,
            rows,
        }],
    }
}

/// The approved tasks that cannot be claimed.
///
/// The blocked branch is forked under the first of a task's dependencies
/// that is not done, and the row names what it waits on. A task that is held
/// by an overlay or by a claim another session holds says so on its row.
fn blocked(work: &Work, now: &str) -> BranchView {
    let mut groups: Vec<Group> = Vec::new();
    for task in work
        .tasks
        .iter()
        .filter(|task| task.status == Status::Approved)
    {
        let hold = if has_overlay(task) {
            Some(Hold::Overlay)
        } else if live_claim(task, now) {
            Some(Hold::Claim)
        } else {
            None
        };
        let waits_on = unfinished(task, work);
        if hold.is_none() && waits_on.is_empty() {
            // Nothing holds it: it is claimable, and it is on the spine.
            continue;
        }
        let row = Entry {
            id: task.id.clone(),
            title: task.title.clone(),
            status: task.status,
            waits_on,
            hold,
        };
        // The group of a task is the first of its dependencies that is not
        // done.
        let blocker = row.waits_on.first().cloned();
        match groups.iter_mut().find(|group| group.blocker == blocker) {
            Some(group) => group.rows.push(row),
            None => groups.push(Group {
                blocker,
                rows: vec![row],
            }),
        }
    }
    // The groups in the order of the dependencies they wait on.
    groups.sort_by_key(|group| group.blocker.clone());
    let count = groups.iter().map(|group| group.rows.len()).sum();
    BranchView {
        branch: Branch::Blocked,
        count,
        groups,
    }
}

/// The ids of the tree's rows, in the order they are drawn: the spine,
/// then the branches.
///
/// The detail pane opens from this tab: the rows are what it can open.
pub fn row_ids(work: &Work, now: &str) -> Vec<String> {
    tree(work, now).row_ids()
}

/// Draw the tab through the window the keys move.
///
/// `selected` is the row the cursor is on: its cells take the selection's
/// paint. Every other row keeps its own. The window is settled so the cursor
/// is on a row the pane shows. `now` is the frame's instant: the cursor's
/// rows and the drawn tree answer from the same one.
pub fn render_view(
    frame: &mut Frame,
    area: Rect,
    work: &Work,
    theme: &Theme,
    selected: Option<&str>,
    view: &mut View,
    now: &str,
) {
    let block = Block::bordered()
        .title("path tree")
        .title_style(block_title_style(theme))
        .border_style(theme.chrome.border);
    if let Some(missing) = work.missing {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(missing.headline(), theme.semantic.error)),
                Line::from(Span::styled(
                    format!("looked in {}", work.root.display()),
                    theme.text.dim,
                )),
            ])
            .wrap(Wrap { trim: true })
            .block(block),
            area,
        );
        return;
    }
    if work.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no tasks yet", theme.text.dim))).block(block),
            area,
        );
        return;
    }
    let tree = tree(work, now);
    if tree.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("no live work yet", theme.text.dim)))
                .block(block),
            area,
        );
        return;
    }
    let width = usize::from(area.width).saturating_sub(BORDER);
    let height = usize::from(area.height).saturating_sub(BORDER);
    let ids = tree.row_ids();
    let rows = view.draw(&ids, selected, |first| {
        window(&tree, width, height, theme, selected, first)
    });
    frame.render_widget(
        Paragraph::new(rows).wrap(Wrap { trim: false }).block(block),
        area,
    );
}

/// A piece of the tree.
enum Piece {
    /// The head: the heading and the spine.
    Head(HeadPiece),
    /// A branch: its header and its blocks.
    Branch(BranchPiece),
}

/// The head of the tree: the heading and the spine.
struct HeadPiece {
    /// The heading of the tree.
    heading: Line<'static>,
    /// The rows of the spine: one block per task.
    spine: Vec<Chunk>,
    /// The tasks of the spine.
    tasks: usize,
}

/// A block of the tree: the lines of a task, or of a group of tasks.
///
/// A block is shown whole or not at all: a row of the tree is never cut in
/// half. The lines that head it — the blocker of a group — are not part of
/// the row: they give way when the pane is too short to hold both.
struct Chunk {
    lines: Vec<Line<'static>>,
    /// The lines at the start of the block that head it rather than being
    /// the row itself.
    lead: usize,
    /// The row of the tree the block holds, in the order the rows are drawn.
    row: usize,
    /// The tasks the block holds.
    tasks: usize,
}

/// A branch, ready to be drawn.
struct BranchPiece {
    /// The header of the branch.
    header: Option<Line<'static>>,
    /// The prefix of the rows of the branch.
    prefix: String,
    /// The blocks of the branch.
    blocks: Vec<Chunk>,
    /// The tasks in the branch.
    count: usize,
    /// The name of the branch, for the announcement.
    name: &'static str,
}

/// A branch that is not shown.
#[derive(Debug)]
struct Dropped {
    name: &'static str,
    count: usize,
}

/// A layout the tree could use for the window.
struct Candidate {
    /// The layout shows the cursor's row.
    has_cursor: bool,
    /// The announcement is whole.
    whole: bool,
    /// The rows of the tree, before the announcement.
    tree_rows: usize,
    lines: Vec<Line<'static>>,
    /// The rows of the tree the layout shows.
    rows: Vec<usize>,
}

/// The lines of the tree at the top of its rows.
#[cfg(test)]
fn lines(
    tree: &Tree,
    width: usize,
    height: usize,
    theme: &Theme,
    selected: Option<&str>,
) -> Vec<Line<'static>> {
    window(tree, width, height, theme, selected, 0).0
}

/// The lines of the tree, from the row at `first`, and the rows they show.
///
/// The tree is taller than a pane now and then. What does not fit is said
/// out loud: a count is never shown without the group it counts, and a
/// group that is not shown says how many tasks it holds. The rows above the
/// window are named the same way.
///
/// The task that happens next is never lost without a word: when the spine
/// does not fit, the trunk is named in the announcement.
///
/// A pane too short for the announcement and the cursor's row draws the row
/// alone: the selection is never on nothing.
fn window(
    tree: &Tree,
    width: usize,
    height: usize,
    theme: &Theme,
    selected: Option<&str>,
    first: usize,
) -> (Vec<Line<'static>>, Vec<usize>, bool) {
    let pieces = pieces(tree, width, theme, selected);
    let all = tree.row_ids();
    // The tree that is shown is the one that still leaves room to say what
    // is not shown. The announcement comes first: a name that is cut is a
    // name the reader does not have. A layout that shows the cursor wins
    // over one that leaves it off the screen.
    //
    // The bars of the spine are decoration: when the pane is tight they
    // give way, so that a task is not lost to a bar.
    let mut best: Option<Candidate> = None;
    for bars in [true, false] {
        for capacity in (0..=height).rev() {
            let (shown, dropped, rows) = layout(&pieces, capacity, width, bars, theme, first);
            let has_cursor = selected.is_none_or(|id| {
                rows.iter()
                    .any(|index| all.get(*index).is_some_and(|other| other == id))
            });
            if dropped.is_empty() {
                // The whole tree fits: there is nothing to say.
                if has_cursor {
                    return (shown, rows, true);
                }
                continue;
            }
            let summary = summary(&dropped, width, theme);
            let room = height.saturating_sub(shown.len());
            if room == 0 {
                // There is no room to say what is hidden: a shorter tree is
                // tried next.
                continue;
            }
            let whole = summary.len() <= room;
            let mut out = shown;
            let tree_rows = out.len();
            let mut summary_rows = summary.into_iter().take(room).collect::<Vec<_>>();
            if !whole {
                // The announcement is cut: it says so.
                if let Some(last) = summary_rows.last_mut() {
                    let text: String = last
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect();
                    last.spans = vec![Span::styled(
                        cut_to(&format!("{text}…"), width),
                        theme.text.dim,
                    )];
                }
            }
            out.append(&mut summary_rows);
            let candidate = Candidate {
                has_cursor,
                whole,
                tree_rows,
                lines: out,
                rows,
            };
            let better = best.as_ref().is_none_or(|best| {
                (candidate.has_cursor, candidate.whole, candidate.tree_rows)
                    > (best.has_cursor, best.whole, best.tree_rows)
            });
            if better {
                best = Some(candidate);
            }
        }
    }
    // The pane is too short for the window and its announcement, but the
    // cursor's row still has to be drawn: the row is shown alone. The lines
    // that head its block give way first; a row taller than the pane stays
    // off screen.
    if let Some(id) = selected
        && !best.as_ref().is_some_and(|best| best.has_cursor)
        && let Some(cursor) = all.iter().position(|other| other == id)
        && let Some(chunk) = cursor_chunk(&pieces, cursor)
        && chunk.lines.len() - chunk.lead <= height
    {
        return (chunk.lines[chunk.lead..].to_vec(), vec![cursor], false);
    }
    best.map_or_else(
        || (Vec::new(), Vec::new(), false),
        |best| {
            (
                best.lines,
                best.rows,
                selected.is_none() || (best.has_cursor && best.whole),
            )
        },
    )
}

/// The tree, laid out in `capacity` rows.
///
/// What does not fit is returned, so that it can be named: the branches that
/// are not shown, and the trunk when the spine is cut.
fn layout(
    pieces: &[Piece],
    capacity: usize,
    width: usize,
    bars: bool,
    theme: &Theme,
    first: usize,
) -> (Vec<Line<'static>>, Vec<Dropped>, Vec<usize>) {
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut dropped: Vec<Dropped> = Vec::new();
    let mut shown_rows: Vec<usize> = Vec::new();
    let mut used = 0usize;

    let branches: Vec<&BranchPiece> = pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Branch(branch) => Some(branch),
            _ => None,
        })
        .collect();

    let Some(Piece::Head(head)) = pieces.first() else {
        return (out, dropped, shown_rows);
    };

    if capacity == 0 {
        // Nothing fits: everything is named in the announcement.
        if head.tasks > 0 {
            dropped.push(Dropped {
                name: SPINE,
                count: head.tasks,
            });
        }
        dropped.extend(branches.iter().map(|branch| Dropped {
            name: branch.name,
            count: branch.count,
        }));
        return (out, dropped, shown_rows);
    }

    // The head: the heading and the spine. The heading belongs to the top
    // of the tree: a window that starts lower does not repeat it.
    if first == 0 {
        out.push(head.heading.clone());
        used += 1;
    }
    let mut shown_tasks = 0usize;
    for chunk in &head.spine {
        if chunk.row < first {
            continue;
        }
        let rows = chunk.lines.len();
        if bars && used + 1 + rows <= capacity {
            // The bar that holds the row, and the row.
            out.push(bar(theme));
            used += 1;
        } else if used + rows > capacity {
            // The row does not fit: the spine is cut here.
            break;
        }
        out.extend(chunk.lines.iter().cloned());
        used += rows;
        shown_tasks += chunk.tasks;
        shown_rows.push(chunk.row);
    }
    if shown_tasks < head.tasks {
        // The spine is cut: the trunk is named in the announcement. The
        // rows above the window are part of what is not shown.
        dropped.push(Dropped {
            name: SPINE,
            count: head.tasks - shown_tasks,
        });
    }

    // The branches, in the order of the tree. The fork belongs to the
    // boundary between the spine and the branches: a window that starts at
    // or below it has already passed the fork.
    let mut forked = first > 0 && first >= head.tasks;
    for (index, branch) in branches.iter().enumerate() {
        let start = branch.blocks.first().map_or(0, |block| block.row);
        if branch.count > 0 && start + branch.count <= first {
            // The whole branch is above the window.
            dropped.push(Dropped {
                name: branch.name,
                count: branch.count,
            });
            continue;
        }
        let room = capacity.saturating_sub(used);
        let Some(header) = branch.header.clone() else {
            // The branch cannot be headed: it is not shown.
            dropped.extend(branches[index..].iter().map(|branch| Dropped {
                name: branch.name,
                count: branch.count,
            }));
            break;
        };
        let skip = first.saturating_sub(start).min(branch.count);
        let visible = branch.count - skip;
        // The fork: the place where the branches leave the trunk. It is
        // drawn once, before the first branch that is shown.
        let fork_rows = if forked {
            0
        } else if room >= 2 {
            2
        } else if room == 1 {
            1
        } else {
            0
        };
        let room = room - fork_rows;
        // The header needs a row of its own.
        let (whole, whole_tasks, whole_height, whole_rows) =
            take(&branch.blocks, room.saturating_sub(1), skip);
        if room >= 1 && whole_tasks == visible {
            // The whole branch fits. The rows above the window are not
            // drawn: they are named, as every other hidden row is.
            let above = branch.count - whole_tasks;
            if above > 0 {
                dropped.push(Dropped {
                    name: branch.name,
                    count: above,
                });
            }
            if !forked {
                fork(&mut out, fork_rows, theme);
                used += fork_rows;
                forked = true;
            }
            out.push(header);
            out.extend(whole);
            used += 1 + whole_height;
            shown_rows.extend(whole_rows);
            continue;
        }
        // The branch is cut: it needs the header, some rows, and the
        // announcement of what is not shown.
        let (rows, shown, _, row_indices) = take(&branch.blocks, room.saturating_sub(2), skip);
        if room >= 3 && !rows.is_empty() {
            if !forked {
                fork(&mut out, fork_rows, theme);
            }
            out.push(header);
            out.extend(rows);
            out.push(announcement(
                branch.count - shown,
                &branch.prefix,
                width,
                theme,
            ));
            shown_rows.extend(row_indices);
        } else if room >= 2 {
            // No row of the branch fits: it keeps its header, and says how
            // many of its tasks are not shown.
            if !forked {
                fork(&mut out, fork_rows, theme);
            }
            out.push(header);
            out.push(announcement(branch.count, &branch.prefix, width, theme));
        } else {
            // The branch does not fit: it is not shown at all.
            dropped.push(Dropped {
                name: branch.name,
                count: branch.count,
            });
        }
        // The branches after this one are not shown either.
        dropped.extend(branches[index + 1..].iter().map(|branch| Dropped {
            name: branch.name,
            count: branch.count,
        }));
        break;
    }
    (out, dropped, shown_rows)
}

/// The block of the tree that holds a row.
fn cursor_chunk(pieces: &[Piece], cursor: usize) -> Option<&Chunk> {
    for piece in pieces {
        let chunks = match piece {
            Piece::Head(head) => &head.spine,
            Piece::Branch(branch) => &branch.blocks,
        };
        if let Some(chunk) = chunks.iter().find(|chunk| chunk.row == cursor) {
            return Some(chunk);
        }
    }
    None
}

/// The blocks that fit in `room` rows: their lines, the tasks they hold, the
/// rows they take and the rows of the tree they show.
///
/// The first `skip` rows of the branch are above the window and are left
/// out.
fn take(
    blocks: &[Chunk],
    room: usize,
    skip: usize,
) -> (Vec<Line<'static>>, usize, usize, Vec<usize>) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut tasks = 0usize;
    let mut height = 0usize;
    let mut rows: Vec<usize> = Vec::new();
    for block in blocks.iter().skip(skip) {
        if height + block.lines.len() > room {
            break;
        }
        height += block.lines.len();
        tasks += block.tasks;
        lines.extend(block.lines.iter().cloned());
        rows.push(block.row);
    }
    (lines, tasks, height, rows)
}

/// The lines that name what is not shown.
///
/// The summary sits in the tree's column, like everything else. A count is
/// never parted from the group it counts: an entry is never broken over two
/// rows.
fn summary(dropped: &[Dropped], width: usize, theme: &Theme) -> Vec<Line<'static>> {
    if dropped.is_empty() {
        return Vec::new();
    }
    // The summary is in the tree's column: it starts one column in.
    let width = width.saturating_sub(1);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut first = true;
    for branch in dropped {
        let entry = summary_entry(branch, width);
        let separator = if first { "" } else { ", " };
        if !current.is_empty()
            && current.chars().count() + separator.chars().count() + entry.chars().count() > width
        {
            // The entry does not fit on this row: it starts the next one.
            lines.push(std::mem::take(&mut current));
            current.push_str(&entry);
            first = false;
            continue;
        }
        if current.is_empty() && first {
            // The words that say what the row is about, when there is room
            // for them.
            let prefix = "not shown: ";
            if prefix.chars().count() + entry.chars().count() <= width {
                current.push_str(prefix);
            }
        }
        current.push_str(separator);
        current.push_str(&entry);
        first = false;
    }
    lines.push(current);
    lines
        .into_iter()
        .map(|line| Line::from(Span::styled(format!(" {line}"), theme.text.dim)))
        .collect()
}

/// The name of a group and the count of the tasks in it.
///
/// The count stays whole: the name gives way when the row is too narrow.
fn summary_entry(branch: &Dropped, width: usize) -> String {
    let count = branch.count.to_string();
    let fixed = branch.name.chars().count() + 3 + count.chars().count();
    if fixed <= width {
        return format!("{} · {count}", branch.name);
    }
    let room = width.saturating_sub(3 + count.chars().count());
    cut_to(&format!("{} · {count}", cut_to(branch.name, room)), width)
}

/// The pieces of the tree: the head, then the branches.
fn pieces(tree: &Tree, width: usize, theme: &Theme, selected: Option<&str>) -> Vec<Piece> {
    let mut pieces = Vec::new();

    // The head: what the spine is, and the spine itself.
    let spine: Vec<Chunk> = tree
        .spine
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let prefix = if index == 0 {
                format!(" {NEXT} ")
            } else {
                " ├─ ".to_string()
            };
            Chunk {
                lines: row(&prefix, entry, width, theme, is_selected(entry, selected)),
                lead: 0,
                row: index,
                tasks: 1,
            }
        })
        .filter(|chunk| chunk.lines.iter().all(|line| line.width() <= width))
        .collect();
    pieces.push(Piece::Head(HeadPiece {
        heading: heading(width, theme),
        spine,
        tasks: tree.spine.len(),
    }));

    // The branches, in the order of the tree.
    let last = tree.branches.len().saturating_sub(1);
    let mut branch_base = tree.spine.len();
    for (index, view) in tree.branches.iter().enumerate() {
        let is_last = index == last;
        // The branch hangs on the trunk: the trunk's column comes first.
        let branch_prefix = if is_last { "     " } else { " │   " };
        let mut blocks: Vec<Chunk> = Vec::new();
        let mut row_index = branch_base;
        branch_base += view.count;
        if view.branch == Branch::Blocked {
            // The blocked branch is forked: a group per blocker, and the
            // tasks of the group under it. The name of the blocker heads
            // the first row of its group.
            let last_group = view.groups.len().saturating_sub(1);
            for (group_index, group) in view.groups.iter().enumerate() {
                let group_is_last = group_index == last_group;
                let group_prefix = if group_is_last { "    " } else { "│   " };
                let last_row = group.rows.len().saturating_sub(1);
                for (row_at, entry) in group.rows.iter().enumerate() {
                    let mut lines: Vec<Line<'static>> = Vec::new();
                    let mut lead = 0;
                    if row_at == 0
                        && let Some(blocker) = &group.blocker
                    {
                        // The name of the blocker heads its group.
                        let prefix = format!("{branch_prefix}{}", connector(group_is_last));
                        lines.push(group_header(&prefix, blocker, width, theme));
                        lead = 1;
                    }
                    let prefix = format!(
                        "{branch_prefix}{group_prefix}{}",
                        connector(row_at == last_row)
                    );
                    lines.extend(row(
                        &prefix,
                        entry,
                        width,
                        theme,
                        is_selected(entry, selected),
                    ));
                    blocks.push(Chunk {
                        lines,
                        lead,
                        row: row_index,
                        tasks: 1,
                    });
                    row_index += 1;
                }
            }
        } else {
            let last_row = view
                .groups
                .first()
                .map_or(0, |group| group.rows.len())
                .saturating_sub(1);
            if let Some(group) = view.groups.first() {
                for (row_at, entry) in group.rows.iter().enumerate() {
                    let prefix = format!("{branch_prefix}{}", connector(row_at == last_row));
                    blocks.push(Chunk {
                        lines: row(&prefix, entry, width, theme, is_selected(entry, selected)),
                        lead: 0,
                        row: row_index,
                        tasks: 1,
                    });
                    row_index += 1;
                }
            }
        }
        // A branch whose rows do not fit the pane is not shown at all: it
        // is named in the announcement.
        let fits = blocks
            .iter()
            .all(|block| block.lines.iter().all(|line| line.width() <= width));
        let header = if fits {
            branch_header(view, is_last, width, theme)
        } else {
            None
        };
        pieces.push(Piece::Branch(BranchPiece {
            header,
            prefix: branch_prefix.to_string(),
            blocks,
            count: view.count,
            name: view.branch.label(),
        }));
    }
    pieces
}

/// The connector of an item: the last of its siblings hangs on a `└─`, the
/// others on a `├─`.
fn connector(is_last: bool) -> &'static str {
    if is_last { "└─ " } else { "├─ " }
}

/// The name of the blocker, heading its group.
fn group_header(prefix: &str, blocker: &str, width: usize, theme: &Theme) -> Line<'static> {
    let room = width.saturating_sub(prefix.chars().count());
    Line::from(vec![
        Span::styled(prefix.to_string(), theme.text.dim),
        Span::styled(
            cut_to(&format!("blocked-by {blocker}"), room),
            theme.text.normal,
        ),
    ])
}

/// The heading of the tree: what the spine is.
fn heading(width: usize, theme: &Theme) -> Line<'static> {
    let forms = [
        ("  ready · approved · deps met", "EXECUTION"),
        ("  ready", "EXECUTION"),
        ("", "EXECUTION"),
    ];
    let (note, label) = forms
        .iter()
        .find(|(note, label)| 1 + label.chars().count() + note.chars().count() <= width)
        .unwrap_or(&("", "EXECUTION"));
    let mut label = label.to_string();
    let mut note = note.to_string();
    if 1 + label.chars().count() > width {
        // The pane is too narrow for the name: it is cut, and the note
        // gives way.
        label = cut_to(&label, width.saturating_sub(1));
        note = String::new();
    }
    Line::from(vec![
        Span::styled(format!(" {label}"), theme.text.strong),
        Span::styled(note, theme.text.dim),
    ])
}

/// The bar of the spine.
fn bar(theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(" │", theme.text.dim))
}

/// The mark of the place where the branches fork off.
///
/// The fork sits one column right of the trunk, as the sketch of the spec
/// has it.
fn fork_line(theme: &Theme) -> Line<'static> {
    Line::from(Span::styled("  ╵", theme.text.dim))
}

/// The fork: the bar that holds it, and the fork itself.
fn fork(out: &mut Vec<Line<'static>>, rows: usize, theme: &Theme) {
    if rows >= 2 {
        out.push(bar(theme));
    }
    out.push(fork_line(theme));
}

/// The header of a branch: its name and the number of tasks in it.
fn branch_header(
    view: &BranchView,
    is_last: bool,
    width: usize,
    theme: &Theme,
) -> Option<Line<'static>> {
    let prefix = if is_last { " └─ " } else { " ├─ " };
    let label = view.branch.label();
    let count = view.count.to_string();
    let separator = " · ";
    let style = phase_style(theme, view.branch.phase());
    let fixed = prefix.chars().count() + separator.chars().count() + count.chars().count();
    if fixed > width {
        // Not even the count fits: the branch is not shown, and the
        // announcement says so.
        return None;
    }
    let room = width - fixed;
    let label = if label.chars().count() <= room {
        label.to_string()
    } else {
        cut_to(label, room)
    };
    Some(Line::from(vec![
        Span::styled(prefix.to_string(), theme.text.dim),
        Span::styled(label, style),
        Span::styled(separator.to_string(), theme.text.dim),
        Span::styled(count, theme.text.strong),
    ]))
}

/// A row of the tree: the prefix, the id, the title and what it waits on.
///
/// The title is never cut: when it does not fit on one row, it wraps under
/// its own column. A selected row is painted whole: its wrapped lines and
/// its note are part of it.
fn row(
    prefix: &str,
    entry: &Entry,
    width: usize,
    theme: &Theme,
    selected: bool,
) -> Vec<Line<'static>> {
    let style = phase_style(theme, entry.status);
    let id = entry.id.as_str();
    let content = prefix.chars().count();
    let title_col = content + id.chars().count() + GAP;
    let available = width.saturating_sub(title_col);
    // The title is never lost: when the pane is too narrow for it beside
    // the id, it goes under it.
    let beside = available > 0;
    let text_col = if beside { title_col } else { 0 };
    let chunks = wrap(&entry.title, if beside { available } else { width });
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut head = vec![
        Span::styled(prefix.to_string(), theme.text.dim),
        Span::styled(id.to_string(), style),
    ];
    let mut used = content + id.chars().count();
    let mut chunks = chunks.iter();
    match chunks.next() {
        Some(first) => {
            if beside {
                head.push(Span::raw(" ".repeat(GAP)));
                head.push(Span::styled(first.clone(), style));
                used += GAP + first.chars().count();
            }
            lines.push(Line::from(head));
            for chunk in chunks {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(text_col)),
                    Span::styled(chunk.clone(), style),
                ]));
                used = text_col + chunk.chars().count();
            }
        }
        None => lines.push(Line::from(head)),
    }
    // What the row says about the task: what it waits on, and what holds
    // it when it is not only the dependencies.
    let mut note: Vec<String> = Vec::new();
    if !entry.waits_on.is_empty() {
        note.push(format!("waits on {}", entry.waits_on.join(", ")));
    }
    if let Some(hold) = entry.hold {
        note.push(hold.label().to_string());
    }
    if !note.is_empty() {
        let note = note.join(" · ");
        let note_width = note.chars().count();
        if used + 2 + note_width <= width {
            if let Some(line) = lines.last_mut() {
                line.spans.push(Span::raw("  "));
                line.spans.push(Span::styled(note, theme.text.dim));
            }
        } else {
            for chunk in wrap(&note, width.saturating_sub(text_col)) {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(text_col)),
                    Span::styled(chunk, theme.text.dim),
                ]));
            }
        }
    }
    if selected {
        return lines
            .into_iter()
            .map(|line| selected_line(line, width, theme))
            .collect();
    }
    lines
}

/// Whether the row is the one the cursor is on.
fn is_selected(entry: &Entry, selected: Option<&str>) -> bool {
    selected == Some(entry.id.as_str())
}

/// The line that says how much is not shown.
fn announcement(count: usize, prefix: &str, width: usize, theme: &Theme) -> Line<'static> {
    let text = format!("{prefix}+{count} more tasks");
    Line::from(Span::styled(cut_to(&text, width), theme.text.dim))
}

/// The text, wrapped into rows of `width` columns.
///
/// A word longer than a row is broken over rows: no letter of it is lost.
/// The break is on a grapheme boundary, as the shared helper cuts titles: a
/// scalar break can leave a combining mark stranded on the next row.
fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut rest = word.to_string();
        while text_width(&rest) > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut head = head_to(&rest, width);
            if head.is_empty() {
                // The first grapheme is wider than the row: it goes on a
                // row of its own rather than looping here.
                head = first_grapheme(&rest);
            }
            if head.is_empty() {
                break;
            }
            rest = rest[head.len()..].to_string();
            lines.push(head);
        }
        if current.is_empty() {
            current = rest;
        } else if text_width(&current) + 1 + text_width(&rest) <= width {
            current.push(' ');
            current.push_str(&rest);
        } else {
            lines.push(std::mem::take(&mut current));
            current = rest;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Style;

    use super::*;
    use crate::data::Missing;

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    fn theme() -> Theme {
        Theme::preset(coding_crew_theme::Preset::Dark)
    }

    /// The tests' clock: one instant for the tree and the draw that shows it.
    const NOW: &str = "2026-11-19T08:00:00Z";

    fn now() -> &'static str {
        NOW
    }

    /// Draw the tab and return the rows it shows.
    fn draw(work: &Work, width: u16, height: u16) -> Vec<String> {
        let mut view = View::default();
        draw_at(work, width, height, now(), &mut view)
    }

    /// Draw the tab at a given instant.
    fn draw_at(work: &Work, width: u16, height: u16, now: &str, view: &mut View) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal
            .draw(|frame| render_view(frame, frame.area(), work, &theme(), None, view, now))
            .expect("drawing the path");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "))
                    .collect::<String>()
            })
            .collect()
    }

    /// The text of the rows, without the border.
    fn text_of(rows: &[String]) -> String {
        rows.iter()
            .map(|row| row.trim_end().to_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// The rows of the pane, without the border of the block.
    fn inner(rows: &[String]) -> Vec<String> {
        rows.iter()
            .skip(1)
            .take(rows.len().saturating_sub(2))
            .map(|row| {
                let row = row.trim_end();
                match row.strip_prefix('│').and_then(|row| row.strip_suffix('│')) {
                    Some(inner) => inner.trim_end().to_string(),
                    None => row.to_string(),
                }
            })
            .collect()
    }

    /// The text of a line of the tree.
    fn text_of_line(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    /// The lines of the tree that are wider than the pane.
    ///
    /// The check the width test is made of: it must be able to fail.
    fn too_wide(lines: &[Line<'_>], width: usize) -> Vec<String> {
        lines
            .iter()
            .filter(|line| line.width() > width)
            .map(text_of_line)
            .collect()
    }

    #[test]
    fn the_counts_of_the_fixtures_are_pinned() {
        // The tree of the mock data, in numbers: the spine, and the five
        // branches. The numbers are pinned here, so a tree that drifts from
        // the spec fails here.
        let tree = tree(&mock_work(), now());
        assert_eq!(tree.spine.len(), 1, "the trunk of the fixtures");
        assert_eq!(tree.rows(Branch::AwaitingApproval), 2);
        assert_eq!(tree.rows(Branch::Blocked), 3);
        assert_eq!(tree.rows(Branch::Running), 6);
        assert_eq!(tree.rows(Branch::Review), 3);
        assert_eq!(tree.rows(Branch::Escalated), 3);
    }

    #[test]
    fn the_path_reads_one_instant_for_the_rows_and_the_draw() {
        // A lease expiring between two instants: before it the task is held
        // and sits under BLOCKED; at it the task is claimable and sits on the
        // spine. The cursor's row list and the drawn tree answer from the
        // instant they are given, so one frame cannot show the task in both
        // places or in neither.
        let mut task = Task::new("TSK01", "the claimed task");
        task.status = Status::Approved;
        task.claim = Some(coding_crew_core::Claim {
            by: "runner-01".into(),
            session: Some("session-runner-01".into()),
            at: "2029-12-31T23:00:00Z".into(),
            lease_expires_at: "2030-01-01T00:00:00Z".into(),
        });
        let work = Work {
            root: std::path::PathBuf::new(),
            tasks: vec![task],
            items: Vec::new(),
            errors: Vec::new(),
            missing: None,
        };
        let held = "2029-12-31T23:59:59Z";
        let free = "2030-01-01T00:00:00Z";
        let before = tree(&work, held);
        assert!(
            before.spine.is_empty(),
            "a live claim is on the spine: {before:?}"
        );
        assert_eq!(before.rows(Branch::Blocked), 1, "the claim holds the task");
        let after = tree(&work, free);
        assert_eq!(after.spine.len(), 1, "the expired claim frees the task");
        assert_eq!(after.rows(Branch::Blocked), 0, "the task is still blocked");
        assert_eq!(row_ids(&work, free), vec!["TSK01".to_string()]);
        let mut view = View::default();
        let text = text_of(&draw_at(&work, 60, 12, free, &mut view));
        assert!(text.contains(" ● TSK01"), "the spine is not drawn: {text}");
        assert!(
            !text.contains("BLOCKED"),
            "a freed task is still blocked: {text}"
        );
        let text = text_of(&draw_at(&work, 60, 12, held, &mut view));
        assert!(
            text.contains("BLOCKED · 1"),
            "the branch is not drawn: {text}"
        );
        assert!(
            !text.contains(" ● TSK01"),
            "a held task is on the spine: {text}"
        );
        assert!(text.contains("claim held elsewhere"), "{text}");
    }

    #[test]
    fn the_spine_is_the_claimable_task() {
        let tree = tree(&mock_work(), now());
        assert_eq!(tree.spine[0].id, "TSK29");
        assert!(tree.spine[0].waits_on.is_empty(), "it waits on nothing");
    }

    #[test]
    fn the_blocked_tasks_are_forked_under_what_holds_them() {
        let tree = tree(&mock_work(), now());
        let blocked = tree
            .branches
            .iter()
            .find(|view| view.branch == Branch::Blocked)
            .expect("the blocked branch");
        // The blocked branch is forked under the first of a task's
        // dependencies that is not done.
        let blockers: Vec<Option<&str>> = blocked
            .groups
            .iter()
            .map(|group| group.blocker.as_deref())
            .collect();
        assert_eq!(blockers, vec![Some("TSK04"), Some("TSK05"), Some("TSK06")]);
        // The row names what it waits on.
        let first = &blocked.groups[0].rows[0];
        assert_eq!(first.id, "TSK07");
        assert_eq!(
            first.waits_on,
            vec!["TSK04".to_string(), "TSK05".to_string()]
        );
    }

    #[test]
    fn the_fixtures_cover_every_blocking_reason() {
        // A dependency that is not done, an overlay, and a claim held
        // elsewhere: the three reasons a task is not claimable.
        let tree = tree(&mock_work(), now());
        let blocked = tree
            .branches
            .iter()
            .find(|view| view.branch == Branch::Blocked)
            .expect("the blocked branch");
        let holds: Vec<Option<Hold>> = blocked
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .map(|row| row.hold)
            .collect();
        assert_eq!(holds, vec![None, Some(Hold::Overlay), Some(Hold::Claim)]);
    }

    #[test]
    fn the_tree_holds_no_draft_and_no_done_task() {
        let work = mock_work();
        let tree = tree(&work, now());
        let seen = tree.row_ids();
        for task in &work.tasks {
            if task.status == Status::Draft || task.status == Status::Done {
                assert!(!seen.contains(&task.id), "{} is history", task.id);
            }
        }
    }

    #[test]
    fn the_tree_has_the_shape_of_the_spec() {
        let rows = draw(&mock_work(), 100, 40);
        let text = text_of(&rows);
        // The spine, with the marker of what happens next.
        assert!(text.contains(" ● TSK29"), "{text}");
        // The branch headers, with the count of the group they count.
        assert!(text.contains("AWAITING APPROVAL · 2"), "{text}");
        assert!(text.contains("BLOCKED · 3"), "{text}");
        assert!(text.contains("RUNNING · 6"), "{text}");
        assert!(text.contains("REVIEW · 3"), "{text}");
        assert!(text.contains("ESCALATED · 3"), "{text}");
        // The blocked tasks are forked under the dependency they wait on.
        assert!(text.contains("blocked-by TSK04"), "{text}");
        assert!(text.contains("blocked-by TSK05"), "{text}");
        // A row names what it waits on, and what holds it.
        assert!(text.contains("waits on TSK04, TSK05"), "{text}");
        assert!(text.contains("overlay"), "{text}");
        assert!(text.contains("claim held elsewhere"), "{text}");
    }

    #[test]
    fn the_tree_has_one_trunk_column() {
        // The sketch of the spec: the trunk holds the spine and the
        // branches, and the branches hang off it.
        let rows = inner(&draw(&mock_work(), 100, 40));
        // The spine hangs on the trunk.
        assert!(
            rows.iter().any(|row| row.starts_with(" ● TSK29")),
            "{rows:?}"
        );
        // The fork sits one column right of the trunk.
        assert!(rows.iter().any(|row| row.starts_with("  ╵")), "{rows:?}");
        // The branch headers hang on the trunk.
        assert!(
            rows.iter()
                .any(|row| row.starts_with(" ├─ AWAITING APPROVAL · 2")),
            "{rows:?}"
        );
        // The rows of a branch hang under its header.
        assert!(
            rows.iter().any(|row| row.starts_with(" │   ├─ TSK04")),
            "{rows:?}"
        );
        // The blocked group is forked under the dependency it waits on.
        assert!(
            rows.iter()
                .any(|row| row.starts_with(" │   ├─ blocked-by TSK04")),
            "{rows:?}"
        );
        assert!(
            rows.iter().any(|row| row.starts_with(" │   │   └─ TSK07")),
            "{rows:?}"
        );
    }

    #[test]
    fn an_empty_spine_draws_one_bar() {
        // Nothing is claimable: the tree shows the branches, and one bar.
        let theme = theme();
        let tree = Tree {
            spine: Vec::new(),
            branches: vec![BranchView {
                branch: Branch::Running,
                count: 1,
                groups: vec![Group {
                    blocker: None,
                    rows: vec![Entry {
                        id: "TSK10".to_string(),
                        title: "a task".to_string(),
                        status: Status::Executing,
                        waits_on: Vec::new(),
                        hold: None,
                    }],
                }],
            }],
        };
        let rows: Vec<String> = lines(&tree, 60, 20, &theme, None)
            .iter()
            .map(text_of_line)
            .collect();
        let bars = rows.iter().filter(|row| row.trim() == "│").count();
        assert_eq!(bars, 1, "{rows:?}");
    }

    #[test]
    fn the_next_task_is_never_unnamed() {
        // The pane is too short for the tree: the task that happens next is
        // shown, or the trunk is named in the announcement.
        let work = mock_work();
        for width in [40u16, 60, 100] {
            for height in [4u16, 6, 8, 12, 24, 40] {
                let text = text_of(&draw(&work, width, height));
                assert!(
                    text.contains("TSK29") || text.contains("EXECUTION ·"),
                    "{width}x{height}: {text}"
                );
            }
        }
    }

    #[test]
    fn the_summary_sits_in_the_tree_column() {
        // The summary is in the tree's column, like everything else.
        let dropped = vec![
            Dropped {
                name: "RUNNING",
                count: 6,
            },
            Dropped {
                name: "REVIEW",
                count: 3,
            },
        ];
        for row in summary(&dropped, 60, &theme()) {
            let text = text_of_line(&row);
            assert!(text.starts_with(' '), "{text:?}");
        }
        // And on the screen, nothing is said in the margin.
        for row in inner(&draw(&mock_work(), 100, 10)) {
            assert!(!row.starts_with("not shown"), "{row:?}");
        }
    }

    #[test]
    fn no_line_is_longer_than_the_width() {
        // The check is on the lines the tree builds: a line that is too wide
        // would be wrapped by ratatui, and the tree would ask for more rows
        // than the pane has.
        let work = mock_work();
        let tree = tree(&work, now());
        for width in [40usize, 60, 100] {
            for height in [8usize, 12, 24, 40] {
                let rows = lines(&tree, width, height, &theme(), None);
                assert!(
                    too_wide(&rows, width).is_empty(),
                    "{width}x{height}: {:?}",
                    too_wide(&rows, width)
                );
                assert!(
                    rows.len() <= height,
                    "{width}x{height}: {} rows",
                    rows.len()
                );
            }
        }
    }

    #[test]
    fn the_tree_fits_at_any_size() {
        // Every size: no line is wider than the pane, and the tree asks for
        // no more rows than the pane has.
        let work = mock_work();
        let tree = tree(&work, now());
        for width in 20..=120 {
            for height in 4..=40 {
                let rows = lines(&tree, width, height, &theme(), None);
                assert!(
                    too_wide(&rows, width).is_empty(),
                    "{width}x{height}: {:?}",
                    too_wide(&rows, width)
                );
                assert!(
                    rows.len() <= height,
                    "{width}x{height}: {} rows",
                    rows.len()
                );
            }
        }
    }

    #[test]
    fn the_width_check_can_fail() {
        // A check that cannot fail is worse than no check: the check is run
        // against a line that is too wide, and it must catch it.
        let wide = vec![Line::from(Span::raw("x".repeat(41)))];
        assert_eq!(too_wide(&wide, 40).len(), 1);
        let exact = vec![Line::from(Span::raw("x".repeat(40)))];
        assert!(too_wide(&exact, 40).is_empty());
    }

    #[test]
    fn the_rows_drawn_are_the_rows_asked_for() {
        // What the tree asks for is what is drawn: nothing is wrapped, and
        // nothing is cut.
        let work = mock_work();
        let tree = tree(&work, now());
        for width in [40u16, 60, 100] {
            for height in [8u16, 12, 24, 40] {
                let asked: Vec<String> = lines(
                    &tree,
                    usize::from(width) - BORDER,
                    usize::from(height) - BORDER,
                    &theme(),
                    None,
                )
                .iter()
                .map(text_of_line)
                .collect();
                let drawn = inner(&draw(&work, width, height));
                assert_eq!(asked, drawn[..asked.len()], "{width}x{height}");
                assert!(
                    drawn[asked.len()..].iter().all(|row| row.trim().is_empty()),
                    "{width}x{height}"
                );
            }
        }
    }

    #[test]
    fn no_title_is_cut_at_any_width() {
        let work = mock_work();
        let tree = tree(&work, now());
        for width in [40u16, 60, 100] {
            // A title that does not fit wraps under its own column: no
            // letter of it is lost. The pane is tall enough for the whole
            // tree, so every title is on the screen.
            let text: String = inner(&draw(&work, width, 120))
                .iter()
                .flat_map(|row| row.chars())
                .filter(|ch| !ch.is_whitespace())
                .collect();
            for entry in &tree.spine {
                let title: String = entry
                    .title
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect();
                assert!(text.contains(&title), "{width}: {}", entry.title);
            }
            for view in &tree.branches {
                for group in &view.groups {
                    for entry in &group.rows {
                        let title: String = entry
                            .title
                            .chars()
                            .filter(|ch| !ch.is_whitespace())
                            .collect();
                        assert!(text.contains(&title), "{width}: {}", entry.title);
                    }
                }
            }
        }
    }

    #[test]
    fn a_long_word_is_broken_on_a_grapheme_boundary() {
        // A one-word title of joined graphemes: a scalar break would keep the
        // `e` and leave its combining mark to head the next row. The break is
        // the shared helper's, on the boundaries a terminal draws with.
        let word = "e\u{301}".repeat(10);
        for width in 1..=10 {
            let rows = wrap(&word, width);
            assert_eq!(rows.concat(), word, "{width}: a letter was lost");
            for row in &rows {
                assert!(
                    is_grapheme_prefix(row, &word),
                    "{width}: {row:?} cuts a grapheme"
                );
                assert!(
                    !row.starts_with('\u{301}'),
                    "{width}: {row:?} starts with a combining mark"
                );
            }
        }
        assert_eq!(
            wrap(&word, 4),
            vec![
                "e\u{301}".repeat(4),
                "e\u{301}".repeat(4),
                "e\u{301}".repeat(2),
            ]
        );
    }

    /// Whether `head` is a run of whole graphemes of `text`.
    fn is_grapheme_prefix(head: &str, text: &str) -> bool {
        let mut built = String::new();
        for grapheme in Span::raw(text.to_string()).styled_graphemes(Style::default()) {
            built.push_str(grapheme.symbol);
            if built == head {
                return true;
            }
            if built.len() >= head.len() {
                break;
            }
        }
        built == head
    }

    #[test]
    fn a_count_is_never_shown_without_the_group_it_counts() {
        for width in [40u16, 60, 100] {
            for height in [8u16, 12, 24, 40] {
                let text = text_of(&draw(&mock_work(), width, height));
                for (branch, count) in [
                    (Branch::AwaitingApproval, 2),
                    (Branch::Blocked, 3),
                    (Branch::Running, 6),
                    (Branch::Review, 3),
                    (Branch::Escalated, 3),
                ] {
                    // The count of a branch is always on a row that names the
                    // branch: either its header, or the line that says the
                    // branch is not shown.
                    let found = text.lines().any(|line| {
                        line.contains(branch.label()) && line.contains(&count.to_string())
                    });
                    assert!(found, "{width}x{height}: {branch:?} has no count");
                }
            }
        }
    }

    #[test]
    fn a_branch_that_is_not_shown_is_announced() {
        // The pane is too short for the whole tree: the branches that are not
        // shown are named, with the count of the tasks they hold.
        let text = text_of(&draw(&mock_work(), 100, 16));
        assert!(text.contains("not shown"), "{text}");
        assert!(text.contains("REVIEW · 3"), "{text}");
    }

    #[test]
    fn a_hidden_task_is_announced() {
        // The pane is too short for the whole tree: what is not shown is
        // said out loud.
        let text = text_of(&draw(&mock_work(), 100, 14));
        assert!(
            text.contains("more task") || text.contains("not shown"),
            "{text}"
        );
    }

    #[test]
    fn the_empty_state_is_not_a_blank_screen() {
        let rows = draw(&Work::load(Path::new("/nowhere/coding-crew/work")), 60, 12);
        let text = text_of(&rows);
        assert!(text.contains("no coding-crew folder found"), "{text}");
        assert!(
            text.contains("looked in /nowhere/coding-crew/work"),
            "{text}"
        );
        assert!(!text.contains("no work folder"), "{text}");
    }

    #[test]
    fn a_project_without_work_says_so_and_names_the_path() {
        let work = Work::load_missing(
            Path::new("/nowhere/project/coding-crew/work"),
            Missing::NoWork,
        );
        let text = text_of(&draw(&work, 100, 10));
        assert!(text.contains("this project has no work folder"), "{text}");
        assert!(
            text.contains("looked in /nowhere/project/coding-crew/work"),
            "{text}"
        );
        assert!(!text.contains("no coding-crew folder found"), "{text}");
    }

    /// Draw the tab and return every cell, with its colour.
    fn draw_cells(work: &Work, width: u16, height: u16) -> Vec<Vec<(char, ratatui::style::Color)>> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        let mut view = View::default();
        terminal
            .draw(|frame| render_view(frame, frame.area(), work, &theme(), None, &mut view, now()))
            .expect("drawing the path");
        let buffer = terminal.backend().buffer();
        let area = *buffer.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| {
                        let cell = buffer.cell((x, y)).expect("a cell");
                        (cell.symbol().chars().next().unwrap_or(' '), cell.fg)
                    })
                    .collect()
            })
            .collect()
    }

    /// The colour of a cell of `needle`, `offset` cells into it.
    fn colour_at(
        cells: &[Vec<(char, ratatui::style::Color)>],
        needle: &str,
        offset: usize,
    ) -> Option<ratatui::style::Color> {
        for row in cells {
            let text: String = row.iter().map(|(symbol, _)| *symbol).collect();
            let Some(byte) = text.find(needle) else {
                continue;
            };
            // The index of a string is a byte index; a cell is a character.
            let start = text[..byte].chars().count();
            return row.get(start + offset).map(|(_, colour)| *colour);
        }
        None
    }

    #[test]
    fn the_status_is_the_colour_of_the_row() {
        // The branch says who the next action belongs to; the status is the
        // colour of the row, from the theme's phase mapping.
        let theme = theme();
        for status in [
            Status::Approved,
            Status::Groomed,
            Status::CodeReview,
            Status::NeedsHuman,
            Status::Executing,
            Status::Remediating,
        ] {
            let entry = Entry {
                id: "TSK01".to_string(),
                title: "a task".to_string(),
                status,
                waits_on: Vec::new(),
                hold: None,
            };
            let lines = row(" ● ", &entry, 100, &theme, false);
            let style = phase_style(&theme, status);
            // The id and the title of the row carry the colour of the
            // phase; the tree around them is dim.
            assert_eq!(lines[0].spans[1].style, style, "{status:?}");
            assert_eq!(lines[0].spans[3].style, style, "{status:?}");
            assert_eq!(lines[0].spans[0].style, theme.text.dim, "{status:?}");
        }
    }

    #[test]
    fn a_selected_row_takes_the_selection_slot_and_the_others_keep_their_phase() {
        let theme = theme();
        let selection = theme.text.selected;
        let style = phase_style(&theme, Status::Approved);
        let entry = Entry {
            id: "TSK01".to_string(),
            title: "a task".to_string(),
            status: Status::Approved,
            waits_on: Vec::new(),
            hold: None,
        };
        // A row that is not the cursor's keeps the slot of its phase.
        let plain = row(" ● ", &entry, 60, &theme, false);
        assert_eq!(plain[0].spans[1].style, style, "the id");
        assert_eq!(plain[0].spans[3].style, style, "the title");
        // The cursor's row takes the selection's pair on every cell, and the
        // bar runs to the edge of the pane.
        let selected = row(" ● ", &entry, 60, &theme, true);
        assert_eq!(selected.len(), 1, "a short row is one line");
        assert_eq!(selected[0].width(), 60, "the selection bar stops short");
        for span in &selected[0].spans {
            assert_eq!(span.style, selection, "{:?} is not the selection", span);
        }
        // A wrapped row is painted whole: every line of it, note included.
        let mut wrapped = entry.clone();
        wrapped.title = "a title that does not fit the pane at all, not at any width".to_string();
        wrapped.waits_on = vec!["TSK02".to_string()];
        let selected = row(" ● ", &wrapped, 30, &theme, true);
        assert!(selected.len() > 1, "the fixture wraps: {selected:?}");
        for line in &selected {
            assert_eq!(line.width(), 30, "a line of the bar stops short");
            for span in &line.spans {
                assert_eq!(span.style, selection, "{:?} is not the selection", span);
            }
        }
    }

    #[test]
    fn the_branches_are_painted_with_the_phase_they_name() {
        let theme = theme();
        for (branch, phase, count) in [
            (Branch::AwaitingApproval, Status::Groomed, 2),
            (Branch::Blocked, Status::Approved, 3),
            (Branch::Running, Status::Executing, 6),
            (Branch::Review, Status::CodeReview, 3),
            (Branch::Escalated, Status::NeedsHuman, 3),
        ] {
            let view = BranchView {
                branch,
                count,
                groups: Vec::new(),
            };
            let header = branch_header(&view, false, 100, &theme).expect("a header");
            // The name of the branch takes the slot of the phase it names,
            // and the count beside it is a count.
            assert_eq!(
                header.spans[1].style,
                phase_style(&theme, phase),
                "{branch:?}"
            );
            assert_eq!(header.spans[3].style, theme.text.strong, "{branch:?}");
        }
    }

    #[test]
    fn the_count_of_a_branch_is_a_count() {
        // The count beside a branch is `text.strong`, as every other count
        // on the screen is.
        let theme = theme();
        let cells = draw_cells(&mock_work(), 100, 40);
        let count = colour_at(&cells, "BLOCKED · 3", "BLOCKED · ".chars().count());
        assert_eq!(count, theme.text.strong.fg);
    }
}
