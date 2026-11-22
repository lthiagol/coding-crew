# coding-crew — orchestration state

Updated 2026-11-22 by the orchestrator, after unit 67.
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

Queued in `docs/plan.md`, in order. Units 49–67 and 63b are closed; their record is in
`DONE.md`.

- 68 · The document in colour (L)

---

## DECISIONS OF RECORD

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
- **An absent effort is a gap** (2026-11-22, unit 65 review). An agent with no
  `effort` is not yet picked: `s` refuses to start over it, and the sidebar
  says `pick an effort` on the warning slot. `task-model.md` and `screens.md`
  say so. This replaces the earlier note that an absent effort runs on the
  harness's default.
- **Recessed surfaces** (2026-11-22, unit 67). The human ruled hugo's sidebar
  surface a step darker than the base, not lighter, so the 3:1 floor holds in
  every colour preset. The surface is the base mixed 30% toward black; the
  selected tint is the surface mixed 10% toward the accent.

---

## THE FOLLOW-UPS

None blocking. Carried from the closed units.

- **Store discovery accepts any folder named `coding-crew`** (2026-11-22,
  review F4). A checkout of this repository is itself discovered as a
  project, because ADR 0010 says the nearest `coding-crew/` owns the result
  with or without `work/`. Left as a known limit; changing it means amending
  ADR 0010 to require a marker — the human's call.
- **Settings: inline tables and dotted keys are text** (2026-11-22, review of
  unit 63). `[raul] tasks = { … }` and `tasks.sort = …` are valid and saved
  into, but drawn as text with no stop.
  Each value needs its own stop and its own cursor on a shared line, or the
  line is split. Design call for the human; `docs/tui/screens.md` § Settings
  says so.
- **The fixture replay cannot check reviewer independence** (2026-11-22). An
  external round records its reviewer's actor id, not a session, so
  `fixtures_keep_their_gates` replays the review with an invented session.
  The fix adds a `session` to `Round` — a model change, for the human.
- **opencode's effort levels are assumed** (2026-11-22, unit 64): `low medium
  high`, as `#variant` suffixes. Only the free `opencode/*` models are listed
  here, so no provider's variants could be read. The human confirms the list.
- **The flow files name claude ids** (2026-11-22, unit 64). `flow/builder.json`
  and `flow/reviewer.json` are in opencode's format, and they now carry
  `claude-haiku-5-5` and `claude-opus-5-5` so the pin test matches the new
  defaults. opencode does not know those ids. The human chooses: keep the pin,
  or drop the model pin and let the files name only the agent.
- **An unknown harness makes the file unreadable** (2026-11-22, unit 64). An
  agent naming a harness outside the table, such as `aider`, or an effort its
  harness does not take, fails the read: the defaults show and the file is left
  alone. The human confirms the rule.
- **The old `[harness]` table stays in the file** (2026-11-22, unit 64). A save
  leaves it as the human's text. Agents name their own harness, so it is inert.
  The human decides whether it is removed.
- **The opencode live check is not run** (2026-11-22, unit 64). An opencode role
  started through herdr should show its configured free model in its pane. It
  was optional and was not done.
- **The screen mock still shows old model names** (2026-11-22, unit 64).
  `docs/hugo/screens.md`, the ASCII pictures near lines 22–31, is illustrative
  and was left as it was.
- **The reset level is `medium`** (2026-11-22, unit 65). A harness switch that
  the new harness does not take resets the effort to `medium`, the level the
  table now names as its default. The human confirms it or picks another.
- **Should the effort list offer `default`?** (2026-11-22, unit 65). The list
  has no entry that clears an effort, so an agent cannot be set back to none.
  The human decides whether it should.
- **A harness switch fills an absent effort** (2026-11-22, unit 65).
  `switch_harness` sets the harness default when the agent has no effort. That
  is the switch rule, not the gap rule. The human confirms it reads right.
- **`skill.rs` still writes `effort default`** (2026-11-22, unit 65).
  `crates/hugo/src/skill.rs:128` and its test pin it. The line is unreachable,
  since the crew text is built only after the `s` gap check. Left alone to keep
  unit 65 to its findings. Align it or leave it.
- **Esc from the id input drops the catalogue's error** (2026-11-22, unit 65).
  After a model catalogue fails, `esc` in `type an id…` returns to the list
  without the error line. Left as it was.
- **cursor-agent's list and the opencode list are untested live** (2026-11-22,
  unit 65). Both were exercised with a fake tool only.
- **The effort ruling is not in the design record** (2026-11-22, unit 65).
  `docs/hugo/design-record.md` has the harness and model rule but not the
  effort gap. Add it there at the next edit to the design record.
- **The detail pane's scroll keys are in a `?` overlay** (2026-11-22, unit 66).
  The footer is `esc close · ? help · q quit`, as the unit's item 2 says, and
  the scroll keys moved to the overlay. The build took this without asking.
  The human confirms it, or puts the scroll keys back in the footer.
