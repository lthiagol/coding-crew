# coding-crew — orchestration state

Updated 2026-11-23 by the orchestrator, after unit 93.
Branch `main`, remote `https://github.com/lthiagol/coding-lanes.git`.
The gate is `make check` (`make ci` is an alias): `cargo fmt --all -- --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`,
`bash scripts/test-pty-sweep.sh`.

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

The build loop is the global skill `herdr-crew` (`~/.claude/skills/herdr-crew/SKILL.md`): Haiku builders fresh per job, one Opus reviewer cleared per unit, round files in the session scratchpad. `flow/` is not read by it.

Round files do not survive the machine; copy what matters into this file or the design record before a unit closes.

The hugo proposals' rulings are in `docs/hugo/design-record.md`, and the
defects the review found are in the known-issues block of
`docs/hugo/tui-proposals/index.html`.

---

## THE OPEN UNITS

Queued in `docs/plan.md`, in order. Units 0–93, 63b and 70b are closed; their record is in
`DONE.md`. Open, in order:

- 94 · Review G · docs — S

Grooming (G1–G4) and question Q5 wait on decisions or the flow.

---

## DECISIONS OF RECORD

- **The crew is the roles present** (2026-11-22, unit 70). Three agents are one
  per role; two are the orchestrator and the builder, and the orchestrator
  reviews; one is the orchestrator alone, which builds and reviews. The
  orchestrator is always present.
- **27 roles, ruled by the human 2026-11-22.** The document's palette grows
  from eighteen roles to twenty-seven: the eighteen, eight syntax roles for
  the code's tokens — `document.syntax.keyword`, `.string`, `.number`,
  `.function`, `.type`, `.comment`, `.variable` and `.tag` — and
  `document.quote`, a block quote's bar and text, from the human's coloured
  design of the same day. The count of 2026-11-21 is superseded. The
  apartness rules hold for the new roles too, except that list depths 1 and 2
  may share a hue with a heading or an alert (2026-11-22, amending the rule in
  palettes.md); depth 3 keeps it.
- **The document gets eighteen roles of its own** (2026-11-21). Not leaf's
  thirty-six — our chrome palette is deliberately quiet, and the document
  should not fight it. Eighteen is the number the human chose; the syntax
  roles and the quote role of 2026-11-22 make it twenty-seven.
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
- **Done tasks are counted on Path, listed on Tasks** (2026-11-22, the human's
  ruling). Path ends with a FINISHED row that counts the done tasks; the Tasks
  tab is their home. Closes the open question on done tasks. Unit 77.
- **Store discovery accepts any folder named `coding-crew`** (2026-11-22, the
  human, review F4). Kept as is. A checkout of this repository is discovered as
  a project; the limit is recorded in ADR 0010. A marker would fix it, and waits
  for a writer.
- **A round is numbered by the model, unpadded** (2026-11-21). The review
  heading reads `Round 1`, not `Round 01`. The self pre-screen is round 0, and
  padding it would render "Round 00"; the model's own number is what the
  record holds. The human's sketch wrote "Round 01" to show the nesting, and
  the number was its example. Ruled on the SPEC review's reading of the
  kickoff; a one-line change and its pins would overrule it.
- **An absent effort is a gap** (2026-11-22, unit 65 review). An agent with no
  `effort` is not yet picked: `s` refuses to start over it, and the sidebar
  says `pick an effort` on the warning slot. `task-model.md` and `screens.md`
  say so. This replaces the earlier note that an absent effort runs on the
  harness's default.
- **Recessed surfaces** (2026-11-22, unit 67). The human ruled hugo's sidebar
  surface a step darker than the base, not lighter, so the 3:1 floor holds in
  every colour preset. The surface is the base mixed 30% toward black; the
  selected tint is the surface mixed 10% toward the accent.
