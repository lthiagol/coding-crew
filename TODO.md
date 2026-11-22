# coding-crew — orchestration state

Updated 2026-11-21 by the orchestrator.
Branch `main`, remote `https://github.com/lthiagol/coding-lanes.git`.
The gate is `make check` (`make ci` is an alias): `cargo fmt --all -- --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`,
`bash scripts/test-pty-sweep.sh`, the flow suites in the global skill (skipped
when the skill is not installed).

**What is built is in [`DONE.md`](DONE.md).** This file is the live state: what
is open, what is next, and what has been decided.

---

## SCOPE

Two TUIs: **raul** for the work, **hugo** for the flow. The Autopilot tab is
gone from raul; the automation's questions live in hugo, which is one sidebar
for configuration and one main pane for what is running, with no tabs. Its
configuration is centralized under `~/.config/coding-crew/`, and that folder is
created on demand when it does not exist.

**talita is not being built.** Do not write talita code, do not extend its
commands, do not wire raul to it, until that is said explicitly.

`crates/core` is shared and stays editable.

---

## HOW TO RESUME

1. Read `AGENTS.md`, then `docs/`.
2. Read this file, and `docs/plan.md` — the plan holds the small units with
   their done-when lines, the grooming, and the open questions.
3. Read the last review's rulings — they are durable, in the design record for
   the unit that was reviewed.
4. Pick the first open item you can finish.
5. Run `make check` before you claim anything is done.
6. Update this file when the item is closed, and move its record to
   `DONE.md`.

The build loop itself is the global skill `orchestration-flow`, at
`~/.config/opencode/skills/orchestration-flow/SKILL.md` — the three agents, the
shared Orchestration workspace, the pane recipe, the rounds, the budget. The
project's prefix is in `.orchestration.json`; the role configs live in `flow/`,
and the flow skill does not read them — it generates one per session, outside
the repository.

The findings of the review rounds are not durable — `/tmp` does not survive.
Copy what matters into this file, or into the design record, before the round is
closed. The hugo proposals' rulings are in `docs/hugo/design-record.md`, and the
defects the review found are in the known-issues block of
`docs/hugo/tui-proposals/index.html`.

---

## THE OPEN UNITS

Three units, in the order they run. The order is in `docs/plan.md` — rows 49 to
51 — and this is the list of them.

### 49 · The projection earns the colours

The projection emits `##` and `###` headings, bold, GFM tables, `- ` lists and
`<br>` — and, since the review nests, `###`/`####` headings with bullets for
the rounds and the findings. Nothing in the engine's vocabulary is reached:
no blockquote, no task list, no rule, no link, no alert. A new palette would
stay dead colour.

**Done when:** the projection emits the constructs the palette paints — a
level-one heading, sub-headings, blockquotes, task lists, rules and links — so
no role in the document palette is dead; and the detail pane's snapshot is
updated to the new document.

### 50 · Per-preset syntax, the code gutter, the inline chip

`crates/raul/src/detail.rs:59-63` pairs `base16-ocean.dark` with every
preset, so Nord, Catppuccin and Dracula all wear ocean's code tokens. Inline
code has no background at all, because `ChromeBorder` carries no `bg`.

**Done when:** each preset pairs a syntax theme of its own; the fenced code
carries a numbered gutter; and inline code has a background of its own.

### 51 · The criteria table reads

The Acceptance criteria section is a **seven-column** table — id, description,
verification, status, evidence, deferral, failure reason — squeezed into the
pane's width. At 40 columns every column is one character wide and the header
wraps vertically, one letter per line. It is the same defect the review table
had, in a different section.

**Done when:** each criterion reads as a block with its own fields, the way
Steps and the review read; the section is readable at 40 columns as well as at
100; the verification and the evidence stay inside their code spans, and the
injection property holds.

---

## DECISIONS OF RECORD

- **The document gets eighteen roles of its own** (2026-11-21). Not leaf's
  thirty-six — our chrome palette is deliberately quiet, and the document
  should not fight it. Eighteen is the number the human chose.
- **A row's second line** (2026-11-21): the review state on Needs you and Path,
  the item's own body on Backlog. Those are the facts a human decides on, and
  all of them are already in the model. **The mechanism is settled per tab:**
  the review state is a compact **suffix** on the row, not a second line — a
  second line would move the Path tree's row arithmetic, and a Path row that
  grows a line changes what the tab counts. The **Backlog's body is a second
  line**, dim and indented to where the row's title starts, cut by the theme
  crate's shared `cut_to` so a cut body ends in the tab's own `…`; a row with
  a body is two lines everywhere the tab counts, and a row is drawn in both
  its lines or in neither.
