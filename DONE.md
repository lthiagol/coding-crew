# DONE — the archive

What this project has built, and how it got there. **Nothing here is a plan.**
The live state — what is open, what is next — lives in [`TODO.md`](TODO.md).

This file was split out of `TODO.txt` on 2026-11-21, when the state file
became a Markdown pair: `TODO.md` for the live state, `DONE.md` for the
archive. The record below is carried over unchanged.

---

## What exists now

**The project.** `coding-crew` — a task management tool for agents. Three
interfaces: **talita** (CLI, the agent's tool), **raul** (TUI for the work,
the human's tool), **hugo** (TUI for the flow, the human's tool). The
Autopilot tab is gone from raul. The packages are `coding-crew-core`,
`-engine`, `-raul`, `-talita` and `-theme`. The repository directory and
the GitHub remote keep their old names for now.

**The model** (`crates/core`). A task file with intent, scope, acceptance
criteria, steps, review rounds, findings, questions, notes, dependencies
and subtasks. Eight statuses, three overlays, the gates that move a task
between them, the roles and the session boundaries, evidence with derived
criterion status, smaller tasks with shaping and one-way promotion, and a
store of one file per work unit with atomic writes. A task projects to
Markdown through one pure, terminal-free interface.

**The engine** (`crates/engine`). leaf 1.29.2's markdown modules, vendored
with a pinned commit, a checksummed deviation ledger, a safe sync script
and the ported upstream tests.

**The theme** (`crates/theme`). Eighteen slots, five presets — dark, nord,
catppuccin, dracula, terminal — each from its own family page, with the
figures recomputed by test. Both TUIs paint with it, so one palette serves
the whole product.

**raul.** The window shell and five tabs — Overview, Needs you, Path,
Backlog, Settings. The Overview measures its text the way ratatui draws it,
so a count is never shown as if it were the whole count. The Needs you tab
measures the human wait and says what it left out. The Path tab draws the
tree of the live work. The Backlog tab keeps ideas, smaller tasks and
drafts in one derived list. The detail pane opens full screen from the row
tabs and renders the real projected document through the vendored engine,
painted by the theme bridge, with scroll keys. The projected document reads
as a history: each review round a heading with its own fields, and the
findings it raised under it.

**hugo.** One sidebar for the configuration, one main pane for what is
running, no tabs. The main pane leads with the crew — one row per agent
with its role, its model, what it holds and how long it has held it — then
the queue, the round, the budget and the findings. The configuration is
`~/.config/coding-crew/config.toml`, written on demand, reloaded with `r`,
and the orchestrator's model is picked from the harness catalogue in the
sidebar.

**The mock data** (`mock-data/generate.py`). Thirty tasks and five items, in
the shapes the interface has to read: tasks with zero, one, two, three, five
and six review rounds, open and resolved findings with resolutions and
comments, open and resolved questions, a parent with its subtasks, and five
done tasks carrying their history. The text is this project's own work. A test
walks every field of the model's seventeen structs, so a field nobody fills
fails the gate, and the folder's own README is pinned to the fixtures. The
fixtures are written only through the generator, which reproduces the
committed tree byte for byte.

**The flow.** The build loop is the global skill `orchestration-flow`, and
the two-axis review is the global skill `code-review`. The upstream skills —
`codebase-design`, `diagnosing-bugs`, `domain-modeling`,
`writing-for-agents` — live global at `~/.config/opencode/skills` with the
MIT licence and their provenance beside them. This repository carries none
of them.

**The docs.** The task model, the smaller tasks, the roles, the vocabulary,
raul's screen spec, hugo's screens and design record, the palette tables,
and eleven decision records.

---

## The record

Every closed unit, in the order it closed.

