# hugo — screens

hugo is the TUI for the flow (**H**uman-guided **U**nified **G**overnance &
**O**rchestration). It reads the runtime and shows it. It does not drive the
loop, and it writes nothing but its own configuration.

The interface was chosen in [design-record.md](design-record.md): one sidebar
for the configuration and one main pane for what is running, with no tabs. The
reading is **the crew leads**.

## The window

The screen below is the mock's 100-by-40 render: the pinned screen in
`crates/hugo/tests/snapshots/screen-100.txt`.

```text
 hugo
┌configuration─────────────────┐┌flow──────────────────────────────────────────────────────────────┐
│agents                        ││┌crew────────────────────────────────────────────────────────────┐│
│builder · runner              │││orchestrator  glm-5.3#high  —  held —  idle                     ││
│  opencode-go                 │││runner  deepseek-v4.1-flash#high  TSK10  held 10d  executing    ││
│  deepseek-v4.1-flash#high    │││reviewer  glm-5.3#high  TSK13  held 8d  reviewing               ││
│reviewer                      │││one per role · the reviewer reviews                             ││
│  opencode-go                 ││└────────────────────────────────────────────────────────────────┘│
│  glm-5.3#high                ││┌queue───────────────────────────────────────────────────────────┐│
│orchestrator                  │││the automation's queue · 1 task                                 ││
│  opencode-go                 │││TSK29  approved                                                 ││
│  glm-5.3#high                ││└────────────────────────────────────────────────────────────────┘│
│harness                       ││┌round───────────────────────────────────────────────────────────┐│
│opencode                      │││TSK13 · external · round 2                                      ││
│  --standalone · role config  ││└────────────────────────────────────────────────────────────────┘│
│review-round limit            ││┌budget──────────────────────────────────────────────────────────┐│
│5 build-and-fix rounds        │││external rounds 18 of 45                                        ││
│order                         ││└────────────────────────────────────────────────────────────────┘│
│1 groom · creator             ││┌findings────────────────────────────────────────────────────────┐│
│  groomed → approved          │││21 open findings                                                ││
│2 build · runner              ││└────────────────────────────────────────────────────────────────┘│
│  approved → executing        ││                                                                  │
│3 self-review · runner        ││                                                                  │
│  executing → code-review     ││                                                                  │
│4 review · reviewer           ││                                                                  │
│  code-review → done          ││                                                                  │
│  code-review → remediating   ││                                                                  │
│  code-review → needs-human   ││                                                                  │
│5 remediate · runner          ││                                                                  │
│  remediating → code-review   ││                                                                  │
│6 re-review · reviewer        ││                                                                  │
│  code-review → done          ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││                                                                  │
│                              ││● running since 2026-11-09T08:30Z                                 │
└──────────────────────────────┘└──────────────────────────────────────────────────────────────────┘
 q  quit
```

| part | behaviour |
|---|---|
| title line | the name of the tool, `hugo`; a window too narrow for it shows no title rather than a cut of the name |
| sidebar | the configuration, on the left: the agents and the model each runs, the harness, the review-round limit and the order, in that order. It is read-only — nothing on it is changed from the screen, and the footer names no key that changes it |
| main pane | what is running: the crew, the queue, the round, the budget and the findings, with the stamp at its foot |
| footer | the keys that work — never a key that does nothing. The window's keys are `q quit` and `ctrl-c quit`; the footer carries `q quit` |
| tab bar | none: the flow has one screen, not a set of tabs |

The two empty states are painted on `text.dim`, as raul's empty states are.
The main pane says `nothing is running` when there is no runtime record to
read; the sidebar says `no configuration yet` when there is no home to hold
one.

## The main pane

The blocks are read in this order; the crew leads.

| order | block | shows |
|---|---|---|
| 1 | `crew` | one entry per agent: its role, the model it runs, the task it holds (or `—`), how long it has held it, and its state; then the composition, in words |
| 2 | `queue` | **the automation's queue**: the unclaimed work, an id and a status — never a title, a note or an intent |
| 3 | `round` | the review in flight: the task, the stage and the round's number; `no review in flight` when there is none |
| 4 | `budget` | the external rounds spent of the total |
| 5 | `findings` | the open findings, as a count |
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
entry and never in the queue, even when a writer left it in both.

**The same fact is painted once.** A count appears once, the queue's count is
the unclaimed work, and the round's task is named as the review, not counted
again.

The queue's row is an id and a status. Its count names its set: `the
automation's queue · 1 task`. The budget's count names its set: `external
rounds 18 of 45`. The findings' count names its set: `21 open findings`.

The stamp is a line: `● running since 2026-11-09T08:30Z` when the automation
runs, `● not running: <why>` when it does not. A stop with no reason still
says a word: `not running: no reason given`.

An agent's entry is one line while its facts fit; a narrower pane gives it
two lines, then three. No fact is dropped with the width: the model, the
held task, the length of the hold and the state are all on the entry at every
width the pane is drawn at. The model is the model's own name; the provider
belongs to the configuration the sidebar shows.

The hold is a length, not a clock reading: `held 10d`, from the record's
`since` and the frame's one instant. It is written in whole days, hours or
minutes; a hold that is under a minute says `held under a minute` rather than
`0m`. The pane reads the clock once per frame, in `main.rs`, and every module
below it takes the instant it is given.

A block that does not fit the pane is left out whole, and one line says
`not shown:` with each block's name **and what it held**: `crew (5 hidden)`,
the way the row tabs name the rows they hide. The boxes go before the words;
the stamp is the last fact to give way.

## The configuration

The sidebar is the configuration: what the human governs, read-only. It shows
four groups, in this order:

| order | group | shows |
|---|---|---|
| 1 | `agents` | one entry per agent: its name and role, then the model it runs — the provider and the model's own name on the rows under it |
| 2 | `harness` | the tool the agents run in, and the way it is driven |
| 3 | `review-round limit` | the build-and-fix cycles a task gets, and the set the count counts |
| 4 | `order` | the numbered steps, each with the gates it passes |

The configuration is **provisional**: the flow has not written one yet, and the
format may change when it does. Its shape is
[tasks/task-model.md § 11.3](../tasks/task-model.md#113-the-configuration-provisional).

The configuration file is `~/.config/coding-crew/config.json`. When it is
absent, hugo writes the defaults — the project's own flow: the builder on
`opencode-go/deepseek-v4.1-flash#high` and the reviewer on
`opencode-go/glm-5.3#high`, with the orchestrator beside them; the `opencode`
harness, `--standalone` with each agent's role config; five build-and-fix
cycles; and the order [roles/roles.md](../roles/roles.md#the-order) records.

Nothing on the sidebar is changed from the screen: whether hugo drives the loop
or only shows it is still open, so no key changes the configuration and no key
starts a run. The footer names only `q quit`.

**Nothing is dropped without a word.** A row too wide for the pane is cut with
the theme's marker — the provider and the model's own name are on rows of their
own, so the model is whole at the sidebar's 32 columns. A group that does not
fit the height is left out whole, and one line under the groups says
`not shown:` with each group's name and how much it held: `order (14 hidden)`.

A configuration file that is there but cannot be read is not the empty state:
the sidebar says `configuration is unreadable`, names the file and the
error, and paints the error on the failure slot. hugo only writes the defaults
over an absent file, never over one a human has edited.

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
`config.json` is not there, it writes the defaults. A machine with no home
folder has nowhere for hugo to write: the shell still runs, and the sidebar
says `no configuration yet`.

The configuration is read at launch. hugo changes nothing of it from the
screen: whether hugo drives the loop or only shows it is still open.

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