- **The state file is a Markdown pair** (2026-11-21): `TODO.md` for the live
  state, `DONE.md` for the archive. The archive no longer lives in the live
  file.
- **raul draws task content; hugo draws tasks only as items of a queue.** The
  seam is the data.
- **The Path tree holds the live work only.** Done tasks have no home of their
  own, and that is accepted for now. **Open question for the human:** the
  five done fixtures now carry a full history, and no tab shows them — the
  Path tree hides them and no other view holds them, so a reader cannot reach
  a finished task without knowing its id. Either the Path tab grows a
  finished branch, or a Done view exists.
- **A round is numbered by the model, unpadded** (2026-11-21). The review
  heading reads `Round 1`, not `Round 01`. The self pre-screen is round 0, and
  padding it would render "Round 00"; the model's own number is what the
  record holds. The human's sketch wrote "Round 01" to show the nesting, and
  the number was its example. Ruled on the SPEC review's reading of the
  kickoff; a one-line change and its pins would overrule it.

---

## THE FOLLOW-UPS

None blocking. Carried from the closed units.

- **The document overflows below 14 columns** (2026-11-22). The header table
  is 11 columns wide, so an inner width of 10 (a 12-column pane) overflows; a
  quoted code line overflows at 11 inner (13 columns). The detail test checks
  the body at 12 and the overflow only from 14 up.
- **The document's roles are slots in the rule's sense** (2026-11-21). Rule 1
  means every colour is named by something the theme crate owns and a preset
  decides — so the document's eighteen roles are slots, and the rules bind
  them as they bind the chrome's. The rule was **extended** to say so, not
  weakened: `theme.md` rule 5 and `palettes.md` rule 11.
- **The heading levels are settled** (2026-11-21): the ten sections are `#`,
  an item under a section `##`, a finding under its round `###`. The engine's
  `heading_4` and `heading_other` join the deepest, because the document has
  no level below `###`. **The document has no h1 of its own** — the record's
  title is a label and must not be interpreted, so it is never a heading.
- **The theme catalog's count guard** recognises a count only through a fixed
  noun vocabulary — `crates/theme/src/theme.rs:2515` holds fourteen nouns —
  so a count phrased outside it escapes silently. The same shape was found
  and **fixed** in the mock data's README guard: the guard must decide by the
  number the sentence states, and a number that is not a count must be listed
  explicitly. The pattern is known; the theme guard has not been changed.
- **`number_word` mis-sums a scaled hyphen compound** — "two-hundred" reads as
  102, "three-thousand" as 1003. Detection is unaffected; only a recompute
  would be wrong, and no prose states one today.
- **The document palette, in Dracula:** six roles share `#6272a4` —
  `heading_finding`, `list_depth_3`, `code.chip`, `rule.line`, `search.hit`
  and `task.open` — so a `###` heading reads like a table border. The
  apartness pins reach only *within* a group, so the cross-group repeats are
  unpinned. They are the family's own narrowness: Dracula names two whites
  and one grey, and rule 9 forbids inventing hues. Documented, not hidden.
- **The Backlog:** a one- or two-row pane below 39 columns gives a total hidden
  count without naming the groups; the compact announcement can resemble the
  group header's shape; very narrow panes cut or drop group names and counts.
- **The Backlog's body line:** at narrow widths the id and kind columns drop,
  so the body is flush-left and "belongs to the row above" rests on the dim
  slot alone; a one-line pane drops the cursor row's body unnamed, the
  footer's count naming hidden rows and not the absent body; a multi-line
  body is normalised to one line — every word kept, so not a summary — and
  no fixture body is multi-line, so no pinned render exercises it.
- **The Backlog's counting:** the comment *"only a pane of one line shows the
  row alone"* is misleading — a two-line row on a two-line pane also leaves
  no room and shows the row alone with no announcement; `body_line` does not
  guard `width == 0` the way `columns_line` does, so a body row would paint a
  second blank line at width 0 (unreachable below height 2); and the
  atomicity pin sweeps only `first == 0`, so the scrolled window with
  two-line rows is covered only indirectly.
- **The shared-helper copy scan** — `crates/raul/src/data.rs:648` — covers
  **three of the theme crate's five** text helpers: `head_to` and `text_width`
  would pass it. It also scans **only `crates/raul`**, so a private copy in
  hugo would pass it. The same limitation the `View::draw` pin has.
- **The description pin** checks the phrase, not the attribution, and knows four
  connectives.