- **Builder choices confirmed** (2026-11-22, the human, triage). Kept as built:
  - Settings' step-role picker lists `creator`.
  - The old `[harness]` table stays in the file; a save leaves it.
  - Effort is cleared, not reset.
  - A typed model's `none` is stored as `effort = ""`.
  - A bare base with levels starts bare only by typing its id and `none`.
  - Model and effort reads run on the screen (5 s bound, cached); background only if it bites.
  - cursor-agent and opencode efforts are checked late.
  - Haiku's curated levels for the `claude` harness: `low`, `medium`, `high`.
  - opencode's levels from `opencode api --standalone model.list`; parsers pinned on captured output.
  - The detail pane's scroll keys are in the `?` overlay.
  - Settings' footer keeps one row; `enter` stays in `?`.
  - The Path fork's column is 1.
  - Palette rules 2 and 9 keep their amended wording.
  - `let` and `u32` share one colour.
  - The Catppuccin heading and important alert stay; they clear the floor.
  - `priority:medium` stays an alias for `normal`.
  - The filter's count gives way at 40 columns.
  - All five filter choices stay.
  - The config guard stays before the write.
  - Settings preview: the flag wins.
  - `ctrl-c` quits at once from Settings.
  - `r` drops the session's picks; the file is the restart point.
  - The footer names `c crew size`.
  - The session tag sits on the agent's name row.
  - Captions cut at 40 columns.
  - The builder's unit 72 choices stand.
  - The wheel moves the cursor, not a free viewport.
  - Mouse choices stand.
  - Comment depth 3; width floor 16.
  - The round heading wraps at 40 columns.
  - Item document and Settings colour choices stand.
  - FINISHED hides silently with the cursor above it: option (a). Its colours stand. A Path with only done tasks shows the FINISHED row.
  - To unit 82: effort offers `type a level…` after a failed read; the `?` popup grows to its content; `r` in Settings asks when unsaved, as `q` does.
  - To unit 82, spec notes: the priority row (`screens.md:188`) becomes `urgent · high · normal · low`; the filter's AND rule and the `/ filter` chip go in `screens.md`; a resized list's whole rewrite goes in `screens.md` § Settings; `HUGO_SESSION` goes in `docs/hugo/`.
  - Open, no unit: strike "Not ruled" at `docs/hugo/design-record.md:265`.
- **Triage rulings** (2026-11-22, the human). The recommended option of each:
  - Config read: hard errors stay for no orchestrator and unknown harness; unknown `[agents.<name>]` tables and odd crews load with a sidebar warning. → unit 81.
  - Inline tables and dotted keys stay text. → no change. (The ruling also asks for an "edit by hand" note on the line; no unit carries it.)
  - Fixture replay: add `session` to `Round` when the review writer lands. → with the review writer.
  - Flow files: drop the model pin, agent name only; hugo's defaults pin elsewhere. → unit 81.
  - The engine's dim on line numbers stays. → no change.
  - Plain identifiers keep `document.code`. → no change.
  - TOML: add a grammar (`two-face` or a bundled `.sublime-syntax`). → unit 83.
  - Blocked state: `AgentState::Blocked` in core and `semantic.warn` in hugo, now. → unit 81.
  - Round 1's findings: add via `mock-data/generate.py`, accepting snapshot churn. → unit 83.
  - The queue offers only claimable tasks; one claimable waiting task in the mock. → unit 81.
  - Settings table headers and comments are inert. → unit 82.

---

## THE FOLLOW-UPS

None blocking. Carried from the closed units.

- **2026-11-23, unit 93:** `mock-data/generate.py:2779` (the TSK22 note) still says
  the gate regenerates the column table. Fixing it needs a generator change and a
  regenerate. Recorded, not ruled.
- **2026-11-23, unit 93:** `make columns` writes nothing and has no regenerate
  target. The finding's fix line was the help text alone; a `columns-write`
  target was not added. Recorded, not ruled.
- **2026-11-23, unit 93:** `TocEntry.title` in the engine still carries a heading
  link's URL. Only the paint changed (F23). Recorded, not ruled.
- **2026-11-23, unit 92:** the fixtures hold no code-review task before its first
  external round, since the one-review-round shape is gone. A fixture would be
  needed if raul or hugo must read that state. Not done: it changes the fixture
  count and the pins. Recorded, not ruled.
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
  alternative was the nearest size. Recorded, not ruled.
