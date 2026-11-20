# hugo — screens

hugo is the TUI for the flow (**H**uman-guided **U**nified **G**overnance &
**O**rchestration). It reads the runtime and shows it. It does not drive the
loop, and it writes nothing but its own configuration home.

The interface was chosen in [design-record.md](design-record.md): one sidebar
for the configuration and one main pane for what is running, with no tabs.

## The window

```text
 hugo
┌configuration─────────────────┐┌flow──────────────────────────────────────────────┐
│no configuration yet          ││nothing is running                                │
│                              ││                                                  │
│                              ││                                                  │
│                              ││                                                  │
└──────────────────────────────┘└──────────────────────────────────────────────────┘
 q  quit
```

| part | behaviour |
|---|---|
| title line | the name of the tool, `hugo`; a window too narrow for it shows no title rather than a cut of the name |
| sidebar | the configuration, on the left: the agents, the models, the harness, the review-round limit and the order. The shell has no configuration content yet, so the pane says `no configuration yet` |
| main pane | what is running: the crew, the round and the queue. The shell has nothing running yet, so the pane says `nothing is running` |
| footer | the keys that work — never a key that does nothing. In the shell that is `q quit` |
| tab bar | none: the flow has one screen, not a set of tabs |

The two empty states are painted on `text.dim`, as raul's empty states are.

## The layout

The sidebar is 32 columns wide, its border included. At 80 columns and wider
the window holds both panes side by side: the sidebar on the left and the
main pane, at least 48 columns, taking the rest.

Below 80 columns the sidebar is dropped whole — it is never squeezed, and the
main pane is never squeezed to keep it. The main pane then takes the whole
window, and the drop is **announced**: the line above the panes reads

```text
 sidebar not shown: configuration (it needs 80 columns)
```

The line is secondary text, and it names what is not shown. Where the full
line does not fit, the short form `sidebar not shown: configuration` still
names it; where even that does not fit, the line is cut and the cut is
marked. The drop is never silent.

A window too short for the whole frame gives up its title and its footer
first, then the panes' boxes: the fact that the sidebar is not shown is the
last line to give way, and a pane without room for a box keeps its words.
The same degradation is raul's: the words matter more than the chrome.

## The configuration home

The configuration lives in `~/.config/coding-crew/`. When the folder is not
there, hugo creates it — the tool creates its home rather than failing. A
machine with no home folder has nowhere for hugo to write, and the shell
still runs.

The shell does not read the configuration yet, and it changes nothing from
the screen: whether hugo drives the loop or only shows it is still open.

## The render mode

```sh
hugo --render --plain [--width N] [--height N]
```

prints the screen as plain UTF-8 and exits 0, the same contract raul has, so
the screens can be pinned byte for byte. The output is deterministic: the
same bytes on every run. `--width` and `--height` are bounded to 1 up to
1000 and default to 100 by 40. `--theme` chooses one of the shared presets —
`dark`, `nord`, `catppuccin`, `dracula`, `terminal` — and changes the paint,
never the words.

The pinned screens are `crates/hugo/tests/snapshots/screen-40.txt`,
`screen-60.txt` and `screen-100.txt`.
