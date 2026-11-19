# Plan

The work ahead, in small units. `TODO.txt` is the orchestrator's memory — this
is the groomed backlog it points at, and **the order of record**.

Every unit has a **done when**: the sentence that says the unit is finished.
A unit without one is not ready, and belongs under **Grooming** instead.

| mark | meaning |
|---|---|
| **S** · **M** · **L** | size, the same scale as a task's effort |
| ⬚ | waits on another unit |
| ? | waits on a decision — see **Questions** |
| ● | in flight |

## Ready

| # | unit | size | done when |
|---|---|---|---|
| 0 | **Close the theme unit** | ✓ | done: eight build and review rounds, committed by staging by name — `crates/raul`, `docs/tui/theme.md`, `docs/tui/palettes.md`. The slots, the phase and reason mappings, the dark and terminal presets, the family tables and the figures are in the code, and five tests read the docs to keep them honest |
| 1 | **`raul --render --plain`** ✓ | done | the current screen prints as deterministic plain text without a terminal, exits 0, and prints the same bytes across runs and filesystems; `--width` and `--height` are bounded and default to 100 by 40; the Overview's text is a snapshot file in the repo, and the real-command snapshot test fails when the file and the output differ; wide graphemes and duplicate record ids render deterministically |
| 2 | **The lease fix in core** ✓ | done | a lease written without seconds expires on the minute it says; equivalent RFC 3339 offsets compare as the same instant; the rule compares parsed instants, not timestamp strings; empty and malformed lease cases are pinned |
| 3 | **The store discovery in core** ✓ | done | core owns the project-boundary rule from ADR 0010: the nearest `coding-lanes/` stops discovery, even without `work/`, and an ancestor project's store is never borrowed; raul calls the strict core rule, no coding-lanes and no-work are distinct screens, and `crates/talita` stays untouched |
| 4 | **The Path data layer** ✓ | done | the spine and the five branches are derived from the fixtures: trunk 1, AWAITING APPROVAL 2, BLOCKED 3, RUNNING 6, REVIEW 3, ESCALATED 3; a blocked task is forked under the first of its dependencies that is not done and names the rest; the fixtures gained a case for each blocking reason — dependency not done, overlay, claim held elsewhere — through `mock-data/generate.py` |
| 5 | **The Path drawing** ✓ | done | at 100 columns the tree has the shape of `docs/tui/screens.md`: the spine, the `●` on the first row, the branch headers with their counts, and the blocked rows forked under their blocker; at 40, 60 and 100 columns no line exceeds the width, a count is never shown without the group it counts, and a hidden count is announced. The drawing was ported from master-plan's `path_view.rs` — the technique, not its model |
| 6 | **The Needs you tab** ✓ | done | approve, review and escalated each list their tasks with the Overview's counts; the age measures the wait from `status_since` with the model's `created` fallback; the not-shown announcement counts omitted tasks in a shape distinct from the header total, names every reason including zero, and paints by the theme; every fixture carries a chronological `status_since`; the gate and the generator's byte-for-byte reproduction are green |
| 7 | **The Backlog tab** ✓ | done | smaller-task items and draft tasks appear together in derived IDEAS, SMALL TASKS, and DRAFTS groups; dismissed and promoted items remain visible as such; rows carry id, kind, title, known scope, and state; short panes announce hidden rows; narrow widths and Unicode titles render without cutting fixed columns |
| 8 | **The theme catalog: Nord** ✓ | done | the preset is built from Nord's own page — every value fetched live and verified — its eighteen slots mapped per the `palettes.md` rules with the derivations written down, its contrast figures recomputed by test, and the render tests run it on every tab at 40/60/100; it joins `Preset::ALL` so every preset test covers it; `--theme` gained the name, no new flag; `nord-light` waits with the light theme |
| 9 | **The theme catalog: Catppuccin** ✓ | done | the `mocha` preset is built from `catppuccin.com/palette` — all 26 mocha values fetched live and verified, the eighteen slots mapped per the `palettes.md` rules with the derivations written down, the contrast figures recomputed by test, the search pairing recomputed to 1.40:1, and the render tests run it on every tab at 40/60/100; it joins `Preset::ALL`; the terminal preset is untouched; the light flavours wait with the light theme |
| 10 | **The theme catalog: Dracula** ✓ | done | the `dracula` preset is built from `draculatheme.com/spec` — twelve colour-palette and sixteen ANSI values fetched live and verified, the eighteen slots mapped per the `palettes.md` rules, the figures recomputed by test, the search pairing recomputed to 2.26:1, and the render tests run it on every tab at 40/60/100; it joins `Preset::ALL`; the terminal preset is untouched; `alucard` waits with the light theme |
| 11 | **The projection in core** ✓ | done | `task_to_markdown(task, context)` is pure and terminal-free; the document carries the ten detail-pane sections in order; ADR 0009's escaping rule distinguishes literal fields from free-text markdown; reverse dependencies and child progress come from the context; the complete fixture output is asserted byte for byte against a committed snapshot |
| 12 | **`crates/engine`: vendor leaf's module** ✓ | done | leaf 1.29.2 is pinned by full commit in `crates/engine/VENDORED.md`; all 18 upstream markdown modules and their exact file set are recorded; every intentional deviation has a numbered ledger entry and checksum; the theme is collapsed to markdown palettes, upstream's app-only code is removed, and the ten markdown test suites are ported; `sync.sh` fetches the pin and checks or safely updates the copy; the crate uses ratatui 0.30, pulldown-cmark 0.12, syntect 5.2, mmdflux 2.5, unicodeit 0.2, unicode-width 0.2, and unicode-segmentation 1 |
| 13 | **`crates/engine`: bridge the slots** ✓ | done | all 47 fields of `MarkdownTheme` map to our slots, each named once; the five fields the engine paints with a background are derived from the engine's own sources in the test, not hand-pinned; both presets, palette containment, and the doc's table are pinned against the code. The one-palette promise stops at `MarkdownTheme` — syntect's code tokens stay a third palette the detail-pane unit must handle |
| 14 | **The detail pane: the shell** ✓ | done | enter on a row opens the task as a full-screen overlay — the spec's split-or-full fork is decided and recorded in `docs/tui/screens.md`; `esc` returns to the tab and the selection it came from, `enter` reopens; it opens from Path, Needs you and Backlog and nowhere else; the header labels id and escaped title (`escape_literal` in core is now pub — one escaping rule), title on `text.strong`, id on `text.normal`, `text.selected` unused until unit 15; `--detail TASK-ID` in render mode is bounded by `--width`/`--height`; the title cut is grapheme-boundary aware and pinned by tests |
| 15 | **The detail pane: the document** ✓ | done | the body is `task_to_markdown` over the record's context, rendered through the vendored engine and painted by the unit-13 bridge — one palette; the ten sections read from `docs/tui/screens.md` in order; scroll keys with a clamped viewport, footer names only working keys with its degradation order pinned; `text.selected`'s first user is the engine mark on a `==mark==` in free text, both presets pinned; the backslash-in-code-span question probed on a real task; the snapshot is the real TSK13 document, byte for byte; syntect's tokens keep `base16-ocean.dark` in both presets, recorded in `docs/tui/theme.md` |
| 16 | **The pty harness** ✓ | done | `scripts/pty-sweep.py` sweeps `raul --render --plain` at every width 40–120 inclusive, one file per width; an over-long line fails the command naming width, line and measured columns; stdlib-only, `--height` passed through, temp-dir default reported and kept; `make ci` sweeps and the step is proven able to fail; the Overview is pinned at 40, 60 and 100; 74 script assertions with a faked raul. What is left of a pty's value — keys, resize, the alternate screen — is no unit yet |
| 17 | **The Settings tab** ✓ | done | the tab is a named empty state — "nothing to set yet" over "will hold keybinds, sort keys, theme" — on `text.dim`; no rows, so `enter` opens nothing and the footer names no open or scroll key; render mode gained `--tab NAME`, refused outside render mode and on unknown names, reachability pinned at 40/60/100 |
| 18 | **The tab closer** ✓ | done | `herdr-close-pane.sh` takes the **pane** id the orchestrator holds, resolves the tab id itself, closes it, and proves the close — the tab no longer appears in `herdr tab list`; `--audit` lists the workspace's tabs with their agent state; every herdr call is bounded with `timeout --signal=KILL`, only stdout is parsed, `--audit` with a pane id is a usage error. It now lives beside the flow skill, with no suite of its own |
| 19 | **The global flow** ✓ | done | the flow skill and the review skill live at `~/.config/opencode/skills`, loaded by their IDs, and the repository points at them; the skill records where it lives, that the scripts stay in the project, the project prefix that names every tab and agent, and the once-per-session model question — asked with the configured pair recommended and applied through a generated role config written outside the repository; `.orchestration.json` carries the prefix and the role-config paths; the shared workspace's teardown and audit rules are prefix-scoped, so a loop never closes another project's tab |
| 20 | **The upstream skills move global** ✓ | done | `codebase-design`, `diagnosing-bugs`, `domain-modeling` and `writing-for-agents` live whole at `~/.config/opencode/skills`, every file beside its `SKILL.md`, byte-identical to the copies they replace; the MIT licence sits beside them and the provenance records the upstream repository, the pinned commit `5beb35b` and each skill's upstream path, plus the rule that they are never edited in place; the repository carries no skills at all, and the sync machinery and its test are gone from `make ci` |
| 21 | **The flow scripts move into the skill** ✓ | done | the flow skill carries its own tools beside `SKILL.md` — `herdr-wake.sh`, `herdr-close-pane.sh`, `review-snapshot.sh` with the wake and snapshot suites — so it is self-contained; every path in the skill text is absolute; `make test-flow` runs the suites and joins the gate, skipping cleanly with exit 0 where the skill is not installed, so the gate still fails when a suite fails; the repository keeps only `pty-sweep.py`, which tests raul's rendering |
| 22 | **The row tabs scroll** ✓ | done | Needs you, Path and Backlog scroll their rows: the viewport follows the cursor, `pgup`/`pgdn` move it by the rows the pane shows, `g`/`G` reach the first and last row, and the content-pane row of `docs/tui/screens.md` is true; the cursor is never on a row the pane does not draw — a two-line chunk counts as two rows — and each tab's "not shown" count names every row it hides, including the ones above the window; the footer and the help name exactly the keys that work, with the degradation pinned at 40, 60 and 68 columns |
| 23 | **The mock-data dependency graph** ✓ | done | every task past `approved` waits only on `done` work, so the fixtures tell a history the model's claim gate allows — the claim comes after the dependency's completion, the done set is closed under waits-on, and the graph is acyclic; an `approved` task may still wait on unfinished work, which is what holds it; the rule is pinned by a test that reads the real fixture tree, by the generator's own self-check, and by the byte-for-byte reproduction now in the gate; the Path and Needs you counts are unchanged, and the documents that describe the graph say what the model says |
| 24 | **The small follow-ups** ✓ | done | one instant serves a whole frame — the Needs you, Path and Overview paths all take the frame's `now`, the fresh-clock wrappers are gone, and a guard test scans the source tree so no other module can read the clock again; the `--tab` with `--detail` combination is refused before anything renders, with a message naming both flags; the help popup fits the window, measures its lines by their wrapped height, and keeps `q` and `?` at heights 14 and 10 and at the narrow widths where lines wrap; the `view.rs` clause names the one-content-row announcement |
| 25 | **The render_view extraction** ✓ | done | the ids→cursor→settle→re-window→page block the three row tabs each copied lives once in `View::draw` and they call it; the dead all-fits line is gone; the drawn-order id assembly is one `Tree::row_ids`, and the test asserting no history reads it too; nothing moved — the review byte-compared 21 render outputs before and after and every one was identical; a sharing pin stops a fourth copy |