- **2026-11-23, unit 90:** F2's option to drop only the offending row, not the
  branch, was not taken. The builder read it as optional. Recorded, not ruled.
- **2026-11-23, unit 90:** F6's test was not run against the pre-fix code, so it
  is not shown to fail without the fix. Recorded, not ruled.
- **2026-11-23, unit 90:** the `evidence.command` fixture check (D1) has no
  reverse test: no fixture holds a non-manual entry with an empty command. The
  reviewer accepted it. Recorded, not ruled.
- **2026-11-23, unit 89:** `ProjectLock::save` takes `&mut Task`, so the caller's
  copy follows the file's revision. The alternative `&Task` leaves a stale copy
  that the next save of the same object refuses. Open for the human to rule.
- **2026-11-23, unit 89:** F3 and F4 have no fail-before run. F3's test uses the
  new signature and `StoreError::Revision`, so it cannot compile on the old code.
  F4 removed the unlocked writers, so no runtime test can fail on the old API.
  Recorded, not ruled.
- **2026-11-23, unit 89:** `Store::duplicate_ids()` is in core, but nothing calls
  it. `project check` is a talita verb and waits until talita is opened.
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
  events instead is open. Recorded, not ruled.
- **2026-11-23, unit 87:** the creating-writers test
  (`two_writers_creating_at_once_take_no_duplicate_id`) fails unlocked in 5 of 6
  runs. It is sensitive, not deterministic. Recorded, not ruled.
- **2026-11-23, unit 87:** `config.rs` has its own `write_beside`, which takes no
  lock and is not atomic. Left alone. Recorded, not ruled.
- **2026-11-23, unit 87:** raul's id sort (`crates/raul/src/data.rs:122`) is not
  the core's numeric order yet. It is raul's follow-up, not core's.
- **2026-11-23, unit 87:** the Windows lock (std `File::try_lock`) is untested.
- **2026-11-23, unit 87:** the journal and runtime fixtures' byte-for-byte
  regeneration is not checked; only the struct round-trip is.
- **2026-11-23, unit 86:** `status` has no owner in `ownership.rs` or task-model
  § 2.5, and `roles.md:60` and `commands.md:38-42` now say transitions alone
  write it. Round two applied the reviewer's recommendation; no human ruling.
  Confirm, or revert both docs and make `Section::Status` runner-owned in
  `ownership.rs` with the TM line at 942.
- **2026-11-23, unit 86:** who owns `questions` is unsettled. `roles.md` says
  the creator, task-model § 2.5 says any role (append-only), and `commands.md`
  says `ask` is any and `answer` is the creator. `ownership.rs` follows
  roles.md. Recommended: split ask (any) from answer (creator) when T6 writes
  the question verbs; that needs a `Section` split and a § 2.5 change. Nothing
  calls `authorize` yet. Needs a ruling.
- **2026-11-23, unit 86:** a takeover in `executing` ignores overlays. ADR 0020
  is silent, so a blocked executing task can be taken over once its lease
  expires. Needs a ruling: refuse, or allow.
- **2026-11-23, unit 86:** `task cancel` does not release a live claim yet
  (ADR 0020). It belongs with the overlay verbs in T7.
- **2026-11-23, unit 86:** `release` requires a reason and stores none. The
  journal writer (T11) carries it.
- **2026-11-23, unit 86:** a mirror with neither evidence nor deferral writes
  `pending`, so a cleared deferral cannot stick. Recorded, not ruled.
- **2026-11-23, unit 86:** the claim operations take `now` as an RFC 3339
  stamp, but `transition()` still reads the clock itself. Recorded, not ruled.
- **2026-11-23, unit 86:** the optional `NotHolder` hint (a holder's `take`
  points to `renew`) was not taken. `take` returns `Held` alone. Recorded, not
  ruled.
- **2026-11-22, unit 85:** the round-filing session check (ADR 0017) is not in
  core. Core has no round-filing operation, so a round filed by the claim's own
  session is refused only at the reviewer transitions. talita's T10 should call
  the same check.
