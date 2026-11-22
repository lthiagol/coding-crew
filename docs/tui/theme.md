# Theme

The colours raul and hugo paint with.

Both interfaces have a theme. The domain does not: a task file never names a
colour, and `crates/core` stays UI-free.

## The slots

A slot names a **role**, not a colour. Two presets can paint the same slot
differently.

| slot | what it paints |
|---|---|
| `chrome.title` | the title line, the product name |
| `chrome.tab_active` | the tab that is showing |
| `chrome.tab_inactive` | the other tabs |
| `chrome.tab_index` | the `3/6` index |
| `chrome.border` | a block's border |
| `chrome.border_focus` | the border of the focused block |
| `chrome.footer` | the filter note's count, `2 of 5 · 3 filtered out`, above the footer |
| `chrome.key` | a key label in the footer and in the help overlay |
| `chrome.surface` | the background of hugo's sidebar: a step darker than the base, so the sidebar is recessed |
| `text.normal` | ordinary text |
| `text.dim` | secondary text: the empty state, a marker, a hint under a line, a separator, a path, a tab that is not built yet |
| `text.strong` | a count, a title that matters |
| `text.selected` | the mark in a document: a search or a marked span, the accent at full opacity |
| `text.selected_bg` | the selected row: a subtle tint under the row, which keeps its own colours |
| `semantic.ok` | a clean result, a finished task |
| `semantic.warn` | something wants attention and is not broken |
| `semantic.error` | a failure: an unreadable file, a failed criterion, an escalation, no coding-crew folder found |
| `semantic.info` | neutral information |
| `semantic.human` | the human lane: approve, review, and the facts that wait on it — "wait on a human" |
| `semantic.agent` | the agent lane: claimable, running |
| `role.orchestrator` | the orchestrator's icon and name, in the crew and the sidebar |
| `role.runner` | the runner's icon and name |
| `role.reviewer` | the reviewer's icon and name |

The creator takes `semantic.human`, the lane its icon is drawn from.

The words that name a lane take the lane's slot: "wait on a human" is
`semantic.human`, and the number beside it is `text.strong`. The same fact
is never painted two ways on one screen.

## The phases and their slots

Eight phases, six slots: five semantic ones and the dim. A phase never names
a colour; it names a slot.

| phase | slot | why |
|---|---|---|
| `draft` | `text.dim` | nothing has been said about it yet |
| `groomed` | `semantic.human` | a human approves it |
| `approved` | `semantic.agent` | an agent can take it |
| `executing` | `semantic.agent` | an agent is on it |
| `code-review` | `semantic.human` | a human reviews it |
| `remediating` | `semantic.warn` | it is being fixed |
| `needs-human` | `semantic.error` | the review budget ran out; a person has to step in |
| `done` | `semantic.ok` | it is finished |

## The reasons and their slots

Three reasons wait on a human, and they take the same slots as the phases
they stand for:

| reason | slot | why |
|---|---|---|
| `approve` | `semantic.human` | a human approves it |
| `review` | `semantic.human` | a human reviews it |
| `escalated` | `semantic.error` | the review budget ran out; a person has to step in |

An escalation is the one reason that is a failure, and it is painted like one.
The count beside a reason is `text.strong`, as every other count is.

## The titles of the blocks

The title of a block is `text.strong`: a title that matters. It sits on the
border, and the border's attributes bleed into it, so the dimming of a dimmed
border is taken away again: a title is never dimmed.

The border of a block is `chrome.border`, and the border of the focused one is
`chrome.border_focus`.

hugo's main pane is the one exception: its block titles and section headings
take the accent, `chrome.tab_active`, the bold the sidebar's group labels use,
so the pane's sections read as the sections of one screen. raul's titles stay
`text.strong`.

## The selection

A selected row is `▌` on `chrome.border_focus`, and the row on
`text.selected_bg`. The row's text keeps its own colours, so a status stays
readable on the selection; the tint carries the selection, the bar marks it.

The mark in a document is the one place the accent is painted whole:
`text.selected` is the accent with the base on it, as a search or a marked
span is.

## The markdown engine

The detail pane draws with the vendored leaf engine. Its `MarkdownTheme` has
47 colour fields and its own four palettes; raul uses none of them. Instead
`Theme::markdown_theme()` paints every field from the **document's own
palette**: twenty-seven roles, which are not the chrome's eighteen slots. The
roles are recorded in [palettes.md](palettes.md), role by role, with the
value each preset gives them and the figures the tests recompute.

