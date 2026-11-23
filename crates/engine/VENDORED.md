# Vendored leaf markdown engine

This crate is [leaf](https://github.com/rivolink/leaf)'s markdown engine,
adapted into a library. It is intentionally not byte-identical to upstream;
the numbered ledger below records every deviation.

## Upstream

| field | value |
|---|---|
| repository | https://github.com/rivolink/leaf |
| tag | `1.29.2` |
| commit | `29f6ad480d95a4240c63f63d390e9909afdad9e8` |
| commit date | 2026-10-25 |
| license | MIT (see `LICENSE`) |
| vendored date | 2026-11-17 |
| upstream path | `src/markdown/` (18 modules) and the markdown parts of `src/theme/` |

The pin is a full commit, not a moving branch. `sync.sh check` fetches that
commit and compares it with the copy here.

The crate's SPDX licence is `MIT` because the vendored code is MIT-only; see
`crates/engine/Cargo.toml`.

## Upstream files

Every file in `src/markdown` at the pinned commit is vendored. The manifest
also records the SHA-256 of each side; a test reads it so the ledger and the
bytes cannot drift apart silently.

| upstream file | upstream sha256 | vendored file | vendored sha256 | deviation |
|---|---|---|---|---|
| `src/markdown/blocks.rs` | `8c4e53212383b794247b2e7502a6d6b74e01266a322ad42c787b98c0524360f7` | `crates/engine/src/markdown/blocks.rs` | `8474530a55fa7ed5865a48b0a9e4c25532323d0e9916b4093defb357c4c6d714` | 1, 6, 11, 16, 17, 21, 22 |
| `src/markdown/fences.rs` | `e5810317dd42f09780ddbd800d6ef7d81e213461181f38096cc0821d554d6ce0` | `crates/engine/src/markdown/fences.rs` | `99cc29ba50ee9b591dded59ef745d03dbd6cc82ec5c910268246e81400242487` | 11, 12 |
| `src/markdown/footnotes.rs` | `40a60dbff36117b6e4a6803bca6ba43b4b5965c523285d2d8471e89f9014ee76` | `crates/engine/src/markdown/footnotes.rs` | `0c4a012058588f9b34c6bfcef85382296a077fdb3c836e512a58636d32ca01aa` | 1, 4, 11 |
| `src/markdown/frontmatter.rs` | `de8dd7ae2337ab6a2c7837e98826280f4e0374fa809c3cfbaf6af7f0918d1e8c` | `crates/engine/src/markdown/frontmatter.rs` | `de8dd7ae2337ab6a2c7837e98826280f4e0374fa809c3cfbaf6af7f0918d1e8c` | identical |
| `src/markdown/highlight.rs` | `7e165aaf89a47d464dd2e1f460f1562db323ace19bf7ca619aad802bd8a6504d` | `crates/engine/src/markdown/highlight.rs` | `98a1304ac81d7ec84edc8a1525a2857b6e050d521cfd336c0370e3ac14a750e6` | 1, 6, 7, 11 |
| `src/markdown/latex.rs` | `33b46c28d939af3ef114e1743e6a0fd8630f9d85433c86297b6a57d52b6a8c21` | `crates/engine/src/markdown/latex.rs` | `7ee7f100464e49241553a60f4458ef3979a3e92fdea5f2e13392a04b7e86ad44` | 11, 12 |
| `src/markdown/links.rs` | `59ec5ee1efa8d08023958f77d4e7b3859d37798831578c84dbc699299e4d856b` | `crates/engine/src/markdown/links.rs` | `6e4fb7072760b67ca8778b10234e67a15cb72f544ffc4141a7004f7cc9e38b77` | 1, 6, 9, 11 |
| `src/markdown/lists.rs` | `dd16f32b5fbb766560e7c0e8395d00813496ba4ade00bb07bf330e1bff127f75` | `crates/engine/src/markdown/lists.rs` | `a3b96e07fa0b01dc5aed15cef9326700382c4e154fd4fc9840b7dfc17b6cba70` | 1, 4, 6, 11, 18 |
| `src/markdown/markers.rs` | `2e9f315bad630b9c651d79bc1118e9ba5bcded9cebc81fd37e15472bfaf841ca` | `crates/engine/src/markdown/markers.rs` | `f07514a3ecb6d37ae767c49ec860986bda079e56e79d3b96e53b93491f8eaa79` | 1, 3, 10, 11 |
| `src/markdown/mermaid.rs` | `dc2b217974a36dcd06dc0a5c199bdc7cadef743ec7719279f304e01a677df9da` | `crates/engine/src/markdown/mermaid.rs` | `f1aeaebd7a4ce303098046cffa017fdb0ab27939f594efed2f14f5c112b10274` | 1, 11, 12 |
| `src/markdown/mod.rs` | `53177c3e6b309cdca9c9af0e9ef511e7bb7581c29248a4c489fa76b63bfbcebb` | `crates/engine/src/markdown/mod.rs` | `66d0328d26037453ae6e6b4072c1be4697850e5810ea1881ce69ebd1d69b19bc` | 1, 4, 5, 6, 7, 9, 11, 18, 19, 20, 21, 22 |
| `src/markdown/spans.rs` | `51f40f57a3fbea5638ee3d74b157122c7616b3c480721b7888e6fedd3e2e3be6` | `crates/engine/src/markdown/spans.rs` | `ef8844a9c341d9a5bc79d04e3129baa902d5b7daaaf95eeaecd746593ddd50a6` | 7, 11, 21, 24 |
| `src/markdown/syntax.rs` | `b46409c050d4c9fc859be1acf958859958f000c77217cec453caff49b81e1403` | `crates/engine/src/markdown/syntax.rs` | `6c60870f950e420442734ce06fe612dcbd7813ba41a45443e3f85a11c72bfbf3` | 1, 6, 11, 12, 23 |
| `src/markdown/table_layout.rs` | `05e2d328a05ae048f2d52c4f72eb365414e21fe621ac62e67f04f8b9f03703a7` | `crates/engine/src/markdown/table_layout.rs` | `108c3621ecf055de7253310473c5cf6df9b93a583a877676584a1e0665801b9c` | 7, 11, 12, 15, 21, 24, 25 |
| `src/markdown/tables.rs` | `b824647992b0bb665f516e95b8537594ceba3b6846f2d845d66695febfa66a10` | `crates/engine/src/markdown/tables.rs` | `9c2782f24a93aafc2f94ae555ae82e04dc00e907dc590f20f7906418699cd250` | 2, 3, 5, 7, 11, 15, 20, 21, 24 |
| `src/markdown/toc.rs` | `68ae47f2387f7cb66ae50929d6554684a25312eb1a5323bbefdf5b82aadca0bd` | `crates/engine/src/markdown/toc.rs` | `9640a529ae05aff4ca96cdde1ff72772bceb7ca336294a81bf79722beec873e8` | 1, 4, 6, 9, 11 |
| `src/markdown/width.rs` | `e577f4896abf16ec68a9c3a837ddd6e743e1f4da4e86c52b8fa9411b458a7ad2` | `crates/engine/src/markdown/width.rs` | `3becdd8c8c833d7deca5a027a9e29a585b7731109450af386737f32f0f0346bc` | 1, 6, 7, 11 |
| `src/markdown/wrapping.rs` | `758c50ef5145f2d9d331f8ac0b9951ab4ea1e96ebf0160ff597ab4a9431d213d` | `crates/engine/src/markdown/wrapping.rs` | `6ca53158fc40769cd08f36ddbfbc40c557aee276c1b654163f44ba94e90848e0` | 11 |

## Upstream theme sources

The markdown palette was not a module under `src/markdown`. These upstream
files were read to build `crates/engine/src/theme.rs`; none is copied
verbatim, and the checksum contract below covers only the 18 markdown
modules.

| upstream file | disposition |
|---|---|
| `src/theme/mod.rs` | `MarkdownTheme` kept; app-wide theme state removed (deviation 3) |
| `src/theme/presets.rs` | the four markdown palettes kept; UI, diff and app composition removed (deviation 3) |
| `src/theme/resolution.rs` | not vendored: custom theme resolution is app config (deviation 3) |
| `src/theme/serde.rs` | not vendored: custom theme loading is app config (deviation 3) |

## Added engine files

These files do not exist upstream.

| file | purpose |
|---|---|
| `crates/engine/Cargo.toml` | library manifest and the engine's dependency set |
| `crates/engine/src/lib.rs` | library root, public re-exports, syntax assets |
| `crates/engine/assets/toml.sublime-syntax` | the TOML grammar `syntax_set()` adds (deviation 26) |
| `crates/engine/src/theme.rs` | collapsed markdown theme and presets |
| `crates/engine/src/tests/mod.rs` | shared test helpers |
| `crates/engine/src/tests/markdown_blocks.rs` | ported `src/tests/markdown_blocks.rs` |
| `crates/engine/src/tests/markdown_embedded.rs` | ported `src/tests/markdown_embedded.rs` |
| `crates/engine/src/tests/markdown_footnotes.rs` | ported `src/tests/markdown_footnotes.rs` |
| `crates/engine/src/tests/markdown_html_tag.rs` | ported `src/tests/markdown_html_tag.rs` |
| `crates/engine/src/tests/markdown_links.rs` | ported `src/tests/markdown_links.rs` |
| `crates/engine/src/tests/markdown_list_blocks.rs` | ported `src/tests/markdown_list_blocks.rs` |
| `crates/engine/src/tests/markdown_lists.rs` | ported `src/tests/markdown_lists.rs` |
| `crates/engine/src/tests/markdown_page.rs` | each construct of the page at 40 and 100 columns |
| `crates/engine/src/tests/markdown_rendering.rs` | ported `src/tests/markdown_rendering.rs` |
| `crates/engine/src/tests/markdown_tables.rs` | ported `src/tests/markdown_tables.rs` |
| `crates/engine/src/tests/markdown_toml.rs` | a `toml` fence resolves to the bundled grammar and paints its roles |
| `crates/engine/src/tests/wrapping.rs` | ported `src/tests/wrapping.rs` |
| `crates/engine/tests/vendored.rs` | checks this document against the vendored bytes |
| `crates/engine/LICENSE` | leaf's MIT license |
| `crates/engine/VENDORED.md` | this record |
| `crates/engine/sync.sh` | fetch and compare, or update with explicit consent |

The workspace `Cargo.toml` gained the member and the engine dependencies;
`Cargo.lock` records the resolved set.

## Deviation ledger

### 1. Relocated paths

Upstream `src/markdown/<module>.rs` becomes
`crates/engine/src/markdown/<module>.rs`. The module names and the internal
module graph are unchanged. Upstream's `src/theme/` is not copied as a tree;
its markdown parts become `crates/engine/src/theme.rs` (deviation 3).

### 2. Binary crate turned into a library crate

Leaf has only a binary target. This crate has a library target: `src/lib.rs`
owns the public surface, and all application code (`src/main.rs`, `src/app`,
`src/render`, `src/runtime`, `src/diff`, `src/cli.rs`, and the rest) is not
vendored. The workspace edition is 2024 rather than upstream's 2021.

The edition change also touches `tables.rs`: edition 2024's default binding
mode makes `if let Some(CellFragment::LinkMarker(style))` a mutable binding,
replacing upstream's explicit `ref mut`. Behaviour is unchanged.

### 3. Theme machinery collapsed

`MarkdownTheme` survives with the same fields and colours. The four built-in
markdown palettes survive as `ARCTIC`, `FOREST`, `OCEAN_DARK`, and
`SOLARIZED_DARK`, with a public `ThemePreset` enum, `theme_by_preset`,
`parse_theme_preset`, labels, syntect theme names, and a `Default`
equivalent to leaf's `OceanDark`.

Removed: `AppTheme`, `UiTheme`, `DiffTheme`, `ThemeSelection`, `CustomTheme`,
`CustomThemeConfig`, custom theme file loading and validation
(`src/theme/serde.rs`, `src/theme/resolution.rs`), the `CURRENT_THEME`
`RwLock`, `set_theme_*`/`current_theme_*`/`app_theme()` global accessors, and
all unrelated app configuration.

### 4. Removed app-only helpers

`markdown::read_file_state` is gone because it returned leaf's
`crate::app::FileState`. No module refers to `crate::app` after this.

The app-only helpers with no engine caller are gone too: `hash_str` and
`hash_file_contents` in `mod.rs`, `is_footnote_number` in `footnotes.rs`,
`is_ordered_marker` in `lists.rs`, and `TocLevels`, `TocLevels::display_level`,
and `toc_levels` in `toc.rs`. Their only callers were leaf's application code
and its mixed app/engine test suite.

### 5. Theme threaded through table rendering

Upstream `tables.rs` called the global `theme::app_theme()` inside
`TableBuf::render`. `render` now takes `&MarkdownTheme`, and
`handle_table_event` passes the caller's theme to it. Nothing in the engine
reads global theme state.

### 6. Visibility changes for a library surface

Items the next unit and the ported tests need are `pub` instead of upstream's
`pub(crate)`/`pub(super)`. The public surface is exactly:

- `parse_markdown`, `parse_markdown_with_width`;
- `ParseResult` and its fields, `CodeBlockInfo` and its fields, and `CodeRow`
  and its fields;
- `TocEntry` and its fields; `toc` itself stays a crate-private module;
- `LinkId` (including its field) and `LinkSpan` and its fields;
- `line_plain_text`, `display_width`, `build_searchable_lines`,
  `iter_cluster_widths`, `truncate_display_width`, and `rendered_span_width`;
  `width` itself stays a crate-private module;
- `highlight_line`, `highlight_matches_only`;
- `resolve_syntax`;
- `BULLET_L1`, `BULLET_L2`, `BULLET_DEEP`; and
- `BLOCKQUOTE_BAR`.

The link glyph is not a module item: it is `MarkdownTheme::link_glyph`
(deviation 7), so the output depends only on the arguments.

Everything else keeps upstream's `pub(crate)`/`pub(super)` or private
visibility. Nothing was renamed.

### 7. The link glyph is configuration

Upstream keeps the link marker in a thread-local `#` that every parse reads and
a test setter changes, so a later parse on the thread depends on hidden state.
Here it is `MarkdownTheme::link_glyph`, set by the caller as the task glyphs
are (deviation 18); the four presets draw `→`. `with_link_marker`,
`set_link_marker` and `DEFAULT_LINK_MARKER` are gone. The marker's span, its
modifiers, the table cell, the highlight and the searchable text read the
theme's glyph, and `build_searchable_lines` takes the theme for it. raul sets
the glyph from its icon table (`detail.rs`).

A forced update rewrites these files to upstream's thread-local marker. That
erases this deviation and the ported link tests' setup (deviation 10); reapply
the glyph.

### 8. Removed leaf root exports

Leaf's crate root and binary re-exports are not reproduced. The engine does
not export `App`, `FileState`, the CLI, config, diff, render, runtime,
clipboard, update, or terminal items. Only the markdown engine and its theme
are preserved.

### 9. New public engine API

`src/lib.rs` re-exports `parse_markdown`, `parse_markdown_with_width`,
`ParseResult`, `CodeBlockInfo`, `TocEntry`, `LinkId`, `LinkSpan`,
`MarkdownTheme`, and `ThemePreset`. It adds `syntax_set()` and `theme_set()`
for leaf's `SyntaxSet::load_defaults_newlines()` and `ThemeSet::load_defaults()`,
and `syntect_theme(preset, themes)` for the old `current_syntect_theme`
selection, without global state. `syntax_set()` also adds the bundled TOML
grammar (deviation 26).

`syntax_set()` and `theme_set()` build the whole default set on every call.
The doc comments say to load them once at startup, keep the result, and never
call them from a draw loop.

### 10. Ported tests

The markdown-relevant upstream suites are under `src/tests`: blocks, embedded,
footnotes, HTML tag, links, list blocks, lists, rendering, tables, and
wrapping. They now import the public engine interface explicitly instead of
leaf's `crate::*`, use `MarkdownTheme::default()`/the OceanDark preset instead
of the global theme, and use a local `fence_wrap` helper for leaf's
`App::fence_wrap`.

The in-module unit tests in `markers.rs`, `links.rs`, and `mermaid.rs` are
kept, with the markers test reading the OceanDark preset directly. That
reading is the markers-test adaptation: a forced update rewrites `markers.rs`
to upstream's global-theme test and erases deviation 10.

### 11. Formatted with this workspace's rustfmt

The vendored modules are formatted by the toolchain pinned in
`rust-toolchain.toml`, so whitespace and line breaks differ from upstream even
where the logic does not. This keeps `make ci`'s `cargo fmt --check` honest.

### 12. Clippy fixes for the workspace gate

A few upstream constructs trip current clippy: nested `if`s are collapsed into
let-chains in `fences.rs`, `latex.rs`, `mermaid.rs`, `syntax.rs`, and
`table_layout.rs`, and `ThemePreset` derives `Default` with `#[default]`.
Behaviour is unchanged.

### 13. Dependency set reduced

The library depends directly only on ratatui 0.30, pulldown-cmark 0.12,
syntect 5.2 with leaf's feature set (`default-syntaxes`, `default-themes`,
`regex-onig`) and `yaml-load` for the TOML grammar (deviation 26), mmdflux 2.5 without default features, unicodeit 0.2,
unicode-width 0.2, and unicode-segmentation 1. crossterm, serde, and semver are
not direct dependencies of the engine: crossterm and semver arrive transitively
through ratatui's default `crossterm` feature, and serde through mmdflux.
leaf's other application dependencies (anyhow, reqwest, sha2, toml, fs2) are
absent.

