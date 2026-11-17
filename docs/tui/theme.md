# Theme

The colours of raul.

The TUI has a theme. The domain does not: a task file never names a colour,
and `crates/core` stays UI-free.

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
| `chrome.footer` | the footer's text |
| `chrome.key` | a key label in the footer and in the help overlay |
| `text.normal` | ordinary text |
| `text.dim` | secondary text: the empty state, a marker, a hint under a line, a separator, a path, a tab that is not built yet |
| `text.strong` | a count, a title that matters |
| `text.selected` | the selected row |
| `semantic.ok` | a clean result, a finished task |
| `semantic.warn` | something wants attention and is not broken |
| `semantic.error` | a failure: an unreadable file, a failed criterion, an escalation, no coding-lanes folder found |
| `semantic.info` | neutral information |
| `semantic.human` | the human lane: approve, review, and the facts that wait on it — "wait on a human" |
| `semantic.agent` | the agent lane: claimable, running |

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

## The markdown engine

The detail pane draws with the vendored leaf engine. Its `MarkdownTheme` has
47 colour fields and its own four palettes; raul uses none of them. Instead
`Theme::markdown_theme()` maps every field onto one of the eighteen slots, so
`MarkdownTheme`'s document speaks the same palette as the chrome.

That promise stops at `MarkdownTheme`. The engine paints the **tokens of a
code block** with syntect's own theme: the default preset pairs with
`base16-ocean.dark`, and each of the engine's four presets has its own
syntect theme. Raul does not reach those colours. A detail pane
with fenced code therefore shows syntect's palette alongside ours, and the
detail-pane unit must either avoid syntax highlighting or handle syntect's
theme deliberately.

The detail pane handles it deliberately: the code tokens pair with the
pairing that sentence names — the engine's own default, `base16-ocean.dark`
— and the pairing is the same in both of raul's presets. There is no
avoiding it: an empty syntax set panics, and a scope-less syntect theme
paints the tokens black. The engine's code tokens are the one paint the
presets do not change.

A field that paints a **foreground** takes its slot's foreground. A field that
paints a **background** takes its slot's background. `MarkdownTheme` carries
colours only: no slot and no attributes reach it. A slot with no colour in the
preset becomes `Color::Reset`, so the terminal preset names no colour at all,
and what still tells one role from another is the engine's own modifiers —
bold on headings and strong text, underlined on links among them.

**The search is not offered in either preset.** The engine paints a hit's
background and keeps the searched span's own foreground. In the dark preset
the mapping gives the hit the selection's background, but the foreground
stays `text.normal`, which reads at 1.26:1 on that background — no slot
carries a dark background to pair with the accent. In the terminal preset
the engine's fields are `Color::Reset`, so the engine paints no background
at all: the current match is a bold word with no box around it, and the
other hits are not marked. Search waits for a readable pairing.

| field | slot | why |
|---|---|---|
| `search_highlight_bg` | `text.selected` | a hit takes the selection's background; the engine keeps the span's own foreground, so the hit is unreadable and search is not offered |
| `search_match_bg` | `text.selected` | the current match; the same background and the same limitation |
| `code_gutter` | `text.dim` | line numbers are words in the document, so they take the text grade, never the chrome index |
| `code_frame` | `chrome.border` | a code block's frame is a border |
| `code_label` | `text.dim` | the language label is secondary text |
| `inline_code_fg` | `text.strong` | inline code is raised prose |
| `inline_code_bg` | `chrome.border` | the code family's chrome; it has no background, so Reset |
| `blockquote_marker` | `chrome.border` | the quote's marker is a line |
| `blockquote_text` | `text.dim` | quoted prose is secondary |
| `rule` | `chrome.border` | a rule is a line |
| `table_border` | `chrome.border` | a table's border is a border |
| `table_separator` | `chrome.border` | the separator is a line |
| `table_header` | `text.strong` | a table header is strong text |
| `table_cell` | `text.normal` | a cell is ordinary prose |
| `list_level_1` | `text.normal` | the first list level is ordinary text |
| `list_level_2` | `text.dim` | the second is a step dimmer |
| `list_level_3` | `text.dim` | the third stops at the text grade: a document word never drops to the chrome index |
| `ordered_list` | `text.normal` | an ordered marker ranks with the first list level |
| `heading_1` | `chrome.title` | the brightest foreground, bold |
| `heading_2` | `text.strong` | the raised step |
| `heading_3` | `text.normal` | the ordinary step |
| `heading_4` | `text.dim` | the muted step |
| `heading_other` | `text.dim` | h5 and h6 share the dimmest heading step |
| `heading_underline` | `chrome.border` | an underline is a line |
| `link_icon` | `semantic.info` | links are neutral information |
| `link_text` | `semantic.info` | links are neutral information, never a lane |
| `link_hover` | `semantic.info` | mapped for the 47, but the vendored engine never reads it |
| `text` | `text.normal` | ordinary prose |
| `strong_text` | `text.strong` | strong prose |
| `latex_inline_fg` | `text.strong` | inline math is raised prose |
| `latex_inline_bg` | `chrome.border` | the code/maths family's chrome; no background, so Reset |
| `latex_block_fg` | `text.strong` | block math is raised prose |
| `mermaid_keyword` | `text.strong` | a diagram keyword is raised text |
| `mermaid_arrow` | `chrome.border` | an arrow is a line |
| `mermaid_label` | `text.normal` | a node label is ordinary text |
| `mermaid_block_fg` | `text.dim` | the diagram's block chrome is secondary |
| `mark_fg` | `text.selected` | a marked word takes the selection's own foreground, the pair the palette records |
| `mark_bg` | `text.selected` | a mark is a highlight; selection is the slot with a background |
| `task_checked` | `semantic.ok` | a checked task is a clean result |
| `task_unchecked` | `text.dim` | an unchecked task is secondary |
| `alert_note` | `semantic.info` | a note is neutral information |
| `alert_tip` | `semantic.ok` | a tip is a clean result |
| `alert_important` | `semantic.info` | an important callout is neutral information, not a fact that waits on a human |
| `alert_warning` | `semantic.warn` | a warning wants attention and is not broken |
| `alert_caution` | `semantic.error` | a caution is a failure; it never shares the warning's slot |
| `footnote_ref` | `semantic.info` | a reference is neutral information |
| `footnote_text` | `text.dim` | a note is secondary text |

