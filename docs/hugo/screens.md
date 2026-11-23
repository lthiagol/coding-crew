# hugo — screens

hugo is the TUI for the flow (**H**uman-guided **U**nified **G**overnance &
**O**rchestration). It reads the runtime and shows it. It starts the
orchestrator, which runs the loop, and it writes nothing but its own
configuration.

The interface was chosen in [design-record.md](design-record.md): one sidebar
for the configuration and one main pane for what is running, with no tabs. The
reading is **the crew leads**.

## The window

The screen below is the mock's 100-by-40 render: the pinned screen in
`crates/hugo/tests/snapshots/screen-100.txt`.

```text
 󰚩 hugo · mock-data                                                                       󰑮 running
╭─ 󰒓 configuration ────────────╮╭─ 󰑮 flow ─────────────────────────────────────────────────────────╮
│ AGENTS ──────────────────────││╭─ 󰚩 crew ───────────────────────────────────────────────────────╮│
│ 󰚩 orchestrator               │││the session's crew, and what each agent holds                   ││
│   harness claude             │││role           model            task  held state                ││
│   model   claude-opus-5-5    │││󰚩 orchestrator claude-opus-5-5  —     —    idle                 ││
│   effort  medium             │││󰖷 builder      claude-haiku-5-5 TSK14 󰥔 8d idle                 ││
│ 󰖷 builder                    │││󰈈 reviewer     claude-opus-5-5  TSK14 󰥔 7d ⠋ reviewing          ││
│▌  harness claude             │││one per role · the reviewer reviews                             ││
│   model   claude-haiku-5-5   ││╰────────────────────────────────────────────────────────────────╯│
│   effort  high               ││╭─ 󰚇 queue ──────────────────────────────────────────────────────╮│
│ 󰈈 reviewer                   │││tasks waiting, in order                                         ││
│   harness claude             │││the automation's queue · 2 waiting · 2 done                     ││
│   model   claude-opus-5-5    │││ [ ] TSK29  approved                                            ││
│   effort  medium             │││ [ ] TSK07  approved                                            ││
│ ROUNDS ──────────────────────││╰────────────────────────────────────────────────────────────────╯│
│ 5 build-and-fix rounds       ││╭─ 󰈈 round ──────────────────────────────────────────────────────╮│
│ ORDER ───────────────────────│││where the task in flight is in its build/review loop            ││
│ 1 groom · creator            │││TSK14 · external  round 1 󰄬 build · 󰄬 review 3 findings         ││
│   groomed → approved         │││› round 2 󰄬 build · ⠋ review                                    ││
│ 2 build · builder            ││╰────────────────────────────────────────────────────────────────╯│
│   approved → executing       ││╭─ 󰥔 budget ─────────────────────────────────────────────────────╮│
│ 3 self-review · builder      │││build/review rounds used of the limit                           ││
│   executing → code-review    │││rounds ████░░░░░░ 2 of 5                                        ││
│ 4 review · reviewer          ││╰────────────────────────────────────────────────────────────────╯│
│   code-review → done         ││╭─ 󰅚 findings ───────────────────────────────────────────────────╮│
│   code-review → remediating  │││the task in flight's open findings, by severity                 ││
│   code-review → needs-human  │││1 open finding · 1 high · 0 medium · 0 low                      ││
│ 5 remediate · builder        ││╰────────────────────────────────────────────────────────────────╯│
│   remediating → code-review  ││                                                                  │
│ 6 re-review · reviewer       ││                                                                  │
│   code-review → done         ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││󰑮 running since 2026-11-05T14:30Z                                 │
╰──────────────────────────────╯╰──────────────────────────────────────────────────────────────────╯
────────────────────────────────────────────────────────────────────────────────────────────────────
                 enter choose · r reload · c crew size · tab queue · ? help · q quit
```