Resolved at vendoring: mmdflux 2.6.1, syntect 5.3.0, pulldown-cmark 0.12.2,
ratatui 0.30.2, unicodeit 0.2.2, unicode-width 0.2.3, unicode-segmentation
1.13.3. Upstream's lock differs in three direct dependency versions: unicodeit
0.2.1 to 0.2.2, unicode-width 0.2.0 to 0.2.3, and unicode-segmentation 1.12.0
to 1.13.3; the other four direct dependency versions match. Upstream enables
ratatui's `unstable-rendered-line-info`; this crate drops it, and no vendored
code reads rendered line info, so it changes nothing.

### 14. The copy is not byte-identical on purpose

The checksum table above is the contract: the vendored SHA-256 must match the
bytes on disk, a file marked `identical` must hash the same as upstream, and a
file with deviation numbers must differ. `crates/engine/tests/vendored.rs`
enforces that offline; `sync.sh check` enforces both checksum columns against
the fetched upstream commit and prints the diffs.

### 15. Table fallback keeps each column's drawn minimum

When the cells cannot fit, the fallback in `fit_table_widths` gave every
column width 1, so a column holding code, math, a mark or a wide grapheme drew
wider than its border and the `│` stopped lining up with the `┼` and `┬`
crossings. `min_table_cell_floor` gives each column the narrowest width that
still draws its fragments whole: 3 cells for code, math and marks, 2 for a wide
grapheme. The fallback starts from those floors, and the normal path never
narrows a column below them. `table_layout.rs` and `tables.rs` carry this
deviation. Plain-text tables render as before. A forced update of either file
erases it; reapply it.