## Grooming

Scope is not settled. These need design before they can be tasks with a
"done when".

| # | what | what has to be decided first |
|---|---|---|
| G1 | **The Autopilot tab** | what the automation *is*. Our flow — three agents, five rounds, a queue — is the candidate, but the tab is not the flow. maitre's Sous-Chef answers this with an order, a queue and a stamp; we would need the same, and we do not have an autopilot |
| G2 | **The journal** | the event kinds, and where the file lives. `activity.json` is named in the model and nothing writes it; the kinds are undefined |
| G3 | **talita's operation surface** | the verbs, the gates each one enforces, and the output shape. Out of scope until it is asked for explicitly |
| G4 | **The review's interaction** | what raul can do about a finding, if anything. `docs/tui/screens.md` lists it under "not defined yet" and nobody owns it |

## Questions

Marked so we can verify them later.

| # | question | why it matters |
|---|---|---|
| Q1 | **The Path tree's footer.** master-plan shows "N complete · open Milestones to list". We have no tab that lists done tasks | drop the footer, or give the done tasks somewhere to be seen? |
| Q2 | **The active tab's highlight.** It lost its inversion in the theme unit — now the main foreground in bold against the secondary | the mapping table asks for it, but it is a visible change. Keep, or restore the inverted block? |
| Q3 | **The journal's home.** Next to the tasks in `coding-lanes/`, or elsewhere in the project? | it decides where the file lives, and therefore what the work folder contains |
| Q4 | ✓ **answered** — our slots map onto `MarkdownTheme`, one palette for the document; the promise stops at syntect's code tokens, a third palette the detail-pane unit must avoid or handle | the theme rule says a theme changes the paint and never the words, and the same fact is never painted two ways on one screen |

## Deferred

- **The light themes** — our `light` preset and the light variants of the
  families (Catppuccin Latte, Dracula Alucard, Nord Light). Deferred until
  that changes.
- **The theme file on disk**, and a user theme that is not one of the
  built-ins.
- **talita** — no code, no commands, until it is asked for explicitly.
