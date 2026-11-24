# coding-crew — orchestration state

Updated 2026-11-24, evening.
Branch `main`, remote `https://github.com/lthiagol/coding-crew.git`.
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

**talita is built** (the human, 2026-11-23): T5, T6, T7, T8, T9, T10, T11,
T12, T13 and T14 of `docs/talita/roadmap.md` are closed. T8 (claim) and T9
(steps at work and evidence) have the word too (the human, 2026-11-23):
they come before the dogfood, which needs claims and evidence. T14 (docs and
skills) has the word as well: the docs and the flow skills speak talita's
grammar, and a test pins them to it.
Then agents dogfood it into a sandbox project under `examples/`, which
replaces random fixtures in `mock-data/`.

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

### Where the last session stopped (2026-11-24, evening)

- `main` holds this session's work, not yet pushed: DF3–DF6, F1, F2, O1, O2
  (O2a, O2b, O2c). `make check` green after each merge. The worktree
  `../coding-crew-cc2` (branch `cc2/followups`) is merged into `main` and can
  be removed.
- Crew: Haiku builders (effort high), Opus reviewer (effort medium), herdr-crew
  slug `cc` on `main`, `cc2` in the worktree for parallel units.
- O2 is built, not yet run live: check L5 in `docs/plan.md`.
- Waiting on the human: push; L5 (hugo's `s` on a sandbox, one task to
  `done`); then DF, this repository on its own `coding-crew/`; unit 101 waits
  for the `v0.1.0` tag. Ask; don't pick.
- Lesson: a Haiku builder hit a permission prompt on a Python heredoc editing
  the Makefile (the classifier misread `$$` as `rm`); auto mode allowed it in
  time. When `unit.sh` stops on a blocked builder that then finishes, resume
  the review over its report rather than spawning a new builder.

---

## THE OPEN UNITS

Queued in `docs/plan.md`, in order. Units 0–100, 63b and 70b are closed; their
record is in `DONE.md`. Nothing is in flight; the tree is clean. The crew
wound down at 13:55 on 2026-11-23 for the weekly limits reset. The talita
queue (T5 → T6 → T7 → T10 → T11 → T13, then the `examples/` dogfood) stopped
after T6: T5 and T6 are closed. Since then the opencode crew (deepseek-v4.1-flash builder, glm-5.3
reviewer, slug `oc`) fixed the macOS catalogue failure and built T7. The long-path sweep is closed. T13 is closed too: a MiniMax orchestrator on branch
`mm/talita` (a trial of an opencode orchestrator) ran the opencode crew again — deepseek-v4.1-flash builder,
glm-5.3 reviewer, slug `mm` — and the review was clean in one round. T10 is closed too, on the same branch and the same crew, one clean round. T11 is closed as well: the journal writer, two rounds — the review found the two-file window a failed journal write leaves, now on the record in ADR 0018, and the cancel row of `commands.md`. T8 is closed: the claims, one clean round; the crew-row side effects wait for T12. T9 is closed too: the steps at work and the evidence, two rounds — the review found the override on a deferred criterion, masked by the deferral, and a stale `--expect-revision` checked only after the command had run. The dogfood run is closed too: two sandbox projects under `examples/`, written by talita alone, and `examples/DOGFOOD.md` with nine entries. Unit T11a is closed: `task approve --instructed-by` writes the human's name into the `approved` note, and the fifth round's escalation is ruled intended (2026-11-23). Unit DF1 is closed too: the six `examples/DOGFOOD.md` entries of the run — the finish and escalate remedies, the human's route, `task list --overlay`, the readiness's `overlay.clear`, and `project init`'s identity — are fixed. Unit DF2 is closed as well: `question answer --instructed-by` writes the human on the question's own `instructed_by`, and every journal event carries the call's optional `--model`/`--harness` (their envs too); the sandboxes' journals and task files are untouched. All nine `examples/DOGFOOD.md` entries are closed: six fixed by DF1, one by T11a, one ruled intended, one closed by DF2's ruling on the actor. T12 is closed too: the runtime writer — `run`, `crew` and `queue` write `runtime.json`, every talita write recomputes its derived fields, the task verbs make their crew rows certain, and the record talita writes for the mock's state is the mock's record, field for field (only the run's `started_at` and the rows' `since` are this run's). Unit T12a is closed too: the record's last write is its own optional `updated_at` — talita stamps it on every write of the record, `run show` measures staleness from it against the longest lease the crew holds, a record without it is never stale, and the mock carries it through the generator. T14 is closed too: the skill docs and the flow skills speak talita's noun–verb grammar, the drift table's doc-fix rows are closed, and a test pins the docs to the grammar. On 2026-11-24 the second dogfood run (DF3) and its fixes (DF4–DF6), the follow-ups F1 and F2, and O1 and O2 closed; their record is in `DONE.md`. Next: L5, the orchestrator live, then DF. Unit 101 waits for the human's `v0.1.0` tag.

