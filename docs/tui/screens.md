# raul — screens

The TUI for the work. It reads the work folder and shows it. It writes
nothing.

The domain is in [tasks/task-model.md](../tasks/task-model.md); the names are
in [vocabulary.md](../vocabulary.md); the roles are in
[roles/roles.md](../roles/roles.md).

## The window

```
┌────────────────────────────────────────────────────────────────────┐
│ ◆ coding-crew · ~/projects/app                                     │
│  ▦ Overview ◉ Needs you 8 ⑂ Path ☑ Tasks ≡ Backlog 11 ⚙ Settings   │
│─▔▔▔▔▔▔▔▔▔▔▔▔───────────────────────────────────────────────────────│
│                                                                    │
│  content pane — rows of the active tab                             │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│         enter open · o/O sort · / filter · ? help · q quit         │
└────────────────────────────────────────────────────────────────────┘
```

| part | behaviour |
|---|---|
| header | the app's icon and name, a dim `·`, then the folder the tool reads |
| tab bar | the six tabs, each its icon and name; `Needs you` and `Backlog` carry their counts as badges; the active tab is bold and underlined by a `▔` on the rule under the bar; `1`–`6`, `←`/`→`, `h`/`l`, `tab`/`shift+tab` on every tab, Settings included |
| content pane | the active tab's rows; `↑`/`↓`, `j`/`k`, `pgup`/`pgdn`, `g`/`G` |
| detail pane | `enter` on a row opens the task as a full-screen overlay; `esc` returns to the tab and the selection it came from, and `enter` again reopens |
| footer | the active tab's own keys, centred under a thin rule, then `? help · q quit` — never a key that does nothing. Navigation keys are in the `?` overlay, not the footer. When the content pane has no room for the hidden-rows announcement, the footer carries that count first, and the content pane says nothing about it — the same fact is never painted twice |
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
| Tasks | what tasks exist, in any state | every task file, searchable and sortable |
| Backlog | what is not a task yet | smaller tasks and draft tasks |
| Settings | how the tool behaves | the configuration file, as a file |

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
- The trunk is one column: the spine's marker and bar, the fork `╵` and the
  branch connectors `├` `└` all start in the column after the selection bar's.
  A task under a branch hangs from that branch, one column further in.

This is master-plan's shape with our statuses: its lanes were *awaiting
approval*, *blocked*, *grooming* and *review*, and it grouped the blocked
items under their first dependency too.

### Tasks

The index: **every task file**, in every status from `draft` to `done` — the
one place a finished task can be reached without knowing its id. Needs you
and Path stay the views that act; Backlog stays the view that shapes. A draft
is in both Backlog and Tasks, as itself. Decided 2026-11-22.

**The row** is the Needs you row: the status icon on its slot, the id, the
title. The right-hand column shows **the value of the sort key in force** —
the created date when sorted by creation, the priority when sorted by
priority — so the order explains itself and no fact is painted twice. Sorted
by id, the column is the status. A subtask is a row like any other, flat,
with its parent in the suffix: `↑ TSK03` (the parent icon). Subtasks are never
nested: nesting breaks under any sort but the id and under a filter.

**The order.** `o` cycles the sort key — `id`, `created`, `updated`,
`status` (lifecycle order: draft → done), `priority`, `effort` — and `O`
reverses it. Ties break by id, so the order is stable. The default is `id`
ascending. Sorted by a category (`status`, `priority`, `effort`), the rows sit
under group headers in the Backlog's style, the count a badge: `DONE  5`; a
sort by id or a date is flat. The Settings tab's sort group names the order in
force on its Tasks row.

**The filter** is one line of text, not a form. `/` opens it on the row above
the footer and every key narrows the rows as it is typed; `enter` keeps it and returns
to the rows, `esc` clears it. The text is words and `key:value` tokens:

