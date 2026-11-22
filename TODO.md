# coding-crew — orchestration state

Updated 2026-11-22 by the orchestrator, after unit 77.
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

Queued in `docs/plan.md`, in order. Units 0–77, 63b and 70b are closed; their record is in
`DONE.md`.

None. Every Ready row is closed.

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

---

## THE FOLLOW-UPS

None blocking. Carried from the closed units.

- **The design record still says one agent means the human reviews** (2026-11-22,
  unit 70). `docs/hugo/design-record.md:27` predates the 3/2/1 rule. Left for
  the human, since the design record holds rulings.
- **An unknown `[agents.<name>]` table is ignored without a message** (2026-11-22,
  unit 70). `[agents.creator]`, say, loads silently. The human decides whether
  it warns.
- **A config with no orchestrator is unreadable** (2026-11-22, unit 70). The
  defaults show and the file is left alone; the old shape loaded. The builder
  chose this and the human has not confirmed it.
- **Settings' step-role picker lists `creator`** (2026-11-22, unit 70). It sits
  beside orchestrator, builder and reviewer. The human confirms whether the
  picker should offer it.
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
- **Effort is cleared, not reset** (2026-11-22, unit 71). A model or harness
  change that the new model does not take clears the effort. It no longer
  resets to `medium` or fills the switch's default. This replaces the unit 65
  entries on the reset level and the switch fill. The human confirms it.
- **A typed model's `none` is stored as `effort = ""`** (2026-11-22, unit 71).
  Not `"none"`, because real `none` variants (opencode `hy4-flash-free`,
  cursor's `…-none`) would collide with it. An empty effort starts with no
  effort flag. The builder's choice; the orchestrator confirms it.
- **A bare base with levels has no entry that starts it bare** (2026-11-22,
  unit 71). `gpt-5.3-codex` is listed bare and at levels, and a listed model
  shows only its levels. It starts bare only by typing its id and picking
  `none`. The human decides whether a listed base should offer `none`.
- **Should the effort list offer `default`?** (2026-11-22, unit 65). The list
  has no entry that clears an effort, so an agent cannot be set back to none.
  The human decides whether it should.
- **The model and effort reads run on the screen** (2026-11-22, unit 71). raul
  reads synchronously, bounded at 5 s and cached per session, so a slow
  `cursor-agent --list-models` can hold Settings on the first `enter`. hugo
  reads in the background. Moving raul's read to the background is the
  follow-up.
- **A failed read leaves a typed model with no effort choice** (2026-11-22,
  unit 71). When the effort catalogue read fails, the list is empty, so a typed
  model cannot pick an effort until the read works.
- **cursor-agent and opencode efforts are checked late** (2026-11-22, unit 71).
  The config read checks claude's levels only. A cursor or opencode effort is
  checked when its list is read (raul's save), and nowhere at start, so a wrong
  level fails at the harness. The human confirms the rule.
- **Haiku's levels** (2026-11-22, unit 71). The curated table gives Haiku `low`,
  `medium`, `high`. `cursor-agent --list-models` also lists `xhigh` and `max`
  for `claude-haiku-5-5`. The human confirms which is right.
- **opencode's levels come from an undocumented operation** (2026-11-22, unit
  71). `opencode api --standalone model.list` gives the variants, and
  `opencode models` has no variant flag. The human confirms relying on it.
- **The live runs are not done** (2026-11-22, unit 71). The parsers are pinned
  by captured `cursor-agent` and `opencode` output. The bounded reads and a
  live start through hugo or raul were not run. The cursor start form
  `--model <id>-<effort>` comes from the reviewer's live runs.
- **`skill.rs` still writes `effort default`** (2026-11-22, unit 65).
  `crates/hugo/src/skill.rs:128` and its test pin it. The line is unreachable,
  since the crew text is built only after the `s` gap check. Left alone to keep
  unit 65 to its findings. Align it or leave it.
- **Esc from the id input drops the catalogue's error** (2026-11-22, unit 65).
  After a model catalogue fails, `esc` in `type an id…` returns to the list
  without the error line. Left as it was.
- **The effort ruling is not in the design record** (2026-11-22, unit 65).
  `docs/hugo/design-record.md` has the harness and model rule but not the
  effort gap. Add it there at the next edit to the design record.
- **The detail pane's scroll keys are in a `?` overlay** (2026-11-22, unit 66).
  The footer is `esc close · ? help · q quit`, as the unit's item 2 says, and
  the scroll keys moved to the overlay. The build took this without asking.
  The human confirms it, or puts the scroll keys back in the footer.
- **Settings' footer lacks `enter` and `←→`** (2026-11-22, unit 66; amended
  unit 69). The footer reads `w save · u undo · r reload`. `←→` now switches
  tabs on Settings, and `enter edit or pick` is in the `?` overlay. An `enter`
  chip pushed the footer onto two rows at 38 columns and broke a pinned test,
  so it was left out. The human decides whether it joins the footer, and
  whether the footer may take a second row.
