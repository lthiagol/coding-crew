# The redesign

raul and hugo get one modern visual language: rounded panes, icons, chips,
bars and a document that reads like a page. This file is the spec; each part
below is a unit in `docs/plan.md` (rows 52–58).

The rules of `theme.md` still bind: every colour is a slot, a theme changes the
paint and never the words. This redesign adds glyphs, not words.

## The rules

1. **An icon decorates a word; it never replaces one.** Every fact still has
   its word — `executing`, `needs you`, `3 open findings`. Strip every icon and
   the screen says the same thing.
2. **An icon is followed by one space**, so a glyph a font draws two cells wide
   bleeds into the space, never into the word.
3. **Two glyph sets, one table.** `nerd` (Nerd Fonts v3, `nf-md-*`) and
   `unicode` (single-width, non-emoji). Every icon names both, in one table in
   `crates/theme/src/icons.rs`; nothing else writes a glyph.
4. **The set is chosen, not detected.** `--icons nerd|unicode` on both TUIs,
   then `CODING_CREW_ICONS`, then the default `nerd`. raul's Settings tab
   shows the set in force.
5. **A glyph's width is measured, not assumed.** The sweep and the column
   table treat every glyph in both sets as one column; a test walks the table
   and pins it.

## The icons

| name | nerd | unicode | used for |
|---|---|---|---|
| `app_raul` | `󰙅` U+F0645 | `◆` | raul's title |
| `app_hugo` | `󰚩` U+F06A9 | `◇` | hugo's title |
| `tab_overview` | `󰕮` U+F056E | `▦` | Overview tab |
| `tab_needs` | `󰂚` U+F009A | `◉` | Needs you tab |
| `tab_path` | `󰘬` U+F062C | `⑂` | Path tab |
| `tab_backlog` | `󰚇` U+F0687 | `≡` | Backlog tab |
| `tab_settings` | `󰒓` U+F0493 | `⚙` | Settings tab |
| `status_draft` | `󰝦` U+F0766 | `○` | draft |
| `status_groomed` | `󰪥` U+F0AA5 | `◔` | groomed |
| `status_approved` | `󰄲` U+F0132 | `◑` | approved |
| `status_executing` | `󰐊` U+F040A | `▶` | executing |
| `status_review` | `󰈈` U+F0208 | `◎` | code-review |
| `status_remediating` | `󰖷` U+F0587 | `↻` | remediating |
| `status_needs_human` | `󰀦` U+F0026 | `!` | needs-human |
| `status_done` | `󰄬` U+F012C | `✓` | done |
| `kind_idea` | `󰌵` U+F0335 | `✦` | idea |
| `kind_feature` | `󰐕` U+F0415 | `+` | feature |
| `kind_bug` | `󰃤` U+F00E4 | `✗` | bug |
| `role_orchestrator` | `󰚩` U+F06A9 | `◇` | orchestrator |
| `role_runner` | `󰖷` U+F0587 | `▸` | runner / builder |
| `role_reviewer` | `󰈈` U+F0208 | `◎` | reviewer |
| `role_human` | `󰀄` U+F0004 | `☺` | human |
| `blocked` | `󰌾` U+F033E | `⊘` | blocked, waits on |
| `finding` | `󰅚` U+F015A | `✗` | open finding |
| `link` | `󰌷` U+F0337 | `→` | a dependency, a link |
| `clock` | `󰥔` U+F0954 | `◷` | an age, a hold, a stamp |
| `running` | `󰑮` U+F046E | `●` | the automation runs |
| `stopped` | `󰏤` U+F03E4 | `■` | the automation stopped |
| `config` | `󰒓` U+F0493 | `⚙` | hugo's configuration |
| `next` | `󰑊` U+F044A | `●` | the task that happens next (Path's marker) |
| `cursor_bar` | `▌` U+258C | `▌` | the selected row's bar, in both sets |
| `in_use` | `󰐾` U+F043E | `●` | the value in use (Settings) |
| `task_open` | `󰄱` U+F0131 | `○` | an open task-list item |
| `alert_note` | `󰋼` U+F02FC | `ℹ` | `> [!NOTE]` |
| `alert_tip` | `󰛩` U+F06E9 | `✦` | `> [!TIP]` |
| `alert_important` | `󰍢` U+F0362 | `!` | `> [!IMPORTANT]` |
| `alert_warning` | `󰀦` U+F0026 | `⚠` | `> [!WARNING]` |
| `alert_caution` | `󰅜` U+F015C | `✗` | `> [!CAUTION]` |
| `tab_marker_left` | `‹` U+2039 | `‹` | the mark before the active tab's label, in both sets |
| `tab_marker_right` | `›` U+203A | `›` | the mark after the active tab's label, in both sets |
| `tab_underline` | `▔` U+2594 | `▔` | the rule under the active tab's label, in both sets |

A status icon takes its status's slot (`theme.md` § the phases). Other icons
take the slot of the word they sit beside.

## The chrome (both TUIs)

- **Panes have rounded borders** (`╭╮╰╯`) in `chrome.border`; the focused pane
  is `chrome.border_focus`. A pane's title sits in the top border with a space
  each side and its icon: `╭─ 󰘬 path ───…╮`.
- **The header line** is the app icon and name in `chrome.title`, a dim `·`,
  then the project path in `text.dim`; the right edge holds a status pill.
- **The tab bar** (raul) is icon + name per tab; the active tab is
  `chrome.tab_active` bold with a `▔` underline row under its label (the row
  below the tabs is a thin rule `─` in `chrome.border`, broken by the
  active tab's `▔`). A tab whose count matters carries a badge: `Needs you 8`,
  `Backlog 11`, the count in `text.strong`.
- **The footer** is key chips: the key in `chrome.key` between `‹ ›`-free
  padding — ` q ` reversed on the key slot — then the action in
  `chrome.footer`. Chips are separated by two spaces. What does not fit is
  dropped from the right, the hidden count kept, as today.
- **A progress bar** is `█` filled and `░` empty in the value's slot and
  `text.dim`, followed by the numbers in words: `██████░░░░ 18 of 45`.
- **A spinner** (hugo only, for `working` agents) cycles `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` on the
  tick; the plain render shows the first frame, so it stays deterministic.

## raul

**Overview** is cards in a responsive grid (two columns ≥ 80, one below):

```
╭─ 󰕮 status ─────────────────────────╮ ╭─ 󰂚 needs you ──────────────────────╮
│ 30 tasks                           │ │ 8 wait on a human                  │
│ 󰝦 draft        6 ██████            │ │ 󰪥 approve   2                      │
│ 󰪥 groomed      3 ███               │ │ 󰈈 review    3                      │
│ 󰄲 approved     4 ████              │ │ 󰀦 escalated 3                      │
│ …                                  │ ╰────────────────────────────────────╯
╰────────────────────────────────────╯
```

The status card's bar is proportional to the largest count, in the status's
slot. The activity card prints a time column (`HH:MM`, the date once per day
as a dim divider), the event kind's icon, the task id strong, then the words.

**Needs you, Path, Backlog rows** lead with the status (or kind) icon in its
slot, then the id in `text.strong`, the title in `text.normal`, and the suffix
(review state, waits on) in `text.dim` with its own icon: `󰅚 2 open (1 high)`,
`󰌾 waits on TSK04`. The selected row is a full-width bar in `text.selected`
with a `▌` in the focus slot at its left edge. The Path tree keeps its spine
and branches; its branch headers become `├─ 󰐊 RUNNING  6` with the count as a
badge.

**Settings** gains an `icons` group: `nerd`, `unicode`.

## hugo

**The sidebar** is the configuration card: a header `󰒓 configuration`, then
each group under a dim small-caps label (`AGENTS`, `HARNESS`, `ROUNDS`,
`ORDER`); an agent is `role icon  name` with the model under it. The cursor is
the same `▌` bar as raul's rows.

**The main pane**:

- **crew** is a table: icon + role, model, task, hold (`󰥔 10d`), state; a
  `working` state carries the spinner, `idle` is dim. The composition line
  is under it in `text.dim`.
- **budget** is a progress bar: `external rounds ██████░░░░ 18 of 45`.
- **findings** names the severities: `17 open findings · 3 high · 9 medium ·
  5 low`, each count on its severity's slot, when the record holds them.
- **the stamp** is a pill at the foot: `󰑮 running since 2026-11-09T08:30Z` on
  `semantic.ok`, `󰏤 not running: why` on `semantic.warn`.

## The document (the engine)

The detail pane is a page, not a dump. Changes live in `crates/engine` and are
recorded in `VENDORED.md`'s ledger as deviations.

- **Headings** drop the `═══` underline. `#` is a full-width bar: the heading's
  text in bold on its role with a `▍` lead glyph and a thin `─` rule after it to
  the edge. `##` is bold with a `▸` lead. `###` is bold, no lead.
- **Code blocks** are a rounded box with the language as a label in the top
  border (`╭─ rust ─…╮`), a dim line-number gutter (unit 50), and the preset's
  own syntax theme (unit 50).
- **Inline code** is a chip: the code role's foreground on the code
  background, padded by a space each side (unit 50).
- **Tables** use rounded outer corners and a single-line header rule (`─`, not
  `═`); body rows are not separated by rules — alternate rows are not tinted.
- **Blockquotes** are a `▎` bar in the quote role, the text indented two.
- **Task lists** use `󰄬`/`✓` done and `󰄱`/`○` open from the icon set (the
  engine takes the two glyphs as configuration, so it stays icon-agnostic).
- **Alerts** (`> [!NOTE]` …) take a coloured bar, an icon and the label.
- **Links** show their text underlined in the link role; the URL follows dim
  in parentheses only when it differs from the text.

**The header section** of a task stops being a two-column table: it is a card
of `key value` pairs, one per row — id, status, kind, priority, effort, then
tags as chips and the claim on its own line. The status word carries no icon
in the card, because the detail pane's border title already shows it. The
projection emits it as a list the engine can lay out; how is the unit's to
decide, within ADR 0009's escaping rule.

The criteria section is blocks, not a seven-column table — unit 51.
