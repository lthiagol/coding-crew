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
 󰚩 hugo · mock-data
╭─ 󰒓 configuration ────────────╮╭─ 󰑮 flow ─────────────────────────────────────────────────────────╮
│ AGENTS                       ││╭─ 󰚩 crew ───────────────────────────────────────────────────────╮│
│▌󰖷 builder · runner           │││󰚩 orchestrator glm-5.3#high             —     —     idle        ││
│   opencode-go                │││󰖷 runner       deepseek-v4.1-flash#high TSK10 󰥔 10d ⠋ executing ││
│   deepseek-v4.1-flash#high   │││󰈈 reviewer     glm-5.3#high             TSK13 󰥔 8d  ⠋ reviewing ││
│ 󰈈 reviewer                   │││one per role · the reviewer reviews                             ││
│   opencode-go                ││╰────────────────────────────────────────────────────────────────╯│
│   glm-5.3#high               ││╭─ 󰚇 queue ──────────────────────────────────────────────────────╮│
│ 󰚩 orchestrator               │││the automation's queue · 1 task                                 ││
│   opencode-go                │││TSK29  approved                                                 ││
│   glm-5.3#high               ││╰────────────────────────────────────────────────────────────────╯│
│ HARNESS                      ││╭─ 󰈈 round ──────────────────────────────────────────────────────╮│
│ opencode                     │││TSK13 · external · round 1                                      ││
│   --standalone · role config ││╰────────────────────────────────────────────────────────────────╯│
│ ROUNDS                       ││╭─ 󰥔 budget ─────────────────────────────────────────────────────╮│
│ 5 build-and-fix rounds       │││external rounds ████░░░░░░ 18 of 45                             ││
│ ORDER                        ││╰────────────────────────────────────────────────────────────────╯│
│ 1 groom · creator            ││╭─ 󰅚 findings ───────────────────────────────────────────────────╮│
│   groomed → approved         │││17 open findings · 8 high · 8 medium · 1 low                    ││
│ 2 build · runner             ││╰────────────────────────────────────────────────────────────────╯│
│   approved → executing       ││                                                                  │
│ 3 self-review · runner       ││                                                                  │
│   executing → code-review    ││                                                                  │
│ 4 review · reviewer          ││                                                                  │
│   code-review → done         ││                                                                  │
│   code-review → remediating  ││                                                                  │
│   code-review → needs-human  ││                                                                  │
│ 5 remediate · runner         ││                                                                  │
│   remediating → code-review  ││                                                                  │
│ 6 re-review · reviewer       ││                                                                  │
│   code-review → done         ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││󰑮 running since 2026-11-09T08:30Z                                 │
╰──────────────────────────────╯╰──────────────────────────────────────────────────────────────────╯
 q  quit   r  reload   ↑↓ j/k  move   ?  help