- **The Path fork's column is the builder's call** (2026-11-22, unit 69). The
  fork line `╵` moved from column 2 to column 1, with the trunk and the bar. The
  spec sketch in `docs/tui/screens.md` had it one column right, so the build
  changed the sketch too. If the offset was meant, revert `fork_line` in
  `crates/raul/src/path.rs` and the sketch. The human confirms.
- **The freeze is not reproduced live** (2026-11-22, unit 69). The fix was
  measured on the mock data and pinned by a test that counts engine runs, not
  in a terminal. The folder read and the projection still run on every redraw,
  and on a large work folder they may still cost. The key drain in
  `crates/raul/src/main.rs` has no test: it lives in the interactive loop.
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
- **The engine dims the code's line numbers** (2026-11-22, unit 68).
  `crates/engine/src/markdown/blocks.rs:317` adds `Modifier::DIM` to the
  gutter, so a terminal paints the numbers dimmer than the grey the word-floor
  pin checks (`document.list.3`, 5.07:1 in dark). Changing it is an engine
  change and a ledger entry in `crates/engine/VENDORED.md`; left alone. The
  human decides whether the numbers keep the dim.
- **Plain identifiers take the code's ink** (2026-11-22, unit 68). A token no
  scope names — a plain identifier, a generic parameter in `Vec<u8>` — paints
  `document.code`, warm orange in dark, not the variable role. `palettes.md`
  records that default. The human decides whether it should be the variable
  colour.
- **`let` and `u32` share one colour** (2026-11-22, unit 68). The bundled
  grammars scope a primitive type as `storage`, as they scope `let`, so both
  paint as keywords, and the type role takes no `storage.type`. `palettes.md`
  records the trade. The human confirms it.
- **TOML has no highlighting** (2026-11-22, unit 68). The bundled syntect set
  has no TOML grammar, so a ```` ```toml ```` block paints in plain ink, and the
  paint test uses YAML. A grammar is a dependency change; the human takes it or
  leaves it. `palettes.md` § Not yet has the entry.
- **The Catppuccin heading and important alert are close** (2026-11-22, unit
  68). `#cba6f7` and `#b4befe` clear the 20-channel apartness floor by a gap of
  24, but read as one hue family. Not changed; the human decides whether the
  pair moves.
- **The apartness check has no near-duplicate case** (2026-11-22, unit 68).
  `the_headings_and_the_alerts_are_apart` has a 20-channel floor, but no
  fixture feeds it a near-duplicate pair, so the floor is proven by reading.
- **Dracula's selected tint clears 3:1 by a hair** (2026-11-22, unit 67). The
  comment grey reads 3.00004:1 on the tint. The words test pins it, so a palette
  change that drifts it will fail the gate.
- **The crew has no blocked state** (2026-11-22, unit 67). `AgentState` holds
  idle, executing and reviewing only (`crates/core/src/runtime.rs`), so hugo
  paints no `semantic.warn` for a blocked agent. Recommendation: add the state
  in core first, then wire the colour.

- **The document overflows below 16 columns** (2026-11-22). A comment's id,
  nested at the third list depth (the review's comments sit one level under
  their findings), is wider than a pane under 16 columns. The header table is
  11 columns wide, so an inner width of 10 (a 12-column pane) overflows; a
  quoted code line overflows at 11 inner (13 columns). The detail test checks
  the body at 12 and the overflow only from 16 up.
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
- **`r` drops the session's picks** (2026-11-22, unit 70b). A reload starts the
  session over from the file. The design record says "Not ruled". The human
  decides whether picks survive `r`.
- **The footer names `c crew size`** (2026-11-22, unit 70b). The pinned hugo
  screens changed on the footer line only. The human confirms the footer.
- **A crew of neither one, two nor three roles** (2026-11-22, unit 70b). Such a
  file, orchestrator with a reviewer say, opens the selector on size one. A
  size pick sets the crew to that size's roles. The human decides whether it
  should be unreadable, as a file with no orchestrator is.
- **The session tag sits on the agent's name row** (2026-11-22, unit 70b). A
  tag on each field row does not fit the 30-column pane. Changed field keys
  are strong. The human confirms the placement.
- **`HUGO_SESSION` is a contract** (2026-11-22, unit 70b). `s` sets it as JSON
  on the orchestrator's tab only, and `hugo --skill` reads it, else the file.
  The name and shape are the builder's; the human confirms them.
- **The flow suites did not run on unit 70b** (2026-11-22, unit 70b). The
  `orchestration-flow` skill is not installed here, so `make check` skipped
  them. The gate was green without them.
- **Round 1's findings are in no fixture** (2026-11-22, unit 72). The mock's
  TSK14 has one self round and one open high; the story's three external
  findings from round 1 are not in `activity.json` or the work folder, so hugo's
  findings and raul's TSK14 disagree. Adding them changes raul's snapshots and
  the mock gates. The human decides.
- **The queue holds unclaimable work** (2026-11-22, unit 72). TSK07 is
  approved, but its dependencies TSK04 and TSK05 are only groomed, so `s` would
  start a task it cannot claim. The human decides whether the queue offers only
  claimable work, which means another waiting task in the mock.