- **Settings' footer lacks `enter edit` and `←→ change`** (2026-11-22, unit
  66). The footer reads `w save · u undo · r reload`, as the brief's example
  does. Both keys are in the `?` overlay. The human decides whether they join
  the footer.
- **The `?` popup is twenty rows** (2026-11-22, unit 66). `enter open` takes a
  line, so at twenty rows the Tasks help drops `g/G`. The human decides whether
  the popup should be taller.
- **hugo has no width sweep** (2026-11-22, unit 66). The footer sweep covers
  raul at 40–120 columns. hugo's footer is pinned at 40, 60 and 100 only.
- **The palette rules 2 and 9 were amended for unit 67** (2026-11-22, unit 67).
  Rule 2 makes the selection a tint and rule 9 allows two derived backgrounds.
  The human confirms the amended wording in `docs/tui/palettes.md`.
- **The derivation shares are the builder's** (2026-11-22, unit 67). The brief
  said "about 25%"; the build uses 30% for the surface and 10% for the tint,
  because 25% met neither the floor nor the step. The human confirms the figures.
- **Dracula's selected tint clears 3:1 by a hair** (2026-11-22, unit 67). The
  comment grey reads 3.00004:1 on the tint. The words test pins it, so a palette
  change that drifts it will fail the gate.
- **The crew has no blocked state** (2026-11-22, unit 67). `AgentState` holds
  idle, executing and reviewing only (`crates/core/src/runtime.rs`), so hugo
  paints no `semantic.warn` for a blocked agent. Recommendation: add the state
  in core first, then wire the colour.

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
- **Backlog keeps its own announcement code** (2026-11-22, unit 60). Tasks and
  Needs you share `crates/raul/src/announce.rs`; the Backlog still has a copy
  with a dropped-header variant. Moving it onto `announce.rs` is the follow-up.
- **The Tasks nerd glyph is unchecked** (2026-11-22, unit 60). The unicode set
  draws the tab as `☑`, because `☰` measures two columns. The nerd glyph
  (U+F0AE7) was not checked against a font; no nerd font is installed here.
- **`priority:medium` is an alias for `normal`** (2026-11-22, unit 61). The
  spec's table lists `low · medium · high`; the model has `urgent · high ·
  normal · low`. The filter reads `medium` as `normal`. The human decides
  whether the spec row changes or the alias goes.
- **The filter's count gives way at 40 columns** (2026-11-22, unit 61). While
  typing at 40 columns the count takes 27 columns, leaving about nine
  characters of the typed text. Recommendation: leave it. A shorter count when
  narrow would change the spec's wording.
- **Filter choices to check** (2026-11-22, unit 61). Plain words are AND'd; the
  spec is silent. `status:review` matches `code-review` by a hyphen-separated
  word. `tag:` and `parent:` are exact and case-sensitive. `esc` clears a
  filter before it quits. The footer has a `/ filter` chip the spec does not
  list.
- **The config guard has no test** (2026-11-22, unit 62). No current file shape
  reaches the refusal that compares the written text with the config. It runs
  before the write, so a mismatch leaves the file alone. The human says whether
  it should run after.
- **A resized list is written whole** (2026-11-22, unit 62). When `agents` or
  `order` changes length, `save` rewrites the list, and comments inside it are
  lost. A list of the same length keeps its comments.
- **The key table's words are gone** (2026-11-22, unit 63). `keys.rs` no longer
  holds `key` and `says`, since nothing read them. The footer and the help name
  keys by hand. Making the help read the table is a larger change, not done.
- **The Settings preview ignores a flag** (2026-11-22, unit 63). With
  `--theme dark`, a pick of `nord` previews `dark` and says `(overridden by
  --theme dark)`. The spec says a pick repaints the screen. The human decides
  which. The switch is `Panel::theme` and `Panel::icons`.
- **Quitting from Settings** (2026-11-22, unit 63). `q` and `esc` ask when there
  are unsaved changes. `ctrl-c` quits at once, and `r` drops unsaved changes
  without asking. Check both against `docs/tui/screens.md` § Settings.
- **The debug-build numbers came from a wrapper** (2026-11-22, unit 63b).
  `/usr/bin/time` is not installed here, so the before and after figures are
  from a Python wrapper that reads `ru_maxrss`. Each figure is one run.

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

- **A builder must never ask interactively** (2026-11-22). A Haiku builder
  opened a question dialog nobody watched, and the loop kept dispatching
  rounds past it. The runner skill and the builder rules now say: decide
  within the docs, or leave it and put the question in the report; and the
  loop stops when a builder is blocked.
- **A builder cannot `git checkout`** under auto mode — the classifier treats
  it as destructive. A revert the reviewer asks for is the orchestrator's to
  run, after checking the diff holds only that round's work.

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
