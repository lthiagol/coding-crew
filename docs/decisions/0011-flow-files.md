# 0011. The flow's files live with the project

- **Status:** ✅ accepted
- **Date:** 2026-11-21
- **Deciders:** coding-crew

## Context

Three sets of files carry the flow, and they were in three places:

- `flow/builder.json` and `flow/reviewer.json` — the project's role configs: the
  agent each role runs, its mode, its description, its model.
- `.orchestration.json` — the project prefix, the slug the global skill names
  every tab and agent with.
- `~/.config/coding-crew/config.toml` — hugo's configuration: the agents, the
  models, the harness, the review-round limit, the order.

The first two sat at the repository root. The third is on the tool's side: it
exists on every machine where hugo runs, and it holds policy a human edits.

The flow skill stopped reading the role configs — it generates a role config
per session, outside the repository — so nothing at runtime read `flow/`. Only
the gate did: `crates/core/tests/config.rs` pins hugo's defaults against them.
The project's own data has a home already: `coding-crew/` inside the managed
project, beside its code ([ADR 0002](0002-work-folder.md)), discovered by
walking up to the nearest `coding-crew/` folder
([ADR 0010](0010-project-boundary.md)).

Three homes were possible for the role configs: the repository root, the
project's `coding-crew/` folder, or the tool's home beside the configuration.

## Decision

**The project's flow files live with the project, the tool's configuration lives
on the tool's side, and one file stays at the repository root.**

- The role configs are per-project flow data. When coding-crew manages a
  project, they belong at `<project>/coding-crew/flow/`. Until then they stay
  at the repository root, where the gate's pin reads them.
- `.orchestration.json` stays at the repository root. The global skill reads the
  project prefix from there, and the skill is not ours to change: an exception
  the tool imposes, not a preference.
- `~/.config/coding-crew/` holds hugo's configuration alone. It is the tool's
  home — per machine, policy a human edits — and the project's own data never
  goes there.
- The flow's findings live inside the task, in `review.findings[]`, not in
  `/tmp`. The skill keeps them in `/tmp` for a project that does not manage
  itself; a managed project's findings belong to the task they are about.
- The orchestrator's memory is the task folder. `TODO.md` and `docs/plan.md`
  hold the state of the work today because nothing writes the task files yet;
  once talita writes them, the tasks are the memory.

The rejected alternative was to move the role configs into
`~/.config/coding-crew/` beside the configuration: one place for the flow's
files. It would have put a per-project policy in a per-machine folder shared
by every project on the machine, and it would have broken the gate — the pin
that keeps hugo's defaults and the project's declaration together would have
to read the home folder, so the gate's result would depend on the machine it
ran on.

## Consequences

### Positive

- The flow's files sit with the code they are about; the tool's home holds
  only the tool's configuration.
- A repository can check itself: the gate reads the role configs from the tree.
- A round's findings survive a restart, because they are in the task.

### Negative

- Two files live outside `coding-crew/`: `.orchestration.json` at the root, and
  `TODO.md` until the writers land.
- The role configs move when dogfooding starts — a path change in the gate's
  pin, in the vocabulary and in AGENTS.md.

### Follow-up

- The move happens with the dogfooding unit, when the writers exist.
- `mock-data/generate.py` holds a third copy of the model pair, unpinned
  against the role configs.