The engine's own four markdown palettes — arctic, forest, ocean and
solarized-dark — are never read. A family palette from
[palettes.md](palettes.md) reaches `MarkdownTheme` through the same eighteen
slots as the chrome; syntect's code-token theme is the one part that does not.

## The presets

| preset | what it is |
|---|---|
| `dark` | the default: the dark palette |
| `terminal` | no colours of our own — the terminal's palette is used |

The preset is chosen with a flag for now: `raul --theme dark|terminal`.
A theme file on disk is later.

The `dark` preset is ours, hand-tuned, and its eighteen values are written
down in [palettes.md](palettes.md), with their roles and their contrast.

**The dark preset assumes a terminal with a dark background, and does not
paint one.** The background is the terminal's own; every colour of the
palette is picked to read on a dark one.

**The light themes are deferred.** No light preset is built until that changes:
not ours, and not the light variants of the families — Catppuccin Latte,
Dracula Alucard and Nord Light wait with them. The slots are shaped so a light
preset drops in without touching anything else.

**Selection is the accent at full opacity.** A terminal has no alpha
compositing, so a translucent selection is not available; every colour
preset uses the accent colour whole. The terminal preset has no accent:
its selection is reversed video, and its own section below says so.

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
- **the selection is not a colour.** `text.selected` is reversed video: the
  shape of the selection carries the meaning, not a hue.
- **the document loses its distinctions.** `MarkdownTheme` carries colours
  only, so every field becomes `Color::Reset`: a checked task, an unchecked
  task, a warning and a caution are painted alike. Only the engine's own
  modifiers survive — bold on headings and strong text, underlined on links
  among them.
- **some roles are weaker than they would be with a colour.** `semantic.info`
  is bold and dim, so the heading is louder than the `draft` row under it,
  which is dim alone; with a colour it would be a hue of its own.
  `semantic.warn` is underlined and `semantic.error` is bold, underlined and
  reversed: loud, but not red.
- **what it cannot do:** it cannot say *which* hue a thing is, and it cannot
  tell a colour from the terminal's own foreground. The five attributes are
  enough for the eight phases and not enough for the eighteen slots: the
  weak chrome slots — the muted foreground, the dimmed foreground, the
  border, the footer and the inactive tab — all hold DIM, and the title,
  the strong text and the focused border all hold BOLD. The showing tab
  and every key label hold BOLD and REVERSED together, so those two paint
  alike. The terminal preset tells the phases apart; it does not tell the
  chrome apart.

The preset exists for terminals whose palette the reader has chosen. Where
the paint matters, the `dark` preset is the one.

## Rules

1. No colour is written inline. Every colour comes from a slot.
2. A slot names a role. A preset decides what the role looks like.
3. The domain knows nothing about any of this.
4. A theme changes the paint, never the words: the same screen says the same
   thing in every preset.

## Not yet

- **the catalog.** Catppuccin (latte, frappé, macchiato, mocha), Dracula
  (dracula, alucard) and Nord (nord, nord-light). The values come from each
  family's own page, and the mapping to our slots is in
  [palettes.md](palettes.md).
- **a theme file on disk.** The preset is a flag for now; a file comes with
  the catalog.