| phase | units | needs |
|---|---|---|
| 1 · cleanup | ~~95 hugo on PATH~~ · ~~96 docs drift~~ · ~~97 consistency~~ · ~~98 test hardening~~ · ~~99 rulings in the UI~~ | nothing; ready |
| 2 · distribution | ~~100 version and releases~~ · 101 Homebrew formulae | 101 needs a first tag. The taps exist: `lthiagol/homebrew-tap`, `lthiagol/homebrew-dev`; the `homebrew` project skill publishes to them |
| 3 · talita | ~~T5–T14~~ | closed |
| 3 · the orchestrator | ~~O1 its job, designed~~ · ~~O2 its job, built~~ | L5, the live run |
| 3 · dogfooding | DF this repository on its own `coding-crew/` | T12 and O2 |
| 4 · live checks | L1–L4 in `docs/plan.md` § Live checks | a human, a machine or a font |

Units in one phase that touch different crates can run in parallel worktrees:
96 (docs, theme comments, generator note) and 97 (core, raul, engine) do not
overlap; 98 and 99 both touch raul and run after them. Deferred: the light
themes and a theme file on disk (`docs/plan.md` § Deferred).

---

## DECISIONS OF RECORD

- **The orchestrator's job is accepted** (2026-11-24, the human). Every
  recommendation of `docs/hugo/orchestrator.md` § Open questions, Q1–Q14:
  the orchestrator commits one task at `done`; a job is done when its file is
  in and the status agrees; agents are briefed by `hugo --skill <role>` plus a
  brief path; the orchestrator stops writing `TODO.md`; the orchestrator's row
  takes the review state (`crew assign --review`) and ADR 0005 follows the crew
  rule; the task's `max_rounds` binds; hugo mints the orchestrator's session;
  `task resume` and `task reopen` release the claim; a lost reviewer is
  re-prompted, else the run stops; after `needs-human` the run continues;
  the pane stays up after a run; `make install` ships talita; the gate runs
  three times; the next orchestrator stops a dead run.
- **The review state** (2026-11-24, the human): `open-findings` when any
  finding is open; `pending-review` in `code-review` with no external round
  filed since entering it; `clean` when the latest external round is filed and
  none is open; none otherwise. `task-model.md` § 12.
- **Work and evidence** (2026-11-24, the human, the second dogfood run): a
  failed step is retried with `step begin` before `step done`; `evidence
  override` on a criterion its command already proves is refused; `task groom`
  refuses a criterion with no verification.

- **Staleness is the record's `updated_at` against the longest lease**
  (2026-11-23, the human). `run show` calls the record stale when the time
  since its last write — the record's own optional `updated_at`, stamped by
  talita on every write — passes the length of the longest lease the crew
  holds. The file's modification time plays no part: the record lives in the
  project's folder, so a clone or a copy would make a dead run look fresh. The
  lease's length is the claim's `lease_expires_at` minus its `at`; a lease that
  has already expired still counts (the crashed case), and a record with no
  `updated_at`, or with no held lease, is never stale. `run start` writes a new
  record (ADR 0018).