| part | behaviour |
|---|---|
| title line | the tool's icon and name, `hugo`, then a dim `·` and the project's folder when one is found; a window too narrow for the name shows no title rather than a cut of the name |
| sidebar | the configuration, on the left: each agent with its harness, model and effort, the review-round limit and the order, in that order. The agents group has a cursor on each agent's harness, model and effort: `↑ k` and `↓ j` move it, and `enter` on one of those rows opens its choice. An agent is named by its role, its table's name: a name or role is read-only, and nothing in the sidebar edits it. The sidebar shows the session's crew, and marks what the session changed from the file — see [the configuration](#the-configuration). `r` reads the file again and starts the session over from it |
| main pane | what is running: the crew, the queue, the round, the budget and the findings, with the stamp at its foot |
| footer | the keys that work, centred under a thin rule — never a key that does nothing. The window's keys are `enter choose` (on an agent's row), `r reload`, `c crew size`, `tab queue` (when a claimable task waits) and `s start` (when it can act: [starting the orchestrator](#starting-the-orchestrator)), then `? help · q quit`; `ctrl-c quit` works too, unnamed. While the queue has the keys its keys are `space mark`, `a mark all`, `s start` (when it can act) and `tab configuration`, then `? help · q quit`. [Picking the queue](#picking-the-queue) says what they do. While a choice is open its keys are `esc close`, `enter pick` (when the cursor's row can be picked), then `? help · q quit`; while the id input is open its keys are `esc back`, `enter pick` and `ctrl-c quit`, and every other key types; while the crew-size selector is open its keys are `esc close`, `enter pick`, then `? help · q quit`. The movement keys are in the help, not the footer |
| choice | the choice of the row under the cursor, titled with the agent and the field (`builder model`): a small list with a cursor of its own. A harness lists the table's three; a model lists the harness's models, with `type an id…` last; an effort lists the harness's levels. `enter` picks it for the session, `esc` closes without picking |
| crew-size selector | `c` opens the three crew sizes, over the window. See [the crew size](#the-configuration) |
| help | `?` opens the keys, in the sidebar, over the choice and over the selector; any key closes it and gives the choice back |
| tab bar | none: the flow has one screen, not a set of tabs |

The two empty states are painted on `text.dim`, as raul's empty states are.
The main pane says `nothing is running` when there is no runtime record to
read; the sidebar says `no configuration yet` when there is no home to hold
one.

## The main pane

The blocks are read in this order; the crew leads.

| order | block | shows |
|---|---|---|
| 1 | `crew` | a table under a header, one row per agent: its role's icon and role, the model it runs, the task it holds (or `—`), the hold, and its state; then the composition, in words. Drawn alone, a row says `holds …` in words instead |
| 2 | `queue` | **the automation's queue**: the tasks waiting, in order, an id and a status each — never a title, a note or an intent. A task the crew holds is not a row; a done one is counted, not drawn |
| 3 | `round` | **a timeline** of the task in flight: each round's build and review, and the findings each review raised; `no review in flight` when there is none |
| 4 | `budget` | the rounds the task in flight has used, of its limit, as a bar and the numbers |
| 5 | `findings` | the task in flight's open findings, as a count, and by severity when the record holds them |
| 6 | stamp | at the foot: whether the automation runs, **and why** when it does not |

**The crew's size is a composition, not a number.** The crew is the roles
present. Three agents are one per role and the reviewer reviews; an
orchestrator and a builder, and the orchestrator reviews; the orchestrator
alone builds and reviews, and the orchestrator reviews. A record with no
orchestrator has the human reviewing, a crew the configuration cannot produce.
The pane says who reviews in words — one per role · the
reviewer reviews; the orchestrator reviews; the human reviews — and never
prints a bare count of agents. A crew with more than one agent in a role has
no such rule: the pane says `more than one agent per role` rather than
claiming a rule the crew does not follow.

**A blocked agent is named on the warning slot.** Its state is `blocked` on
`semantic.warn`, under the theme's blocked icon. It holds its task and is not
working, so it carries no spinner.

**A held task belongs to the crew, not the queue.** It appears in its agent's
row and never in the queue, even when a writer left it in both.

**Every block opens with a caption.** Each block's first line is a dim line
that says what the block counts, and it is the block's own words, not a
count:

| block | caption |
|---|---|
| `crew` | the session's crew, and what each agent holds |
| `queue` | tasks waiting, in order; the claimable ones are picked |
| `round` | where the task in flight is in its build/review loop |
| `budget` | build/review rounds used of the limit |
| `findings` | the task in flight's open findings, by severity |

**The round is a timeline.** It names the task in flight and its stage, then
each round in order: `✓ build` and `✓ review` with the findings the review
raised, the review still under way as the spinner (`⠋ review`) or waiting as
`○ review`. The icons are the theme's table. The timeline is one line while it
fits, and one round to a line once it does not, a round after the first led by
`›`:

```text
TSK14 · external
round 1 ✓ build · ✓ review 3 findings
› round 2 ✓ build · ⠋ review
```

**The same fact is painted once.** A count appears once, the queue's count is
the waiting tasks, and the round's task is named as the review, not counted
again.

The queue's row is an id and a status. A waiting task the record does not call
claimable is drawn dim, with `not claimable` after its status, and it takes no
mark and no cursor. Its count names its set: `the
automation's queue · 2 waiting · 2 done`, the done tasks counted after the
waiting ones when there are any. The budget's count names its set:
`rounds ████░░░░░░ 2 of 5`, the rounds the task in flight has used of its own
limit. The findings' count names the task in flight's set: `1 open finding`, or
`17 open findings` for more. When the record holds the breakdown, it follows
on the same count: `· 1 high · 0 medium · 0 low`, each count in its
severity's slot — `high` on `semantic.error`, `medium` on `semantic.warn`,
`low` on `semantic.info`. A narrow pane wraps the breakdown under the count
rather than cutting it.

The stamp is a line: `󰑮 running since 2026-11-09T08:30Z` on `semantic.ok` when
the automation runs, `󰏤 not running: <why>` on `semantic.warn` when it does
not. A stop with no reason still says a word: `not running: no reason given`.

**The crew is a table.** A dim header names its columns — `role model task
held state` — and the columns are as wide as their widest cell, so the rows
line up under it while the table fits the pane. A working agent carries the
spinner's frame before its state; an idle one is dim, the row and its state
alike. The spinner turns on the loop's tick: every 120 milliseconds while an
agent works, once a second otherwise. A render shows the first frame, so a
pinned screen does not move. Where the table does not fit, the header goes and
each row is drawn alone, with its task and hold in words: `holds TSK10 · held
10d`, or `holds nothing`. The rows take one form, the widest every row fits:
one line each, else two, else three; the cells are one space apart, and every
line under the agent is indented two columns. No fact is dropped with the
width. The model is the model's own name; the provider belongs to the
configuration the sidebar shows.

The hold is a length, not a clock reading: `󰥔 10d`, from the record's `since`
and the frame's one instant. It is written in whole days, hours or minutes; a
hold that is under a minute says `󰥔 under a minute` rather than `0m`. A row
with no hold shows `—`. The pane reads the clock once per frame, in `main.rs`,
and every module below it takes the instant it is given.

A block that does not fit the pane is left out whole, and one line says
`not shown:` with each block's name **and what it held**: `crew (5 hidden)`,
the way the row tabs name the rows they hide. The boxes go before the words;
the stamp is the last fact to give way.

## The configuration

The sidebar is the configuration: what the human governs. It shows three
groups, in this order:

| order | group | shows |
|---|---|---|
| 1 | `AGENTS` | one entry per agent: its role's icon and role, then the harness it runs in, the model it runs and its effort, on the rows under it |
| 2 | `ROUNDS` | the build-and-fix cycles a task gets, and the set the count counts |
| 3 | `ORDER` | the numbered steps, each with the gates it passes |

The configuration is **provisional**: the flow has not written one yet, and the
format may change when it does. Its shape is
[tasks/task-model.md § 11.3](../tasks/task-model.md#113-the-configuration-provisional).
A file that reads but holds what the flow does not run — a table of a name that
is no role, or a crew the flow does not run — is used as it is, and the sidebar
names each warning on its own line above the groups, on the warning slot.

The configuration file is `~/.config/coding-crew/config.toml`, and its format
is **TOML**: it is the one file a human edits, and a comment is how a
configuration explains itself, so the defaults hugo writes carry a short note
beside each part. raul's Settings edits the same file. When the file is
absent, hugo writes the defaults — the project's own flow: the builder on
`claude-haiku-5-5` at `high`, the reviewer and the orchestrator on
`claude-opus-5-5` at `medium`, every agent in the `claude` harness; five
build-and-fix cycles; and the order [roles/roles.md](../roles/roles.md#the-order)
records. Each agent names its own harness, model and effort; the harnesses
are the table's three, `claude`, `opencode` and `cursor-agent`, and nothing
else is one.

The old `config.json` is **not read** any more, and it is never converted: the
format is provisional and a conversion would guess. When one sits beside the
configuration, the sidebar says so — `config.json is not read` — rather than
ignoring it silently.

The sidebar acts in two small ways. `r` reloads the configuration: the file is
read again, the session starts over from it — the picks made this session are
dropped — and the sidebar shows it at once, so a hand-edited file takes effect
without a restart. Reloading is not starting the tool: the runtime record is
already read on every redraw, so the reload touches the configuration only. A
file that does not parse after a reload is handled as it is at startup — the
file and the error are named, the file is left alone, and the defaults are
shown beside it — and a reload with no configuration file writes the defaults.

Every agent is configured from hugo. The cursor moves over each agent's
harness, model and effort, and `enter` opens that row's choice: the builder's
and the reviewer's rows open theirs as the orchestrator's does. A pick applies
at once, **for the session**: it is drawn and used on the spot, and nothing is
written to the file. `esc` closes a choice without picking anything. The title
of a choice is the agent and the field, as `builder model`.

**The file's values are the session's defaults.** The session starts from the
file as it is at launch, and a pick is a difference from it. raul's Settings
edits the file's values, and a hand edit does too; a session that has picked a
value keeps its pick until the session ends or `r` starts it over.

**The session tag.** The sidebar draws the session's crew, and marks what
differs from the file: an agent with a value that differs carries a dim
`session` after its name, and the key of each field that differs is strong.
An agent the file does not have is tagged the same way. The `AGENTS` group
carries `session` too when the session's crew has other roles than the file's.

**The crew size.** `c` opens the crew-size selector: three lines, one per size,
each drawn as the roles it has and the roles it leaves out, then its hint.

```text
 󰚩○○ single agent
 󰚩󰖷○ orchestrator also reviews
 󰚩󰖷󰈈 one agent per role
```

The roles are the icons of the sidebar: the orchestrator, the builder and the
reviewer; an empty circle stands for a role the size leaves out. The unicode
set draws the same roles with its own glyphs, as the sidebar does. The cursor
starts on the size in force, `↑ k` and `↓ j` move it, `enter` picks it for the
session and `esc` closes it. The orchestrator is always there, so the sizes
are one, two and three roles, not a count.

A size takes the roles it has and leaves the rest off the session's crew. A
role that is left off keeps its values, so a role the crew takes back comes
with them; a role the session never had comes with the file's values, else the
defaults'. The crew's composition is still the one the main pane states: one
per role, the orchestrator reviews, or the orchestrator alone.

**The harness choice.** The list is the table's three harnesses. A harness the
machine does not have is listed dim as `not installed` and cannot be picked.
Picking another harness reads that harness's catalogue first, bounded as below,
and then switches the agent. The model stays only when the new harness lists
it; otherwise it is cleared, and the row says `pick a model` on the warning
slot. The effort stays when the model takes it; otherwise it is cleared, since
an effort is a level of a model. Nothing is carried over silently. A catalogue
that cannot be read refuses the switch, and the choice names why.

**The model choice.** The list is the harness's own list: `claude` lists its models
from the table, `opencode` reads its server's `model.list`, and `cursor-agent`
runs `cursor-agent --list-models`. It is derived when the harness is first read,
never written out by hand. A pick that takes a model the new level list lacks
clears the effort: a model change keeps only the levels the new model takes. The last entry, `type an id…`, opens an inline
input. A typed id is checked by its shape — one word or `provider/model`, no
whitespace, no `#variant` — and not against the list; one that is not shaped
so says `not a model id` on the input's line. While the input is open
every key is text but `ctrl-c`, and `esc` returns to the list. The read is
bounded: the choice opens at once and says `reading the model catalog` while
the tool runs, and the window keeps drawing and taking its keys meanwhile. A
tool that takes longer than 5 seconds is given up, and one that is missing or
fails names its error in the choice — `the model catalog cannot be read` and
the reason — so no key waits on a harness tool. The choice still lists
`type an id…` under the reason, so a model can be typed in by hand.

**The effort choice.** The list is the levels of the agent's model, lowest
first, with the level in force under the cursor. The levels come from the model,
not the harness: `claude`'s Opus, Sonnet and Fable take five, Haiku three;
`opencode`'s are its variants; `cursor-agent`'s are the level ending each id.
An agent with no effort says `pick an effort` on the warning slot, and `s`
refuses until one is picked. A model that takes no level shows `—` and needs
none. With no model the choice says `pick a model first`. A model the catalogue
does not list has no levels of its own, so its list is the harness's levels, then
`none` (start with no effort flag, stored as an empty `effort`), then `type a
level…`, an inline input that takes one word, unchecked; any of them completes the
effort.

Nothing else on the sidebar is changed from the screen. `s` starts the
orchestrator over the session's crew; the loop it runs is the orchestrator's.

**Nothing is dropped without a word.** A row too wide for the pane is cut with
the theme's marker — the provider and the model's own name are on rows of their
own, so the model is whole at the sidebar's 32 columns. A group that does not
fit the height is left out whole, and one line under the groups says
`not shown:` with each group's name and how much it held: `ORDER (14 hidden)`.

A configuration file that is there but cannot be read is not the empty state:
the sidebar says `configuration is unreadable`, names the file and the error,
and paints the error on the failure slot. Under them it shows the defaults,
named by `using the defaults:`, so the human sees both what is wrong and what
the tool would use. hugo only writes the defaults over an absent file, never
over one a human has edited.

## Picking the queue

The claimable tasks are pickable. `tab` moves the keys from the configuration to
the queue, and back: it moves only when the queue has a claimable task. A waiting
task that is not claimable is drawn and not picked: the cursor passes over it,
`space` and `a` do not mark it, and a click on it does nothing. While the queue
has the keys:

| key | does |
|---|---|
| `↑ k` and `↓ j` | move the queue's cursor over the claimable tasks, and no further |
| `space` | marks the task under the cursor, or unmarks it |
| `a` | marks every claimable task, or unmarks them all when they are all marked |
| `s` | starts the orchestrator over the marked tasks, or over the whole queue when none is marked — see [starting the orchestrator](#starting-the-orchestrator) |
| `tab` | gives the keys back to the configuration |

The cursor's row carries the cursor bar and a marked task carries `[x]`; the
sidebar's cursor is not drawn while the queue has the keys. The configuration's
keys do nothing while the queue has them. The marks are the session's: a
reload with `r` drops them, and nothing is written to the record or the file.

## The runtime record

hugo reads the nearest project's `coding-crew/runtime.json`, discovered from
the current directory by the store's own project boundary. The record sits
beside the journal; its shape and its provisional state are in
[tasks/task-model.md § 11.2](../tasks/task-model.md#112-the-runtime-record-provisional).

The record's queue is the run's tasks in queue order, each with its status and
its `claimable` flag: the done ones and the held one are in it, and the pane
draws the waiting ones, picking those that are claimable.
The round, the budget and the findings are the task in flight's. The record is
the flow's: hugo reads it, and writes nothing. A project
without a record shows the named empty state `nothing is running`. A record
that cannot be read is not that state: the pane says
`the runtime record cannot be read`, names the file and the error, and paints
the error on the failure slot, as raul names a file it cannot read.

## Starting the orchestrator

`s` starts the orchestrator. hugo opens a herdr tab in the current workspace,
starts the agent the orchestrator row names, on that row's harness and model,
and prompts it to run `hugo --skill orchestrator` over the queue's tasks, in
order. The tab's environment carries the session's crew, in `HUGO_SESSION`, so
the skill prints the crew the session runs. The loop is the orchestrator's:
hugo starts it and does not run it.

The queue `s` starts is the claimable waiting work, in queue order.
When some of it is marked, `s` hands over the marked tasks alone, still in queue
order; otherwise the whole queue. A task an agent holds is that agent's, so no
second orchestrator is given it. The footer names `s start`
only when it can act: herdr hosts hugo, the queue has work, every agent has its
harness, model and effort, and no start is in flight.

| state | what `s` does |
|---|---|
| outside herdr (`HERDR_ENV` is not `1`) | starts nothing: `start needs herdr` |
| the queue is empty | starts nothing: `the queue is empty` |
| tasks wait, none claimable | starts nothing: `no waiting task is claimable` |
| an agent has no harness, model or effort | starts nothing: `the orchestrator did not start:` and the gaps, named — `builder has no model` — on the failure slot |
| herdr already names an orchestrator | starts nothing: `an orchestrator is already running in herdr` |
| starting | `starting the orchestrator…`, dim. The herdr calls run beside the window, which keeps drawing and taking keys |
| started | `orchestrator started in tab <id>`, on `semantic.ok` |
| failed | `the orchestrator did not start:` and the error, on the failure slot. When the tab was already open, the error names it |

The note sits on its own row above the footer's rule while a start has
something to say. The last thing a start said stays until the next start.

## Mouse

The mouse works where the keys do. The wheel scrolls what is under the
pointer, three rows a notch, and a burst of notches costs one redraw.

| gesture | does |
|---|---|
| wheel over the sidebar | moves the agents' cursor, and takes the focus as a click does |
| wheel over the queue | moves the queue's cursor, and takes the focus |
| click a sidebar row | moves the cursor to that field; a second click on it opens its choice, as `enter` does |
| click a queue row | moves the queue's cursor to it and takes the focus; a second click on it marks the task, as `space` does |
| click a choice or crew-size entry | moves the picker to it; a second click picks it, as `enter` does |

The sidebar takes no clicks where the window drops it, below 80 columns. A
click outside these regions does nothing, and while the help or the id input
holds the keys the pointer waits. Most terminals keep their own text selection
for a drag held with shift.

## The layout

The sidebar is 32 columns wide, its border included. At 80 columns and wider
the window holds both panes side by side: the sidebar on the left and the
main pane, at least 48 columns, taking the rest.

Below 80 columns the sidebar is dropped whole — it is never squeezed, and the
main pane is never squeezed to keep it. The main pane then takes the whole
window, and the drop is **announced**: the line above the panes reads

```text
 sidebar not shown: configuration (it needs 80 columns)
```

The line is secondary text, and it names what is not shown. Where the full
line does not fit, the short form `sidebar not shown: configuration` still
names it; where even that does not fit, the line is cut and the cut is
marked. The drop is never silent.

A window too short for the whole frame gives up its title and its footer
first, then the panes' boxes: the fact that the sidebar is not shown is the
last line to give way, and a pane without room for a box keeps its words.
The same degradation is raul's: the words matter more than the chrome.

## The configuration home

The configuration lives in `~/.config/coding-crew/`. When the folder is not
there, hugo creates it — the tool creates its home rather than failing. When
`config.toml` is not there, it writes the defaults, comments included. A
machine with no home folder has nowhere for hugo to write: the shell still
runs, and the sidebar says `no configuration yet`.

The configuration is read at launch, and `r` reads it again without a
restart. hugo writes nothing there from the screen: its picks last the session,
and the file keeps its comments and its values. `s` starts the orchestrator
without writing anything.

The records are not part of this file and do not follow it: the journal and
the runtime record are machine-written, so they stay JSON.

## The render mode

```sh
hugo --render --plain [--width N] [--height N]
```

prints the screen as plain UTF-8 and exits 0, the same contract raul has, so
the screens can be pinned byte for byte. The output is deterministic: the
same bytes on every run. A render draws at the fixed instant
`2026-11-19T09:00:00Z`, not the wall clock, so a pinned screen never moves
with the day. `--width` and `--height` are bounded to 1 up to 1000 and default
to 100 by 40. `--theme` chooses one of the shared presets —
`dark`, `nord`, `catppuccin`, `dracula`, `terminal` — and changes the paint,
never the words.

The pinned screens are `crates/hugo/tests/snapshots/screen-40.txt`,
`screen-60.txt` and `screen-100.txt`. They are rendered from `mock-data/`,
against a home whose configuration the tool wrote, so they carry the mock
runtime record and the default configuration. The mock run is a queue of five
tasks: two done, one in flight in round 2 with the reviewer working and the
builder holding it too, and two waiting. Round 1 of the task in flight raised
three findings, one high and two medium; two are resolved, so the one high is
still open — the task's own fixture holds that finding.

## The skills

```sh
hugo --skill
hugo --skill orchestrator|builder|reviewer
```

`--skill` prints text and exits; no window is set up. A bare `--skill` lists
the three roles. A role prints that role's skill, opened by a **crew** section
built from the crew in force: the session's, when the orchestrator's tab sets
`HUGO_SESSION`, else the file's. Each agent's role, harness, model and effort
with the arguments and environment that start it, and the review-round limit,
are named; a role the crew leaves out is named `none`. A configuration that cannot be read prints the defaults and says so in
one line. The texts live in `crates/hugo/skills/` and are embedded with
`include_str!`. `--skill` does not combine with `--render`.