- **Captions cut at 40 columns** (2026-11-22, unit 72). The caption for the
  findings block is cut with `…` at 40 columns. A shorter caption when narrow
  would change the wording the brief names.
- **The builder's unit 72 choices await confirmation** (2026-11-22, unit 72).
  `tab` moves focus between the sidebar and the queue; `r` clears the marks; the
  record's queue holds done and held tasks, and the pane offers only the waiting
  ones; the budget and findings are the task in flight's; a cut line in hugo now
  ends in `…` (this touches every fitted line, not only the new ones); the
  findings noun is singular at one; the crew caption is the builder's wording.
- **Stale round wording outside the unit** (2026-11-22, unit 72). `18 of 45` and
  `17 open findings` remain in `docs/tui/redesign.md:107,157-158` and
  `docs/hugo/tui-proposals/minimax/06-review-budget.html:73`. `design-record.md:100`
  and `task-model.md:613` say "unclaimed work"; the builder left them, unsure
  whether that is the model's term. Check and correct.
- **A click on a Settings table header selects the value below it** (2026-11-22,
  unit 73). `settings::blocks` puts a header's lines with the value under them.
  The round-1 report recommends headers and comments inert, so only value lines
  select; the round-2 review recommends keeping it, as the block model the keys
  use does. The human decides.
- **The wheel moves the cursor, not a free viewport** (2026-11-22, unit 73). The
  window follows the cursor, so a list scrolls because the selection moves. A
  pure viewport scroll would snap back on the next draw. The human confirms.
- **Mouse choices to confirm** (2026-11-22, unit 73). A second click on the
  selected row opens it, so on a fresh tab the first click on row 1 opens it.
  In hugo, a second click on a queue row marks it, as `space` does; the brief
  said only "selects". A wheel over hugo's sidebar or queue takes the focus, as
  a click does. A second click in Settings edits, as `enter` does. At 40
  columns raul's tab bar takes no clicks.
- **The mouse wait and the panic restore are checked by reading** (2026-11-22,
  unit 73). The wait in raul's and hugo's event loops needs a terminal, so it
  has no test. The restore on panic was checked by reading `leave_terminal` in
  `crates/core/src/screen.rs`; no harness forces a panic.
- **hugo's queue picks are pinned, not proven** (2026-11-22, unit 73). The mock
  gives the same lines under the old offset, so the test shows the behaviour
  and not the fix.
- **The comment depth is the builder's reading** (2026-11-22, unit 74). The
  done-when says comments sit "one level deeper" than their finding's details,
  so they are at depth 3, under a `comments:` label at depth 2. If it meant one
  deeper than the finding, the items move to depth 2. The human decides.
- **The width floor is 16, not 14** (2026-11-22, unit 74). A comment's id at
  depth 3 needs 14 content columns, so a 14-column pane overflows. The floor
  moved to 16 in `screens.md` and the test. The orchestrator confirms it, and
  the human decides whether 14 must hold; that needs comments at depth 2 or a
  different id rendering.
- **The round heading wraps at 40 columns** (2026-11-22, unit 74). Four parts
  (kind, number, verdict, reviewer) run over two lines at 40. The render test
  allows it. The human confirms it is acceptable.
- **Item document choices to confirm** (2026-11-22, unit 75). The item header
  omits tags, which the task header shows, and the History section is always
  present, with `_none_` for an empty time. The brief said history "when the
  model has them". The human confirms both.
- **Settings colour choices to confirm** (2026-11-22, unit 76). A key is
  painted as a variable, not a tag, though the palette lists attribute names
  under tag. A section header takes the heading role. Read-only lines of the
  file get the same colours, and values the file leaves out stay dim. The
  human confirms all four.
- **`Theme::document_style` rebuilds the document palette on every call**
  (2026-11-22, unit 76). Not cached. The builder judged it cheap, pure
  arithmetic. No action unless a profile shows it.
- **FINISHED hides silently with the cursor above it** (2026-11-22, unit 77).
  When every live row fits but FINISHED does not, the row is off screen with no
  word until the cursor reaches the last task. The builder took option (a), the
  reviewers' recommendation, and `docs/tui/screens.md` § Path says so. Naming it
  in the footer was rejected: it crowded a 40×4 pane. At 40×6 with the cursor on
  the last task, FINISHED is also off screen; the layout rules allow it. The
  human confirms (a).
- **The `window` doc comment omits the FINISHED exception** (2026-11-22, unit
  77). `crates/raul/src/path.rs:532` says what does not fit is said out loud,
  which FINISHED does not do. One line, left as it was.
- **The no-selection case is unchecked** (2026-11-22, unit 77). With no
  selection nothing scrolls, so FINISHED stays hidden when every row fits. The
  build did not check whether the app ever passes no selection with rows present.
- **FINISHED's colours** (2026-11-22, unit 77). The icon and label take
  `semantic.ok`, the words `text.dim`. The brief said "dim, on the done phase's
  colour"; the builder read it as that split. The human confirms it.
- **A Path with only done tasks** (2026-11-22, unit 77). It shows the FINISHED
  row, not "no live work yet". The builder's choice; the human confirms it.

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