- **The review budget's last round escalates** (2026-11-23, the human). When
  the budget is spent, the last round escalates to a human with its findings
  open; that is the intended behaviour, not a defect. `examples/DOGFOOD.md`
  keeps the rule; no code change.
- **`project init` needs no identity** (2026-11-23, the human). It creates
  `coding-crew/work/` and writes no task and no journal, so there is nothing
  to attribute. It is the one write that needs none; ADR 0015 names the
  exception.
- **The actor is not recorded** (2026-11-23, the human). The journal's `by`
  stays the role word; the role and the model are what matter, and the harness
  is an extra. Every event carries an optional `model` and `harness`, from
  `--model`/`--harness` or `TALITA_MODEL`/`TALITA_HARNESS`, flag over env
  (ADR 0018).
- **A question's answer names its human** (2026-11-23, the human). The name
  lives on the question's own `instructed_by`, because questions are not
  journaled: `question answer --instructed-by` writes it. The journal's note
  stays the home for `approve`, `resume` and `reopen` (ADR 0015).
- **talita's design is accepted** (2026-11-22, the human). `docs/talita/` and
  ADRs 0012–0020; `docs/talita/roadmap.md` holds the build order, units
  T1–T14. T1–T4 are built in core (units 84–87), with no talita code. T5
  onward waits on the human's word.
- **Grooming of 2026-11-23** (the human). The order is cleanup (95–99), then
  distribution (100–101), then talita (T5–T12) and the orchestrator's job (O1,
  O2), then dogfooding (DF). Every recommended ruling was taken:
  `ProjectLock::save` keeps `&mut Task`; transitions alone write `status`;
  `questions` splits into ask (any role) and answer (the creator) at T6; a
  takeover is refused while an `executing` task is blocked; ADR 0018 is
  followed (`approved` and `remediated` as it words them); the footer marks
  dropped chips `+N keys`; the `?` overlay is specified and shows a scroll
  indicator; raul's Settings shows a missing `harness` or `effort` as an empty
  row that can be set; an odd crew paints no size; the journal keeps refusing
  writes over unreadable events; raul's event icons, `orchestrator-01` and
  `needs.rs:338` stand. Distribution is a tagged release plus Homebrew formulae built from source: `coding-crew` in `lthiagol/homebrew-tap`, `coding-crew-dev` in `lthiagol/homebrew-dev`, published by `.claude/skills/homebrew`.

- **hugo is found on PATH** (2026-11-23, the human). Agents run `hugo
  --skill <role>`, so `hugo` must be on PATH; Homebrew is the install that
  gives it. A source build is told which folder to add, and `s` refuses to
  start without it. Unit 95. It replaces the live test's follow-up, which asked
  for hugo's own path in the prompt.

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

- 2026-11-23 · the fixtures hold no code-review task before its first
  external round (unit 92's follow-up, placed in T10). T10 covers that state
  in talita's own tests; the mock still holds no such task, because it
  changes the fixture count and raul's and hugo's pins. Add it when one of
  them must read the state.

Grooming on 2026-11-23 placed every earlier follow-up: the buildable ones
are in units 96–99 of `docs/plan.md`, the talita-bound ones in the units of
`docs/talita/roadmap.md` they belong to, the live checks in `docs/plan.md`
§ Live checks, and the rest are archived in `DONE.md` § Follow-ups closed by
grooming. A new follow-up goes here, dated, until it is placed.

---

## WHAT WAITS ON THE FLOW

- **The orchestrator's job** is built (O2): hugo starts the orchestrator with
  its talita session, the shipped skills run the loop of
  `docs/hugo/orchestrator.md`, and talita writes the journal and
  `runtime.json`. It has not yet driven a task live: L5.

**Waits on a live run:** L1–L5 in `docs/plan.md` § Live checks.

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
