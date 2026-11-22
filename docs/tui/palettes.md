# Palettes

Where raul's themes come from, and how to write a new one.

Our slots are defined in [theme.md](theme.md). This doc is the **source**
side: the palettes a theme may be built from, where each one came from, and
the rules that keep a new theme consistent with the others.

## Provenance

| what | where it comes from | licence |
|---|---|---|
| the slot structure | ours, shaped by leaf's `UiTheme` | — |
| leaf's own presets — `arctic`, `forest`, `ocean`, `solarized-dark` | leaf's `src/theme/presets.rs` | MIT |
| **the dark palette** — our own hand-tuned preset | ours, like leaf's four own presets | — |
| **Catppuccin** — latte, frappé, macchiato, mocha | [catppuccin.com/palette](https://catppuccin.com/palette/) — the values below are that page's | MIT |
| **Dracula** — dracula, alucard | [draculatheme.com/spec](https://draculatheme.com/spec) — the values below are that page's | MIT |
| **Nord** — nord, nord-light | [nordtheme.com/docs/colors-and-palettes](https://www.nordtheme.com/docs/colors-and-palettes) — the values below are that page's | MIT |
| the mapping rules | [Frond's `COLORS.md`](https://github.com/lthiagol/Frond/blob/main/docs/COLORS.md), adapted to our slots | ours |

**Those three pages are the source of truth.** If a colour is disputed, the
page wins; if we need a role the page does not have, it is derived from a
role it does have, and the derivation is written down here.

Frond is our own other project. It did this exact work for the same upstream.
Where its mapping answers a question, that answer is the answer here.

## The slot mapping

Every theme maps the same way. The left column is ours; the middle is leaf's
key; the right is where a family's role comes from.

| our slot | leaf key | family role |
|---|---|---|
| `chrome.title` | `status_brand_fg` | the family's brightest foreground |
| `chrome.tab_active` | `toc_primary_active` | the family's main foreground |
| `chrome.tab_inactive` | `toc_primary_inactive` | the family's second foreground |
| `chrome.tab_index` | `toc_index_inactive` | the family's dimmed foreground |
| `chrome.border` | `toc_border` | the family's border colour |
| `chrome.border_focus` | `toc_accent` | the family's accent |
| `chrome.footer` | `status_shortcut_fg` | the family's dimmed foreground — its muted one where the dimmed is a border grade |
| `chrome.key` | `status_search_fg` | the family's attention colour |
| `chrome.surface` | — ours | the base mixed 30% toward black — derived |
| `text.normal` | `toc_hover_fg` | the family's main foreground |
| `text.dim` | `toc_header_fg` | the family's muted foreground |
| `text.strong` | `toc_secondary_text_active` | a raised foreground |
| `text.selected` | `toc_active_bg` | the accent, at full opacity — the mark in a document |
| `text.selected_bg` | — ours | the surface mixed 10% toward the accent — derived |
| `semantic.ok` | `status_success_fg` | the family's success colour |
| `semantic.warn` | `status_warning_fg` | the family's warning colour |
| `semantic.error` | `status_error_fg` | the family's error colour |
| `semantic.info` | `status_filename_fg` | the family's nearest-neutral foreground |
| `semantic.human` | `toc_accent` | the family's accent |
| `semantic.agent` | `status_watch_fg` | the family's second accent |
| `role.orchestrator` | — ours | a hue of the family's own, named per family |
| `role.runner` | — ours | a hue of the family's own, named per family |
| `role.reviewer` | — ours | a hue of the family's own, named per family |

The last two are ours: the human lane and the agent lane need two colours that
are not "success" and "error". The accent and the second accent are the
natural pair — the accent is already the focus colour, so the agent lane takes
the other one.

**The two backgrounds are derived.** The sidebar is recessed: its surface is
the base mixed 30% toward black, a step darker than the base, so the sidebar
sits below the main pane rather than on top of it. The selected row's tint is
the surface mixed 10% toward the accent. The step is the floor the tint must
clear: at 1.15:1 or more against the surface, the tint shows where the base
and the tint are close. The code computes both from the palette's own values,
and the tables below record the figures the test recomputes. The tint is a
background under the row's text, not a mark: the row keeps its own colours,
and the bar is what marks it.

The shares are set together. At a quarter toward black, the dark palette's
step reaches 1.15:1 only at a tint share near 10%, and there Dracula's comment
falls to 2.9:1 on the tint; no pair holds both. At 30% and 10%, every word
clears 3:1 on the base, the surface and the tint, and every colour preset's
step is 1.16:1 or more.

**The roles are three hues.** `role.orchestrator`, `role.runner` and
`role.reviewer` are the icon and the name of each agent's role, in the crew
and in the sidebar. Each takes a different hue of the family's own. Where a
family has no hue left unspent, a role takes a hue a slot already spends, and
the family's note says which. The creator takes `semantic.human`, the lane its
icon is drawn from.

A family that has no role of its own for one of these takes the colour it
derives for it, and says so in its note below. A family whose dimmed
foreground is too faint for a word — a border grade — lends the footer's
labels its muted foreground instead, and says so too.

`semantic.info` takes a neutral foreground, not a hue of its own: information
is not a lane, and it does not compete with the human lane for attention.

**The test.** A colour is neutral when the spread between its three channels
— the highest minus the lowest — is at most 48 of 255, under a fifth of the
range. It is a tolerance, not an equality: a neutral need not have three
equal channels. The code holds this same test, and it reads the family
tables below.

**How a neutral is derived.** A family's page does not always name a
neutral, so it is derived from the roles the page does have: it is the
family's *nearest-neutral foreground* — of the family's **word greys** (the
grades it paints text with, not its accents and not the chrome grades it
draws lines with), the one whose three channels lie closest together, inside
the tolerance. A word grey the theme spends on another slot is not a
candidate: the main text colour is `text.normal` and the tab that is showing,
the muted one is `text.dim`, the raised one is `text.strong`, the title one is
`chrome.title`, the tabs that are not showing are `chrome.tab_inactive`, the
filter note above the footer is `chrome.footer` and the index is `chrome.tab_index`. A
chrome grade stays a chrome grade: a family that borrows one for a word does
not make it a candidate. Where the family leaves no such foreground, or spends
every one of them on another role, the main foreground is the neutral — the
way Dracula's does. Each family's table below names the role it is and writes
the derivation down.

A family's own blue or pink is kept for the record, and no slot spends it:
every one of those is well outside the tolerance.

## Ours: the dark palette

One hand-tuned palette, not derived from a family: ours, the way leaf's four
own presets are ours. It is the default, and it is the baseline a later
family theme is compared against.

Its values follow the mapping above: the accent is the focus border, the
mark and the human lane; the second accent is the agent lane; the information
colour is a neutral foreground. The selected row is the tint `#141e28` under
the row's own colours, and the mark in a document is the accent whole, with
the background colour as its foreground: 9.7:1.

**The palette assumes a terminal with a dark background, and raul paints no
background of its own beyond the derived ones.** Every figure below is
measured against `#0d1117`, the background the palette is measured against.
The surface and the tint are the only backgrounds the palette paints.

The eighteen slots, with the role each value plays and the attributes it
carries. The doc is the source of truth: the code is checked against this
table by a test.

| slot | role | foreground | background | attributes |
|---|---|---|---|---|
| `chrome.title` | the brightest foreground | `#ffffff` | — | bold |
| `chrome.tab_active` | the main foreground | `#c9d1d9` | — | bold |
| `chrome.tab_inactive` | the second foreground | `#9aa5b1` | — | — |
| `chrome.tab_index` | the dimmed foreground | `#7d8590` | — | — |
| `chrome.border` | the border colour | `#3d444d` | — | — |
| `chrome.border_focus` | the accent | `#79c0ff` | — | — |
| `chrome.footer` | the dimmed foreground | `#7d8590` | — | — |
| `chrome.key` | the attention colour | `#ffd166` | — | bold |
| `chrome.surface` | the base mixed 30% toward black | — | `#090c10` | — |
| `text.normal` | the main foreground | `#c9d1d9` | — | — |
| `text.dim` | the muted foreground | `#8b949e` | — | — |
| `text.strong` | the raised foreground | `#e6edf3` | — | bold |
| `text.selected` | the mark: the accent, at full opacity | `#0d1117` | `#79c0ff` | — |
| `text.selected_bg` | the surface mixed 10% toward the accent | — | `#141e28` | — |
| `semantic.ok` | the success colour | `#7ec699` | — | — |
| `semantic.warn` | the warning colour | `#ff9e64` | — | — |
| `semantic.error` | the error colour | `#e06c75` | — | — |
| `semantic.info` | a neutral foreground | `#b0b0b0` | — | — |
| `semantic.human` | the accent | `#79c0ff` | — | — |
| `semantic.agent` | the second accent | `#d2a8ff` | — | — |
| `role.orchestrator` | the attention yellow | `#ffd166` | — | — |
| `role.runner` | a teal, apart from the agent's violet | `#56d4dd` | — | — |
| `role.reviewer` | a pink, apart from the error's red | `#ff9bd2` | — | — |

**The agent lane does not use blue.** ANSI blue is `#0000ee`: about 2.0:1 on
the background, the lowest-contrast colour of the fifteen other than black,
and the agent lane is the lane the product is built around. The second accent
is a light violet, 9.7:1 on the background.

Contrast against `#0d1117`, for the record:

| colour | ratio | | colour | ratio |
|---|---|---|---|---|
| `#ffffff` | 18.9:1 | | `#79c0ff` accent | 9.7:1 |
| `#e6edf3` | 16.0:1 | | `#d2a8ff` agent | 9.7:1 |
| `#ffd166` | 13.1:1 | | `#7ec699` ok | 9.4:1 |
| `#c9d1d9` | 12.3:1 | | `#ff9e64` warn | 9.3:1 |
| `#b0b0b0` | 8.7:1 | | `#e06c75` error | 5.9:1 |
| `#9aa5b1` | 7.6:1 | | `#8b949e` dim | 6.2:1 |
| | | | `#7d8590` dimmed | 5.1:1 |
| | | | `#3d444d` border | 1.9:1 |
| | | | `#ffd166` orchestrator | 13.1:1 |
| | | | `#56d4dd` runner | 10.7:1 |
| | | | `#ff9bd2` reviewer | 9.8:1 |

Every foreground that carries text is at 5.1:1 or better. The border is the
one value below that: it is a line, not a word.

## Catppuccin

Four flavours: **latte** (light), **frappé**, **macchiato**, **mocha** (dark).
Source: [catppuccin.com/palette](https://catppuccin.com/palette/).

The base is `#303446` in frappé, `#24273a` in macchiato and `#1e1e2e` in mocha
— the family's own. The theme paints no background of its own beyond the derived
ones, and every figure below is measured against the base of the flavour.

| slot | the family's role | frappé | macchiato | mocha |
|---|---|---|---|---|
| `chrome.title` | rosewater — the raised and the brightest | `#f2d5cf` | `#f4dbd6` | `#f5e0dc` |
| `chrome.tab_active` | text — the main foreground | `#c6d0f5` | `#cad3f5` | `#cdd6f4` |
| `chrome.tab_inactive` | overlay2 — borrowed: the second foreground | `#949cbb` | `#939ab7` | `#9399b2` |
| `chrome.tab_index` | overlay0 — the dimmed chrome | `#737994` | `#6e738d` | `#6c7086` |
| `chrome.border` | surface1 — the border | `#51576d` | `#494d64` | `#45475a` |
| `chrome.border_focus` | mauve — the accent | `#ca9ee6` | `#c6a0f6` | `#cba6f7` |
| `chrome.footer` | subtext1 — borrowed: the muted foreground | `#b5bfe2` | `#b8c0e0` | `#bac2de` |
| `chrome.key` | yellow — the attention colour | `#e5c890` | `#eed49f` | `#f9e2af` |
| `chrome.surface` | the base mixed 30% toward black — derived | `#222431` | `#191b29` | `#151520` |
| `text.normal` | text — the main foreground | `#c6d0f5` | `#cad3f5` | `#cdd6f4` |
| `text.dim` | subtext1 — the muted foreground | `#b5bfe2` | `#b8c0e0` | `#bac2de` |
| `text.strong` | rosewater — the raised foreground | `#f2d5cf` | `#f4dbd6` | `#f5e0dc` |
| `text.selected` | mauve — the mark: the accent, at full opacity | `#ca9ee6` | `#c6a0f6` | `#cba6f7` |
| `text.selected_bg` | the surface mixed 10% toward mauve — derived | `#333043` | `#2a283e` | `#272436` |
| `semantic.ok` | green — the success colour | `#a6d189` | `#a6da95` | `#a6e3a1` |
| `semantic.warn` | peach — the warning colour | `#ef9f76` | `#f5a97f` | `#fab387` |
| `semantic.error` | red — the error colour | `#e78284` | `#ed8796` | `#f38ba8` |
| `semantic.info` | subtext0 — **the neutral** | `#a5adce` | `#a5adcb` | `#a6adc8` |
| `semantic.human` | mauve — the accent | `#ca9ee6` | `#c6a0f6` | `#cba6f7` |
| `semantic.agent` | teal — the second accent | `#81c8be` | `#8bd5ca` | `#94e2d5` |
| `role.orchestrator` | pink | `#f4b8e4` | `#f5bde6` | `#f5c2e7` |
| `role.runner` | sapphire | `#85c1dc` | `#7dc4e4` | `#74c7ec` |
| `role.reviewer` | lavender | `#babbf1` | `#b7bdf8` | `#b4befe` |

Contrast against the base, for the record:

| slot | frappé | macchiato | mocha |
|---|---|---|---|
| `chrome.title` | 8.9:1 | 11.2:1 | 12.9:1 |
| `chrome.tab_active` | 8.1:1 | 9.9:1 | 11.3:1 |
| `chrome.tab_inactive` | 4.5:1 | 5.3:1 | 5.8:1 |
| `chrome.tab_index` | 2.9:1 | 3.2:1 | 3.4:1 |
| `chrome.border` | 1.7:1 | 1.8:1 | 1.8:1 |
| `chrome.border_focus` | 5.6:1 | 6.8:1 | 8.1:1 |
| `chrome.footer` | 6.8:1 | 8.2:1 | 9.3:1 |
| `chrome.key` | 7.6:1 | 10.2:1 | 12.9:1 |
| `text.normal` | 8.1:1 | 9.9:1 | 11.3:1 |
| `text.dim` | 6.8:1 | 8.2:1 | 9.3:1 |
| `text.strong` | 8.9:1 | 11.2:1 | 12.9:1 |
| `text.selected` | 5.6:1 | 6.8:1 | 8.1:1 |
| `semantic.ok` | 7.1:1 | 9.2:1 | 11.0:1 |
| `semantic.warn` | 5.8:1 | 7.6:1 | 9.3:1 |
| `semantic.error` | 4.6:1 | 6.0:1 | 7.1:1 |
| `semantic.info` | 5.6:1 | 6.6:1 | 7.4:1 |
| `semantic.human` | 5.6:1 | 6.8:1 | 8.1:1 |
| `semantic.agent` | 6.4:1 | 8.7:1 | 11.0:1 |
| `role.orchestrator` | 7.5:1 | 9.3:1 | 10.7:1 |
| `role.runner` | 6.3:1 | 7.6:1 | 8.7:1 |
| `role.reviewer` | 6.7:1 | 8.2:1 | 9.2:1 |

**The title and the raised text borrow `rosewater`.** Catppuccin names no
raised foreground: in the dark flavours every grey it has is dimmer than its
text colour, and the brightest colour the family has is `rosewater`. It is an
accent, not a word grey, so it is not a candidate for the neutral,
and it is spent on the title and the raised text besides. In mocha its
channels are 25 apart, inside the tolerance of a neutral, and it is still not
a candidate: the neutral is drawn from the word greys.

**The tabs that are not showing borrow `overlay2`.** Catppuccin's three word
greys are spent — on the ordinary text, the dimmed text and the information —
so the second foreground borrows the brightest overlay. The overlay is a
chrome grade, and a chrome grade stays one: it is not a candidate for the
neutral.

**The footer's labels borrow the muted foreground.** `overlay0` is a border
grade — 2.9:1 in frappé and 3.4:1 in mocha — and the footer's labels are
words. The index keeps the border grade; the labels take `subtext1`.

**The neutral is `subtext0`.** Catppuccin's greys below the text are
`subtext1` and `subtext0`. The text colour is spent on `text.normal` and
`subtext1` on the dimmed text and the footer's labels, so `subtext0` is the
grey the information takes. It is inside the tolerance in every flavour — 25
apart in latte, 41 in frappé, 38 in macchiato and 34 in mocha — and nearer to
a plain grey than `subtext1` in every one of them.

**The accent is mauve, and the second accent is teal.** Catppuccin's style
guide selects with a translucent **Overlay 2** — a terminal has no alpha, so
the selection is the accent whole: the base colour on the accent, 5.6:1 in
frappé and 8.1:1 in mocha.

**The blue is kept for the record.** It is not used by any slot.

**The roles are pink, sapphire and lavender.** Pink is the orchestrator's,
sapphire the runner's and lavender the reviewer's. Lavender sits close to
mauve, the accent, and it is the role nearest the human lane's colour; the
roles still read apart in every flavour.

**Only mocha is a preset.** The table above records the three dark flavours
because they share a mapping, but the preset built from this family is
mocha; latte, frappé and macchiato wait with the light theme.

### Mocha, as the page lists it

Every value [catppuccin.com/palette](https://catppuccin.com/palette/) lists
for mocha, in the page's order. The slots spend seventeen of the twenty-six
values the page lists; the rest are kept for the record.

| role | mocha |
|---|---|
| rosewater | `#f5e0dc` |
| flamingo | `#f2cdcd` |
| pink | `#f5c2e7` |
| mauve | `#cba6f7` |
| red | `#f38ba8` |
| maroon | `#eba0ac` |
| peach | `#fab387` |
| yellow | `#f9e2af` |
| green | `#a6e3a1` |
| teal | `#94e2d5` |
| sky | `#89dceb` |
| sapphire | `#74c7ec` |
| blue | `#89b4fa` |
| lavender | `#b4befe` |
| text | `#cdd6f4` |
| subtext1 | `#bac2de` |
| subtext0 | `#a6adc8` |
| overlay2 | `#9399b2` |
| overlay1 | `#7f849c` |
| overlay0 | `#6c7086` |
| surface2 | `#585b70` |
| surface1 | `#45475a` |
| surface0 | `#313244` |
| base | `#1e1e2e` |
| mantle | `#181825` |
| crust | `#11111b` |

## Dracula

Two variants: **Dracula Classic** (dark) and **Alucard Classic** (light).
Source: [draculatheme.com/spec](https://draculatheme.com/spec).

The base is `#282A36` — the family's own background. The theme paints no
background of its own beyond the derived ones, and every figure below is
measured against the base.

| slot | the family's role | dracula |
|---|---|---|
| `chrome.title` | ANSI bright white — the raised and the brightest | `#FFFFFF` |
| `chrome.tab_active` | foreground — the main foreground | `#F8F8F2` |
| `chrome.tab_inactive` | comment — borrowed: the second foreground | `#6272A4` |
| `chrome.tab_index` | comment — the dimmed foreground | `#6272A4` |
| `chrome.border` | current line — the border | `#6272A4` |
| `chrome.border_focus` | purple — the accent | `#BD93F9` |
| `chrome.footer` | comment — the dimmed foreground | `#6272A4` |
| `chrome.key` | yellow — the attention colour | `#F1FA8C` |
| `chrome.surface` | the base mixed 30% toward black — derived | `#1C1D26` |
| `text.normal` | foreground — the main foreground | `#F8F8F2` |
| `text.dim` | comment — the muted foreground | `#6272A4` |
| `text.strong` | ANSI bright white — the raised foreground | `#FFFFFF` |
| `text.selected` | purple — the mark: the accent, at full opacity | `#BD93F9` |
| `text.selected_bg` | the surface mixed 10% toward purple — derived | `#2C293B` |
| `semantic.ok` | green — the success colour | `#50FA7B` |
| `semantic.warn` | orange — the warning colour | `#FFB86C` |
| `semantic.error` | red — the error colour | `#FF5555` |
| `semantic.info` | foreground — **the neutral** | `#F8F8F2` |
| `semantic.human` | purple — the accent | `#BD93F9` |
| `semantic.agent` | cyan — the second accent | `#8BE9FD` |
| `role.orchestrator` | pink — the family's one unspent hue | `#FF79C6` |
| `role.runner` | cyan — the agent lane's hue, reused | `#8BE9FD` |
| `role.reviewer` | yellow — the attention hue, reused | `#F1FA8C` |

Contrast against the base, for the record:

| slot | dracula |
|---|---|
| `chrome.title` | 14.2:1 |
| `chrome.tab_active` | 13.4:1 |
| `chrome.tab_inactive` | 3.0:1 |
| `chrome.tab_index` | 3.0:1 |
| `chrome.border` | 3.0:1 |
| `chrome.border_focus` | 5.9:1 |
| `chrome.footer` | 3.0:1 |
| `chrome.key` | 12.7:1 |
| `text.normal` | 13.4:1 |
| `text.dim` | 3.0:1 |
| `text.strong` | 14.2:1 |
| `text.selected` | 5.9:1 |
| `semantic.ok` | 10.4:1 |
| `semantic.warn` | 8.4:1 |
| `semantic.error` | 4.5:1 |
| `semantic.info` | 13.4:1 |
| `semantic.human` | 5.9:1 |
| `semantic.agent` | 10.3:1 |
| `role.orchestrator` | 6.0:1 |
| `role.runner` | 10.3:1 |
| `role.reviewer` | 12.7:1 |

Dracula's status colours are **syntax** colours — the spec assigns
Green/Orange/Red/Yellow to code tokens, and a terminal theme reuses them.
Faithful to Dracula, not an oversight. The spec also publishes *functional*
colours for UI, marked **not for terminal applications** — we use the syntax
ones — and UI backgrounds. Neither is a palette colour, and no slot spends
either: the theme paints no background of its own. Both lists are recorded
below, in the page's order.

**The neutral is the `foreground`.** Dracula publishes two near-grey word
colours, the foreground and the ANSI bright white, and no other: the comment
is a blue of its own, 66 channels apart, well outside the tolerance, and the
selection is a background and not a word colour at all. The foreground's
three channels are 6 apart. Both greys are spent on other roles — the bright
white on the title and the raised text, the foreground on the ordinary text —
so the information takes the main foreground, the way the rule says.

**The raised and the brightest foreground are the ANSI bright white.**
Dracula's spec publishes an ANSI palette for terminal applications, and raul
is one: its bright white is `#FFFFFF`, 14.2:1 on the background, a step
above the plain foreground `#F8F8F2` at 13.4:1.

**The tabs that are not showing borrow the comment.** Dracula names no second
foreground: besides the two whites it publishes one dimmed grey. The comment
reads at 3.0:1 on the background, and it carries the muted text, the dimmed
chrome and the tabs that are not showing. The spec gives the current line and
the comment the same value on purpose, so the border is that grey too.

**The footer's labels take the comment as well.** It is the only muted
foreground the family has, and at 3.0:1 it is at the floor a word needs
rather than below it, so rule 4 lets it carry the labels.

**The muted text clears the floor on every background.** The comment reads at
3.0:1 on the base, 3.6:1 on the recessed surface and 3.0:1 in the selected
tint. The tint is the tightest figure, and it is at the floor, not below it:
the sidebar's keys, which are the muted text, read in this preset as they do
in the others.

**The selected row is a tint, and the mark is the accent.** Dracula selects
with its own selection colour `#44475A`, which the theme does not spend: the
selected row is the surface a step toward the purple, the same derivation as
every family's, and the mark in a document is the accent whole, with the base
colour on it, 5.9:1.

**The pink is the orchestrator's role.** It is the family's only hue no other
slot spends, so it takes the orchestrator; the runner and the reviewer take
the cyan and the yellow that are already spent, so a hue carries a second
role.

### The functional colours, as the page lists them

The page marks these **not for terminal applications**: they are recorded for
the record, and no slot spends one.

| role | dracula |
|---|---|
| Functional Red | `#DE5735` |
| Functional Orange | `#A39514` |
| Functional Green | `#089108` |
| Functional Cyan | `#0081D6` |
| Functional Purple | `#815CD6` |

### The UI backgrounds, as the page lists them

The page names these UI backgrounds, a duplicate included; none is spent. The
surface and the tint are derived from the base and the accent, not taken from
the page's backgrounds.

| context | dracula |
|---|---|
| Floating interactive elements | `#343746` |
| Background Lighter | `#424450` |
| Background Light | `#343746` |
| Background Dark | `#21222C` |
| Background Darker | `#191A21` |

### The current-line fallback, as the page lists it

The page names an opaque fallback for the current line, `#353747`, for
editors that cannot alpha-blend it. It is out-of-palette implementation
guidance, not a palette colour: it adds no token, and no slot spends it.

### Dracula Classic, as the page lists it

Every value [draculatheme.com/spec](https://draculatheme.com/spec) lists in
its color palette for Dracula Classic, in the page's order. The slots spend
eleven of the twelve values in this table; the selection is kept for the
record.

| role | dracula |
|---|---|
| background | `#282A36` |
| current line | `#6272A4` |
| selection | `#44475A` |
| foreground | `#F8F8F2` |
| comment | `#6272A4` |
| red | `#FF5555` |
| orange | `#FFB86C` |
| yellow | `#F1FA8C` |
| green | `#50FA7B` |
| cyan | `#8BE9FD` |
| purple | `#BD93F9` |
| pink | `#FF79C6` |

### The ANSI palette, as the page lists it

The page publishes a second palette for terminal applications: sixteen
values, eight of them restating a colour above and eight of them its own. The
theme takes one of the sixteen by name — the bright white, for the raised and
the brightest foreground; the other fifteen are kept for the record. With the
color palette's eleven, the theme spends twelve values of the page, and the
restatements are not counted twice.

| ANSI token | dracula |
|---|---|
| AnsiBlack | `#21222C` |
| AnsiRed | `#FF5555` |
| AnsiGreen | `#50FA7B` |
| AnsiYellow | `#F1FA8C` |
| AnsiBlue | `#BD93F9` |
| AnsiMagenta | `#FF79C6` |
| AnsiCyan | `#8BE9FD` |
| AnsiWhite | `#F8F8F2` |
| AnsiBrightBlack | `#6272A4` |
| AnsiBrightRed | `#FF6E6E` |
| AnsiBrightGreen | `#69FF94` |
| AnsiBrightYellow | `#FFFFA5` |
| AnsiBrightBlue | `#D6ACFF` |
| AnsiBrightMagenta | `#FF92DF` |
| AnsiBrightCyan | `#A4FFFF` |
| AnsiBrightWhite | `#FFFFFF` |

**Only Dracula Classic is a preset.** It ships as `dracula`; alucard waits
with the light theme, and its palette is recorded in *Not yet*.

## Nord

The palette has sixteen colours in four named groups: **polar night**
(nord0–3), **snow storm** (nord4–6), **frost** (nord7–10), **aurora**
(nord11–15).
Source: [nordtheme.com/docs/colors-and-palettes](https://www.nordtheme.com/docs/colors-and-palettes).

The base is `#2e3440` — nord0, the family's own. The theme paints no
background of its own beyond the derived ones, and every figure below is
measured against the base.

| slot | the family's role | nord |
|---|---|---|
| `chrome.title` | nord6 — the raised and the brightest | `#eceff4` |
| `chrome.tab_active` | nord5 — the main foreground | `#e5e9f0` |
| `chrome.tab_inactive` | nord9 — the frost, the second foreground | `#81a1c1` |
| `chrome.tab_index` | nord2 — the dimmed chrome | `#434c5e` |
| `chrome.border` | nord3 — the border | `#4c566a` |
| `chrome.border_focus` | nord8 — the accent | `#88c0d0` |
| `chrome.footer` | nord4 — borrowed: the muted foreground | `#d8dee9` |
| `chrome.key` | nord13 — the attention colour | `#ebcb8b` |
| `chrome.surface` | the base mixed 30% toward black — derived | — | `#20242d` |
| `text.normal` | nord5 — the main foreground | `#e5e9f0` |
| `text.dim` | nord4 — the muted foreground | `#d8dee9` |
| `text.strong` | nord6 — the raised foreground | `#eceff4` |
| `text.selected` | nord8 — the mark: the accent, at full opacity | `#88c0d0` |
| `text.selected_bg` | the surface mixed 10% toward nord8 — derived | — | `#2a343d` |
| `semantic.ok` | nord14 — the success colour | `#a3be8c` |
| `semantic.warn` | nord13 — the warning colour | `#ebcb8b` |
| `semantic.error` | nord11 — the error colour | `#bf616a` |
| `semantic.info` | nord5 — **the neutral** | `#e5e9f0` |
| `semantic.human` | nord8 — the accent | `#88c0d0` |
| `semantic.agent` | nord7 — the second accent | `#8fbcbb` |
| `role.orchestrator` | nord15 — the aurora's purple | `#b48ead` |
| `role.runner` | nord12 — the aurora's orange | `#d08770` |
| `role.reviewer` | nord9 — the frost | `#81a1c1` |

Contrast against the base, for the record:

| slot | nord |
|---|---|
| `chrome.title` | 10.8:1 |
| `chrome.tab_active` | 10.3:1 |
| `chrome.tab_inactive` | 4.6:1 |
| `chrome.tab_index` | 1.4:1 |
| `chrome.border` | 1.7:1 |
| `chrome.border_focus` | 6.2:1 |
| `chrome.footer` | 9.2:1 |
| `chrome.key` | 8.0:1 |
| `text.normal` | 10.3:1 |
| `text.dim` | 9.2:1 |
| `text.strong` | 10.8:1 |
| `text.selected` | 6.2:1 |
| `semantic.ok` | 6.1:1 |
| `semantic.warn` | 8.0:1 |
| `semantic.error` | 3.1:1 |
| `semantic.info` | 10.3:1 |
| `semantic.human` | 6.2:1 |
| `semantic.agent` | 6.0:1 |
| `role.orchestrator` | 4.4:1 |
| `role.runner` | 4.4:1 |
| `role.reviewer` | 4.6:1 |

Nord's light variant follows Nord's own **bright ambiance** style: snow storm
as the base, polar night as the text. Nord's aurora colours work the same in
both.

**Nord's greys carry the words.** Nord's greys
are the polar night (nord0 to nord3) and the snow storm (nord4 to nord6).
The polar night is a chrome grade — the base, the elevated chrome, the dimmed
chrome and the border — and on the family's own base it is faint: nord3 reads
at 1.7:1. The snow storm is what carries words: the muted text, the ordinary
text, and the raised and the brightest.

**The muted text is derived: the dimmest snow storm grey, nord4.** Nord has
no muted foreground of its own for words — the greys other than the snow
storm are the polar night, and they are a chrome grade: nord3 reads at 1.7:1
on the base, which is a border, not a word, and rule 3 says the muted text is
still readable. So the muted text is the dimmest snow storm grey, nord4, which
reads at 9.2:1: a step down from the text, and still a word.

**The neutral gives way.** The snow storm's three greys are all spent on the
text roles, so no foreground is left for the information; the information
takes the main foreground, nord5, the way Dracula's does. It is 11 channels
apart — nearer to a plain grey than the muted text's nord4, which is 17. The
frost and the aurora are hues and not word greys: nord7 is 45 channels apart
and near enough to the tolerance, but it is the agent lane.

**nord1 and nord10 are kept for the record.** nord1 `#3b4252` is the elevated
polar night; the chrome needs only nord2 and nord3. nord10 `#5e81ac` is a frost
blue, 78 channels apart: a hue of its own, so the information takes a neutral
instead. The aurora's orange nord12 and purple nord15 are the runner's and the
orchestrator's roles, and the reviewer takes the frost, nord9: the family has
no third aurora hue that reads on the base.

**The second foreground is the frost, nord9.** Nord's palette names no second
foreground; the theme takes nord9 for the tabs that are not showing, and it
reads at 4.6:1 on the base.

**The footer's labels borrow the muted text.** nord2 is a border grade — 1.4:1
on the base — and the footer's labels are words. The index keeps the border
grade; the labels take nord4.

**The error is the family's own red.** nord11 reads at 3.1:1 on the base: the
aurora is low-contrast on the polar night, and rule 7 asks for the family's
own semantics, so the failure keeps the family's red.

**The light variant waits with the light theme.** Nord Light spends the snow
storm on its base and takes its words from the polar night, so both its muted
text and its neutral wait with the light theme. Its palette is recorded in
*Not yet*.

## leaf's own

Four hand-tuned presets, not derived from a family: **arctic**, **forest**,
**ocean**, **solarized-dark**. The values are leaf's constants, taken
verbatim from `src/theme/presets.rs`. `ocean` is leaf's default and the model
for ours.

## The document palette

The document — the markdown pane — has a palette of its own: **eighteen
roles**, which are not the chrome's eighteen slots. A role is a job the
document has, and each preset paints it from the same family palette the
chrome draws from, so the document and the chrome are one palette and not
two. The chrome keeps its own slots; nothing here changes them. The roles are
bound by [the rules for a new theme](#the-rules-for-a-new-theme) as the
chrome's slots are.

The document's headings are the three levels the projection writes: `#` for a
section, `##` for an item under it, and `###` for a finding under its round.
The engine's `heading_4` and `heading_other` join the deepest, because the
document has no level below `###`. The record's title is a label, not prose,
and is never a heading.

### The roles

Eighteen is the human's number. Eleven are fixed by the constraints: three
heading levels, five alert severities and three list depths. Seven are the
rest of the document's work: the prose, the links, the code, the lines, the
mark, the search and the tasks. Six of those seven name a **pair**, because
their job has two sides, and the unit says which: the prose at two weights,
the code's ink and its chip, the document's line and the table's header rule,
the mark's foreground and its background, the search's hit and its current
match, and a task's done and open boxes. The other twelve roles name one
colour.

| role | job | sides |
|---|---|---|
| `document.heading.section` | the `#` line: one of the ten sections | one colour |
| `document.heading.item` | the `##` line: an item under a section | one colour |
| `document.heading.finding` | the `###` line: a finding under its round; `####` and deeper join it | one colour |
| `document.text` | the document's words | `body`, `emphasis` |
| `document.list.1` | the first list depth, and an ordered list's marker | one colour |
| `document.list.2` | the second list depth | one colour |
| `document.list.3` | the third list depth | one colour |
| `document.alert.note` | a `[!NOTE]` alert | one colour |
| `document.alert.tip` | a `[!TIP]` alert | one colour |
| `document.alert.important` | a `[!IMPORTANT]` alert | one colour |
| `document.alert.warning` | a `[!WARNING]` alert | one colour |
| `document.alert.caution` | a `[!CAUTION]` alert | one colour |
| `document.link` | a link's marker, its text, its hover state, and a footnote's reference | one colour |
| `document.code` | code, maths and a diagram: the ink and the chip behind it | `text`, `chip` |
| `document.rule` | the document's lines, and the table's header rule | `line`, `header` |
| `document.selection` | a mark: the reader's highlight | `foreground`, `background` |
| `document.search` | the search: a hit and the current match | `hit`, `match` |
| `document.task` | a task list's box | `done`, `open` |

The first list depth and an ordered list's marker share a role: an ordered
marker is a list marker at the first depth. The palette tells the three depths
apart; it does not tell a bullet from a number, because both are the marker of
the same depth. A heading level, an alert severity and a list depth never
share one: they are three kinds of job, and the palette keeps them apart in
every preset.

The prose's emphasis takes the family's raised foreground, which is one of the
heading grades in every palette — the section's in Nord, Catppuccin and
Dracula, the item's in dark. A bold word and a heading share a colour, and the
family has one raised foreground; the heading is on a line of its own and the
word is inside a sentence, so the shape tells them apart.

The link takes the family's accent, the hue the chrome gives the focused
border and the human lane. The document is not a lane: a link is the one thing
in it that leads somewhere, and the accent is the product's own "this leads
somewhere" hue. It is apart from the five alerts in every preset.

### The values

Each preset paints the eighteen roles from its own family. Each cell states
the colour and the figure the palette's test recomputes; the figure is the
one that decides whether the colour works:

- for a word or a line, the colour's contrast against the preset's base;
- for the code, the ink's contrast on its chip — and, for the chip, the
  chip's contrast against the base;
- for the mark, the foreground's contrast on its background — and, for the
  background, the background's contrast against the base;
- for the search, the prose's contrast on the background, which is the figure
  that decides whether the search can be offered.

The bases are the dark palette's `#0d1117`, Nord's nord0 `#2e3440`, Catppuccin
mocha's base `#1e1e2e` and Dracula's `#282A36`. The dark palette is ours, and
it holds its document words to 4.5:1 the way it holds the chrome's; the
families hold theirs to the 3:1 floor the rules set — Nord's and Dracula's
dimmest document word is at 3.0:1, Catppuccin's at 5.8:1.

| role | side | dark | nord | catppuccin | dracula |
|---|---|---|---|---|---|
| `document.heading.section` | — | `#ffffff` 18.92:1 | `#eceff4` 10.84:1 | `#f5e0dc` 12.95:1 | `#ffffff` 14.24:1 |
| `document.heading.item` | — | `#e6edf3` 16.02:1 | `#e5e9f0` 10.26:1 | `#cdd6f4` 11.34:1 | `#f8f8f2` 13.36:1 |
| `document.heading.finding` | — | `#9aa5b1` 7.56:1 | `#81a1c1` 4.64:1 | `#9399b2` 5.81:1 | `#6272a4` 3.03:1 |
| `document.text` | body | `#c9d1d9` 12.26:1 | `#d8dee9` 9.25:1 | `#bac2de` 9.26:1 | `#f8f8f2` 13.36:1 |
| `document.text` | emphasis | `#e6edf3` 16.02:1 | `#eceff4` 10.84:1 | `#f5e0dc` 12.95:1 | `#ffffff` 14.24:1 |
| `document.list.1` | — | `#c9d1d9` 12.26:1 | `#e5e9f0` 10.26:1 | `#cdd6f4` 11.34:1 | `#ffffff` 14.24:1 |
| `document.list.2` | — | `#8b949e` 6.15:1 | `#d8dee9` 9.25:1 | `#bac2de` 9.26:1 | `#f8f8f2` 13.36:1 |
| `document.list.3` | — | `#7d8590` 5.07:1 | `#81a1c1` 4.64:1 | `#a6adc8` 7.37:1 | `#6272a4` 3.03:1 |
| `document.alert.note` | — | `#b0b0b0` 8.73:1 | `#e5e9f0` 10.26:1 | `#a6adc8` 7.37:1 | `#f8f8f2` 13.36:1 |
| `document.alert.tip` | — | `#7ec699` 9.39:1 | `#a3be8c` 6.13:1 | `#a6e3a1` 11.03:1 | `#50fa7b` 10.38:1 |
| `document.alert.important` | — | `#d2a8ff` 9.72:1 | `#8fbcbb` 5.99:1 | `#94e2d5` 11.01:1 | `#8be9fd` 10.29:1 |
| `document.alert.warning` | — | `#ff9e64` 9.31:1 | `#ebcb8b` 8.00:1 | `#fab387` 9.27:1 | `#ffb86c` 8.36:1 |
| `document.alert.caution` | — | `#e06c75` 5.92:1 | `#bf616a` 3.05:1 | `#f38ba8` 7.08:1 | `#ff5555` 4.53:1 |
| `document.link` | — | `#79c0ff` 9.73:1 | `#88c0d0` 6.24:1 | `#cba6f7` 8.07:1 | `#bd93f9` 5.90:1 |
| `document.code` | text | `#e6edf3` 8.33:1 | `#eceff4` 6.40:1 | `#f5e0dc` 7.20:1 | `#ffffff` 4.71:1 |
| `document.code` | chip | `#3d444d` 1.92:1 | `#4c566a` 1.69:1 | `#45475a` 1.80:1 | `#6272a4` 3.03:1 |
| `document.rule` | line | `#3d444d` 1.92:1 | `#4c566a` 1.69:1 | `#45475a` 1.80:1 | `#6272a4` 3.03:1 |
| `document.rule` | header | `#e6edf3` 16.02:1 | `#eceff4` 10.84:1 | `#f5e0dc` 12.95:1 | `#ffffff` 14.24:1 |
| `document.selection` | foreground | `#0d1117` 9.73:1 | `#2e3440` 6.24:1 | `#1e1e2e` 8.07:1 | `#282a36` 5.90:1 |
| `document.selection` | background | `#79c0ff` 9.73:1 | `#88c0d0` 6.24:1 | `#cba6f7` 8.07:1 | `#bd93f9` 5.90:1 |
| `document.search` | hit | `#3d444d` 6.38:1 | `#4c566a` 5.46:1 | `#45475a` 5.15:1 | `#6272a4` 4.41:1 |
| `document.search` | match | `#79c0ff` 1.26:1 | `#88c0d0` 1.48:1 | `#cba6f7` 1.15:1 | `#bd93f9` 2.26:1 |
| `document.task` | done | `#7ec699` 9.39:1 | `#a3be8c` 6.13:1 | `#a6e3a1` 11.03:1 | `#50fa7b` 10.38:1 |
| `document.task` | open | `#8b949e` 6.15:1 | `#d8dee9` 9.25:1 | `#bac2de` 9.26:1 | `#6272a4` 3.03:1 |

### The derivations

**The dark palette.** The section heading takes the brightest foreground, the
item the raised one, and the finding the second foreground, a cool step below
the prose — the family names no third white. The prose is the main foreground
and its emphasis the raised one; the three list depths are the main, the muted
and the dimmed foreground. The five alerts are the neutral, the success, the
second accent, the warning and the error; the link is the accent. The code's
ink is the raised foreground on the border's chip; the table's line is the
border and its header rule the header's own grade; the mark is the selection's
pair; the search takes the border for a hit and the accent for the current
match; a task's boxes are the success and the muted foreground.

**Nord.** Nord names four word grades — the three snow storm greys and the
frost — and the document's eight word roles take them. The section is nord6,
the item nord5, the prose nord4 (still a word, at 9.2:1) and the finding the
frost nord9, a cool shade below the prose. The list depths are nord5, nord4
and nord9. The note is nord5, the grey the chrome's information takes,
because Nord spends every neutral; the tip is nord14, importance nord7, the
warning nord13 and the caution nord11. The link is the accent, nord8. With
four words for eight roles, grades repeat: the section heading and the prose's
emphasis share nord6; the item heading, the first list depth and the note share
nord5; the prose and the second list depth share nord4; the finding and the
third depth share nord9. The roles are apart; the family simply has four
words, and no other grade is readable.

**Catppuccin.** The section heading takes rosewater, the item text, and the
prose subtext1, a step under the chrome's ordinary text; the finding takes
overlay2, the cool grey the chrome gives the tabs that are not showing. The
list depths are text, subtext1 and subtext0; the note takes subtext0. The
family carries five word colours for eight word roles, so they repeat: the
section heading and the prose's emphasis share rosewater; the item heading and
the first list depth share text; the prose and the second list depth share
subtext1; the note and the third depth share subtext0. The finding alone takes
overlay2. The alerts, the link, the code and the mark take the same roles the
other presets give them: the success, the second accent, the warning and the
error for the alerts, mauve for the link, rosewater on surface1 for the code,
surface1 with rosewater for the table, and the mark in mauve with the base on
it.

**Dracula.** Dracula names two whites and one blue-grey. The section heading
takes the bright white; the item heading and the prose take the foreground,
and the weight is what separates them — the family has no third white; the
finding takes the comment, a step well below the prose, so the level the
report named is apart. The list depths are the bright white, the foreground
and the comment. The note takes the foreground, the way Dracula's information
does; the tip is the green, importance the cyan, the warning the orange and
the caution the red. The link is the purple; the code's ink is the bright
white on the current line's chip; the table's line is the current line and
its header rule the bright white; the mark is the purple with the background
on it.

### The pairs

The mark is the chrome's own selection pair — the background colour on the
accent — so the palette's second rule holds for the document too: the marked
word reads at 9.7:1 in dark, 6.2:1 in nord, 8.1:1 in catppuccin and 5.9:1 in
dracula. The code's ink reads on its chip at 8.3:1, 6.4:1, 7.2:1 and 4.7:1.

The table's two line grades are apart in every preset: the border is the
faint line (1.9:1 in dark) and the header rule takes the header's own grade
(16.0:1), because the rule under a header is part of the header. The three
list depths are apart, and the three heading levels are apart.

### The search

The search's hit takes the line's grade and its current match takes the
accent. The prose on the hit reads at 6.4:1 in dark, 5.5:1 in nord, 5.2:1 in
catppuccin and 4.4:1 in dracula; the prose on the current match reads at
1.26:1, 1.48:1, 1.15:1 and 2.26:1. The current match is below 3:1 in every
colour preset, so the search stays un-offered: the document's own palette
made the hit readable and did not make the match readable. The ruling in
[theme.md](theme.md) stands, with its figures recomputed.

### The declared roles

Some of the eighteen roles are lit by nothing the record carries today. The
palette covers the **engine's vocabulary**, not only what the projection
writes: the engine can paint an alert, a nested list or a search hit the
moment the document holds one, and the palette has to say what those look
like when they happen.

So these roles are **declared, not lit**:

| role | lit by |
|---|---|
| the five alerts | a task's **overlays** — `blocked`, `deferred`, `cancelled` — once the projection writes them as alerts |
| `list_depth_2`, `list_depth_3` | a nested list; the model has no nested steps today |
| `search` | the search, which is **not offered** in any preset — its own section says why |

A declared role is not dead colour: dead colour would be a role the document
*can* show and does not. These are roles the document cannot show yet.

### The terminal preset

The terminal preset paints none of the eighteen roles: every one resolves to
`Color::Reset`, and what still tells one role from another is the engine's
own modifiers. The document says the same thing in every preset; only the
paint changes.

## The code's syntax

The code tokens of a fenced block are the one paint outside the eighteen roles:
the engine highlights them with a syntect theme. Each preset pairs its own,
derived in `crates/raul/src/detail.rs` (`code_theme`) from the preset's
document palette, so the tokens follow the preset and no bundled syntect theme
is fetched at build time. Each token class takes one role:

| token class | scopes | role | side |
|---|---|---|---|
| the default | a token no scope names | `document.code` | `text` |
| comment | `comment` (italic) | `document.list.3` | — |
| keyword | `keyword`, `storage` | `document.link` | — |
| string | `string` | `document.alert.tip` | — |
| constant, number | `constant` | `document.alert.warning` | — |
| function | `entity.name.function`, `support.function` | `document.alert.important` | — |
| type | `entity.name.type`, `support.type`, `support.class` | `document.heading.item` | — |
| operator, punctuation | `keyword.operator`, `punctuation` | `document.list.2` | — |
| invalid | `invalid` | `document.alert.caution` | — |

A colour the mapping names as the terminal's own is alpha 0 in syntect, which
the engine reads as `Color::Reset`, so the terminal preset emits no RGB for its
tokens. The engine's own four markdown presets are never read.

The code's frame, label and numbers are the document's: the frame is the
line, the label the prose, and the numbers the gutter's dim prose. Inline code
is a chip, `document.code` with its ink on the chip, padded a space each side.

## The theme file

A theme is a TOML file with the slots under a section. The format is leaf's,
so a theme written for leaf can be read here:

```toml
# coding-crew theme
[theme]
preset = "nord"

[colors]
chrome.title = "#eceff4"
chrome.tab_active = "#e5e9f0"
chrome.tab_inactive = "#81a1c1"
chrome.tab_index = "#434c5e"
chrome.border = "#4c566a"
chrome.border_focus = "#88c0d0"
# the footer's labels borrow the muted text: the dimmed chrome is a border grade
chrome.footer = "#d8dee9"
chrome.key = "#ebcb8b"
chrome.surface = "#20242d"
text.normal = "#e5e9f0"
text.dim = "#d8dee9"
text.strong = "#eceff4"
text.selected = "#88c0d0"
text.selected_bg = "#2a343d"
semantic.ok = "#a3be8c"
semantic.warn = "#ebcb8b"
semantic.error = "#bf616a"
# the neutral: no grey is left, so the information takes the main foreground
semantic.info = "#e5e9f0"
semantic.human = "#88c0d0"
semantic.agent = "#8fbcbb"
role.orchestrator = "#b48ead"
role.runner = "#d08770"
role.reviewer = "#81a1c1"
```

The file format is later; the presets ship as code first.

## The rules for a new theme

1. **Every slot gets a value.** A theme that leaves a slot unset falls back
   to the default, and the screen changes between presets — which breaks the
   rule that a theme changes the paint and never the words.
2. **Selection is a tint; the mark is the accent at full opacity.** A
   terminal has no alpha compositing, so a translucent selection does not
   exist. A selected row takes `text.selected_bg`, a tint a step from the
   surface toward the accent, and a mark in a document takes the accent whole.
   This is leaf's rule for the mark, and Frond kept it.
3. **Dimmed is still readable.** `text.dim` is for secondary text, not for
   text that may be missed: it reads at 3:1 or better on the background it
   sits on. If a family has no dimmed role, derive one from its muted
   foreground — do not reach for black. A theme that takes its muted text
   from another of the family's colours says so in its note.
4. **Chrome is not a word.** A border, an index and the tab bar's chrome may
   sit below 3:1 — they are lines. The footer's labels are words and are
   not: a family whose dimmed foreground is a border grade lends them its
   muted foreground instead, and says so in its note.
5. **The human lane and the agent lane are different colours.** They are the
   two halves of the product; they must never share one.
6. **The lanes are readable.** The lane colours carry the two headlines the
   product is built around, so they are picked to read on the background
   they sit on — ANSI blue on the background is 2.0:1, which is not
   readable, and the agent lane does not use it.
7. **Status semantics come from the family's own semantics.** If a family has
   no warning colour, it has no warning colour — use its accent and say so in
   the theme's note, rather than inventing a hue.
8. **The information colour is a neutral.** It is the family's
   nearest-neutral foreground, derived and named above. A theme that has no
   such role — or that spends every one of them on another role — says in
   its note where its neutral comes from.
9. **A theme is a palette, not a mood.** No new colours beyond the family's
   roles, except where rule 6 applies, or the two backgrounds the mapping
   derives: `chrome.surface` and `text.selected_bg`, each computed from the
   family's own values and recorded in its table.
10. **The preset changes nothing but colour.** The same words, the same layout,
    the same counts. This is checked by the render tests, which run against
    every preset, on every tab.
11. **The document's roles are slots too.** A theme that paints the document
    gives every one of its eighteen roles a value from the family's own
    palette; a role it leaves unset falls back to the default, and the
    document changes between presets — which breaks rule 10 the same way an
    unset chrome slot does. The roles are the theme crate's
    [document palette](#the-document-palette), and the rules above bind them
    as they bind the chrome's.

## Not yet

- **The rest of the catalog as presets** — Catppuccin's latte, frappé and
  macchiato, Dracula's alucard, and `nord-light`. The values above are the
  work plan; the mapping is done, the code is not. Mocha ships as
  `catppuccin`, Dracula Classic as `dracula`, and Nord ships as `nord`
- the theme file on disk
- a user theme that is not one of the built-ins

**The other variants are deferred with the light theme**: Catppuccin Latte,
Frappé and Macchiato, Dracula Alucard and Nord Light wait until that changes.
Their palettes are recorded here, so they are ready when the time comes:

- **Latte**: base `#eff1f5`, mantle `#e6e9ef`, crust `#dce0e8`, surface0
  `#ccd0da`, surface1 `#bcc0cc`, surface2 `#acb0be`, overlay0 `#9ca0b0`,
  overlay1 `#8c8fa1`, overlay2 `#7c7f93`, text `#4c4f69`, subtext0 `#6c6f85`,
  subtext1 `#5c5f77`, rosewater `#dc8a78`, mauve `#8839ef`, teal `#179299`,
  green `#40a02b`, peach `#fe640b`, red `#d20f39`, yellow `#df8e1d`, blue
  `#1e66f5`.
- **Alucard**: background `#FFFBEB`, background darker `#BCBAB3`, current line
  `#6C664B`, selection `#CFCFDE`, foreground `#1F1F1F`, comment `#6C664B`,
  purple `#644AC9`, cyan `#036A96`, green `#14710A`, orange `#A34D14`, red
  `#CB3A2A`, yellow `#846E15`, pink `#A3144D`.
- **Nord Light**: base `#d8dee9` nord4, elevated `#e5e9f0` nord5, dimmed
  chrome `#434c5e` nord2, border `#4c566a` nord3, main foreground `#2e3440`
  nord0, raised `#eceff4` nord6, second foreground `#81a1c1` nord9, accent
  `#88c0d0` nord8, agent `#8fbcbb` nord7, ok `#a3be8c` nord14, warn `#ebcb8b`
  nord13, error `#bf616a` nord11, key `#ebcb8b` nord13, frost `#5e81ac`
  nord10.