WHERE WE ARE
  Unit closed: raul window shell and Overview tab. Six build and review
  rounds, the sixth clean, committed by staging by name.
  Unit closed: the theme - the slots, the presets and the docs. Eight build
  rounds and eight review rounds; the first four rounds were real defects in
  the code (the agent lane was ANSI blue, the terminal preset told the phases
  apart by nothing, the human lane's reasons bypassed the mapping, the slots
  were looked up by name, the render tests ran on one tab, the block titles
  had no slot, the summary line lost the lane colours, the dark preset was
  recorded nowhere, a count could be silently cut at a width where the screen
  looked fine, the palette was not pinned to the table that records it, and a
  border colour was doing a word's job), and the last four were the two docs
  not agreeing with themselves or with each other. The unit ended with five
  tests that read the tables and the figures, so the drift is now the gate's
  problem rather than a reviewer's.
  Unit closed: the Path tab - the tree of the work. Two rounds, and the
  counts were pinned by tests from the start, so the review spent its round
  on the drawing - the geometry did not match the spec sketch, the next task
  could vanish unnamed on a short pane, and the width test could not fail
  because it measured the rendered buffer, which is the width wide by
  construction. All three are fixed, and the tree is the shape of
  docs/tui/screens.md. The lesson held: pin the numbers before the first
  review round, and check that the check can fail.
  Unit closed: the flow's role models. The builder runs DeepSeek V4.1 Flash
  with the high variant; the reviewer runs MiniMax M3 with thinking. The
  direct model IDs carry their variants on opencode 2.3.0, confirmed by live
  panes and by the model catalog.
  Unit closed: the Herdr wake signal. Four build and review rounds replaced
  herdr agent wait --until idle with scripts/herdr-wake.sh: it captures
  completion_seq before dispatch, sends one-line prompts, and waits on a hard
  command bound. The script has 91 assertions in the gate, and a live smoke
  test dispatched a builder and returned on completion.
  Unit closed: the Needs you tab - the human lane. Seven build and review
  rounds: the first five found and closed the wait, announcement, fixture,
  save-load, timezone, history, and paint defects; the human authorized two
  focused rounds for the last two low findings, and both closed. The tab
  lists approve, review, and escalated with the Overview's counts. The age
  measures the wait from status_since, with created as the documented
  lower-bound fallback. The not-shown announcement counts omitted tasks in
  a shape distinct from the header total, names every reason including zero,
  keeps hidden in a narrow pane, and paints by the theme. The fixtures carry
  status_since and their histories follow the model's phase order. The gate
  is green: 179 Rust tests and 91 script assertions; the generator reproduces
  the fixture tree byte for byte. The colour decision: docs/tui/theme.md is
  authoritative - approve and review use semantic.human, escalated uses
  semantic.error.
  Unit closed: raul --render --plain. Five build and review rounds closed the
  wide-grapheme column, filesystem ordering, duplicate-id tie-break, custom
  dimension coverage, and allocation bound. The command prints deterministic
  plain text without a terminal, takes bounded --width and --height options,
  exits 0, and prints the same bytes across runs and filesystems. The real
  Overview output is compared with crates/raul/tests/snapshots/overview.txt.
  The gate is green: 196 Rust tests and 91 script assertions.
  Unit closed: the lease fix in core. One clean build and review round.
  Claim::lease_expired now compares parsed RFC 3339 instants, so a lease
  written without seconds expires on the minute it says; offsets compare as
  instants, an empty lease never expires, a malformed nonempty lease expires,
  and an invalid now cannot establish expiry. The gate is green: 202 Rust
  tests and 91 script assertions.
  Unit closed: the store discovery in core. Four build and review rounds.
  Core owns the project-boundary rule recorded in ADR 0010: the nearest
  coding-crew folder stops discovery, even when work is absent, and an
  ancestor project's store is never borrowed. Raul calls the strict core
  rule and says no project and no work differently; talita stays untouched
  and keeps writing into the nearest project's work folder. The gate is
  green: 223 Rust tests and 91 script assertions.
  Unit closed: the Backlog tab. Two build and review rounds. Ideas, smaller
  tasks, and draft tasks share one list in derived groups; dismissed and
  promoted items stay visible as such; short panes announce hidden rows;
  fixed columns are never cut; and Unicode titles are cut on grapheme
  boundaries. The gate is green: 247 Rust tests and 91 script assertions.
  Unit closed: the markdown projection in core. Four build and review rounds.
  task_to_markdown is one pure interface with no terminal or clock; the
  document carries the ten detail-pane sections in order; fixed fields stay
  literal while free text keeps its markdown; reverse dependencies and child
  progress come from the context; and a committed snapshot pins the complete
  output. The title is a label and is escaped. The gate is green: 269 Rust
  tests and 91 script assertions.
  Unit closed: the vendored leaf engine. Five build and review rounds.
  crates/engine pins leaf 1.29.2 by full commit, carries all 18 markdown
  modules, collapses leaf's theme to the markdown palettes, ports the ten
  markdown test suites, and records every deviation and checksum in
  VENDORED.md. sync.sh checks the pinned upstream and refuses to erase
  semantic deviations silently. The gate is green: 468 Rust tests and 91
  script assertions.
  Unit closed: the coding-crew code-review skill. Three build and review
  rounds adapted mattpocock/skills' two-axis review for this project. A first
  review opens fresh CODE and SPEC reviewers in parallel; fix rounds use a
  delta review over prior findings and changed hunks; low findings are
  follow-ups rather than another round; reviewers run targeted probes while
  the orchestrator owns the full gate and round verdict. A round snapshot
  makes the delta executable. The isolated gate was green: 468 Rust tests and
  144 script assertions.
  Unit closed: the theme bridge. Three rounds. Our eighteen slots map onto
  leaf's MarkdownTheme: all 47 fields named once, the five background fields
  derived in the test from the engine's own sources, both presets, palette
  containment, and the doc's table pinned against the code. The one-palette
  promise stops at MarkdownTheme: syntect's code tokens stay a third palette
  the detail-pane unit must handle. The search is not offered in either
  preset and the doc says why for both; its 1.26:1 figure is pinned by test.
  The gate is green: 475 Rust tests and 144 script assertions.
  Next unit: the detail pane (plan 14-15).
  The delta review left one low follow-up: the search-figure assertion lives
  in only_the_engines_background_fields_take_a_slot_background, a test named
  for background fields; move it to the contrast-figures test or split it
  out.
  Unit closed: the detail pane: the shell. One clean round. enter on a row
  opens the task as a full-screen overlay - the spec's split-or-full fork is
  decided, full screen, and recorded in docs/tui/screens.md; esc returns to
  the tab and the selection it came from; enter reopens; it opens from Path,
  Needs you and Backlog and nowhere else. The header labels id and escaped
  title - core escape_literal is now pub, one escaping rule for the
  projection and the shell; the title sits on text.strong, the id on
  text.normal, and text.selected stays unused for unit 15.
  raul --render --plain --detail TASK-ID is bounded by --width and --height;
  a long title is cut on grapheme boundaries. The gate is green: 499 Rust
  tests and 144 script assertions.
  The review left three low follow-ups, none blocking:
    - needs.rs, path.rs and overview.rs still hold private cut_to/pad
      copies that cut on scalar boundaries; the tabs' own cut paths should
      move to the shared helper - closed by the theme crate unit, which
      retired the copies and made coding_crew_theme::text the one
      implementation
    - app.rs reads a fresh now per key press while needs::render computes
      its own: two instants in one frame; one now per frame, passed through
    - the render test's combining-title fixture replaces exact JSON bytes
      without asserting the replacement matched
  Unit closed: the detail pane: the document. Two rounds. The body is the
  real projection through the vendored engine, painted by the unit-13
  bridge - one palette; reverse dependencies and children progress come
  from the record's context. Scroll keys up/down j/k pgup/pgdn g/G with a
  clamped viewport and a footer that names only working keys, its
  degradation order stated by the comment and pinned by test at 68/55/40.
  text.selected got its first user: the engine mark on a ==key finding==
  in TSK13's note, the selection pair in the dark preset, Reset in the
  terminal one. The backslash question is answered on real data: the
  table parser consumes the projection's pipe escape before the code
  span, so no backslash reaches the render. The committed snapshot is the
  real TSK13 document, byte for byte. The code tokens keep syntect's
  base16-ocean.dark in both presets, recorded in docs/tui/theme.md. The
  gate is green: 513 Rust tests and 144 script assertions.
  The delta review left one low follow-up: the footer order pin does not
  assert that the single-row keys re-fit at 55 and 40 columns once the
  wider parts drop; one assertion closes it. And the unit's recorded
  follow-up stands: no fixture carries a fenced code block, so the
  syntect token paint is unpinned end to end.
  Next unit: the pty harness (plan 16).
  Unit closed: the pty harness. One clean round. scripts/pty-sweep.py runs
  raul --render --plain at every width 40-120 inclusive, one file per
  width, and a width whose output has a line longer than the width fails
  the command naming the width, the line and its measured length;
  python3 standard library only; --height passes through; the output dir
  defaults to a fresh temp dir reported and kept. make ci now sweeps into
  a temp dir, a step proven able to fail by a reverted mutation, and the
  Overview is pinned at 40, 60 and 100 columns. 74 script assertions cover
  the script with a faked raul. The gate is green: 512 Rust tests and 218
  script assertions.
  The review left low follow-ups, none blocking: the sweep and the Rust
  test hold two terminal-column measures that disagree on combining marks
  and Cf - decide the rules once, in one place; the sweep's extra failure
  modes (row count, nonzero exit, timeout) are built and tested though the
  done-when did not name them; and each gate run leaves a kept 81-file
  dir in /tmp that nothing prunes.
  Next unit: the Settings tab (plan 17).
  Unit closed: the Settings tab. One clean round. The tab is a named empty
  state - "nothing to set yet" over "will hold keybinds, sort keys, theme"
  - on text.dim with the theme border; no rows, so enter opens nothing and
  the footer names no open or scroll key; tab switch and highlight behave
  like the other tabs. Render mode gained --tab NAME (overview,
  needs-you, path, backlog, autopilot, settings), refused outside render
  mode and on unknown names, the reachability mechanism the unit pinned
  at 40/60/100. The gate is green: 532 Rust tests and 218 script
  assertions. Only the Autopilot tab is a stub now.
  The review left low follow-ups, none blocking: --tab composes with
  --detail without a guard - the pane covers the window, so --tab is moot
  and the state is one the keys cannot reach; one refusal or doc sentence
  and a test pin it; and the dim-paint test locates a cell by byte offset
  read as a column - move it to the per-cell draw_cells pattern.
  The human authorized the sequence to run without check-ins between
  units: after Nord, Catppuccin and Dracula, the row-tab scrolling unit,
  then the mock-data dependency graph unit, then the batched low
  follow-ups. The orchestrator stops for the human only on a product
  decision inside a round, a BLOCKED builder, a round-5 changes-needed,
  and the Autopilot design (G1), which is groomed with the human before
  anything is built.
  Unit closed: the selection highlight. One round. The selected row of
  Needs you, Path and Backlog is painted with the text.selected slot -
  the documented slot, reversed video, bar to the pane edge - and the
  cursor follows the keys and lands where enter opens; esc restores it.
  The builder found the movement keys did not exist and built up/down and
  j/k on the row tabs, clamped and id-keyed, so paint, clamp and
  enter-open agree even where the layout hides a row. The gate is green:
  532 Rust tests and 218 script assertions.
  The review left one blocking finding the human deferred consciously:
  pgup/pgdn and g/G are dead on the row tabs while screens.md names them
  and the detail pane has all six - they wait for the row-tab scrolling
  unit, which owns the scrolling, those keys, the cursor stepping onto
  announced-hidden rows on short panes, and naming the movement keys in
  the footer and the help overlay. Two more lows stand recorded: the
  two-instants follow-up gains a third caller in rows_of.
  Unit closed: the theme catalog: Nord. Two rounds. The preset comes from
  Nord own page - every value fetched live and checked by both reviewers -
  its eighteen slots mapped per the palettes.md rules, its derivations
  written down, its contrast figures recomputed by test, and the render
  tests run it on every tab at 40/60/100. It joins Preset::ALL, so every
  preset test covers it, and --theme gained the name - no new flag. The
  gate is green: 537 Rust tests and 218 script assertions.
  The first round found one blocking defect - theme.md still described a
  two-preset world, so Nord had no entry and the search figure the
  recompute test hardcoded was Dark alone - and both axes converged on
  two lows: the tab-local preset tests still hardcoded dark-vs-terminal,
  and the Nord section recorded fewer values than the family page lists.
  The delta round closed all three with probes. Two one-line drift
  findings came out of it - theme.md still lists Nord under Not yet, and a
  detail.rs comment still says both of raul presets - and the orchestrator
  ruled them required fixes in the next unit, which edits the same docs.
  Two operational lessons from a server restart: /tmp does not survive it,
  so a round snapshot and the gate log go with it - both are cheap to
  rebuild; and the wake command cannot see a prompt sitting unsubmitted in
  opencode input, because herdr reports idle while the text waits - the
  recovery is a bare Enter through herdr pane send-text, and every dispatch
  now reads the pane afterwards to check.
  Next unit: the theme catalog: Catppuccin (plan 9), with those two
  one-line fixes carried in.
  Unit closed: the theme catalog: Catppuccin mocha. One clean round. The
  preset comes from catppuccin.com/palette - all 26 mocha values fetched
  live by the builder and by both reviewers - its eighteen slots mapped
  per the palettes.md rules, its derivations written down, its contrast
  figures recomputed by test, and the render tests run it on every tab at
  40/60/100. It joins Preset::ALL; the search pairing recomputes to
  1.40:1; the terminal preset is untouched; both carried fixes from the
  Nord unit landed and are pinned by new tests. The gate is green: 544
  Rust tests and 218 script assertions.
  Both axes converged on one low: the count the slots spend - fifteen of
  the twenty-six - is fourteen, in docs/tui/palettes.md and two code
  comments, and no test recomputes it. Ruled a required fix in the next
  unit, which edits the same files, with a test that recomputes the
  count.
  Next unit: the theme catalog: Dracula (plan 10), with that fix carried
  in.
  Unit closed: the theme catalog: Dracula. Two rounds. The preset comes
  from draculatheme.com/spec - twelve colour-palette and sixteen ANSI
  values fetched live by the builder and by both reviewers - its eighteen
  slots mapped per the palettes.md rules, its figures recomputed by test,
  its search pairing recomputed to 2.26:1, and the render tests run it on
  every tab at 40/60/100. It joins Preset::ALL; the terminal preset is
  untouched; alucard is unbuilt and recorded as deferred. The gate is
  green: 557 Rust tests and 218 script assertions.
  The first round found one class of defect - a number or a value in
  prose that no test read - and the orchestrator opened a short fix round
  for it, because the class had already appeared in the Catppuccin unit.
  The fix made the count guard fail-closed and general: every count
  sentence in the family sections is matched to a recomputed claim, a
  count without one fails the gate, a value not on the family page fails,
  and the page lists the functional colours and the current-line fallback
  too. The delta round closed all three findings with probes.
  Two lows stand recorded: the count guard recognises a count only through
  a fixed noun vocabulary, so a count phrased outside it escapes
  silently - probe-confirmed - and the ANSI-spent derivation is duplicated
  verbatim in two tests.
  The theme catalog is done: five presets - dark, nord, catppuccin,
  dracula, terminal - each from its own family page, each with eighteen
  slots painted and figures recomputed by test. The light flavours and
  alucard wait with the light theme.
  Unit closed: the upstream skills move global. One clean round. The four
  upstream skills - codebase-design, diagnosing-bugs, domain-modeling,
  writing-for-agents - live whole at ~/.config/opencode/skills, every
  file beside its SKILL.md, verified byte-identical to the repository
  copies; the MIT licence sits beside them, and the provenance is a few
  lines - the upstream repository, the pinned commit 5beb35b, its date,
  and each skills upstream path - plus the rule that they are never
  edited in place. The repository no longer carries .agents/skills at all;
  the sync machinery and its test are gone, and make ci no longer runs
  them, so CI cannot break on a script that only refreshed copies.
  The review left low follow-ups, none blocking: the DONE summary still
  named the deleted tree, and ADR 0009s rewritten sentence was
  self-referential - both fixed.
  Unit closed: the flow scripts move into the skill. Two rounds. The flow
  skill now carries its own tools beside SKILL.md - herdr-wake.sh,
  herdr-close-pane.sh, review-snapshot.sh, with the wake and snapshot
  suites - so the skill is self-contained and every path in its text is
  absolute. The repository keeps only pty-sweep.py and its test, which
  test rauls rendering and stay in the product gate.
  The first round found the suites had left the gate entirely: nothing
  automatic ran the wake suite any more, so a broken wake script would
  have passed - the skip clause protected nothing. The fix put test-flow
  inside check, and the delta review proved both branches: the gate skips
  with exit 0 without the global folder, and fails when a suite fails.
  The move also caught the orchestrator twice: the snapshot command and
  both reviewer dispatches died with exit 127 because the scripts were no
  longer in the repository. That is the failure the move should cause -
  loud, not silent.
  One low stands recorded: the tab closer moved with no suite of its own,
  the humans pruning decision.
  Unit closed: the row-tab scrolling. Two rounds. The three row tabs -
  Needs you, Path and Backlog - scroll their rows: the viewport follows
  the cursor, pgup and pgdn move it by the rows the pane shows, g and G
  reach the first and last row, and the content pane row of screens.md
  is now true. The cursor is never on a row the pane does not draw, and
  the footer and the help name exactly the keys that work. The gate is
  green: 571 Rust tests and 218 script assertions.
  The first round found two blocking defects, both the projects own
  classes: Paths announcement undercounted after scrolling - it named the
  rows it cut but not the rows it scrolled past, so the summary could say
  RUNNING 6 over four rows - and the cursor could sit on a row the pane
  could not draw, because a blocked groups first row is a two-line chunk
  and the window ignored a rows height. The fix round closed both, with
  the reviewer's own probes: after G at content height 7 the announcement
  reads 14 and 14 of 18 rows are hidden, and at content height 2 walking
  Up through every Path row keeps the cursor drawn and painted.
  Three follow-ups stand, none blocking: the triplicated render_view
  orchestration across the three tabs is an extraction unit of its own;
  the help popup clips the movement keys, q and ? on windows of 14 rows
  or fewer, which is a design decision; and view.rs keeps one clause
  about the last window that draws a row, which does not hold at content
  height 1.
  Unit closed: the mock-data dependency graph. Two rounds. Every task past
  approved now waits only on done work - executing, code-review,
  remediating, needs-human and done - so the fixtures tell a history the
  model allows: the claim gate says a task starts when its dependencies
  are satisfied, and the graph is closed under waits-on and acyclic. The
  rule is enforced three ways: a test reads the real fixture tree, the
  generator self-checks, and the generators byte-for-byte reproduction is
  now in the gate, closing a long-recorded gap. The Path counts are
  unchanged - trunk 1, AWAITING APPROVAL 2, BLOCKED 3, RUNNING 6, REVIEW 3,
  ESCALATED 3 - and so are the Needs you counts.
  The first round found two blocking defects, both the same class: the
  structural fix moved the contradiction from structure into time, since
  every claim predated the completion of its dependency, and the two
  documents that describe the graph still said it runs backwards only.
  The fix round corrected both, and the delta review confirmed the
  chronology over the committed tree.
  One low was fixed at the close: a README sentence made a universal
  claim that four fixtures contradicted.
  Unit closed: the small follow-ups. Three rounds. Four recorded lows
  closed: the --tab with --detail combination is refused before anything
  renders, with a message naming both flags; the help popup fits the
  window and keeps q and ? at heights 14 and 10 and at the narrow widths
  where lines wrap; the view.rs clause names the case that renders; and
  one instant now serves a whole frame - the Needs, Path and Overview
  paths all take the frames now, the fresh-clock wrappers are gone, and a
  guard test scans the source tree so no other module can read the clock
  again. The gate is green: 583 Rust tests and 197 script assertions.
  The unit opened as four unrelated small lows and the reviews turned it
  into the discovery that the one-instant guarantee was half implemented
  across three tabs - a frame could answer from four instants and draw a
  claimable count beside a next task the counts instant says is held,
  two numbers on one screen disagreeing. Both the guard and the boundary
  pins were probed: a rogue clock-reading module dropped into src fails
  the guard, and a fresh clock fails the expired side by design.
  Two lows carry into the next unit: the drawn-order id assembly exists in
  three copies in path.rs, and the guard comment says the crate when the
  scan covers src only.
  Unit closed: the render_view extraction. Two rounds. The three copies of
  the ids-to-cursor-to-settle-to-window block live once in View::draw and
  the three row tabs call it; the dead all-fits line is gone; the
  drawn-order id assembly in path.rs is one, Tree::row_ids, and the test
  that asserts no history reads it too. Nothing moved: the review built
  HEAD in a throwaway worktree and byte-compared 21 render outputs - three
  tabs, widths 40/60/100, heights 3 to 40 - and every one was identical.
  That probe is the method for a refactor here: the before-and-after
  comparison, not the green gate.
  The first round found the third copy still walking the spine by hand -
  only two of the three were unified - and the second closed it with two
  probes: a panic planted in Tree::row_ids fails the test, and a widened
  condition catches TSK10, so the assertion is not vacuous.
  One low stands recorded: the sharing pin is a text scan, so a copy that
  inlines settles walk without calling it would evade the guard.
  Unit closed: the terminal column measures. One clean round. The rules
  for how many columns a line takes are decided once, in
  scripts/terminal-columns.txt - a combining mark, a format character and a
  control character take no column, a wide or fullwidth character takes
  two, a tab advances to the next stop from the running column - and both
  the sweep and the Rust check read that one file, so neither can drift.
  The table is the rule, not an example list: the ranges unroll the prose
  rules over every codepoint. Tests prove the two agree by running the
  sweeps check-table as a subprocess and comparing every case, with a
  guard against a vacuous pass, and the gate still catches an overflow.
  Three lows stand recorded: the measure is per codepoint, so a joined
  cluster counts six where a terminal draws two, and the repo already cuts
  titles on cluster boundaries - the fix is to measure per cluster; conjoining
  Hangul jamo default to one column where unicode-width gives none; and
  the header claim that the ranges were generated from the rules is prose
  no gate reads, so put the regeneration probe in the gate.
  Unit closed: the rename to coding-crew. Two rounds. The project is
  coding-crew: the data folder is coding-crew/, the packages are
  coding-crew-core, -engine, -raul and -talita, the mock tree is at
  mock-data/coding-crew/work/, and the configuration home is
  ~/.config/coding-crew/. The repository directory and the GitHub remote
  keep their names for now. ADR 0001 is overwritten with the new names
  reasoning - a governed crew with an interface for the crews governance -
  and it settles the residue: the lanes are the tasks own structure, the
  crew is who does the work, so the interfaces expansions keep Lane while
  the project stopped being that word. A guard test walks the working tree
  and fails on either spelling of the old name, so a half-done rename
  fails the gate; only the sanctioned remote URL is allowed, and TODO.txt
  keeps the old stem in its history by choice, recording the truncation
  bug the old name produced.
  The first round found one blocking defect in the rename itself: the
  narrow-title pin had been renamed but its guard still looked for the old
  names stem, so the branch could never fire and a cut of the new name
  passed - a check that cannot fail, the projects own defect class. The
  fix round closed it and three more, all probe-confirmed.
  Two orchestrator mistakes of my own, recorded in HAPPENED: a commit that
  swept the builders staged work, and a staging that left talita, the mock
  tree and the sweep scripts out of the unit commit - the commit was
  amended to match the reviewed state.
  Unit closed: the shared theme crate. Two rounds. The slots, the presets
  and the palettes live in crates/theme - package coding-crew-theme - and
  raul paints with it; the measuring and cutting helpers moved with them,
  so there is one implementation of each, and the three tabs cut titles on
  the grapheme boundary as the detail pane already did. The renders are
  byte-identical for the ASCII fixtures - the review proved it twice,
  sixty then fifty-four outputs - and the one behaviour that changed on
  purpose, a combining mark in a title, was proven live against a build of
  the previous commit: the old build left a bare e, the new one does not.
  The first round found two blockers, both invisible in a green gate: a
  raul domain type - the Reason enum, the work-lane vocabulary - had
  moved into the paint crates public API, where hugo would have inherited
  it, and it now lives in crates/core beside Status with the theme mapping
  it as it maps phases; and the four private cut_to and pad copies stayed
  behind, so one implementation of each was still unmet.
  Two pins came with it: a test that fails if any module but the shared
  crate defines cut_to or pad - it scans the source, so even an
  uncompiled copy is caught - and a test that reads AGENTS.md and checks
  the crates it lists against the workspace members, failing in either
  direction. One low stands recorded: path.rs still breaks a long word on
  a scalar boundary, so a combining mark in a one-word title splits
  mid-grapheme there; pre-existing, not a regression.
  Unit closed: the interface expansions. Two rounds. The three interfaces
  carry the human's expansions: talita, Task Administration, Lifecycle and
  Interaction Terminal Application; raul, Review, Assess, Understand and
  Learn; hugo, Human-guided Unified Governance and Orchestration. They read
  as three jobs - the tool the agent drives, the tool the human reads to
  learn the state, the tool the human uses to govern - and the acronyms
  still spell the names exactly. ADR 0001 argues each one, why the lanes
  stay in the model while the name and the expansions no longer spell
  that word, and why Learn was chosen over Lead, which would have read as
  authority over the work.
  The first round found the guards themselves hollow, and found them by
  planting defects: the talita doc-comment pin could not fail because the
  test read its own source, which holds the constant it looked for; the
  root README and both package descriptions were read by no test; the
  check tested that a file contains the expansion somewhere rather than
  that every place spelling it spells it right; and the guards lived in
  crates/talita, the crate the rules keep off-limits. They now sit in
  crates/core/tests beside the project-name guard, share one tree walk,
  read the doc comment and both manifests as separate files, and fail when
  a divergent spelling stands beside a correct one.
  Two lows stand recorded: the per-place trigger still wants a capitalised
  lead word, so a lowercase divergent spelling passes; and the guard does
  not pin the interfaces descriptions, only their expansions - rauls own
  screen spec opened The TUI of coding-crew until the orchestrator fixed
  it at the close.
  Next unit: hugo - the TUI for the flow (plan 31) - but not yet: the
  human wants to discuss how it looks before any of it is written, in a
  later session. The shape is settled - one sidebar for the
  configuration, one main pane for the running state, no tabs, the
  configuration under ~/.config/coding-crew/ created on demand - and what
  is open is the visual and what the main pane reads first: the live build
  loops crew, or the projects automation. It waits as question Q5 in
  docs/plan.md, and the shape is recorded in docs/hugo/design-record.md.
  The batched low follow-ups that remain, none blocking: the measure is
  per codepoint, so a joined cluster counts six where a terminal draws
  two; conjoining Hangul jamo default to one column where unicode-width
  gives none; the tables ranges generation claim is prose no gate reads;
  the sharing pin is a text scan; path.rs still breaks a long word on a
  scalar boundary, so a combining mark in a one-word title splits
  mid-grapheme there; and the expansions guard wants a capitalised lead
  word and does not pin the interfaces descriptions.  The human changed the directives: the Autopilot tab is dropped from raul
  and a third interface appears - hugo, Human-guided Unified Governance and
  Orchestration - a TUI for the flow alone. The seam is the data: raul
  reads the work folder, hugo reads the runtime; raul draws task content,
  hugo draws tasks only as items of a queue. The shape is settled: one
  sidebar for configuration - the agents, the models, the harness, the
  review-round limit, the order - and one main pane for the running state,
  with no tabs; the configuration is centralized under ~/.config/coding-crew/.
  The theme is shared through crates/theme, so both TUIs paint with one
  palette. The work order: directives, then drop the Autopilot tab from
  raul, then the shared theme crate, then hugo. The three variants that
  were compared for the tab are now docs/hugo/design-record.md - their
  content model carried over, their vertical layout did not.
  Unit closed: drop the Autopilot tab from raul. One clean round. raul has
  five tabs - Overview, Needs you, Path, Backlog, Settings - the Autopilot
  variant, its placeholder and its label are gone, the keys 1 to 5 reach the
  five and 6 does nothing, the footer and the help say 1-5, and
  docs/tui/screens.md shows five tabs and points the automations questions
  at docs/hugo/design-record.md. The matches in draw and rows_of are now
  exhaustive, so a future sixth tab fails compilation rather than
  rendering a stub. The doc and the code agree by test, not by review: a
  test reads screens.md with include_str! and pins the sketch, the tab
  table, the key range and the --tab names.
  One low is carried into the next unit that touches crates/raul: the
  hugo-pointer assertion matches the links text, not its target, so it
  fails on the wrong thing.
  Unit closed: the theme-side follow-ups. One clean round. TSK13s note
  carries a fenced rust block, written only through generate.py, so the
  engines code-token paint is pinned end to end: the pane is asserted to
  pair the doc-named base16-ocean.dark by key, and its rendered spans are
  compared per character against an independent syntect run of the same
  theme - so a pane that painted the tokens from our palette, or stopped
  highlighting, or paired another theme, fails. The pairing is
  preset-independent by construction, so the same in every preset is true
  rather than asserted. The search-figure recomputation moved into its own
  named test, verbatim, with every assertion kept.
  One low stands recorded: the theme docs cannot-avoid rationale - an
  empty syntax set panics, a scope-less theme paints black - is prose no
  test reads; the pins prove the pairing, not the unavoidability.
  Next unit: the Autopilot design - what the automation is, which is the
  last piece of work that needs the human rather than a builder.
  Unit closed: the global flow. Three rounds. The flow skill and the
  review skill now live at ~/.config/opencode/skills, loaded by their IDs,
  and the repository points at them; the skill says where it lives, that
  the scripts stay in the project, how the project prefix names every tab
  and agent, and how the session model question works - asked once at the
  start of a session, with the projects configured pair recommended, and
  applied through a generated copy of the role config written outside the
  repository. The project carries .orchestration.json: the prefix cl and
  the paths of its role configs.
  Three rounds, and every round found something real: the move broke
  sync.sh and left the folder index pointing at ghosts; the sharing broke
  the teardown rules, so an orchestrator following them would have closed
  another projects live tab; and the global provenance promised a drift
  report that no longer comes. None of them was visible to the gate.
  The human decided that code-review leaves the adapted list - the version
  we have is enough and updates are a problem for later - so sync.sh now
  manages only the four vendored skills, its adapted mechanism is generic
  and per-entry, and an empty list does not fail it.
  The review left two low follow-ups, none blocking: one stale sentence
  in the global skill still said the orchestrators own workspace, and the
  global ledgers licence link was dead - the MIT text now travels with the
  global copy. The human asked whether sync.sh should go: it only refreshes
  the four vendored skills from upstream, and the gate never checked that
  they match upstream - it tested the script against a fixture. The answer
  is open: keep it if an upstream bump is ever expected, otherwise remove
  the script and its test and keep LICENSE and UPSTREAM.md as the
  provenance.
  Unit closed: the tab closer. Two rounds. scripts/herdr-close-pane.sh
  takes the pane id the orchestrator holds, resolves the tab id from herdr
  pane get, closes it, and proves the close by the tab list; --audit
  lists a workspaces tabs with their agent state so a leak surfaces.
  Every herdr call is bounded with timeout --signal=KILL in the house
  style of herdr-wake.sh, and only stdout is parsed so a herdr warning
  cannot break it. 87 script assertions with a fake herdr, wired into the
  gate. The first round found the calls unbounded and the parse merged
  with stderr; the delta round closed both, with live probes against an
  independent fake. One low stands recorded: a timed-out close exits 4
  with no listing, though the close fate is unknown, so the orchestrator
  must audit to learn whether the tab closed.
  The model experiment concluded: the tested pair is now the default. The
  flow configs in flow/ name builder opencode-go/deepseek-v4.1-flash#high
  and reviewer opencode-go/glm-5.3#high - the same strings the temp test
  configs carried, and the flow files are the only configs that matter
  now; /tmp/opencode/flow-glm/ is no longer needed. The test covered the
  theme bridge's round 3 and units 14, 15, 16 and 17: every unit closed
  clean, the document unit spent one fix round (2 of the 5 budget), the
  rest were one-round units, and the reviewers converged independently on
  the same finding twice (the detail document round 1, the Settings
  round 1). The different-family rule holds: the pair is DeepSeek against
  GLM. Two orchestrator stumbles recorded in HAPPENED: the region-blocked
  DeepSeek pane needed the console privacy region set to Global, and a
  wake command ran to its bound after its tab was closed - never close an
  agent tab while its wake command is in flight.
  Follow-ups the Path tab left, all low, none blocking:
    - the blocked row names every dependency instead of only the ones after
      the fork parent
    - the claim-held-elsewhere fixture leans on an empty lease meaning never
      expires, which nothing in the model says
    - the overlay and the claim fixtures are not isolated - one task is held
      by more than one thing
    - a one-row pane cuts the announcement and leaves a count unnamed
    - the tree does not scroll yet, which the spec says it does
  The working tree is the review surface while a unit is in flight. On a clean
  verdict, stage by name and commit. Never use git add -A or git add .: a
  blanket stage sweeps another agent's in-flight work into the commit. That
  happened twice.
  Unit closed: hugo's visual - thirty proposals. The human wanted to see hugo
  drawn before any of it is built, so three families of model each drew ten
  candidate interfaces under docs/hugo/tui-proposals/, one directory per
  family so the path carries the attribution: deepseek
  (opencode-go/deepseek-v4.1-flash#high), glm (opencode-go/glm-5.3-flash#high)
  and minimax (minimax-coding-plan/MiniMax-M3.1-Flash-tuned). Each page is a
  self-contained HTML sketch on the project's own dark palette - no
  stylesheet, no script, no network - and each family wrote its own index.
  The combined index groups the thirty into the eight readings the set
  contains: the order leads, the crew leads, the findings lead, the queue
  leads, the round leads, the budget leads, the stamp leads, and the flow
  itself is the subject - that last one only glm reached. Five readings
  converge across all three families, which is the evidence that those are
  the obvious ones.
  The families diverged in sensibility, which is why three were run and not
  one: deepseek drew a ledger, glm drew the provenance - every running fact
  citing the order step that produced it - and minimax drew the form, the
  sidebar as a form to fill and the main pane as only what the form is
  doing. Three families is the right number for a generative artefact: one
  family shares one set of priors, and the disagreement between families is
  the signal.
  The review round found seven real defects and eleven lows; the human chose
  not to have them fixed, so the index carries a known-issues block that
  names them - glm's lanes are colour-inverted on all eleven pages, minimax
  paints counts twice and draws a review round the record contradicts,
  deepseek's pages carry no byline, and the count of live tasks varies
  between sixteen and nineteen across the set, both computable: sixteen is
  the automation's queue, nineteen adds the three groomed. The fixtures
  themselves check out - 21 open findings at 8 high, 8 medium and 5 low, 18
  external rounds spent of a 45-round budget across the nine live tasks
  with rounds, TSK29 the one claimable task, and every id-to-status pair
  drawn on any page.
  Two orchestrator notes. The wake's 1800-second bound was too short for a
  turn that draws thirty pages - it hit the bound while the agent was still
  working, and the work was on disk and correct while only the wait was
  short; the two later families ran with 2400. And a unit whose artefact is
  pages does not need the gate to say anything about them: the gate is
  green because the unit adds only new HTML under docs/, so the honesty of
  the sketches was checked by hand against the fixtures instead.
  Next: the human picks a reading, and the pick becomes plan 31 - the hugo
  shell. The open question stays Q5: the shape is settled, and what the main
  pane reads first - the live build loop's crew, or the project's automation -
  is still undecided.
  Unit closed: the batched lows (plan 34). Two build and review rounds. The
  column measure counts grapheme clusters, not codepoints, in the sweep and in
  the Rust check alike, and the two agree; a conjoining Hangul jamo takes one
  column and the rule says so; the table's rules are the table's parsed
  source - scripts/generate-terminal-columns.py parses rule lines out of
  scripts/terminal-columns.txt and derives the ranges, so a rule edited in one
  place only fails the gate; path.rs cuts a long word on the grapheme
  boundary through the theme crate's helpers, so there is one implementation
  of each; the render_view sharing pin holds by construction - View's fields
  and settle are private, so a planted copy in a tab module does not build -
  and the three row tabs are pinned to draw through View::draw; the
  expansions guard accepts a lowercase lead word and pins the descriptions
  by scanning the whole working tree, so a divergent description in a file
  nobody listed fails. The gate is green: 611 Rust tests and 212 script
  assertions.
  The first round found one blocking defect of the project's own class - the
  header's rules were a second, unguarded encoding of the rules the script
  enforces, so a rule edited in prose alone passed the gate green - and the
  axes converged on it independently. The fix moved the rules into the table
  as parsed rule lines, so the prose is the source rather than a copy of it.
  The second round closed all four carried findings with probes. Two lows
  stand recorded: the shared-helper copy scan covers three of the five
  helpers, so a private copy of head_to or text_width passes it; and the
  description pin checks the phrase, not the attribution, and knows four
  connectives.
  Next unit: the announcements (plan 35).
  Unit closed: the announcements (plan 35). Two build and review rounds.
  One promise, three tabs: a hidden row is never a bare count. The Path
  tab names the rows it scrolled past as well as the rows it cut, and a
  partly shown group is never called wholly hidden; a blocked row names
  every dependency it waits on, the fork parent included, and still says
  which one it is forked under; a narrow Backlog pane names the groups it
  hides, a dropped group header is not announced as zero hidden, and
  zero-width content is dropped with a word; the Needs you reason survives
  every width the tab renders at - a name that does not fit gives the row
  to the marker, never to a bare count. The overlay case and the
  claim-held-elsewhere case are separate fixtures now, and the claim
  fixture leans on a lease with a real expiry instead of an empty one; the
  Path and Needs you counts did not move. The gate is green: 624 Rust tests
  and 212 script assertions.
  The first round found a hole the unit was opened to close: at content
  widths of 3 or less the Needs you announcement was a bare count - a
  regression on HEAD, which at least kept the word hidden - and the
  one-row Backlog pane hid nine rows with no word at all, because a real
  pane always has a cursor and the cursor's row is never left out. That
  one was a product decision inside the round, so it went to the human: the
  footer carries the hidden count beside the keys when the content pane has
  no room, the cursor's row stays drawn, and the same fact is never painted
  twice. The ruling is in docs/tui/screens.md.
  One low stands recorded: the footer drops the hidden count at the widths
  where it would fit beside the keys - 17 and 18 columns on the Needs you
  tab - because the count part carries a fixed three-column gap.
  Next unit: the mocked journal (plan 36).
  Unit closed: the mocked journal (plan 36). One clean build and review
  round. The journal lives at coding-crew/activity.json - beside the work
  folder, not inside it, because the work folder holds one file per work
  unit - and the path is derived from the store's work folder, not by a new
  discovery rule. Nine event kinds, each event carrying at, kind, task, by
  and an optional note, defined provisionally in docs/tasks/task-model.md
  11.1: the kinds add no state the lifecycle does not have, and they are
  marked provisional because nothing writes the file yet. The mock is 134
  events written only through mock-data/generate.py, following the
  fixtures' own chronology - no event before the task it names exists, and
  every claim after the dependency's completion - and the generator still
  reproduces the tree byte for byte. The Overview draws the block: the last
  five events newest last, one line each, how many it left out, and "no
  activity yet" alone when there is no journal. The block is a pure
  function of the record, re-read every redraw.
  Four lows stand recorded: one event with an unknown kind or a missing
  field makes the whole journal unreadable, and the kinds are provisional,
  so writer version-skew blanks the history until the reader catches up; a
  present but unparseable journal makes the block say "no activity yet"
  while the errors panel names the file; the mock's approved stamps are
  minutes before the claim and no fixture holds them; and a comment beside
  the activity panel inverts the order the panels give up their rows.
  Next unit: Settings has content (plan 37).
  Unit closed: Settings has content (plan 37). Two build and review rounds.
  The tab is a list of rows with a cursor, in three derived groups - theme,
  keys, sort - so it scrolls and announces like the row tabs, and the
  cursor is never on a row the pane does not draw. It reads: enter opens
  nothing, so the footer and the help name no open key, and the placeholder
  is gone. The theme group lists every preset from the theme crate's
  Preset::ALL and marks the one in use exactly once; the keys group is
  built from the same table the input handling reads, so a key handled but
  not listed fails the gate; the sort group names in words the order each
  tab uses, from the tab's own sort. The gate is green: 680 Rust tests and
  212 script assertions.
  The first round found a behaviour change the unit never asked for: the
  refactor to a key table had quietly dropped esc as a quit key in the
  window - it quit on HEAD - and a new test pinned the removal, so the
  change was deliberate but undocumented. The orchestrator ruled that esc
  quits in the window, as it did before, and docs/tui/screens.md says so.
  The second blocking finding was the project own class: the short-pane
  announcement was a bare count, "not shown: 27", where the unit before it
  had ruled that a hidden row is never a bare count.
  Two lows stand recorded: a test comment names a help footer that does not
  exist, and the Settings announcement names wholly-in-view groups where
  the Backlog tab names only the hidden ones.
  Next: raul is built. The plan's remaining row is 31, hugo - the TUI for
  the flow - which waits on the human's pick of one of the eight readings
  in docs/hugo/tui-proposals/index.html.
  The human picked the reading: the crew leads. The pick is recorded in
  docs/hugo/design-record.md, and the plan's row 31 is split into three
  units - 31 the shell, 38 the runtime record and the crew, 39 the
  configuration and the order.
  Unit closed: hugo's shell (plan 31). One clean build and review round.
  crates/hugo is the crate - package coding-crew-hugo, binary hugo - and it
  depends on the theme and core crates and not on the engine. The window is
  one sidebar and one main pane, no tabs and no tab bar; below 80 columns
  the sidebar is dropped whole before the main pane is squeezed, and the
  drop is announced in a dim line that names what is not shown. hugo
  --render --plain prints the screen as plain text, bounded and
  deterministic, and the screens are pinned byte for byte at 40, 60 and 100
  columns. The configuration home ~/.config/coding-crew/ is created on
  demand, and a test proves it with a temporary home. The empty states are
  named - "no configuration yet" and "nothing is running" - on the theme's
  dim slot. The gate is green: 720 Rust tests and 212 script assertions.
  Three lows stand recorded: hugo does not treat ctrl-c as quit the way
  raul does, so a user carrying the habit is trapped in the alternate
  screen; the plain-render plumbing - plain_text, setup and restore - is
  copied verbatim from raul and can drift, and the project has a rule
  against two copies of one thing; and the core dependency is declared but
  unused until the crew unit lands.
  Unit closed: hugo's runtime record and the crew (plan 38). Two build and
  review rounds. The crew is runtime state, so it comes from a runtime
  record the flow would write - coding-crew/runtime.json, beside the
  journal, its types in crates/core, documented as provisional because
  nothing writes it yet. The main pane leads with the crew, the reading the
  human picked: one row per agent with its role, its model, what it holds
  and how long it has held it. The crew's size is a composition and not a
  number - three agents means one per role, two means the orchestrator
  reviews, one means the human reviews, because an agent cannot review
  itself - and a crew the rule does not cover says what it is. A held task
  belongs to the crew and never to the queue; the queue carries an id and a
  status and no content, which belongs to raul. Then the round, the budget
  and the findings, and the stamp at the foot, which says whether the
  automation runs and why. The mock is written only through
  mock-data/generate.py and every number in it is computable from the
  fixtures. The gate is green: 764 Rust tests and 212 script assertions.
  The first round found three blocking defects, and one of them was a
  truthfulness defect: a record that cannot be read read as "nothing is
  running", so a writer bug that corrupted the record would have shown the
  governance pane the one thing it exists to hide. Absent and unreadable
  are different states now. The second was a duration that was not one -
  "since 09:00Z" for a hold ten days old reads as today - so the row paints
  the length, from one instant per frame, with the render mode on a fixed
  instant so the pinned screens do not move with the wall clock. The third
  was an announcement that named the hidden sets without saying how much
  was hidden.
  Two lows stand recorded: the age units now exist twice, re-implemented
  beside raul's own, and nothing pins the two vocabularies together; and
  the plain-render plumbing is still copied from raul.
  Next unit: hugo's configuration and the order (plan 39).
  Unit closed: hugo's configuration and the order (plan 39). One clean build
  and review round. The configuration is a provisional file in the tool's
  home, ~/.config/coding-crew/config.json, and hugo writes the defaults when
  it is absent - the same home it creates on demand. The defaults are the
  project's own flow: the agents one per role with the models the project
  runs, pinned against flow/builder.json and flow/reviewer.json; the
  harness; the review-round limit of five; and the order - the steps and
  their gates - pinned against docs/roles/roles.md by a test that parses the
  table, so an edit to either side alone fails the gate. The sidebar shows
  the four groups in order, reads and changes nothing, cuts a row that does
  not fit with a marker, names a group it leaves out with its count, and is
  still dropped whole at a narrow width with the drop announced. The gate is
  green: 794 Rust tests and 212 script assertions.
  Six lows stand recorded: the sidebar's unreadable state drops its second
  row when the pane is one row tall, with no marker; a home that cannot be
  made names a file that was never touched; two hugo instances on a fresh
  home share one temporary name and the loser shows a false unreadable
  state; the configuration is read at startup and not on every redraw, so a
  human who fixes a broken file keeps seeing the old state until restart;
  the orchestrator's default model is the reviewer's, a choice the flow has
  to confirm; and the review-round limit's pin rests on a phrase this unit
  added to the model doc, so it has no provenance of its own.
  Next: hugo's shell, its runtime record and its crew, and its
  configuration are built. The plan holds no open row for raul or hugo.
  The human made four decisions after that: the two copies - the plain-render
  plumbing and the age units - become one implementation in crates/core; the
  configuration becomes TOML, because it is a file a human edits and JSON
  has nowhere to put a comment; hugo reads its configuration at startup and
  a key reloads it; and the orchestrator's model is picked in hugo's TUI,
  because the orchestrator is the agent hugo will start. Then a batched
  follow-ups unit closes the recorded lows.
  Unit closed: the shared helpers (plan 40). One clean build and review
  round. The plain-render plumbing - plain_text, setup and restore - and the
  age units now live once in crates/core, and raul and hugo both read them;
  the copies are gone and a guard fails if a second copy appears in either
  src tree, a planted copy included. The renders stay byte-identical: the
  builder diffed twenty-seven raul screens at 40, 60 and 100 columns, the
  details and the short panes, hugo at the same widths, and the whole
  81-width sweep against a build of HEAD - every one identical. One
  vocabulary serves both tools, so the under-a-minute case says "under a
  minute" in raul too, where it said "0m"; no fixture and no snapshot
  reaches that state, and the orchestrator ruled that the clearer wording
  wins. The gate is green: 802 Rust tests and 212 script assertions.
  Three lows stand recorded: a comment and an assertion in needs.rs still
  say the age is days, hours or minutes, and a future sub-minute fixture
  would fail on correct behaviour; the shared-helper guard matches the
  definition text, so a copy that renames the helper or inlines the body
  evades it - the same limitation the cut_to, pad and View::draw pins have;
  and the plan rows for the three units the human asked for sit beside this
  one.
  Next unit: hugo's configuration is TOML (plan 41).
  Unit closed: hugo's configuration is TOML (plan 41). One clean build and
  review round. The configuration is ~/.config/coding-crew/config.toml - a
  file a human edits, so the defaults hugo writes carry a comment above
  every part. The old config.json is named, never read, never deleted and
  never converted: the format is provisional and a conversion would guess.
  A configuration that does not parse is named with the file and the error
  and left byte-identical, with the defaults shown beside it; one that
  parses is used as it is; an absent one is the defaults, written on
  demand. The pins hold and each can fail on its own: the default models
  against flow/builder.json and flow/reviewer.json, the default order
  against docs/roles/roles.md. The records stay JSON - they are written by
  machines. The gate is green: 817 Rust tests and 212 script assertions.
  Five lows stand recorded, and they go to the batched follow-ups: the
  on-screen error text is not pinned end to end, the row-level tests assert
  a wording the real pipeline never produces; a sidebar three rows tall can
  show "using the defaults:" with nothing after it; a wrong-type value and
  a syntax error are told apart only by their message text; with an old
  config.json and a broken config.toml at once, the legacy notice outranks
  the error and the error is the first line cut; and a config.json that is
  a directory is not named.
  Next unit: hugo's sidebar acts (plan 42).
  Unit closed: hugo's sidebar acts (plan 42). Two build and review rounds.
  r reloads the configuration and the sidebar shows the new one at once; a
  configuration that does not parse after a reload is handled exactly as at
  startup - named, left alone, the defaults beside it - and a reload with
  no file writes the defaults. The orchestrator's model is picked in the
  sidebar, from the catalogue the harness tool prints - never written out
  by hand - and the pick is saved to config.toml, so it survives a restart;
  the builder's and the reviewer's models stay read-only, because the flow
  starts those and not hugo. The pick is written through the tmp-then-rename
  path and the edited text is parsed before any write, so a pick cannot
  leave a broken configuration. The gate is green: 854 Rust tests and 212
  script assertions.
  The first round found one blocking defect of the kind this project keeps
  finding: the catalogue was read with no bound, inside the key handler, so
  a harness tool that hung froze the window in raw mode with no way out but
  killing hugo from outside. The read is now bounded at five seconds, runs
  off the window's thread, and the choice opens at once with a reading line;
  a tool that hangs, is missing or fails is named in the choice and the
  window stays live. Four lows stay open on purpose, for the batched unit:
  the footers under-name the keys that work; every line the tool prints
  becomes a pickable model; a hand-edited configuration with TOML single
  quotes breaks the pick; and the help lists enter choose unqualified on
  rows where it does nothing.
  Next unit: the batched follow-ups (plan 43).
  Unit closed: the batched follow-ups (plan 43). One clean build and review
  round. Twenty recorded lows closed in one batch, each with a test that
  pins it: the journal keeps the history when one event is bad and names a
  journal it cannot parse instead of calling it empty; the shared age words
  and raul's test agree; the configuration's error is pinned end to end and
  a wrong-type value is told from a syntax error by kind; the unreadable
  state keeps its rows, the error outranks the legacy notice, a home that
  cannot be made is named, two instances on a fresh home do not collide, a
  directory named config.json is still named, and the review-round limit's
  pin rests on a phrase the project carried before the unit wrote it; the
  footers name the keys that work in the scope they work in, the catalogue
  offers only model ids, single-quoted TOML is read as TOML, and the help
  names a key only where it works; raul's external review rounds are
  numbered 1 to 5 with the self pre-screen at 0, Task::new reads the clock
  once, the leap-second rule checks the year, and --width and --height are
  refused outside render mode. The gate is green: 871 Rust tests and 212
  script assertions.
  Five lows stand recorded, and they go to a later batch: with an old
  config.json and a broken config.toml, the notice and the defaults line can
  be cut silently at a two- or three-row sidebar; docs/hugo/screens.md no
  longer named the footer the shell builds - the orchestrator fixed that
  prose in this round; the activity cut marker drops the count of unreadable
  events; two row-level tests still hand-write an error wording the pipeline
  never produces; and the review-round numbering rule in the model doc was
  stale - the orchestrator fixed that too.
  Next: the plan holds no open row. The interfaces are built; the
  follow-ups stay recorded.

Unit closed: the fixtures grow real (plan 44). Three build and review rounds.
The mock was lorem ipsum - one sentence per field, one acceptance criterion,
one step, no questions, no children, no resolved finding - so a task opened
was a stub and no tab could be judged against it. The fixtures now hold real
work in the shapes the interface has to read: tasks with zero, one, two,
three, five and six review rounds; seventeen open findings and fourteen
resolved, with resolutions and comments; open and resolved questions; a
parent with its subtasks; five done tasks carrying their history; and items
with a body and a scope. The text is this project's own work, and every
finding describes a defect the review rounds have really found. A new test,
every_field_the_model_defines_is_populated_by_a_fixture, walks every field of
the model's seventeen structs and every enum variant, so a field nobody fills
fails the gate. The generator still reproduces the tree byte for byte, and
the counts the interface draws are computed from the fixtures.
The first round found one blocking defect of the project's own class - the
folder's README still said the text was lorem ipsum, still counted five
drafts where the tree holds six, and still said twenty-one open findings
where the record holds seventeen, and no test read any of it - and the fix
added the pin that was missing: a test that recomputes every count and phase
shape the README states. The second round found the pin itself was hollow:
the guard decided that a sentence was a count sentence by a noun list, so the
README's "one reason alone" escaped it entirely, and a probe that changed it
to "seven reasons alone" left the suite green. The fix closed the class - the
guard now decides by the number the sentence states, a number that is not a
count is listed explicitly, and a planted count with no claim fails the gate
with its plant proved to land - which settles the theme catalog's
long-recorded low, whose guard has the same shape and has not been changed.
Two lows were ruled and not fixed: the runtime guards do compute from the
fixtures, and a done task whose rounds raised no finding has none to carry.
One low stands recorded: number_word mis-sums a scaled hyphen compound.

Unit closed: the review nests (plan 45). One clean build and review round.
The model already nests - Round.findings names the findings each round raised
- and the projection ignored it: the rounds became an eight-column table
squeezed into the pane's width, where self rendered as se over lf and
external as ex te rn al with the findings column one character wide, and the
findings then repeated as a separate flat list. The review now reads as a
history. Each round is a heading with its fields as bullets, and the findings
it raised sit one heading level under it; a round with no findings says none;
a round naming a finding that does not exist says so rather than dropping it;
a finding belongs to the first round that claims it and is shown once; and a
finding no round names is still shown, after the rounds, in its own block.
The shape follows the projection's own idiom - the Steps section is already a
heading per item then bullets - so the document keeps one vocabulary, and
moving the notes out of a table cell closed a latent defect: a newline in the
notes would have broken a table row. The review reads at 40 columns as well
as at 100, and both widths are pinned.
Both axes found nothing blocking. Two lows stand recorded: the done-when
wrote Round 01 and the heading reads Round 1, which the orchestrator ruled
on - a round is numbered by the model, unpadded, and the self pre-screen is
round 0 - and docs/tui/screens.md still described the rounds as a table,
which the orchestrator fixed at the close.

Unit closed: the row says the review state (plan 46). Two build and review
rounds. The human reads the Needs you and Path tabs to decide what to look at
next, and the rows carried an id, a reason, a title and an age - and nothing
about the review - so a task with five external rounds spent, five open
findings and two of them high looked exactly like a task with nothing wrong
with it. A row now carries the review state as a suffix before the age: the
findings open with their severities named, then the external rounds spent of
the task's own budget. The self pre-screen is round 0 and is not part of the
external budget, a finding that is fixed or dismissed is not open, and a task
with nothing to report carries nothing - its row is byte-identical to the row
it was. The wording, the severity rule and the cut live once, in
crates/raul/src/review_state.rs, and both tabs call it.
The first round found one blocking finding and one low, and both were of the
project's own class. The blocking one: on the Path tab at 100 columns - the
plain render's default - five of the eight state rows named the count and no
severity, every ESCALATED row among them, so the unit's own motivating case
read "5/5 rounds - 5 open findings..." and never said high. The bare-count
rule was not broken - the count was labelled, whole, and the cut marked - but
the severities were the requirement and they were not met. The reviewer
proposed splitting the severity list per severity, and the orchestrator
measured it and found it did not fit: the Path state takes only the room the
row's last line leaves, twenty-seven columns at width 100, and the reviewer's
form needed thirty-three. What fits is the triage fact first - the findings
and their severities lead, because "is there something high here" is what the
human decides on, and the rounds are context - with the severity list split
per severity, most urgent first, so a cut keeps the most urgent one. The low
was the one-implementation scan: it read four of the fourteen files in the
tree and matched three exact signatures, so a copy under any other name would
pass the test while failing the rule - a check that cannot fail. It now walks
the whole tree and matches on the severity mapping and the state words, and a
planted copy proves the pin can fail. The delta review found nothing at all.

Unit closed: the backlog row says its body (plan 47). One clean build and
review round. The generator writes every item's body - a sentence saying what
the item is about - and the Backlog tab never drew it, so an idea, a smaller
task and a draft task all looked the same: a title and a phrase. The body is
the row's second line, dim and indented to where the row's title starts, so it
reads as belonging to the row above it. It is the item's own body, never a
summary, cut by the theme crate's shared cut_to, so a body that does not fit
ends in the tab's own marker on a grapheme boundary. An item with no body
draws no second line and its row is the row it was. A row with a body is two
lines everywhere the tab counts - the window, the announcement of what it
hides, the footer's count - and a row is drawn in both its lines or in
neither, so a row drawn in two lines and counted as one cannot happen.
Both axes found nothing blocking. Seven lows stand recorded: the body is
flush-left at the narrow widths where the id and kind columns drop; a
one-line pane drops the cursor row's body unnamed; a multi-line body is
normalised to one line and no fixture exercises it; a comment on when the
footer must carry the hidden count misreads a two-line pane as a one-line
one; body_line does not guard width == 0 the way columns_line does; the
atomicity pin sweeps only the unscrolled window; and the two normative texts
- the screen spec and the decision of record - the orchestrator wrote at the
close.

Unit closed: the document gets its own palette (plan 48). Two build and review
rounds. Leaf's markdown palette has 47 fields and 36 distinct colours, and our
bridge collapsed all 47 onto 10 of the chrome's 18 slots - so h3 was the same
colour as body text, table_border the same as table_separator, list_level_2 the
same as list_level_3, and the note and important alerts the same as a link -
and the projection never emitted the constructs that would have used them, so
the document showed five colours. The document now has eighteen roles of its
own, the number the human chose, with all 47 engine fields mapped onto them
and the mapping pinned. Six roles carry a pair - the prose, the code and its
chip, the table's line and its header rule, the mark, the search and the task.
The roles that carry meaning are apart in every preset, and every document
colour is one of that preset's own chrome slot colours, so the document and
the chrome are one palette and not two. The terminal preset paints no colour,
and the search ruling is unchanged: the hit background is readable now and the
doc says so with recomputed figures, but the current match stays below 3:1 in
every colour preset, so the search stays un-offered, and the test fails if any
preset ever reaches 3:1.
The first round found one blocking finding, and it was against the
orchestrator's own ruling. The builder had asked the right question - the
theme's rule 1 says every colour comes from a slot, and the document's colours
now come from its own roles - and correctly refused to change a normative
rule. The orchestrator ruled that the rule means every colour is named by
something the theme crate owns and a preset decides, so the document's roles
are slots in that sense and the rule's wording is to be extended, not
weakened. The substance held and the wording did not: the orchestrator had
not carried out the wording it required, and the fix round inserted it
verbatim - a new rule 5 in theme.md, a new rule 11 in palettes.md, and a
sentence binding the document palette section to the rules. Two lows came with
it and both were closed: Catppuccin's derivation note named none of its
repeats while Nord's and Dracula's named every one, and a future mapping entry
could name the two sides of a single-colour role and pass every pin. Two
lows stand recorded: Dracula's six roles share one colour, which is the
family's own narrowness, and the touch to a raul test was forced by the code
label's remap.

### 2026-11-22 · The full review, the redesign, and hugo's first step

The flow ran on Claude in herdr: one Opus reviewer for the whole night, a
fresh Haiku builder per job, at most five build/review rounds a job.

**The full code review** raised 35 findings (2 high, 13 medium, 20 low) in
eight batches. All closed except two recorded follow-ups (F4, and the
reviewer-session gap). The highs: saving a task deleted other tasks whose
ids it prefixed, and `load` could return the wrong record. Commits
`ca4e475`…`42ddda2`.

**The redesign** (`docs/tui/redesign.md`, units 52–57): an icon table with
Nerd Font and unicode glyphs (`--icons`, `CODING_CREW_ICONS`); rounded panes
with icons in their borders, tab badges and key chips; rows that lead with
their status or kind; the Overview as cards with bars; hugo's crew table,
budget bar, findings by severity; the document with bar-led headings,
rounded tables, alert and quote bars, per-preset syntax, a code gutter, an
inline chip, criteria as blocks and the header as a card. The fixtures grew
the constructs to show it.

**hugo** (units 58–59): `hugo --skill <role>` prints the orchestrator, runner
or reviewer skill embedded in the binary, under the crew the configuration
names; `s` starts the orchestrator in a new herdr tab over the queue —
checked live with a Claude harness.


**The redesign, reviewed whole** (2026-11-22). A second full review read
every screen in both icon sets and found 12 (2 medium, 10 low): Backlog
columns drifting within a group, a narrow table starving its prose column,
glyphs written outside the icon table, a broken Path spine, hugo's flow pane
always wearing the running icon, no status pill, and an orchestrator start
that could wedge `s`. All closed, `0840c05`…`b1b5241`. Two rulings: the
header card is one item per field (the border title carries the status icon),
and only the Overview's cards are padded — a list pane draws against its
border, because its columns are spent on words.

Unit closed: the Tasks tab (plan 60). Two build and review rounds. raul has
six tabs, keys 1–6. Tasks lists every task file, drafts and done included,
with the sort key's value on the right and subtasks flat with `↑ parent`; `o`
and `O` cycle and reverse the six sort keys. Round one found that a category
sort announced a hidden count without naming its groups, and that the suffix
drew a link icon where the spec writes `↑`. Both are fixed. Ruling: the
parent suffix gets a parent icon, and the spec is not changed. Commit
`668a837`.

Unit closed: the Tasks filter (plan 61). Three build and review rounds. `/`
narrows the Tasks rows as it is typed: words, and `key:value` tokens with AND
across keys and OR within one. An unknown token is named in the footer. Round
one found the filter line dropped whole at 40 columns; round two found the
error hidden by the count. The line is now cut from the left, and the error
takes precedence over the count. Ruling: `priority:medium` reads as `normal`,
the model's own word, and the spec row is left for the human to correct.
Commit `2d74aba`.

Unit closed: one configuration, written without loss (plan 62). Three build
and review rounds. `config.toml` is read and written through `toml_edit`, so
comments, order and unknown keys survive a write. It holds a `[raul]` table
beside the flow's sections, and raul reads it at startup with flag over env
over file over default. Ruling, from review round one: a bad file does not
stop raul. It starts on the defaults, names the file and the error, and
leaves the file byte-identical, as hugo does. A save refuses when the file
changed on disk. Round one found a save into an inline table silently
dropped; round two found a refused save leaving its edits in memory. Both
fixed. Commit `030b556`.

Unit closed: Settings is the file (plan 63). Three build and review rounds.
The Settings tab draws `config.toml` as TOML and edits each value in it:
choices cycle with ←→, text and numbers edit inline, and each is checked
before it is kept. A change previews live; `w` saves, `u` undoes, `r`
reloads, and quitting with unsaved changes asks. Round one found that a file
without `[raul]` offered nothing to change and that a refused save from the
quit question said nothing; both fixed. Ruling, from review round two:
inline tables and dotted keys are shown as text, not stops, until a design
call is made. It is recorded in TODO.md. Commit `a066b41`.

Unit closed: lighter debug builds (plan 63b). One build round and one review
round, the review clean. `[profile.dev]` keeps line tables only, and
dependencies carry no debug info. From a clean `target/`, `make check` went
from 138.1 s to 108.3 s, max RSS from 868 MiB to 774 MiB, and `target/` from
3.0 G to 1.3 G. No test reads backtrace or location content. Commit
`f644c1e`.

Unit closed: the harness table, and a harness per role (plan 64). Two build
and review rounds. `crates/core/src/harness.rs` is the one table of harnesses:
claude, opencode and cursor-agent, each with its binary, herdr kind, effort
levels, start args, env and model listing. hugo's skill and launcher read it,
and their own mappings are gone. Every agent in `config.toml` names its own
harness, model and effort, and the global `[harness]` is no longer written.
An old file still loads: an agent with no harness takes the old global one,
and `#effort` splits off the model. Nothing is rewritten until a save.
Round one found that a save changing only an old agent's effort or harness
was refused as lost, because its model line kept `#effort`; the fix rewrites
the model of a changed agent, and both cases are read back in tests. Left for
the human, in TODO.md: the opencode effort levels, the flow files' claude
pins, and whether an unknown harness should make a file unreadable, as it
does now. Commit `d3a55af`.

Unit closed: hugo picks harness, model and effort from lists (plan 65). Three
build and review rounds: the build, then two fix rounds, the last review clean.
Every agent's harness, model and effort are picked in hugo's sidebar. `enter`
opens the table's harnesses (a missing one dim and unpickable), the harness's
own model list with `type an id…` last, or its effort levels. A harness change
clears a model the new harness cannot run and resets an effort it does not
take. `s` refuses while any agent lacks one of the three and names the gap.
Picks save through the format-preserving config. Round one found three: an
absent effort read `default` while `s` refused it, a model catalogue that
failed left no way to type an id, and the doc named the wrong id shape. The
human ruled that an absent effort is a gap, shown as `pick an effort` on the
warning slot. The other two were fixed. Ruled on the way, in the design record:
every agent is configured from hugo. Commit `2cab508`.

Unit closed: the footer: centred, dotted, only what matters (plan 66). Three
build and review rounds: the build, then two fix rounds, the last review
clean. One footer pattern in `crates/theme/src/chrome.rs`, used by raul and
hugo: keys bold, actions dim, ` · ` between them, centred, with a thin rule
above. Each context shows its own actions and ends `? help · q quit`;
navigation keys live in the `?` overlay. Two centred lines when one does not
fit. Round one found raul's `?` overlay did not name `enter open`, and two
doc mockups still showed the old footer; both fixed. Round two found the
detail pane kept its scroll keys in the footer. Ruling: the detail pane
follows item 2 of the job, so its footer is `esc close · ? help · q quit` and
`?` opens an overlay that lists the scroll keys. No one was there to ask, so
the build took it; TODO.md asks the human to confirm. Commit `c674884`.

Unit closed: hugo has depth, and selection is soft (plan 67). One build round
and five fix rounds, the sixth review clean. Four new slots - `chrome.surface`,
`text.selected_bg` and the three role slots - in every preset, and the derived
backgrounds are recorded in `docs/tui/palettes.md`. Selection in both TUIs is
the `▌` bar and a subtle tint, with the text keeping its own colours. hugo's
sidebar sits on its surface, its groups are accent headers with a rule, and the
budget bar colours by what is spent. The review stalled on the 3:1 floor for
four rounds: Nord's error and Dracula's muted text fell below it on the tint. The human ruled
**recessed surfaces**: the surface is a step darker than the base, not lighter.
The surface is the base mixed 30% toward black and the tint is the surface mixed
10% toward the accent, which clears the floor in every colour preset. The brief
said "about 25%"; 25% could not meet both the floor and the step. The
`palettes.md` rules 2 and 9 were amended to allow this. The unit ran on the
herdr-crew skill. Commit `6d7d4ef`.

Unit closed: the document in colour (plan 68). Five build and review rounds.
The document's headings take three hues, list depths 1 and 2 take green and
blue, the quote has a bar and text of its own, and the table header and inline
code are chromatic. Code takes eight syntax roles, so the palette is 27 roles.
Every value is a slot of its preset and derived in `palettes.md`, and every
value keeps at least 3:1 on the base. The build and the first three fix rounds
left list depths and the quote undone: the rule against shared hues and the
eighteen-role count both forbade them, and the builder would not overrule a
decision of record.
The human ruled on 2026-11-22: list depths 1 and 2 may share a hue with a
heading or an alert, depth 3 stays grey, and `document.quote` is role 27,
carrying out the approved coloured design. The ruling went in as the fourth
fix round. The unit ran on the herdr-crew skill, which stopped it with
NO_PROGRESS when the ruling was needed. Commit `6dff7a4`.

Unit closed: raul's Settings picks, the Path spine, no freeze (plan 69). One
build round and one fix round; the second review clean. `←→` switches tabs on
Settings, and `enter` on a choice opens a picker. The picker is shared with
hugo's choice popup (`crates/theme/src/picker.rs`), previews as the cursor
moves and restores on `esc`. The Path fork sits in the trunk's column at 40
and 100. The detail keeps its rendered document per source, width, theme and
icons, and the loop drains queued keys before one redraw. Round one found that
`enter` on a pick opened on an unset effort kept nothing; fixed, with a test.
Left in TODO.md: the fork's column, which the spec sketch had one column
right, and the freeze, measured but not reproduced in a terminal. Commit
`be0b070`.

Unit closed: one table per role, and the builder is the builder (plan 70). One
build round, one fix round; the second review clean. The configuration is
`[agents.orchestrator]`, `[agents.builder]` and `[agents.reviewer]`, and the
crew is the roles present: three agents, two with the orchestrator reviewing,
one with the orchestrator building and reviewing, `/clear` before its own
review. Round one found four: a save dropped the comment above a migrated
entry's first key; the docs still said a crew of one leaves the review to the
human and that the runtime role is `runner`; the orchestrator skill forbade
building and reviewing; and the cursor opened past the agents with a crew of
one. All four were fixed, with tests. Ruling: the orchestrator is always
present. A file without one is unreadable, which the builder chose and the
human has not confirmed. Commit `50b473f`.

Unit closed: hugo's crew size, and choices for this session only (plan 70b).
One build round, one fix round; the second review clean. hugo never writes
`config.toml`: its harness, model, effort and crew picks last the session,
and the file's values are the defaults, edited in raul's Settings. `c` opens
the crew-size selector with its hints; the sidebar tags a value that differs
from the file with a dim `session`; `s` and `hugo --skill` start the session's
crew. Round one found two tests that did not cover what the job named: crew
sizes 2 and 3 were never checked for the roles shown and started, and the
no-write test made no pick at all. Both were fixed with tests. Ruling
(2026-11-22): hugo's picks are session-only. Commit `a6a911e`.

Unit closed: effort comes from the model, and raul picks from lists (plan 71).
Four build rounds and four reviews; the second build round changed nothing,
waiting on a ruling. Each model's levels come from its harness's listing:
claude from a curated table (Opus, Sonnet and Fable low to max, Haiku low,
medium and high), cursor-agent from the id suffixes of `--list-models`,
opencode from the variants in `opencode api --standalone model.list`. The
catalogue moved to core. raul's Settings picks from hugo's lists, with
`type an id…` for a model. Round one found three: cursor-agent was started
with a bracketed id it rejects, so the start now joins the listed id; a base
listed bare and at levels could not be started bare; raul showed a blank for
a no-level model. Round three found a typed model whose effort was cleared and
could never start. The orchestrator ruled: its levels are the harness's known
levels, then `none`, then `type a level…`. The builder stores `none` as
`effort = ""`, not `"none"`, to keep clear of real `none` variants; the
orchestrator confirms it. The last review's one finding, a stale agent-table
row in task-model, is in the commit. Commit `7da52cf`.

Unit closed: hugo shows where the flow is, and picks the tasks (plan 72). One
build round, one fix round; the second review clean. Every block has a dim
caption, the round is a timeline of the task in flight, and the queue is
pickable: `tab` moves focus, `space` marks, `a` marks all, and `s` starts the
marked tasks in queue order, or the whole queue when none is marked. The mock
runtime tells one story: five tasks, two done, one in flight in round two with
the reviewer working, two waiting. Round one found one low: screens.md kept
the old queue, budget and findings wording beside the new. Fixed to match the
code and the pinned screens. Choices the builder made are open for the
orchestrator to confirm (see THE FOLLOW-UPS). Commit `1b29940`.

Unit closed: the mouse works in both TUIs (plan 73). Two review rounds; the
second clean. The wheel scrolls what is under the pointer, three rows a notch,
and each batch of queued events is applied before one draw. A click picks a
tab, a row, a sidebar or queue row, or a picker entry, hit-tested against the
regions the draw stored. Capture is on at start and off on exit and on panic.
Round one found three: pointer motion forced a folder read and a redraw on
every wake, Settings' clicks were untested though the test said they were, and
hugo's queue lines were a guessed offset, not recorded from the draw. The fix
round closed all three with tests. No ruling was made. One choice stays open:
a click on a Settings table header selects the value below it, and the two
reviews disagree on the answer (see THE FOLLOW-UPS). Commit `bd41972`.

Unit closed: the review reads in layers (plan 74). One build round, one fix
round; the second review clean. Each round is a heading with its kind, number,
verdict and reviewer. Its findings are grouped high, medium, low, and each is a
list item, its details one level in and its comments one level deeper. Orphan
findings get their own group. TSK19 renders at 60 and 100 are in the report.
Round one found one low: a comment's id overflows a 14-column pane at its
depth, so the width floor moved to 16, and the known-issue line in TODO.md was
corrected to match. No ruling was made. The comment depth and the floor are
open (see THE FOLLOW-UPS). Commit `ec52a51`.

Unit closed: backlog items have documents (plan 75). One clean build and
review round. An item opens the detail pane from its own projection,
`item_to_markdown`: a header card with id, kind, status and scope, then the
body and the history. The "has no document yet" notice is gone; the notice for
an id outside the work folder stays. Eight tests pin the projection, the
injection rule and a snapshot, and a render at 40 and 100 columns checks the
pane. No ruling was made. The header's omitted tags and the always-present
history are the builder's choices (see THE FOLLOW-UPS). Commit `f305aad`.

Unit closed: Settings in colour (plan 76). Three build rounds and three
reviews; the third clean. The Settings TOML is painted by the document's
syntax roles: headers, keys, `=`, strings, numbers, booleans and comments, each
on its own role. The selected line keeps its bar and tint, and a value's error
keeps its colour. The terminal preset tells the token kinds apart by attribute.
Round one asked whether that attribute table should stand, because theme.md and
palettes.md said the terminal keeps no distinctions beyond the engine's own
modifiers. Ruling: keep the table, since the done-when says the terminal preset
uses attributes; both docs now record it. The key-role and colour choices wait
for the human (see THE FOLLOW-UPS). Commit `0175d28`.

Unit closed: Path counts the finished work (plan 77). Four build rounds and four
reviews; the fourth clean. Path ends with one FINISHED row, `N tasks finished`,
dim, with its branch icon. It is not a row, so `enter` does nothing on it, and
with no done task it is skipped. Done tasks are counted here and listed on the
Tasks tab. The scroll moves for the row only when the cursor is on the last
task. Rounds one to three found four defects: FINISHED unreachable when the tree
scrolls, FINISHED drawn under a cut spine, live rows hidden to make room for a
FINISHED row that still did not fit, and a scroll on every cursor move once
every row fits. All four are fixed, with tests. No ruling was made. FINISHED
closes the scroll and is not announced while off screen: option (a), the
reviewers' recommendation, taken in round two and written into
`docs/tui/screens.md` § Path (see THE FOLLOW-UPS). Commit `d8f7058`.

Unit closed: raul's loose ends (plan 78). One build round and one review round,
clean. The Overview's unreadable line names and counts its files:
`unreadable not shown: 2 files (a: e, b: e)`. The Backlog announces through
`announce.rs`: the dropped-header shape moved there as a `dropped` flag on
`Group`, and the private copy in `backlog.rs`, about 400 lines, is gone.
`body_line` returns a blank line at width 0. The stale comments at
`backlog.rs`, `overview.rs` and `path.rs` now say what the code does. The copy
scan checks all six theme text helpers across `crates/raul` and `crates/hugo`,
and a planted copy fails it.
Ruling: `needs.rs:338` is unchanged, against the done-when. Needs rows are one
line, so the reviewer found the comment accurate and recommended it stay.
Behaviour change: at narrow widths a Backlog label whose count does not fit
keeps its own row and is not cut, as `announce.rs` draws it. No test pinned the
old form. Commit `0d89ff6`.

Unit closed: hugo and theme loose ends (plan 79). Two build rounds and two
review rounds; the second clean. Esc from the id input goes back to the state
it opened from, with its error kept. `skill.rs` writes `unset` for an absent
effort. hugo's screen and footer are swept from 40 to 120 columns in both icon
sets, through the real binary and the sweep's own column reader
(`crates/hugo/tests/render_sweep.rs`). The sidebar tests use the config
pipeline's own error; apartness has a near-duplicate fixture; `states_a_count`
treats any stated number as a count unless it is exempt.
Round one's review found two. The in-process width test measured a buffer that
clips overflow, so it could not fail; it is replaced by the sweep. The queue
fixture did not make the old offset pick a different row. Ruling: the
reviewer's option (a), taken in round two. The recording is proven below the
runtime through `captioned_picks`. No runtime makes today's queue layout differ
from `2 + index`, so the done-when's runtime form cannot be met. A behaviour
change outside the brief: esc from the id input on an empty model choice now
returns to Empty, not Listed. Limits are in THE FOLLOW-UPS. Commit `14c81d5`.

Unit closed: docs and the gate (plan 80). Two build rounds and two review
rounds; the second clean. The design record states the 3/2/1 crew: one agent
means the orchestrator builds and reviews (`docs/hugo/design-record.md:25-27`).
The effort rule is recorded: a switch clears the effort unless the model takes
it, and an absent effort is a gap unless the model takes none. The old text
said the effort stays, which the code did not do. Round one's review found the
gap sentence missed the model that takes no effort; round two fixed it.
`docs/tui/redesign.md` uses the current counts (`2 of 5`, the task in flight's
open findings), as `docs/hugo/screens.md` § counts does. The Makefile drops
`test-flow` and `SKILL_SCRIPTS`; the gate no longer runs the retired flow
suites. No ruling was made; the rule is the one of units 65 and 71. Commit
`a73b873`.

Unit closed: the triage's hugo and config rulings (plan 81). Five build rounds
and five reviews. Config: an unknown `[agents.<name>]` table and a crew the
flow does not run load, each named by a warning on hugo's sidebar and raul's
Settings; a missing orchestrator and an unknown harness stay unreadable. The
flow files name the agent only, and hugo's model defaults are pinned in core's
own test. An agent can be `blocked` (`AgentState::Blocked`, drawn on the warning
slot under its own icon). The queue offers only claimable tasks; an unclaimable
waiting task is drawn dim with `not claimable`. Round one's review found the
queue help still said `a` marks every waiting task; fixed in round two.
Ruling: the done-when's "the mock gains one claimable waiting task" waited in
rounds two to five. The orchestrator ruled after round five, option (a): TSK29
is the claimable waiting task and TSK07 the one that is not, the mock already
shows both, and no fixture was added. The done-when was amended in the job
file, with no code change. Limits are in THE FOLLOW-UPS. Commit `876dd58`.

Unit closed: the triage's raul changes (plan 82). One build round and one
review, both clean. A failed effort read lists `type a level…`, as the model
read lists `type an id…`. The `?` popup is as tall as its lines, capped at the
window; when it overflows, `↑↓ j/k`, `pgup/pgdn` and `g/G` scroll it, and
Tasks help keeps `g/G`. `r` in Settings asks before dropping unsaved edits;
`ctrl-c` stays instant. A click on a Settings heading or comment selects and
edits nothing (D11). Docs: the priority row reads `urgent · high · normal ·
low`; the filter's plain words are AND'd; a resized list is written whole;
`HUGO_SESSION` is documented as one line of JSON. Ruling, from the human: `r`
drops the session's picks, kept as built, recorded in the design record. Limits
are in THE FOLLOW-UPS. Commit `d566c99`.

Unit closed: TOML in colour, and one story for TSK14 (plan 83). Three reviews,
the last clean. The engine highlights `toml` fences through a bundled grammar,
`crates/engine/assets/toml.sublime-syntax`, added to the default syntax set.
Keys take the tag role, tables the type role, booleans and numbers the number
role. Its only new dependency is syntect's `yaml-load` feature, which the loader
needs; it brings `yaml-rust` and `linked-hash-map`. The grammar is deviation 26
in `VENDORED.md`, with its SHA-256. Raul's Settings painter is unchanged. TSK14
carries round 1's findings: an external `changes-needed` round raising FIND-01
(high, still open) and FIND-02 and FIND-03 (medium, fixed). Ruling, from the
orchestrator: option 1. TSK14 stays in `code-review` with round 2 in flight.
The lifecycle reaches that state only by `re_review`, which the model cannot
stamp, so the story is kept as built and recorded in `mock-data/generate.py`
and the README. Limits are in THE FOLLOW-UPS. Commit `7479283`.

Unit closed: the core schema additions (plan 84). One build round, one fix
round and two reviews; the second clean. Core gains the optional fields of
ADRs 0016–0019: `Round.session`, `review.re_reviews[]`, `Evidence.manual`,
`Task.revision`, `crew[].actor` and thirteen journal kinds, read with defaults
so old files still load. The generator writes them. TSK14 carries its
re-review, `session-runner-02-remediation-1`, and its status moves to the
hand-back, which closes the unit 83 gap that the model had no re-review
record. Review one found a medium: every hand-back named the execution
session, which `remediation.session_distinct` refuses. The fix gives all 17
hand-backs a remediating session of their own and pins the rule in the
generator. Ruling: the Overview's activity block changes, because the new
`round_filed` event displaces an older one; the review accepted the snapshot
change. Limits are in THE FOLLOW-UPS. Commit `200eea9`.

Unit closed: T2 · Core gates (plan 85). One build round and one review, clean.
`check()` collects every failing gate as a `GateFailure` with its TM § 3.3
name (ADR 0014), and `transition()` refuses with the whole list. Three gates are
new: `review.round_current` (the latest external round is at or after
`status_since`), `review.round_owner` (the acting session filed that round),
and `task.claimed` on finish (the claim's holder and session, not the lease, per
ADR 0020). The executing session is refused as an external reviewer on
pass_review, request_changes and escalate (ADR 0017). Each gate has a lifecycle
test, and one test shows `check()` returns every failing gate. The generator and
the fixtures are unchanged; the fixture replay in `mock_data.rs` now replays
review moves as the filing reviewer. `TransitionError::Gate` became `Gates`, and
`Cancelled` is new; no crate outside core matches on it. No ruling was made.
Limits are in THE FOLLOW-UPS. Commit `5c19a5d`.

Unit closed: T3 · Core operations (plan 86). Three build rounds and three
reviews; the third clean. Core gains `take`, `renew` and `release` in
`crates/core/src/claim.rs`, with the claimable rule, the takeover rule (an
approved claimable task, or an executing one with an expired lease) and a
two-hour default lease. `ownership.rs` gives each section an owner, and a
section with no owner refuses every writer. `AcceptanceCriterion` stores its
status mirror. Fixtures are unchanged. Round one's review fixed a doc row:
`claim.held` now reads "no live claim", because the holder renews rather than
takes again; a test pins it. Round two's review found `status` owner-less
against `roles.md` and `commands.md`, which named the runner. The builder kept
the code and amended both docs to say transitions alone write `status`. No
human ruling was made on it. Limits are in THE FOLLOW-UPS. Commit `a094e78`.

Unit closed: T4 · Core store (plan 87). Three build rounds and three reviews;
the third clean. Core gains a project lock, `ProjectLock`, an exclusive `flock`
on `coding-crew/.talita.lock` with a 10 s wait, then `StoreError::Locked`; reads
take none. Every store write takes it. `activity.json` and `runtime.json` are
written through a temp file and a rename. Ids sort by number, and `save_item`
removes an item's old file on rename. Against the unlocked code, the
creating-writers test kept 13 of 50 tasks, and the journal test kept 48 of 50
events. The fixtures are unchanged. Round one's review found store tests
locking the shared `/tmp/.talita.lock`; round two found two more test files
doing it. Round three's review was clean. Limits are in THE FOLLOW-UPS. Commit
`5c5628e`.

Unit closed: Review A · core lifecycle, claims, ownership (plan 88). One build
round and one review, clean. Four findings of the full review's batch A, each
with a test. `owner(Review, …)` is phase-dependent: the runner in `executing`
and `remediating` (its self round, per ADR 0017 "Rules at filing"), the
reviewer otherwise. The section is not split. A re-review writes its record in
the same move, and approval sets `planned_by`, per ADR 0015. A lease that would
end after year 9999 is refused with `ClaimError::LeaseTooLong`, not wrapped.
The ownership test's name says what it asserts. No new ruling. The finding's
example `9999999999999h` never wrapped; it was born expired, because its end
does not parse back, and the fix covers both. Limits are in THE FOLLOW-UPS.
Commit `0489893`.

Unit closed: Review B · core store (plan 89). One build round and one review,
clean. Three findings of the full review's batch B, each with a test.
`ProjectLock::save` reads the on-disk revision, refuses a mismatch with
`StoreError::Revision`, and writes the next revision. `Store`'s own writers that
took no lock are gone, so only the lock writes a caller's record. `list` and
`load` keep one record per id, and `duplicate_ids` names the ids held twice; the
talita check that calls it waits for talita. The signature became `save(&mut
Task)`, a choice the orchestrator is asked to rule on. No other ruling. Limits
are in THE FOLLOW-UPS. Commit `6717714`.

Unit closed: Review C · raul (plan 90). One build round, one fix round and two
reviews; the second clean. Six findings of the full review's batch C, each with
a test. Path measures a wide title by its display width, so the title keeps its
branches (F2). A groomed task with an overlay waits in AWAITING APPROVAL with its
hold (F6); the pinned AWAITING APPROVAL count moves from 2 to 3. The Needs you
id column widens past TSK99 (F11). The filter line shows on the Tasks tab alone
(F12). The Overview's next line names what holds the approved tasks (F13).
Manual evidence reads "— manual override", and TSK10 carries an override through
the generator; the regenerated tree changed that one file (F14). Round one's
review found the `evidence.command` fixture check loosened to pass a non-manual
empty command (D1); the fix keeps it for non-manual entries. No ruling. Limits
are in THE FOLLOW-UPS. Commit `457c87d`.

Unit closed: Review D · hugo (plan 91). One build round, one fix round and two
reviews; the second clean. Six findings of the full review's batch D, each with
a test. The crew skill prints `start args` and `start env` as shell words (F7).
A failed start after the tab opens names the prompt when herdr may already have
named the orchestrator (F8). A bare model that also takes levels offers `none`
after them (F18). An odd crew opens with no size painted, and `↑` on the top
row moves nothing (F19). Drains get 500 ms after the child exits, still bounded
(F20). A field error under an agent names its role and field, not the untagged
enum's message (F21). Round one's review found F19 still painted size 1; the fix
gives the picker an optional cursor, so an odd crew paints none. Ruling: an odd
crew opens unpicked, not at the nearest size, the builder's choice on the
reviewer's recommendation. Limits are in THE FOLLOW-UPS. Commit `a538692`.

Unit closed: Review E · fixtures (plan 92). One build round and one review,
clean. Two findings of the full review's batch E. F9 (ruled by the human,
2026-11-23: the fixtures follow ADR 0017): TSK13 and TSK15 carry a findings-free
self round 0 and their findings on an external round 1, as TSK14 does. TSK13's
FIND-02, fixed in code review by the claim's session, is open. F22: the mock README's fan-in counts now match the fixtures, and a test
recomputes them. The generator reproduces the tree byte for byte. Limits are in
THE FOLLOW-UPS. Commit `fc6006c`.

Unit closed: Review F · engine, theme, tooling (plan 93). One build round and
one review, clean. Four findings of the full review's batch F. F23: a link's URL
inside a heading paints dim; `blocks.rs` changed, so its sha256 and deviation 21
are in `VENDORED.md`, with a test. F24 (ruled by the human, 2026-11-23: the code
follows the doc): the terminal preset's footer keys are bold and reversed, as
`docs/tui/theme.md` says, with a test. F25: the pty sweep's fake raul takes
`--tab` and the suite asserts it is forwarded, with a test. F26: `make columns`
says it writes nothing; no regenerate target was added, no test. Limits are in
THE FOLLOW-UPS. Commit `024ea01`.

Unit closed: Review G · docs (plan 94). One build round, one fix round and two
reviews; the second clean. Six findings of the full review's batch G. F10:
TODO's WHAT WAITS ON THE FLOW says the orchestrator's start is built and driving
the loop is not; AGENTS.md was already updated by the orchestrator and not
touched. F27: the task model's `claim` has its own row, § 4.4 moves after § 4.3,
and the § 15 link resolves. F28: "eighteen slots" is twenty-three in theme.md,
palettes.md and a theme.rs comment. Round one's review found one more in
theme.rs's test comment (D1), fixed in round two. F29 and F30: hugo's and raul's
screens match the code. F31: Settings names the order the window starts in.
Doc and comment changes only; no test owed, no ruling. Limits are in
THE FOLLOW-UPS. Commit `f29fec8`.

Unit closed: HUGO_CREW (the opencode trial). hugo's crew variable is
`HUGO_CREW`, per ADR 0015; `HUGO_SESSION` is no longer read, with a test. Built
by opencode, not Claude: builder `opencode-go/deepseek-v4.1-flash` on max,
reviewer `opencode-go/glm-5.3` on high, through herdr-crew unchanged — the
model and effort travel in `OPENCODE_CONFIG_CONTENT`. One build round, one
review, clean. Built in a worktree and cherry-picked. Commit `d05bb29`.

Unit closed: HS (live test). hugo's `s` failed inside herdr with no
orchestrator named: herdr writes `agent_not_found` on stderr with stdout
empty, and hugo read stdout only. It now reads both, with a test that fails on
the old code. Built by the opencode crew, clean in one round. Retested live:
`s` opened the orchestrator tab on opencode, and the model, effort and
`HUGO_CREW` reached it. Commit `4eb8914`.


### 2026-11-23 · Follow-ups closed by grooming

The human ruled on 2026-11-23: every recommendation taken, and follow-ups
that need no work are archived here rather than kept open. Each is as it was
recorded; a ruled item carries its ruling.

- **2026-11-23, unit 92:** TSK13 and TSK15 record `changes_requested` at their
  external round, while the status stays `code-review` until the reviewer calls
  `request-changes`. TSK14 already does this, so it was left as is. Recorded, not
  ruled.
- **2026-11-23, unit 91:** the tests for F7, F8, F18, F20 and F21 were not run
  against the pre-fix code (round 0's report). The reviewer read each as failing
  without its fix. Recorded, not ruled.
- **2026-11-23, unit 91:** F8 and F20's failure paths are covered by unit tests
  of the pure helpers only. Neither runs end to end: F8 needs herdr, F20 needs a
  slow child. Recorded, not ruled.
- **2026-11-23, unit 91:** round 1's `cargo fmt --all` may have reflowed lines in
  `crates/hugo/src/app.rs` that were already in the tree. Nobody diffed them out.
  Recorded, not ruled.
- **2026-11-23, unit 91:** an odd crew opens with no size painted (F19). That is
  the reviewer's recommendation, taken in round 1 with no human ruling. The
  alternative was the nearest size. Recorded, not ruled. Ruled: confirmed — no size painted.
- **2026-11-23, unit 90:** F2's option to drop only the offending row, not the
  branch, was not taken. The builder read it as optional. Recorded, not ruled.
- **2026-11-23, unit 90:** F6's test was not run against the pre-fix code, so it
  is not shown to fail without the fix. Recorded, not ruled.
- **2026-11-23, unit 90:** the `evidence.command` fixture check (D1) has no
  reverse test: no fixture holds a non-manual entry with an empty command. The
  reviewer accepted it. Recorded, not ruled.
- **2026-11-23, unit 89:** `ProjectLock::save` takes `&mut Task`, so the caller's
  copy follows the file's revision. The alternative `&Task` leaves a stale copy
  that the next save of the same object refuses. Open for the human to rule. Ruled: keep `&mut Task`.
- **2026-11-23, unit 89:** F3 and F4 have no fail-before run. F3's test uses the
  new signature and `StoreError::Revision`, so it cannot compile on the old code.
  F4 removed the unlocked writers, so no runtime test can fail on the old API.
  Recorded, not ruled.
- **2026-11-23, unit 89:** an unparseable existing file for an id makes a save
  fail, not overwrite. This is a choice the reviewer accepted. Recorded, not
  ruled.
- **2026-11-23, unit 88:** the new tests were not run against the pre-fix code.
  The reviewer checked each by reading, not by running it, so none is shown to
  fail without its fix. Recorded, not ruled.
- **2026-11-23, unit 88:** `Section::Review` keeps one owner name, phase-dependent,
  and is not split into two sections. The reviewer accepted it. Recorded, not
  ruled.
- **2026-11-23, unit 87:** the journal refuses a write while it holds events it
  cannot read (`StoreError::JournalUnreadable`), because `Activity` drops them
  on read and a rewrite would delete them. Follows decision 17. Keeping unknown
  events instead is open. Recorded, not ruled. Ruled: keep the refusal.
- **2026-11-23, unit 86:** `status` has no owner in `ownership.rs` or task-model
  § 2.5, and `roles.md:60` and `commands.md:38-42` now say transitions alone
  write it. Round two applied the reviewer's recommendation; no human ruling.
  Confirm, or revert both docs and make `Section::Status` runner-owned in
  `ownership.rs` with the TM line at 942. Ruled: confirmed — transitions alone write `status`.
- **2026-11-23, unit 86:** the optional `NotHolder` hint (a holder's `take`
  points to `renew`) was not taken. `take` returns `Held` alone. Recorded, not
  ruled.
- **2026-11-22, unit 85:** `start` checks the holder and the live lease only.
  ADR 0020 asks for no session check there. Recorded, not ruled.
- **2026-11-22, unit 85:** the fixture replay in `crates/core/tests/mock_data.rs`
  replays review moves as the filer of the latest external round, and dates a
  replayed code-review phase at that round's `at`. The fixtures are unchanged.
  Recorded, not ruled.
- **2026-11-22, unit 84:** the `assert_ne!` added to
  `tsk14_is_handed_back_after_its_findings_are_fixed` was not run against the
  old fixture value, so it is not shown to fail without the fix.
- **2026-11-22, unit 84:** raul's icons for the thirteen new kinds
  (`crates/raul/src/overview.rs`, `event_icon`) are the builder's guess. No
  ruling. Ruled: confirmed as built.
- **2026-11-22, unit 84:** `orchestrator-01` is the builder's choice for the
  orchestrator's `crew[].actor`. No fixture names one. Ruled: confirmed as built.
- **2026-11-22, unit 83:** the TOML render test was not run against the old
  code, so it is not shown to fail without the grammar. The resolve assertion
  was probed against the YAML fallback.
- **2026-11-22, unit 81:** none of unit 81's tests was run against the code
  before its fix, so none is shown to fail without it.
- **2026-11-22, unit 81:** the sidebar's config warning is cut at the pane
  width with `…`. At 100 columns the table name stays visible; the full text
  is shown only in the test.
- **2026-11-22, unit 78:** `needs.rs:338` is unchanged. The done-when named
  it; the review found Needs rows are one line, so "Only a pane of one line
  shows the row alone" is accurate. Confirm it, or reopen to name the case. Ruled: confirmed accurate.
- **2026-11-22, unit 78:** the atomicity pin
  (`a_two_line_row_is_drawn_whole_from_any_first_row`) pins current behaviour.
  It was not run against the code before the fix, so it is not shown to fail
  without it.
- **2026-11-22, unit 79:** the width sweep's row check fires only when a glyph
  measures wider than ratatui draws it, because the buffer clips overflow. No
  mutation has shown it fail.
- **2026-11-22, unit 79:** `NOT_COUNTS` exempts five channel-gap phrases that
  no test recomputes, and thirteen contrast-ratio phrases that are exempt
  without a check that any test reads them.

### 2026-11-23 · Unit 95, hugo on PATH

`s` refuses to start unless `hugo` on PATH is the running hugo
(`crates/hugo/src/on_path.rs`), and names the folder to add; a hugo replaced
on disk while running (`<path> (deleted)`) still counts as itself. `make
install` installs hugo and raul and prints the folder. README has a build from
source section; `docs/hugo/screens.md` lists the two refusals. Two rounds:
review 1 found the replaced-binary case, review 2 clean. The builder sat on a
permission prompt for a dry run (`make -n install`) until it timed out; the
loop was resumed from review 1. Commit `f816d24`.

### 2026-11-23 · Unit 96, docs drift

`crates/theme/src/theme.rs`'s five palette comments give their own counts
(17, 17, 14, 17, 12), not eighteen; `mock-data/README.md` says TSK14's
re-review is stamped in `review.re_reviews[]` once its mediums are fixed, its
high still open (review 1 caught a first wording that said all findings were
fixed); TSK22's note says the gate checks the column table. Fixtures
regenerate byte-identical. Two rounds. Commit `428cc44`.

---

## Lessons

HAPPENED
  The orchestrator committed another agents staged work: it ran git add on
  its own two doc paths and then git commit without paths, and the builder
  had already staged the mock-data rename - so the rename went into the
  orchestrators commit. The rule: commit the paths you mean - git commit --
  <paths> - or check what is staged before committing, not only what you
  added.
  /tmp is shared between the projects that run the flow, and this session
  paid for it: one project's round files sat in the same /tmp/opencode
  directory as another project's, and a SPEC reviewer read the other
  project's kickoff and reviewed a unit that does not exist here. The rule:
  the round files live under a project-prefixed path - /tmp/opencode/cl-<unit>
  - and a review brief says so in its first line, so a reviewer that guesses
  a path finds nothing to guess wrong about.
  /tmp filled up mid-session and the gate failed on it: the tmpfs holds
  2 GB, every gate run keeps a swept pty output directory, and two stale
  probe directories from an earlier session held 1.7 GB of build targets.
  The gate reported twenty failures that were all "no space left on
  device", and none of them was a defect. The rule: the sweep's kept output
  and the probe directories are pruned as the session goes, and a gate
  failure that says "no space" is the disk, not the code.
  The known defect came back: both reviewers prompts arrived unsubmitted,
  sitting in the input box while herdr reported the panes idle, and the
  wake commands ran to their bound. The recovery worked - a bare Enter
  through herdr pane send-text, and then the turns ran to the end. The rule
  stands: after every wake, read the pane and check the input box.
  The orchestrator passed pane ids to herdr tab close - wX:p7 where herdr
  wants a tab id, wX:t7 - and herdr answered tab_not_found every time. The
  orchestrator read that as already closed and moved on, so no tab closed
  and the Orchestration workspace piled up thirteen tabs, each holding an
  opencode client, its own --standalone server and a pty daemon: 2106 MB
  of RSS held by finished rounds. The rule now: herdr tab close takes a
  TAB id, and a close that answers tab_not_found means the id was wrong,
  not that the tab is gone. Twelve tabs and 33 processes were reaped,
  freeing about 1.7 GB of pane RSS and 1.2 GB of system memory. The
  orchestrator repeated the mistake once more - it closed a reviewers tab
  while the wake command was still polling, so the wake ran to its bound
  blind - the rule is to wait for the wake to return before closing a tab.
  It happened a third time, and the rule is now written into the skill:
  the report often arrives before the wake notices the turn ended, so a
  tab closed on the report leaves the wake polling a pane that is gone.
  It happened a fourth time, on the dependency-graph delta - and the
  practice, not the rule, is what has to change: the orchestrator closes
  tabs in a batch at the end of a unit, or only after that panes own wake
  notification has arrived. A report is not a signal that the pane is
  free.
  It happened a fifth time, on the hugo proposals: three fix rounds were
  killed on the humans word, and each panes wake was left polling a pane
  that no longer existed, so all three ran to their 2400-second bound and
  then reported a timeout - noise that says the round failed when the
  round was abandoned on purpose. The clause the rule was missing: a pane
  that is killed deliberately takes its wake with it. Kill the pane, then
  kill the wake, and no orphan reports.
  A builder pane was blocked by an account setting, not a defect: the
  opencode-go DeepSeek V4.1 Flash model is region-blocked until the
  console's privacy region is set to Global. And a wake command was left
  running in the background while its tab was closed, so it ran to its
  full bound blind - the turn had long completed. Rule: never close an
  agent tab while its wake command is still in flight.
  The shell and the Overview took six build and review rounds. The reviews
  were worth it: they found that a count could be silently cut at a width
  where the screen looked fine, that the tab bar could be cut in half, and
  that the title line could read "coding-lane".
  The theme bridge's fix round ran builder and reviewer both on
  opencode-go/glm-5.3-flash#high, a human-ordered test of one model family
  for both roles. The builder fixed six findings in 11 minutes and ran its
  own mutation check; the reviewer spent 4.5 minutes, replayed the mutation
  that escaped round 2, and confirmed it now fails. The rule that roles run
  different model families stays; the test showed the flow does not depend
  on it.

  Three mistakes of the orchestrator, all of them committed into history: git
  add -A swept a builder's uncommitted work into a docs commit before it was
  reviewed; a kickoff prompt was left sitting in an input box because enter
  arrived before the text finished typing; and a steer sent to a running agent
  was never submitted, so the correction sat unread in the input box. All
  three are now rules in the skill: stage by name; send the whole kickoff as
  one call, wait a second, send enter, then read the pane to confirm the text
  left the box; and a steer follows the same sequence.
  The reviewers ran pty sweeps over every terminal size, which is how the
  width-dependent defects were found, and which cost about fifty minutes a
  round. From now on a reviewer runs the gate and reads the code. A harness
  worth keeping becomes a unit of its own.