- **The footer** drops the hidden count at the widths where it would fit beside
  the keys — 17 and 18 columns on the Needs you tab.
- **The Settings announcement** names wholly-in-view groups where the Backlog
  names only the hidden ones, and a test comment names a help footer that does
  not exist.
- **The journal:** every fixture's approved stamp is thirty minutes before its
  claim, and no fixture holds an approved task that has waited long; a comment
  beside the activity panel inverts the order the panels give up their rows;
  the kinds are provisional.
- **hugo:** with an old `config.json` and a broken `config.toml`, the notice
  and the defaults line can be cut silently at a two- or three-row sidebar;
  two row-level tests still hand-write an error wording the pipeline never
  produces.
- **The Overview's activity block** names the files it cannot read but not how
  many (`crates/raul/src/overview.rs:600`, *"unreadable not shown: …"*) — the
  announcement rule says what is hidden is named **and counted**. This was
  filed under hugo and belongs to raul.
- **The plain render:** the duplicate-id loader test can be made less
  filesystem-sensitive with more pairs; some test fixtures leave temp folders;
  the flags are documented only in `--help`.
- **The store discovery's** Windows-only path tests are compile-checked but CI
  runs Ubuntu only.
- **The wake script** — in the global skill, not in this repository: bash
  `SECONDS` gives a timeout budget that is off by up to a second; a killed pane
  read can leave partial stdout; a non-JSON Herdr failure loses its message; a
  Herdr warning on stderr can break the merged JSON parse; the test's wall-clock
  windows can flake on a loaded CI runner.

## WHAT WAITS ON THE FLOW

- **The journal's writer** (grooming G2): the kinds are provisional and nothing
  writes `activity.json`.
- **The runtime record's writer:** nothing writes `runtime.json`, so the crew
  hugo shows is the mock's.
- **The orchestrator's job:** the orchestrator role has no job yet. hugo picks
  the model for the agent it will start; opening the herdr tab and starting it
  is its own unit, and it is the first step towards hugo driving the loop
  rather than only showing it. The crew's rules are already in place: three
  agents means one per role, two means the orchestrator reviews, one means the
  human reviews.

**Where the flow's files live** is settled, decided 2026-11-21 and recorded in
`docs/decisions/0011-flow-files.md`: the project's role configs move to
`coding-crew/flow/` when the project is managed, `.orchestration.json` stays at
the repository root because the global skill reads it there, the findings live
inside the task rather than in `/tmp`, and the task folder replaces this file as
the orchestrator's memory. The move waits for the writers. The third copy of
the model pair, in `mock-data/generate.py`, is still unpinned against the role
configs.

**Deferred by the human:** the light themes — our `light` preset and the light
variants of the families (Catppuccin Latte, Dracula Alucard, Nord Light).
**Out of scope:** talita's own verbs, until it is asked for explicitly.

---

## LESSONS THAT STILL BIND

- **Stage by name.** Never `git add -A`, never `git add .` — a blanket stage
  sweeps another agent's in-flight work into the commit. It happened twice.
- **Never close an agent tab while its wake command is in flight.** Wait for
  the wake to return, then close the tab by its **tab** id (`wX:t7`, never the
  pane id). A `tab_not_found` answer means the id was wrong, not that the tab
  is gone. A pane killed on purpose takes its wake with it — kill the pane,
  then kill the wake. **It happened twice in one session, on unit 46's delta
  reviewer and unit 47's CODE reviewer:** the report arrived, the
  orchestrator closed the tab, and the wake polled a pane that was gone until
  it hit its 1800-second bound and exited 3. No work was lost — the report
  had arrived — but the wake's exit was noise that said the round failed when
  it had not. **The practice, not the rule, is what has to change: the tab is
  closed by a wrapper that waits for the wake's own process to exit**, never
  by a hand-timed close. `/tmp/opencode/cl-flow/close-after-wake.sh <pane>
  <wake-pid>` waits and closes in one step.**
- **`/tmp` does not survive a server restart.** The round files, the snapshots
  and the gate logs go with it; the working tree is the real surface. The round
  files live under a project-prefixed path — `/tmp/opencode/cl-<unit>` — and the
  review brief names that path in its first line.
- **After every wake, read the pane and check the input box.** A prompt can
  arrive unsubmitted; herdr reports the pane idle while the text waits. Submit
  it with a bare Enter.
- **A gate failure that says "no space" is the disk, not the code.** The
  tmpfs holds 2 GB; prune the sweep's kept output as the session goes.
- **The builder is DeepSeek, the reviewer is GLM.** Different families, and
  that difference is what separates the lenses. The pair was tested across
  units 14–17.