| token | matches |
|---|---|
| a plain word | the id, the title or a tag, case-insensitive, as a substring |
| `status:<s>` | the status, by name or prefix: `status:review`, `status:done` |
| `priority:<p>` | `low` · `medium` · `high` (and the model's other values) |
| `effort:<e>` | the effort: `S`, `M`, `L`… |
| `kind:<k>` | the task kind |
| `tag:<t>` | a tag, exactly |
| `claimed:yes` / `claimed:no` | whether a claim is held |
| `parent:<id>` | the subtasks of a task |

Different keys combine with AND; the same key repeated combines with OR —
`status:review status:remediating priority:high`. A token with an unknown key
or value is named on that row (`unknown key: colour`) and matches nothing,
rather than being ignored silently.

**A filtered list says so.** The row above the footer reads `12 of 30 · 18
filtered out` while a filter is in force, and the filter text stays visible; the window's
own announcement of rows scrolled out of view is unchanged and separate. A
filter that matches nothing is a named empty state — `no task matches` and the
filter — never a blank pane.

The order and the filter live for the session. `o` and `O` change the order
for the session only; the window starts in the order `[raul.tasks]` names in
the configuration file. The filter is not saved between runs.

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

Below the row, the **body** — the item's own sentence, saying what the item
is about. It is the row's **second line**, dim, indented to where the row's
title starts, so it reads as belonging to the row above it. It is the item's
own body, never a summary, and it is cut by the theme crate's shared `cut_to`,
so a body that does not fit ends in the tab's own `…` on a grapheme boundary.

An item with **no body** draws no second line at all, and its row is the row
it was. A row with a body is **two lines everywhere the tab counts** — the
window, the announcement of what it hides, and the footer's count — and a row
is drawn in both its lines or in neither.

Smaller tasks are grouped by kind — ideas first, then the small tasks that
could be worked. The grouping is derived from the kind; it is never stored.

### Settings

**The configuration file, as a file.** Settings shows
`~/.config/coding-crew/config.toml` — the one configuration both tools read —
drawn the way the file reads: its comments dim, its `[section]` headers
strong, one `key = value` per line, in the file's own order. The pane's title
names the path, and `● modified` while there are unsaved changes. Decided
2026-11-22: the tab that "persists nothing" is overruled.

**Everything in the file is editable** — raul's own `[raul]` table and the
flow's sections (`round_limit`, each agent's `harness`, `model` and `effort`,
the order's steps' values). An agent's table is its role, so the file names no
role or name for it to edit. Rows cannot be added or removed from
raul; only values change. A comment line is not a row the cursor stops on.
A value written inline (`tasks = { … }`) or as a dotted key
(`tasks.sort = …`) is shown as text, not a stop, until the follow-up in
`TODO.md` makes each one editable.