### 16. Headings draw a bar, a rule or an arrow

`push_heading_lines` in `blocks.rs` draws the heading levels the redesign
names. `#` leads with `▍ ` and closes with a `─` rule to the edge, in
`heading_underline`, which the document maps to the rule role. `##` leads with
`▸ `. Deeper levels are bold text alone, and every level is bold. The `═`
underline and the `─` underline under `##` are gone. A `#` heading too long for
one row wraps without its rule, its later rows indented under the bar.

### 17. The blockquote bar is `▎`

`BLOCKQUOTE_BAR` is `▎ ` (U+258E) in place of `▏ `. The text stays indented
two. The blank-line check in `blocks.rs` follows the glyph.

### 18. Task glyphs are configuration

`MarkdownTheme` gains `task_done_glyph` and `task_open_glyph`, defaulting to
`✓` and `○`. `lists.rs` draws each in place of the `TASK_CHECKED`,
`TASK_CHECKED_ALT` and `TASK_UNCHECKED` constants, which are removed with the
Windows emoji workaround: `✓` and `○` are not emoji. The caller picks the set;
raul sets them from its icon table (`detail.rs`).

### 19. Alert glyphs are configuration

`MarkdownTheme` gains `alert_glyphs`, five glyphs in GitHub's order: note `ℹ`,
tip `✦`, important `!`, warning `⚠`, caution `✗`. They replace `ALERT_ICONS`,
which drew bracketed text; `Important` no longer shares `Warning`'s glyph. The
header is the glyph and the bold label, on the alert's coloured bar.

