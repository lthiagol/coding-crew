# AGENTS.md

Rules for agents working in this repository.

## Project

coding-lanes is a task management tool for agents — a todo list with
superpowers.

- Three interfaces:
  - **talita** — the CLI tool. Agent-facing, not designed for humans.
    **T**ask **A**ccess and **L**ane **I**nteraction **T**erminal **A**pp.
  - **raul** — the TUI for the work. Human-facing, for reviewing tasks and
    understanding the project's flow and status. **R**eview, **A**pprove,
    **U**nderstand **L**ane.
  - **hugo** — the TUI for the flow. Human-facing, for configuring and
    governing the automation that runs the work. **H**uman-guided
    **U**nified **G**overnance **&** **O**rchestration.
- The seam between them is the data: raul reads the work folder, hugo reads
  the runtime. raul draws task content; hugo draws tasks only as items of a
  queue.
- In an interactive session the human and agent groom a task; once agreed, the
  agent calls talita to write it down.
- talita writes and manages tasks; raul shows them; hugo governs the
  automation that acts on them.
- Tasks live in the managed project's own `coding-lanes/` folder, alongside its
  code. No central store — each project keeps its own data.
- `coding-lanes/` holds tasks, metadata, code review findings, and everything
  else the project uses to manage tasks.
- Never hand-edit files under `coding-lanes/`. Use talita.
- Dogfooding coding-lanes on this project is future work.

## Repo

- `crates/core` — shared model.
- `crates/engine` — vendored leaf markdown engine for raul's detail document.
- `crates/talita` — talita; package `coding-lanes-talita`, binary `talita`.
  Off-limits until instructed.
- `flow/` — the orchestration role configs: `builder.json` and `reviewer.json`.
  They name the agent each pane runs and the model it runs on.
- `crates/raul` — raul; package `coding-lanes-raul`, binary `raul`.
- `crates/hugo` — hugo; package `coding-lanes-hugo`, binary `hugo`.
- `crates/theme` — the slots, presets and palettes both TUIs paint with.
- `coding-lanes/` — data folder of a managed project (not created here).

```sh
make check
make raul
```

## Orchestration flow

- The build loop is the global skill `orchestration-flow`
  (`~/.config/opencode/skills/orchestration-flow/SKILL.md`), and its tools
  sit beside it. It reads this project's `TODO.txt`, `docs/plan.md`, and
  `flow/`.
- `.orchestration.json` holds the flow's project prefix, `cl`, and the paths
  of the role configs.

## Hugo

- hugo is the TUI for the flow. **One sidebar and one main pane, no tabs.**
- The sidebar configures the automation: the agents, the models, the harness,
  the review-round limit, and the order — the policy the flow follows.
- The main pane shows what is running: the crew, the current round, the
  budget, the findings.
- Its configuration lives under `~/.config/coding-lanes/`, beside the
  project's other configuration.
- hugo's screens are designed in `docs/hugo/`.

## Upstream skills

- Four skills are used from upstream `github.com/mattpocock/skills`, pinned at
  commit `5beb35b040e3de97e54fa9e386d7e42d15c295c2`: `codebase-design`,
  `diagnosing-bugs`, `domain-modeling`, `writing-for-agents`.
- They live whole at `~/.config/opencode/skills/`, with the MIT licence and
  their provenance beside them. This repository does not carry them.

## Communication

- Be concise. No verbosity in docs, code comments, commit messages, or replies.
- Prefer short, direct sentences. Skip filler, restating the request, and
  summarizing what you just did unless asked.
- Document only what a reader needs; omit obvious explanations.

## Clarification

- Avoid assumptions. If anything is unclear or a decision is needed, use the
  ask tool.
- When asking, always state your recommended option first.

## Git

- Commit messages use conventional commits: `feat:`, `fix:`, `docs:`,
  `chore:`, `refactor:`, `test:`.

## Rules of this file

- Do not edit `AGENTS.md` without explicit instruction.

## Task model

- Task structure, statuses, metadata, and lane definitions are designed in
  `docs/tasks/task-model.md`; roles in `docs/roles/roles.md`; raul's screens
  in `docs/tui/screens.md`; hugo's in `docs/hugo/`.
- Do not invent them ad hoc — follow the docs.
- `TODO.txt` at the root is the state of the work: what is done, what is open,
  what is next. Read it before starting; update it when a unit closes.

## Naming

- The TUI for the work is called **raul**.
- The TUI for the flow is called **hugo**.
- The CLI tool is called **talita**.
- Use only the names defined here — `coding-lanes` (project), `raul` (TUI for
  the work), `hugo` (TUI for the flow), `talita` (CLI tool). Do not invent
  aliases for the project, its crates, or its binaries without asking.
- A *lane* is the human lane or the agent lane, as the task model defines it.
  It is not a synonym for an interface.

## Scope

- Scope is the two TUIs: raul for the work, hugo for the flow. Do not write
  talita code until explicitly told.
- Work order: raul's screens and data flow first, then hugo's — the
  directives, then dropping the Autopilot tab from raul, then the shared
  theme crate, then hugo itself.
- talita is written only when explicitly instructed — never automatically
  because raul looks finished or mature enough.
- `crates/core` stays editable; it is shared with raul and hugo.