| value | how it changes |
|---|---|
| a choice (`theme`, `icons`, a step's `role`, `harness`, `model`, `effort`, `sort`, a boolean) | `enter` opens its list: the picker hugo's choices use, with the value in force under the cursor. `↑` `↓` (`j` `k`) move over the choices, `enter` keeps the one under the cursor, `esc` restores the value the list opened on. A `harness` lists the harness table; a `model` lists its harness's models, with `type an id…` last, which opens the inline input; an `effort` lists the levels of the agent's model, and says why it has none when it has none: `takes no effort level` for a model that takes none, the read's error when the read fails; a model the catalogue does not list has no levels of its own, so its list is the harness's levels, then `none` (start with no effort flag, stored as an empty `effort`), then `type a level…`, an inline input that takes one word, unchecked, and any of them completes the value. A model that takes no level shows its effort as a dim `—` |
| text or a number (a step's `step`, `round_limit`) | `enter` edits it inline in the line; `enter` keeps, `esc` drops |

The arrows and `h` `l` switch tabs on Settings as on every other tab: they
change no value.

**A change previews at once** — move the picker onto `nord` and the screen
repaints in Nord; `esc` puts the old choice back — and is written only by `w`.
`u` undoes the unsaved changes; `r` reloads the file from disk. Quitting with
unsaved changes asks first, in the footer: `w` save, `q` quit without saving,
`esc` stay.

**A value is checked before it is kept.** A choice can only be one of its
choices; a number must be in its range (`round_limit` ≥ 1); a typed `model` is
one word with no `#` — its effort is the agent's own `effort`. An `effort` is one
of its model's levels once they have been read; until then the word is kept as
written. A value that fails names its reason on its own line and is not kept.

**Nothing on disk is lost.** The file is written format-preserving: the
comments, the order, the spacing and the keys raul does not know survive a
save. A file changed on disk since raul read it is not overwritten: `w` says
`the file changed on disk` and offers `r` to reload. A file that does not
parse is named with its error, left alone, and shown read-only with the
defaults beside it — as hugo does.

**The order of precedence**: a flag (`--theme`, `--icons`), then the
environment (`CODING_CREW_ICONS`), then the file, then the defaults. A value
the flag overrides says so on its line: `theme = "nord"  (overridden by
--theme dark)`. raul writes the defaults — every value, with a comment above
each part — when the file is absent, as hugo does; both tools write the same
defaults from `crates/core`.

The `[raul]` table:

| key | values | default |
|---|---|---|
| `theme` | `Preset::ALL`'s names | `dark` |
| `icons` | `nerd` · `unicode` | `nerd` |
| `[raul.tasks] sort` | `id` · `created` · `updated` · `status` · `priority` · `effort` | `id` |
| `[raul.tasks] reverse` | `true` · `false` | `false` |

The key bindings are not configuration yet; the help overlay (`?`) lists them.

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
| acceptance criteria | each: description, verification, status, evidence, deferral, failure reason |
| steps | each: action, done_when, status, deviation |
| review | the budget and verdict, the rounds — kind, focus, verdict, findings — and the findings' threads |
| questions | open and resolved |
| notes | what the execution learned |
| dependencies | what it waits on, what waits on it |
| children | child tasks and their progress |

The projection is the shape, not the wording: each of the ten sections is a
level-one heading, an item under one a level-two, and a finding under its round
a level-three; the header a card: the title, then one list item per field — id,
status, kind, priority, effort, tags, claim and each overlay; each criterion a heading with its id and
status, its fields a list, as the steps are; each review round a heading with
its own fields and the findings it raised under it; the
verification commands as code, with a rule before a fenced one. The evidence is
quoted in a blockquote, the children are a task list — `[x]` for a done subtask,
`[ ]` for one that is not — and a dependency is a link to its record. The
task's own free text — the intent, the criterion descriptions, the notes — is
markdown in its own right and is rendered as such; the title is a label, never
a heading and never a link, and is escaped like the other fields that must not
be interpreted.

Nothing in the detail pane is editable in v1. raul reads.

The pane scrolls: `↑`/`↓`, `j`/`k`, `pgup`/`pgdn`, `g`/`G` move through a
document taller than the pane. The footer is `esc close · ? help · q quit`;
`?` lists the pane's keys over it, scroll keys included. Scrolling back to the
top shows the document as the unscrolled view shows it.

The pane keeps the document it rendered, and a redraw reuses it while the
record, the width, the theme and the icon set are unchanged. Keys that are
queued together are answered before the next draw, so a burst of scrolling is
one redraw.

## Mouse

The mouse works where the keys do. Capture is on while raul runs and off when
it exits, and on a panic.

| gesture | does |
|---|---|
| wheel | scrolls what is under the pointer, three rows a notch: the detail pane, a list tab's rows, Settings, a picker |
| click a tab label | switches to that tab; a bar that shows only the active tab takes no clicks |
| click a row | selects it; a second click on the selected row opens it, as `enter` does |
| click a picker entry | moves the picker to it, previewing it; a second click keeps it, as `enter` does |

A click outside these regions does nothing. The pointer waits while the help,
a question, or a typed filter or value holds the keys. The regions are the ones
the last frame drew, so a click lands on what is on screen. Most terminals keep
their own text selection for a drag held with shift.

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
`tasks`, `backlog`, `settings` — in the same bounded plain render.

`scripts/pty-sweep.py` renders the screen at every width from 40 to 120, keeps
one file per width, and fails when a line is wider than its width. The gate
runs it. The Overview is pinned byte for byte at 40, 60 and 100 columns in
`crates/raul/tests/snapshots/`, and so is the detail pane of TSK13 at 100 columns
(`detail.txt`). The detail document reads at 14 columns and up.

## Not defined yet

- the automation's internals — not a raul tab; the design record is
  [docs/hugo/design-record.md](../hugo/design-record.md)
- the journal — *Activity* is a block in the Overview, not a tab; the event
  kinds are provisional until the writer lands
- persisted keybinds — the key bindings are not configuration yet; the help
  overlay (`?`) lists them
- the review's interaction: **raul reads**. A finding is shown, never acted on — acting on one
  needs a writer, a gate and talita's verbs, which are out of scope for now

## Mock data

raul reads `mock-data/coding-crew/work/` — one task per phase, the edge
cases, and the smaller tasks — and the journal beside it,
`mock-data/coding-crew/activity.json` — so the screens can be built against
real shapes before any data exists to manage. `make raul` points at it.