### 20. Tables round their corners and rule only the header

The outer corners are `╭ ╮ ╰ ╯`. The header rule is one `─` line with `├ ┼ ┤`
in the table separator colour, in place of `═` with `╞ ╪ ╡`. Body rows have no
rule between them. `mark_table_lines` finds a table's border by the new
corners, and a table's body rows count as one run in `line_number_map`; raul
does not read that map. The F15 floors are unchanged.

### 21. Links show their URL, dim, when it differs from the text

Link text was already underlined in the link colour. A link now closes with its
URL in parentheses, dim: ` (url)`, owned by the link so the whole run is
clickable. `push_link_url` in `spans.rs` and `TableBuf::push_link_url` in
`tables.rs` add it unless the link's text already is the URL, so an autolink
shows once. A link inside `<mark>` or `<code>` flushes its buffered text first,
so the URL follows the label; `handle_inline_style_event` takes the HTML buffer
for that. `CellInlineStyle` gains `dim`, which `align_cell` applies. The glyph
before the text is set by deviations 7 and 24. In a heading the URL keeps its
own dim style: `push_heading_lines` in `blocks.rs` restyles every other span to
the heading, and passes a link-owned dim span through.

### 22. Fenced code and diagrams round their frames, and dim their numbers

`push_code_block_lines` and `push_special_block_lines_with_prefix` in
`blocks.rs` draw the frame's corners round, `╭ ╮ ╰ ╯` in place of `┌ ┐ └ ┘`,
as the tables do (deviation 20). Fenced code and the diagram and maths blocks
share the frame. A numbered gutter is dim: its span takes `Modifier::DIM`.
`mark_wrapped` in `mod.rs` finds a block's rows by the round corners as well
as the square ones.