- **2026-11-22, unit 85:** `start` checks the holder and the live lease only.
  ADR 0020 asks for no session check there. Recorded, not ruled.
- **2026-11-22, unit 85:** the fixture replay in `crates/core/tests/mock_data.rs`
  replays review moves as the filer of the latest external round, and dates a
  replayed code-review phase at that round's `at`. The fixtures are unchanged.
  Recorded, not ruled.

- **talita's design is accepted** (2026-11-22, the human). `docs/talita/` and
  ADRs 0012–0020; `docs/talita/roadmap.md` holds the build order, units
  T1–T14. T1–T4 are built in core (units 84–87), with no talita code. T5
  onward waits on the human's word.
- **hugo renames `HUGO_SESSION` to `HUGO_CREW`** (2026-11-22, ADR 0015). It
  holds the crew's configuration, not an actor's session. A small hugo unit.
- **2026-11-22, unit 84:** `mock-data/README.md:115` still says TSK14's
  re-review cannot be stamped. It can now, through `review.re_reviews[]`.
- **2026-11-22, unit 84:** the `assert_ne!` added to
  `tsk14_is_handed_back_after_its_findings_are_fixed` was not run against the
  old fixture value, so it is not shown to fail without the fix.
- **2026-11-22, unit 84:** six of the twenty-two journal kinds have no fixture
  history: `started`, `released`, `resumed`, `reopened`, `unblocked` and
  `undeferred`. Every fixture's `Evidence.manual` is `false`, so no override is
  exercised. Recorded, not ruled.
- **2026-11-22, unit 84:** two ADR 0018 deviations stand. `approved` is still
  read from the `Approved by` note, and `remediated` keeps "answered the
  findings" where ADR 0018 says `after round <n>`. Both change the Overview's
  render, so they wait for a ruling.
- **2026-11-22, unit 84:** raul's icons for the thirteen new kinds
  (`crates/raul/src/overview.rs`, `event_icon`) are the builder's guess. No
  ruling.
- **2026-11-22, unit 84:** `orchestrator-01` is the builder's choice for the
  orchestrator's `crew[].actor`. No fixture names one.
- **2026-11-22, unit 83:** the TOML render test was not run against the old
  code, so it is not shown to fail without the grammar. The resolve assertion
  was probed against the YAML fallback.
- **2026-11-22, unit 83:** raul's Settings TOML painter
  (`crates/raul/src/settings.rs:1535`, `toml_spans`) is hand-written and does
  not use the engine grammar. It can take it later.
- **2026-11-22, unit 82:** `docs/tui/screens.md` has no section for the `?`
  overlay, so its sizing and scroll keys are unspecified. The builder and the
  reviewer both recommend a line under § Mouse or a new overlay section. The
  mouse wheel does nothing while the overlay is open; that is the builder's
  choice and is not specified either.
- **2026-11-22, unit 82:** the `?` overlay draws no scroll indicator when it
  overflows. Not in the brief; the orchestrator may want one.
- **2026-11-22, unit 81:** none of unit 81's tests was run against the code
  before its fix, so none is shown to fail without it.
- **2026-11-22, unit 81:** the sidebar's config warning is cut at the pane
  width with `…`. At 100 columns the table name stays visible; the full text
  is shown only in the test.
- **2026-11-22, unit 78:** `needs.rs:338` is unchanged. The done-when named
  it; the review found Needs rows are one line, so "Only a pane of one line
  shows the row alone" is accurate. Confirm it, or reopen to name the case.
- **2026-11-22, unit 78:** the atomicity pin
  (`a_two_line_row_is_drawn_whole_from_any_first_row`) pins current behaviour.
  It was not run against the code before the fix, so it is not shown to fail
  without it.
- **2026-11-22, unit 79:** the queue-pick proof is partial. `captioned_picks`
  fails on a fixed offset, but `queue_lines` still draws at `2 + index` for
  every runtime, so restoring the old offset there is not caught. No runtime
  shows it today.
- **2026-11-22, unit 79:** the footer drops whole chips with no mark: at 40 and
  60 columns `enter choose` goes. Recommended: a short `+N keys` marker, as
  TSK02 does for rows. Waits on a ruling.
