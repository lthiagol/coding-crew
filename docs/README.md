# Design docs

**coding-crew** — an agentic todo list with superpowers. Three interfaces:
**talita**, the CLI tool that writes and manages tasks (**T**ask
**A**dministration, **L**ifecycle & **I**nteraction **T**erminal
**A**pplication); **raul**, the TUI for the work that shows them (**R**eview,
**A**ssess, **U**nderstand & **L**earn); and **hugo**, the TUI for the flow that
governs the automation (**H**uman-guided **U**nified **G**overnance &
**O**rchestration).

| doc | what |
|---|---|
| [tasks/task-model.md](tasks/task-model.md) | the unit of work: file format, ids, statuses, transitions, gates, review, evidence |
| [tasks/smaller-tasks.md](tasks/smaller-tasks.md) | ideas, bugs and small features — the work that is not a task yet |
| [roles/roles.md](roles/roles.md) | actors, roles, ownership, session boundaries |
| [tui/screens.md](tui/screens.md) | raul: the window, the tabs, the detail pane, the derived views |
| [hugo/screens.md](hugo/screens.md) | hugo: the window, the two panes, the render mode and the empty states |
| [hugo/design-record.md](hugo/design-record.md) | hugo: the interface for the flow — the sidebar, the main pane, and the sketches that were compared |
| [hugo/tui-proposals/](hugo/tui-proposals/) | the thirty candidate screens for hugo, one directory per model family, with the index that compares them |
| [tui/theme.md](tui/theme.md) | the colour slots, the phase-to-slot mapping, the presets |
| [tui/palettes.md](tui/palettes.md) | where the themes come from: the families, the mapping, the rules for a new one |
| [vocabulary.md](vocabulary.md) | the names we use, and the ones we rejected |
| [plan.md](plan.md) | the work ahead: small units with their done-when lines, the grooming, the open questions |
| [decisions/](decisions/) | decision records: what we chose, and why |