The document's headings are the three levels the projection writes: `#` for a
section, `##` for an item under it, and `###` for a finding under its round.
The engine's `heading_4` and `heading_other` join the deepest, because the
document has no level below `###`. The record's title is a label, not prose,
and is never a heading.

A field takes **one side of one role**: a foreground for a field that paints
a word or a line, a background for a field that paints one. Six of the roles
name a pair, and the side column says which side a field takes. `ENGINE_ROLES`
in the theme crate is the mapping; the test below holds this table and the
code together.

That promise stops at `MarkdownTheme`. The engine paints the **tokens of a
code block** with syntect's own theme. Raul does not reach the engine's
themes. A detail pane with fenced code therefore shows syntect's palette
alongside ours, and the detail-pane unit must either avoid syntax highlighting
or handle syntect's theme deliberately.

The detail pane handles it deliberately: each preset pairs the code tokens
with a syntect theme derived from its own document palette, so the tokens
follow the preset. The mapping is in [palettes.md](palettes.md#the-codes-syntax).
There is no avoiding it: an empty syntax set panics, and a scope-less syntect
theme paints the tokens black.

`MarkdownTheme` carries colours only: no role and no attributes reach it. In
the terminal preset every role resolves to `Color::Reset`, so the engine
names no colour at all, and what still tells one role from another is the
engine's own modifiers — bold on headings and strong text, underlined on
links among them.

**The search is not offered in any preset.** The engine paints a hit's
background and keeps the searched span's own foreground. The document
palette gives a hit a background of its own — the line's grade — but the
current match needs a stronger patch, and the palette's strongest patch is
the accent, which the selection already spends on the mark.

- In the **dark** preset the hit's background is the line's `#3d444d`, and the
  prose on it reads at 6.38:1; the current match is the accent `#79c0ff`, and
  the prose on it reads at 1.26:1.
- In the **nord** preset the hit's background is nord3 `#4c566a`, and the
  prose on it reads at 5.46:1; the current match is nord8 `#88c0d0`, and the
  prose on it reads at 1.48:1.
- In the **catppuccin** preset the hit's background is surface1 `#45475a`,
  and the prose on it reads at 5.15:1; the current match is mauve `#cba6f7`,
  and the prose on it reads at 1.15:1.
- In the **dracula** preset the hit's background is the current line
  `#6272a4`, and the prose on it reads at 4.41:1; the current match is purple
  `#bd93f9`, and the prose on it reads at 2.26:1.
- In the **terminal** preset the engine's fields are `Color::Reset`, so the
  engine paints no background at all: the current match is a bold word with
  no box around it, and the other hits are not marked.

The current match reads below 3:1 in every colour preset. Search waits for a
readable pairing; the document's own palette did not give it one, and the
ruling stands.

| field | role | side | why |
|---|---|---|---|
| `search_highlight_bg` | `document.search` | hit | a hit takes the document's hit background; the engine keeps the span's own foreground, so the search is not offered |
| `search_match_bg` | `document.search` | match | the current match takes the stronger background; the same limitation |
| `code_gutter` | `document.list.3` | — | the gutter is quiet: its numbers are words at the quietest grade, and the engine dims them (`Modifier::DIM`) |
| `blockquote_marker` | `document.quote` | — | a quote's bar takes the quote's colour |
| `list_level_1` | `document.list.1` | — | the first list depth is the first step of its own scale |
| `list_level_2` | `document.list.2` | — | the second is a step dimmer |
| `list_level_3` | `document.list.3` | — | the third is a step dimmer again |
| `ordered_list` | `document.list.1` | — | an ordered marker ranks with the first list depth |
| `table_border` | `document.rule` | line | a table's frame is a line |
| `table_separator` | `document.rule` | header | the header rule is part of the header, so it takes the header's grade |
| `table_header` | `document.heading.item` | — | a table header is a chromatic heading, so it reads apart from the prose under it |
| `table_cell` | `document.text` | body | a cell is ordinary prose |
| `heading_1` | `document.heading.section` | — | the `#` line: a section |
| `heading_2` | `document.heading.item` | — | the `##` line: an item under a section |
| `heading_3` | `document.heading.finding` | — | the `###` line: a finding under its round |
| `heading_4` | `document.heading.finding` | — | the document has no level below `###` |
| `heading_other` | `document.heading.finding` | — | h5 and h6 join the deepest level |
| `heading_underline` | `document.rule` | line | an underline is a line |
| `code_frame` | `document.rule` | line | a code block's frame is a line |
| `code_label` | `document.list.3` | — | the language label is a word at the quietest grade, quiet beside the frame |
| `inline_code_fg` | `document.code` | text | inline code is the code's ink |
| `inline_code_bg` | `document.code` | chip | the chip behind the ink |
| `rule` | `document.rule` | line | a rule is a line |
| `link_icon` | `document.link` | — | links are one role |
| `link_text` | `document.link` | — | the text of a link |
| `link_hover` | `document.link` | — | mapped for the 47; the vendored engine never reads it |
| `blockquote_text` | `document.quote` | — | quoted prose takes the quote's colour |
| `text` | `document.text` | body | ordinary prose |
| `strong_text` | `document.text` | emphasis | strong prose is the prose's emphasis |
| `latex_inline_fg` | `document.code` | text | inline maths is the code family's ink |
| `latex_inline_bg` | `document.code` | chip | and its chip |
| `latex_block_fg` | `document.code` | text | block maths is the code family's ink |
| `mermaid_keyword` | `document.code` | text | a diagram keyword is the code family's ink |
| `mermaid_arrow` | `document.rule` | line | an arrow is a line |
| `mermaid_label` | `document.text` | body | a node label is a word |
| `mermaid_block_fg` | `document.code` | text | the diagram's ink |
| `mark_fg` | `document.selection` | foreground | a marked word takes the mark's foreground |
| `mark_bg` | `document.selection` | background | and its background |
| `task_checked` | `document.task` | done | a checked box |
| `task_unchecked` | `document.task` | open | an open box |
| `alert_note` | `document.alert.note` | — | a note is one of the five alerts |
| `alert_tip` | `document.alert.tip` | — | a tip is one of the five |
| `alert_important` | `document.alert.important` | — | importance is one of the five |
| `alert_warning` | `document.alert.warning` | — | a warning is one of the five |
| `alert_caution` | `document.alert.caution` | — | a caution is one of the five |
| `footnote_ref` | `document.link` | — | a footnote's reference ranks with a link |
| `footnote_text` | `document.text` | body | a footnote's note is prose |

The engine's own four markdown palettes — arctic, forest, ocean and
solarized-dark — are never read. A family palette from
[palettes.md](palettes.md) reaches `MarkdownTheme` through the document's own
twenty-seven roles. The eight syntax roles are the code's tokens: the detail pane
pairs each preset with a syntect theme that paints them, so they reach the
engine through that theme and not through `MarkdownTheme`.

## The presets

| preset | what it is |
|---|---|
| `dark` | the default: the dark palette |
| `nord` | the Nord family's palette |
| `catppuccin` | the Catppuccin family's palette, the mocha flavour |
| `dracula` | the Dracula family's palette, the Dracula Classic variant |
| `terminal` | no colours of our own — the terminal's palette is used |

The preset is chosen with a flag for now:
`raul --theme dark|nord|catppuccin|dracula|terminal`. A theme file on disk is
later.

The `dark` preset is ours, hand-tuned, and its values are written down in
[palettes.md](palettes.md), with their roles and their contrast. The `nord`
preset is the Nord family's, and its values are recorded in the same file.
The `catppuccin` preset is the Catppuccin family's mocha flavour, and its
values are recorded there too; the other three flavours wait with the light
theme. The `dracula` preset is the family's Dracula Classic variant, and its
values are recorded there too; alucard waits with the light theme.

The four colour presets paint the document's twenty-seven roles from the same
family palettes; the roles, their values and their figures are recorded in
[palettes.md](palettes.md). The terminal preset paints every document role
`Color::Reset`, the way it paints no slot's colour.

**The dark, nord, catppuccin and dracula presets assume a terminal with a
dark background, and paint none of their own, except two.** The background is
the terminal's own; every colour of the palettes is picked to read on a dark
one. The two exceptions are `chrome.surface` and `text.selected_bg`, derived a
step from the base; the terminal's own background is the base of every row.

**The light themes are deferred.** No light preset is built until that changes:
not ours, and not the light variants of the families — Catppuccin Latte,
Dracula Alucard and Nord Light wait with them, and so do the Catppuccin
frappé and macchiato flavours. The slots are shaped so a light preset drops
in without touching anything else.

**Selection is a tint, and the mark is the accent whole.** A terminal has no
alpha compositing, so a translucent selection is not available: the selected
row is a tint derived from the surface, and a document's mark is the accent
whole. The terminal preset has no tint and no accent: its selection is
reversed video, and its own section below says so.

## The terminal preset

The `terminal` preset names no colour of its own: the terminal's palette is
used. What is left are the attributes every terminal has, and there are only
five of them — bold, dim, italic, underlined, reversed.

So the terminal preset tells things apart by attribute, and it says what it
can and cannot do:

- **the semantic roles are told apart.** The five semantic roles of the
  phases — `semantic.ok`, `semantic.warn`, `semantic.error`,
  `semantic.human` and `semantic.agent` — each hold a different set of
  attributes, and `semantic.info` holds a sixth. None of them holds the
  same set as `text.normal`. A finished task and a task being fixed look
  different, and so do the human lane and the agent lane.
- **the two lanes never share one.** They hold different attributes, so the
  rule that the lanes are two colours holds here too.
- **the selection is not a colour.** `text.selected_bg` is reversed video, as
  the selection always was: the shape of the selection carries the meaning,
  not a hue. The surface is nothing the terminal paints: the sidebar keeps the
  terminal's own background.
- **the roles are attributes.** `role.orchestrator` is italic and underlined,
  `role.runner` is dim and italic, and `role.reviewer` is dim and underlined:
  three shapes no semantic slot holds, so a role does not read as a phase.
- **the document loses its distinctions.** `MarkdownTheme` carries colours
  only, so every field becomes `Color::Reset`: a checked task, an unchecked
  task, a warning and a caution are painted alike. Inside the engine only its
  own modifiers survive — bold on headings and strong text, underlined on links
  among them.
- **the syntax roles are told apart outside the engine.**
  `Theme::document_style` adds an attribute to each code token's role: keyword
  bold, string underlined, number italic, function bold and underlined, type
  italic and underlined, comment dim, tag dim and italic. A variable takes none.
  The engine does not carry these; they come from `document_style` alone.
- **some roles are weaker than they would be with a colour.** `semantic.info`
  is bold and dim, so the heading is louder than the `draft` row under it,
  which is dim alone; with a colour it would be a hue of its own.
  `semantic.warn` is underlined and `semantic.error` is bold, underlined and
  reversed: loud, but not red.
- **what it cannot do:** it cannot say *which* hue a thing is, and it cannot
  tell a colour from the terminal's own foreground. The five attributes are
  enough for the eight phases and not enough for the slots: the
  weak chrome slots — the muted foreground, the dimmed foreground, the
  border, the footer and the inactive tab — all hold DIM, and the title,
  the strong text and the focused border all hold BOLD. The showing tab
  and every key label hold BOLD and REVERSED together, so those two paint
  alike. The terminal preset tells the phases apart; it does not tell the
  chrome apart.

The preset exists for terminals whose palette the reader has chosen. Where
the paint matters, the `dark`, `nord`, `catppuccin` or `dracula` preset is the
one.

## The icons

The icons are one table, `crates/theme/src/icons.rs`, with each icon's glyph
in both sets, `nerd` and `unicode`. The names and glyphs are the spec's:
[redesign.md](redesign.md) § The icons. The chrome's pieces, the pane and the
chips, are in `crates/theme/src/chrome.rs`.

An icon decorates a word; it never replaces one. Every fact keeps its word,
and the screen says the same thing with the icons stripped.

The set is chosen with `--icons nerd|unicode` on both TUIs, then the
`CODING_CREW_ICONS` variable, then the default `nerd`. A name that is neither
set is refused. Every glyph in both sets measures one column, and the column
table pins the glyphs.

## Rules

1. No colour is written inline. Every colour comes from a slot.
2. A slot names a role. A preset decides what the role looks like.
3. The domain knows nothing about any of this.
4. A theme changes the paint, never the words: the same screen says the same
   thing in every preset.
5. The **document's roles are slots** in this rule's sense: the theme crate
   names them, a preset decides what they look like, and the rules above bind
   them as they bind the chrome's. The chrome and the document keep their own
   vocabularies, and neither paints with the other's.

## Not yet

- **the rest of the catalog.** The variants the light theme brings: `latte`,
  `frappé`, `macchiato`, `alucard` and `nord-light`. Their palettes are
  recorded in [palettes.md](palettes.md).
- **a theme file on disk.** The preset is a flag for now; a file comes with
  the catalog.
