# raul — screens

The TUI for the work. It reads the work folder and shows it. It writes
nothing.

The domain is in [tasks/task-model.md](../tasks/task-model.md); the names are
in [vocabulary.md](../vocabulary.md); the roles are in
[roles/roles.md](../roles/roles.md).

## The window

```
┌────────────────────────────────────────────────────────────────┐
│ Overview   Needs you   Path   Backlog   Settings               │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  content pane — rows of the active tab                         │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│ q quit   tab next   enter open   ? help                         │
└────────────────────────────────────────────────────────────────┘
```

| part | behaviour |
|---|---|
| tab bar | the five tabs; `1`–`5`, `←`/`→`, `h`/`l`, `tab`/`shift+tab` |
| content pane | the active tab's rows; `↑`/`↓`, `j`/`k`, `pgup`/`pgdn`, `g`/`G` |
| detail pane | `enter` on a row opens the task as a full-screen overlay; `esc` returns to the tab and the selection it came from, and `enter` again reopens |
| footer | contextual keys for the active tab — never a key that does nothing. When the content pane has no room for the hidden-rows announcement, the footer carries that count beside the keys, and the content pane says nothing about it — the same fact is never painted twice |
| quit | `q` quits, and `esc` quits too — in the window. `esc` in the detail pane returns to the tab instead |

The detail pane is not a tab: it is the same task seen close up, and it opens
from any tab that has a row.

The content pane scrolls when its rows are taller than the pane. The window
follows the cursor, so moving past the last visible row moves the list; a page
key moves by the rows the pane shows, and `g`/`G` reach the first and the last
row. The cursor is never on a row the pane does not show.

## The tabs

| tab | question it answers | rows |
|---|---|---|
| Overview | what is the state of the project | counts, attention, what agents can claim, recent activity |
| Needs you | what waits on a human | tasks whose next action is the human's |
| Path | what happens next, in order | claimable, in flight, blocked — with the reason |
| Backlog | what is not a task yet | smaller tasks and draft tasks |
| Settings | how the tool behaves | the themes, the keys and the sorts |

The automation is not a raul tab. Its questions — the order, the queue, the
crew, whether it runs — belong to hugo; the design record is in
[docs/hugo/](../hugo/design-record.md).

### Overview

Not a list — a summary the human reads in one screen.

| block | shows |
|---|---|
| counts | tasks per status; smaller tasks per kind |
| needs you | the same count *Needs you* shows, with the three reasons: to approve, to review, escalated |
| claimable | how many tasks an agent could take right now |
| activity | the last few events; when there is no journal, the block says so |
| next | the first task of *Path* — what happens next |

### Needs you

The human lane: **approve**, **review**, **escalate**.

| column | shows |
|---|---|
| id | `TSK01` |
| why | `approve` · `review` · `escalated` |
| title | the task title |
| review | the review state: the findings open with their severities, then the external rounds spent of the budget. A task with no open finding and no external round carries none |
| age | how long it has waited |

Rows: `groomed` → approve, `code-review` → review, `needs-human` → escalated.
A task with an overlay (`blocked`, `deferred`) drops out of this tab — it is
not waiting on a human.

The review state is a suffix, before the age. It is given up whole parts at a
time: the most urgent severity is named before the others, and the rounds are
what a narrow row drops first, because the severity is what the human decides
on and the rounds are context.

### Path

Master-plan's Path tab, reproduced: a **spine** of what can run now, with
**branches** for what is waiting, and the blocked branch forked under what
blocks it.

```
 Path Tree 
 EXECUTION  ready · approved · deps met
 │
 ● TSK29   Lorem ipsum dolor sit amet
 │
 ├─ TSK09   Tempor incididunt ut labore et dolore
 │
  ╵
 ├─ AWAITING APPROVAL · 2
 │   └─ TSK04   Consectetur adipiscing elit
 ├─ BLOCKED · 3
 │   └─ blocked-by TSK04
 │       └─ TSK07   Tempor incididunt ut labore
 ├─ RUNNING · 6
 │   └─ TSK10   Tempor incididunt ut labore
 ├─ REVIEW · 3
 │   └─ TSK13   Praesentium voluptatum deleniti
 └─ ESCALATED · 3
     └─ TSK19   Quaerat voluptatem consequatur
```

| part | rule |
|---|---|
| spine | the claimable tasks: `approved`, every dependency `done`, no overlay, no live claim — in dependency order. The first is marked `●`: it is what happens next |
| AWAITING APPROVAL | `groomed` — waiting on the human to approve |
| BLOCKED | `approved` but not claimable. Forked under **the first of its dependencies that is not done**, and the row names every dependency it waits on, that one included |
| RUNNING | `executing` and `remediating` |
| REVIEW | `code-review` |
| ESCALATED | `needs-human` — the review budget ran out |
| order | spine, then the branches in the order above |

Rules:

- The tree holds the **live work**: the tasks that are neither `draft` nor
  `done`. Drafts are the Backlog tab's, and done tasks are history.
- An empty branch is skipped.
- The status is not in the branch — it is the **colour** of the row, from the
  theme's phase mapping. The branch says who the next action belongs to.
- A row names its dependencies: `waits on TSK05, TSK06`.
- A row carries the **review state** as a suffix, before the age — the findings
  open with their severities, then the external rounds spent of the budget.
  The branch already names the phase, so the state does not repeat it. It is
  given up whole parts at a time, and the rounds are what a narrow row drops
  first.
- The tree is drawn as styled lines with a spine, and it scrolls.

This is master-plan's shape with our statuses: its lanes were *awaiting
approval*, *blocked*, *grooming* and *review*, and it grouped the blocked
items under their first dependency too.