- **2026-11-22, unit 79:** the width sweep's row check fires only when a glyph
  measures wider than ratatui draws it, because the buffer clips overflow. No
  mutation has shown it fail.
- **2026-11-22, unit 79:** `NOT_COUNTS` exempts five channel-gap phrases that
  no test recomputes, and thirteen contrast-ratio phrases that are exempt
  without a check that any test reads them.

## WHAT WAITS ON THE FLOW

- **The journal's writer** (grooming G2): the kinds are provisional and nothing
  writes `activity.json`.
- **The runtime record's writer:** nothing writes `runtime.json`, so the crew
  hugo shows is the mock's.
- **The orchestrator's job:** the orchestrator role has no job yet. hugo picks
  the model for the agent it will start; opening the herdr tab and starting it
  is its own unit, and it is the first step towards hugo driving the loop
  rather than only showing it. The crew is the roles present: three agents
  means one per role, two means the orchestrator reviews, one means the
  orchestrator builds and reviews.

**Waits on a live run:**

- **The opencode live check is not run** (2026-11-22, unit 64). An opencode role
  started through herdr should show its configured free model in its pane. It
  was optional and was not done.
- **The live runs are not done** (2026-11-22, unit 71). The parsers are pinned
  by captured `cursor-agent` and `opencode` output. The bounded reads and a
  live start through hugo or raul were not run. The cursor start form
  `--model <id>-<effort>` comes from the reviewer's live runs.
- **The freeze is not reproduced live** (2026-11-22, unit 69). The fix was
  measured on the mock data and pinned by a test that counts engine runs, not
  in a terminal. The folder read and the projection still run on every redraw,
  and on a large work folder they may still cost. The key drain in
  `crates/raul/src/main.rs` has no test: it lives in the interactive loop.
- **The journal:** every fixture's approved stamp is thirty minutes before its
  claim, and no fixture holds an approved task that has waited long; the kinds
  are provisional.
- **The Tasks nerd glyph is unchecked** (2026-11-22, unit 60). The unicode set
  draws the tab as `☑`, because `☰` measures two columns. The nerd glyph
  (U+F0AE7) was not checked against a font; no nerd font is installed here.

**Where the flow's files live** is settled, decided 2026-11-21 and recorded in
`docs/decisions/0011-flow-files.md`: the project's role configs move to
`coding-crew/flow/` when the project is managed, `.orchestration.json` stays at
the repository root because the global skill reads it there, the findings live
inside the task rather than in `/tmp`, and the task folder replaces this file as
the orchestrator's memory. The move waits for the writers. The role configs
name no model since unit 81; the model defaults live in `crates/core/src/config.rs`
(pinned by core's own test) and in the constants of `mock-data/generate.py`,
which no test pins.

**Deferred by the human:** the light themes — our `light` preset and the light
variants of the families (Catppuccin Latte, Dracula Alucard, Nord Light).
**Out of scope:** talita's own verbs, until it is asked for explicitly.

---

## LESSONS THAT STILL BIND

- **A builder must never ask interactively** (2026-11-22). A Haiku builder
  opened a question dialog nobody watched, and the loop kept dispatching
  rounds past it. herdr-crew's builder rules now say: decide
  within the docs, or leave it and put the question in the report; and the
  loop stops when a builder is blocked.
- **A builder cannot `git checkout`** under auto mode — the classifier treats
  it as destructive. A revert the reviewer asks for is the orchestrator's to
  run, after checking the diff holds only that round's work.

- **Stage by name.** Never `git add -A`, never `git add .` — a blanket stage
  sweeps another agent's in-flight work into the commit. It happened twice.
- Close tabs by tab id, only your slug's; wait on files via `await.sh`, never a hand-timed close.
- Round files live in the session's round folder (scratchpad) and do not survive the machine.
- `await.sh` reports a blocked agent (exit 2) and prints its pane.
- Watch the disk: prune sweep output between units.
- The builder is Claude Haiku (effort high), the reviewer Claude Opus (effort medium), via herdr-crew.
