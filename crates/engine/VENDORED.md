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
| `src/markdown/blocks.rs` | `8c4e53212383b794247b2e7502a6d6b74e01266a322ad42c787b98c0524360f7` | `crates/engine/src/markdown/blocks.rs` | `1bf58a6b5332f0de234f59ab6cb32911b6354fa0e0963f87f1be8801eddfd7bd` | 1, 6, 11 |
| `src/markdown/fences.rs` | `e5810317dd42f09780ddbd800d6ef7d81e213461181f38096cc0821d554d6ce0` | `crates/engine/src/markdown/fences.rs` | `99cc29ba50ee9b591dded59ef745d03dbd6cc82ec5c910268246e81400242487` | 11, 12 |
| `src/markdown/footnotes.rs` | `40a60dbff36117b6e4a6803bca6ba43b4b5965c523285d2d8471e89f9014ee76` | `crates/engine/src/markdown/footnotes.rs` | `0c4a012058588f9b34c6bfcef85382296a077fdb3c836e512a58636d32ca01aa` | 1, 4, 11 |
| `src/markdown/frontmatter.rs` | `de8dd7ae2337ab6a2c7837e98826280f4e0374fa809c3cfbaf6af7f0918d1e8c` | `crates/engine/src/markdown/frontmatter.rs` | `de8dd7ae2337ab6a2c7837e98826280f4e0374fa809c3cfbaf6af7f0918d1e8c` | identical |
| `src/markdown/highlight.rs` | `7e165aaf89a47d464dd2e1f460f1562db323ace19bf7ca619aad802bd8a6504d` | `crates/engine/src/markdown/highlight.rs` | `1f488429a62c25cc0aefa0e66f2472b394c8139cad4edb0f992084773356c249` | 1, 6, 11 |
| `src/markdown/latex.rs` | `33b46c28d939af3ef114e1743e6a0fd8630f9d85433c86297b6a57d52b6a8c21` | `crates/engine/src/markdown/latex.rs` | `7ee7f100464e49241553a60f4458ef3979a3e92fdea5f2e13392a04b7e86ad44` | 11, 12 |
| `src/markdown/links.rs` | `59ec5ee1efa8d08023958f77d4e7b3859d37798831578c84dbc699299e4d856b` | `crates/engine/src/markdown/links.rs` | `6e4fb7072760b67ca8778b10234e67a15cb72f544ffc4141a7004f7cc9e38b77` | 1, 6, 9, 11 |
| `src/markdown/lists.rs` | `dd16f32b5fbb766560e7c0e8395d00813496ba4ade00bb07bf330e1bff127f75` | `crates/engine/src/markdown/lists.rs` | `73f472bdba2df77961fb0d8c4cd7ceb08f91e02825beb7ec86afc9c0e8889a30` | 1, 4, 6, 11 |
| `src/markdown/markers.rs` | `2e9f315bad630b9c651d79bc1118e9ba5bcded9cebc81fd37e15472bfaf841ca` | `crates/engine/src/markdown/markers.rs` | `f07514a3ecb6d37ae767c49ec860986bda079e56e79d3b96e53b93491f8eaa79` | 1, 3, 10, 11 |
| `src/markdown/mermaid.rs` | `dc2b217974a36dcd06dc0a5c199bdc7cadef743ec7719279f304e01a677df9da` | `crates/engine/src/markdown/mermaid.rs` | `f1aeaebd7a4ce303098046cffa017fdb0ab27939f594efed2f14f5c112b10274` | 1, 11, 12 |
| `src/markdown/mod.rs` | `53177c3e6b309cdca9c9af0e9ef511e7bb7581c29248a4c489fa76b63bfbcebb` | `crates/engine/src/markdown/mod.rs` | `bce4ac2444787e8da9a4b82287d04878b95aace4ff2baac6ae3d7845a8410a31` | 1, 4, 5, 6, 7, 9, 11 |
| `src/markdown/spans.rs` | `51f40f57a3fbea5638ee3d74b157122c7616b3c480721b7888e6fedd3e2e3be6` | `crates/engine/src/markdown/spans.rs` | `e1e8240ef5ab6dc44f070917889049561e1eb444ce66a52d5d9a91165962dbfd` | 11 |
| `src/markdown/syntax.rs` | `b46409c050d4c9fc859be1acf958859958f000c77217cec453caff49b81e1403` | `crates/engine/src/markdown/syntax.rs` | `d212bd5b3a97ec44e7b69f88d080954830c3f21eb53a9939ccfb74877b5807ce` | 1, 6, 11, 12 |
| `src/markdown/table_layout.rs` | `05e2d328a05ae048f2d52c4f72eb365414e21fe621ac62e67f04f8b9f03703a7` | `crates/engine/src/markdown/table_layout.rs` | `dacd198d6e123b0f58da619d3987113d7ebf419b34c287965f9fd2e5448382a7` | 11, 12 |
| `src/markdown/tables.rs` | `b824647992b0bb665f516e95b8537594ceba3b6846f2d845d66695febfa66a10` | `crates/engine/src/markdown/tables.rs` | `7d222e06bb6f3f27fea566c77e7665f746cc6d0b8e03d44fe6d318c6be1db03f` | 2, 3, 5, 11 |
| `src/markdown/toc.rs` | `68ae47f2387f7cb66ae50929d6554684a25312eb1a5323bbefdf5b82aadca0bd` | `crates/engine/src/markdown/toc.rs` | `9640a529ae05aff4ca96cdde1ff72772bceb7ca336294a81bf79722beec873e8` | 1, 4, 6, 9, 11 |
| `src/markdown/width.rs` | `e577f4896abf16ec68a9c3a837ddd6e743e1f4da4e86c52b8fa9411b458a7ad2` | `crates/engine/src/markdown/width.rs` | `2e6464c3a7b2d6fd882708b08cda9efcfc9e10cd7c1744a5630b6aa6a163baf4` | 1, 6, 11 |
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
| `crates/engine/src/theme.rs` | collapsed markdown theme and presets |
| `crates/engine/src/tests/mod.rs` | shared test helpers |
| `crates/engine/src/tests/markdown_blocks.rs` | ported `src/tests/markdown_blocks.rs` |
| `crates/engine/src/tests/markdown_embedded.rs` | ported `src/tests/markdown_embedded.rs` |
| `crates/engine/src/tests/markdown_footnotes.rs` | ported `src/tests/markdown_footnotes.rs` |
| `crates/engine/src/tests/markdown_html_tag.rs` | ported `src/tests/markdown_html_tag.rs` |
| `crates/engine/src/tests/markdown_links.rs` | ported `src/tests/markdown_links.rs` |
| `crates/engine/src/tests/markdown_list_blocks.rs` | ported `src/tests/markdown_list_blocks.rs` |
| `crates/engine/src/tests/markdown_lists.rs` | ported `src/tests/markdown_lists.rs` |
| `crates/engine/src/tests/markdown_rendering.rs` | ported `src/tests/markdown_rendering.rs` |
| `crates/engine/src/tests/markdown_tables.rs` | ported `src/tests/markdown_tables.rs` |
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
- `DEFAULT_LINK_MARKER`;
- `BULLET_L1`, `BULLET_L2`, `BULLET_DEEP`, `TASK_CHECKED`,
  `TASK_CHECKED_ALT`, `TASK_UNCHECKED`; and
- `BLOCKQUOTE_BAR`.

`DEFAULT_LINK_MARKER` is public because it is a harmless constant. The link
marker itself is not: `set_link_marker` and `with_link_marker` stay
`pub(crate)` because the marker is mutable thread-global state that changes
every later parse on that thread. A public setter would make the output depend
on hidden state rather than only on the arguments. The ported tests live
inside the crate, so they can still set it; `set_link_marker` is compiled only
for tests (deviation 7).

Everything else keeps upstream's `pub(crate)`/`pub(super)` or private
visibility. Nothing was renamed.

### 7. Test-only link-marker setter

The crate has no `#![allow(dead_code)]`. The link-marker setter,
`markdown::set_link_marker`, is compiled only under `cfg(test)` because only
the ported link tests call it. No other dead code remains.

A forced update rewrites `mod.rs` to upstream's unconditional setter, so it
erases this deviation as well as deviation 10; reapply the `cfg(test)` gate.

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
selection, without global state.

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
`regex-onig`), mmdflux 2.5 without default features, unicodeit 0.2,
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