```

| part | behaviour |
|---|---|
| title line | the tool's icon and name, `hugo`, then a dim `·` and the project's folder when one is found; a window too narrow for the name shows no title rather than a cut of the name |
| sidebar | the configuration, on the left: the agents and the model each runs, the harness, the review-round limit and the order, in that order. The agents group has a cursor: `↑ k` and `↓ j` move it, and `enter` on the orchestrator's row chooses its model. The builder's and the reviewer's rows are read-only. `r` reloads the configuration from the file |
| main pane | what is running: the crew, the queue, the round, the budget and the findings, with the stamp at its foot |
| footer | the keys that work — never a key that does nothing. The window's keys are `q quit`, `ctrl-c quit`, `r reload`, `↑ k` `↓ j` move, `enter choose` (on the orchestrator's row), `s start` (when it can act: [starting the orchestrator](#starting-the-orchestrator)) and `? help`; while the choice is open its keys are `esc close`, `enter pick`, `↑ k` `↓ j` move, and `q quit` and `? help` still work |
| choice | the `orchestrator model`, as a small list of the catalogue's models with a cursor of its own; `enter` picks one and saves it, `esc` closes it without saving |
| help | `?` opens the keys, in the sidebar and over the choice; any key closes it and gives the choice back |
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
| 2 | `queue` | **the automation's queue**: the unclaimed work, an id and a status — never a title, a note or an intent |
| 3 | `round` | the review in flight: the task, the stage and the round's number; `no review in flight` when there is none |
| 4 | `budget` | the external rounds spent of the total, as a bar and the numbers |
| 5 | `findings` | the open findings, as a count, and by severity when the record holds them |
| 6 | stamp | at the foot: whether the automation runs, **and why** when it does not |

**The crew's size is a composition, not a number.** Three agents are one per
role and the reviewer reviews; two are an orchestrator and a worker, and the
orchestrator reviews; one leaves the review to the human, because an agent
cannot review itself. The pane says who reviews in words — one per role · the
reviewer reviews; the orchestrator reviews; the human reviews — and never
prints a bare count of agents. A crew with more than one agent in a role has
no such rule: the pane says `more than one agent per role` rather than
claiming a rule the crew does not follow.

**A held task belongs to the crew, not the queue.** It appears in its agent's
row and never in the queue, even when a writer left it in both.

**The same fact is painted once.** A count appears once, the queue's count is
the unclaimed work, and the round's task is named as the review, not counted
again.

The queue's row is an id and a status. Its count names its set: `the
automation's queue · 1 task`. The budget's count names its set: `external
rounds ████░░░░░░ 18 of 45`. The findings' count names its set: `17 open
findings`. When the record holds the breakdown, it follows on the same count:
`· 8 high · 8 medium · 1 low`, each count in its severity's slot — `high` on
`semantic.error`, `medium` on `semantic.warn`, `low` on `semantic.info`. A
narrow pane wraps the breakdown under the count rather than cutting it.

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

The sidebar is the configuration: what the human governs. It shows four
groups, in this order:

| order | group | shows |
|---|---|---|
| 1 | `AGENTS` | one entry per agent: its role's icon, its name and role, then the model it runs — the provider and the model's own name on the rows under it |
| 2 | `HARNESS` | the tool the agents run in, and the way it is driven |
| 3 | `ROUNDS` | the build-and-fix cycles a task gets, and the set the count counts |
| 4 | `ORDER` | the numbered steps, each with the gates it passes |

The configuration is **provisional**: the flow has not written one yet, and the
format may change when it does. Its shape is
[tasks/task-model.md § 11.3](../tasks/task-model.md#113-the-configuration-provisional).

The configuration file is `~/.config/coding-crew/config.toml`, and its format
is **TOML**: it is the one file a human edits, and a comment is how a
configuration explains itself, so the defaults hugo writes carry a short note
beside each part. raul's Settings edits the same file. When the file is
absent, hugo writes the defaults — the project's own flow: the builder on
`opencode-go/deepseek-v4.1-flash#high` and the reviewer on
`opencode-go/glm-5.3#high`, with the orchestrator beside them; the `opencode`
harness, `--standalone` with each agent's role config; five build-and-fix
cycles; and the order [roles/roles.md](../roles/roles.md#the-order) records.

The old `config.json` is **not read** any more, and it is never converted: the
format is provisional and a conversion would guess. When one sits beside the
configuration, the sidebar says so — `config.json is not read` — rather than
ignoring it silently.

The sidebar acts in two small ways. `r` reloads the configuration: the file is
read again and the sidebar shows it at once, so a hand-edited file takes effect
without a restart. Reloading is not starting the tool: the runtime record is
already read on every redraw, so the reload touches the configuration only. A
file that does not parse after a reload is handled as it is at startup — the
file and the error are named, the file is left alone, and the defaults are
shown beside it — and a reload with no configuration file writes the defaults.

The orchestrator's row in the agents group is the one editable thing: the
human moves the cursor to it and presses `enter` to choose the model the
orchestrator would run on. The choice is a small list with a cursor of its
own; `enter` saves the model and returns to the sidebar, and `esc` closes it
without saving anything. The builder's and the reviewer's models are
read-only: the flow starts those, not hugo, so their rows open nothing.

**The models are the catalogue's.** The list is `opencode models`, derived at
the moment the choice opens, never written out by hand. A pick is checked
against the same catalogue the list comes from, so the configuration cannot
hold a model that does not exist. The read is bounded: the choice opens at
once and says `reading the model catalog` while the tool runs, and the window
keeps drawing and taking its keys meanwhile. A tool that takes longer than
5 seconds is given up, and one that is missing or fails names its error in the
choice — `the model catalog cannot be read` and the reason — so no key waits
on a harness tool.

Nothing else on the sidebar is changed from the screen, and no other setting is
written. `s` starts the orchestrator; the loop it runs is the orchestrator's.

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

## The runtime record

hugo reads the nearest project's `coding-crew/runtime.json`, discovered from
the current directory by the store's own project boundary. The record sits
beside the journal; its shape and its provisional state are in
[tasks/task-model.md § 11.2](../tasks/task-model.md#112-the-runtime-record-provisional).

The record is the flow's: hugo reads it, and writes nothing. A project
without a record shows the named empty state `nothing is running`. A record
that cannot be read is not that state: the pane says
`the runtime record cannot be read`, names the file and the error, and paints
the error on the failure slot, as raul names a file it cannot read.

## Starting the orchestrator

`s` starts the orchestrator. hugo opens a herdr tab in the current workspace,
starts the agent the orchestrator row names, on that row's harness and model,
and prompts it to run `hugo --skill orchestrator` over the queue's tasks, in
order. The loop is the orchestrator's: hugo starts it and does not run it.

The queue is the unclaimed work the main pane draws. A task an agent holds is
that agent's, so no second orchestrator is given it. The footer names `s start`
only when it can act: herdr hosts hugo, the queue has work, and no start is in
flight.

| state | what `s` does |
|---|---|
| outside herdr (`HERDR_ENV` is not `1`) | starts nothing: `start needs herdr` |
| the queue is empty | starts nothing: `the queue is empty` |
| herdr already names an orchestrator | starts nothing: `an orchestrator is already running in herdr` |
| starting | `starting the orchestrator…`, dim. The herdr calls run beside the window, which keeps drawing and taking keys |
| started | `orchestrator started in tab <id>`, on `semantic.ok` |
| failed | `the orchestrator did not start:` and the error, on the failure slot. When the tab was already open, the error names it |

The note sits on its own row above the footer while a start has something to
say. The last thing a start said stays until the next start.

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
restart. The only thing hugo writes there from the screen is the
orchestrator's picked model. `s` starts the orchestrator without writing
anything.

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
runtime record and the default configuration.

## The skills

```sh
hugo --skill
hugo --skill orchestrator|runner|reviewer
```

`--skill` prints text and exits; no window is set up. A bare `--skill` lists
the three roles. A role prints that role's skill, opened by a **crew** section
built from the configuration in force: the harness and how it is driven, each
agent's role and model with the arguments that start it, and the review-round
limit. A configuration that cannot be read prints the defaults and says so in
one line. The texts live in `crates/hugo/skills/` and are embedded with
`include_str!`. `--skill` does not combine with `--render`.