### 23. Syntect's alpha 0 is the terminal's own colour

`syntect_to_color` in `syntax.rs` reads a syntect colour with alpha 0 as
`Color::Reset`, and any other alpha as RGB, as before. A theme names the
terminal's own colour that way, so a preset that paints no colour emits no RGB
for its code tokens. raul's derived code themes are the only ones that do.

### 24. A link's label takes a space after its glyph

Upstream draws the glyph against its label, `#leaf`. Here the glyph is followed
by one space, `→ leaf`, unless the label leads with one: a chip's padding is
that space, and a text that leads with a space has its own. In prose the space
is a span of its own, owned by the link, so a row that wraps after the glyph
ends with it, as any spaced word does. In a table cell the space is the label's
leading text, so the column measures it. The cell wrap no longer glues a label
to its marker: the `glue` flag is gone, and the label's first text is separated
by its own `adj` flag alone.

### 25. Table leftover is shared fairly when the minimums do not fit

When the minimum widths exceed the available width, `fit_table_widths` starts
each column at its fair share, `available / cols`, or at its minimum if that is
smaller, and never below its floor (deviation 15). The leftover then goes in
rounds, in proportion to natural width, up to each column's minimum. The
allocation this replaces gave the leftover to the widest minimums first, which
left a prose column at one or two cells. `table_layout.rs` carries this deviation; a forced update of
it erases the deviation and the test that covers it in `markdown_tables.rs`.

### 26. TOML has a grammar

Syntect's default syntaxes have no TOML, so a `toml` fence fell back to YAML.
`crates/engine/assets/toml.sublime-syntax` is a TOML grammar this crate
writes, not a copy of upstream or of a grammar pack. `syntax_set()` adds it to
the defaults, so `resolve_syntax("toml")` names `TOML`. Keys and table names
take the `entity.name.tag` and `entity.name.type` scopes, which the markdown
code themes map to the tag and type roles. Its SHA-256 is
`53c067f4e221ff9f569b9c45670322725d77bc96002f88a99736e3ebc98ce3d7`.

Loading it needs syntect's `yaml-load` feature, which pulls in `yaml-rust`
0.4.5 and `linked-hash-map` 0.5.6. Those are the only new crates; no grammar
crate (such as `two-face`) is used. `tests/markdown_toml.rs` pins the
resolution and the painted roles.