### Backlog

Everything that is not a task yet, in one list — smaller tasks and draft tasks
together, so the human sees one backlog.

| column | shows |
|---|---|
| id | `ITM01` or `TSK01` |
| kind | `idea` · `bug` · `feature` · `draft` |
| title | the title |
| scope | the scope, when it is known |
| state | `open` · `dismissed` · `promoted` |

Smaller tasks are grouped by kind — ideas first, then the small tasks that
could be worked. The grouping is derived from the kind; it is never stored.

### Settings

What the tool is doing, in one list — three groups, all derived, none stored,
in this order.

| group | rows |
|---|---|
| theme | one row per preset in the theme crate's `Preset::ALL`, so a new preset appears here without raul knowing it; the theme in use is marked, and exactly one row is; each row says what the theme is — a palette, or the terminal's own colours |
| keys | one row per binding in raul's key table — the window's keys (quit, help, the tab keys), the row keys (the movement keys, opening a task) and the detail pane's keys (the scroll keys, and the key that returns) — so the list and the input handling are one table, and a key handled but not listed, or listed but not handled, fails the gate; the keys the tool no longer has are not listed |
| sort | one row per tab, naming in words what comes first and the order the tab keeps; the order and the words come from the tab's own sort, so a tab sorted another way says so; the Overview has no sort and says so, and the detail pane is not a tab and has no row |

The rows are a list with a cursor: the tab scrolls when its rows are taller
than the pane, and the cursor is never on a row the pane does not draw — the
same rule the row tabs keep, and the same announcement: what the pane does not
show is named by group and count, so a count never appears without the set
it counts. It reads: nothing here is changed from the tab, so
`enter` opens nothing — the footer names the scroll keys and no open key, and
the help names no open key either. A rebind and a persisted sort are a
configuration file, and that is hugo's home.

## The detail pane

The task, close up — as a **document**. The record is JSON; what the human
reads is markdown, projected from that record and rendered through leaf's
engine ([ADR 0009](../decisions/0009-markdown-projection.md)).

The document carries the task's sections, in reading order:

| section | shows |
|---|---|
| header | id, title, kind, status, overlays, claim, priority, effort, tags |
| intent | outcome and problem |
| scope | in and out |
| acceptance criteria | each: description, verification, status, evidence, deferral |
| steps | each: action, done_when, status, deviation |
| review | the budget and verdict, the rounds — kind, focus, verdict, findings — and the findings' threads |
| questions | open and resolved |
| notes | what the execution learned |
| dependencies | what it waits on, what waits on it |
| children | child tasks and their progress |

The projection is the shape, not the wording: headings per section, a table for
the criteria, lists for the steps, and each review round a heading with its
own fields and the findings it raised under it; the verification commands as
code. The task's own free text — the intent, the
criterion descriptions, the notes — is markdown in its own right and is
rendered as such; the title is a label and is escaped like the other fields
that must not be interpreted.

Nothing in the detail pane is editable in v1. raul reads.

The pane scrolls: `↑`/`↓`, `j`/`k`, `pgup`/`pgdn`, `g`/`G` move through a
document taller than the pane, and the footer names the keys that work
while it is open. Scrolling back to the top shows the document as the
unscrolled view shows it.

## Derived, never stored

Every view above is computed from the task files.

| view | rule |
|---|---|
| lane | human lane = `groomed`, `code-review`, `needs-human`; agent lane = `approved`, `executing`, `remediating` |
| needs you | the human lane, minus the tasks carrying an overlay |
| claimable | `approved`, no overlay, no live claim, every dependency `done` |
| blocked reason | dependency not done · overlay set · claimed by another session |
| review state | `pending-review` · `open-findings` · `clean` |
| criterion status | from the latest evidence |
| children progress | from the children's own statuses |
| next | the first claimable row of *Path* |

## States

| state | what raul shows |
|---|---|
| empty | "no tasks yet" in the pane, not a blank screen |
| no project | "no coding-crew folder found" with the path it looked in |
| no work folder | "this project has no work folder" with the path it looked in |
| unreadable file | the file name and the error; the rest of the list still renders |
| smaller task | "the smaller task \<id\> has no document yet" in the detail pane, not the task it is not |
| stale | the list is read from disk on every redraw; a changed file shows up on the next redraw |

## Rendering a screen as text

`raul --render --plain` prints the current screen as plain text and exits.
It is how a prototype is captured and how a snapshot test is written — the
sketch is the tool's own output, not a drawing of it.

`--tab NAME` renders another tab — `overview`, `needs-you`, `path`,
`backlog`, `settings` — in the same bounded plain render.

`scripts/pty-sweep.py` renders the screen at every width from 40 to 120, keeps
one file per width, and fails when a line is wider than its width. The gate
runs it. The Overview is pinned byte for byte at 40, 60 and 100 columns in
`crates/raul/tests/snapshots/`.

## Not defined yet

- the automation's internals — not a raul tab; the design record is
  [docs/hugo/design-record.md](../hugo/design-record.md)
- the journal — *Activity* is a block in the Overview, not a tab; the event
  kinds are provisional until the writer lands
- persisted sort keys and keybinds — the Settings tab shows what is true today,
  and changing it waits for the configuration file
- the review's interaction: **raul reads**. A finding is shown, never acted on — acting on one
  needs a writer, a gate and talita's verbs, which are out of scope for now

## Mock data

raul reads `mock-data/coding-crew/work/` — one task per phase, the edge
cases, and the smaller tasks — and the journal beside it,
`mock-data/coding-crew/activity.json` — so the screens can be built against
real shapes before any data exists to manage. `make raul` points at it.
