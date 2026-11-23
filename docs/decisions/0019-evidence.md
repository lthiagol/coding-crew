# 0019. Evidence — talita runs the verification

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

ADR 0006 makes evidence a command, an exit code and a summary, with the
criterion's status derived from the latest entry. It leaves open "whether
[talita] runs [the verification command] or accepts the runner's report of a
run" (`0006-evidence.md`, Follow-up).

The failure ADR 0006 guards against is manufactured confidence. A reported
exit code is a claim: an agent can write `exit_code: 0` for a command it
never ran, or ran in another directory, or ran a different command. The
criterion's `verification` field is set by the creator and frozen at
approval, so the command to run is already on the record.

Two more loose ends:

- ADR 0006 says a manual override is evidence "marked as a manual override";
  `Evidence` has no such field (`crates/core/src/task.rs:376-389`).
- The criterion's stored `status` "mirrors" the derived one (TM:158), but
  nothing writes the mirror (`inv-raul.md` gap 6).

## Decision

### `evidence run`: talita executes the frozen verification

```
talita evidence run TSK04 ACCEPT-01 [--timeout 10m] [--summary "..."]
```

talita runs the criterion's `verification` with `sh -c`, from the project
root, with the caller's environment, under a timeout (default 10 min). It
records `{command, exit_code, summary, at, by, session}`:

- `command` is the verification text exactly as stored.
- `exit_code` is what the process returned. On timeout, the entry is still
  written with no exit code: the criterion reads `unproven`.
- `summary` is `--summary` if given, else the last non-empty output line,
  cut to 200 characters. Empty output gives an empty summary: exit 0 with
  nothing said is a failure (TM § 5).

The output returns the exit code, the derived status and the last 40 lines,
so the agent can act on a failure without re-running.

A verification written as a procedure rather than a command fails to run
(exit 127 or similar), and the criterion reads `failed`. The way forward is
an override.

### `evidence override`: the explicit bypass

```
talita evidence override TSK04 ACCEPT-03 --reason "checked by reading the rendered page"
```

Appends `{command: "", exit_code: 0, summary: <reason>, manual: true, at, by,
session}`. The criterion reads `passed`; raul marks the entry manual. This is
ADR 0006's "explicit debt", now with its field: `Evidence.manual`.

### No reported runs

There is no verb that records an exit code the caller supplies. A run that
cannot happen under talita (needs a device, a network the shell lacks) is an
override with the reason saying so.

### The stored status mirror

On every evidence or deferral write, talita sets the criterion's stored
`status` to `effective_status()`. Readers still derive; the mirror keeps
`Finish`'s two reads (LC:239, LC:253) in agreement.

### Alternatives considered

| Option | Lost because |
|---|---|
| Accept the runner's report (`--exit-code 0 --summary …`) | a claim, not proof; the gate becomes theatre (ADR 0008's word) |
| Both: run by default, report allowed and marked `reported` | two kinds of passed; a reviewer must weigh each; the override already covers the honest case |
| Run any command the caller passes | the frozen `verification` is what was approved; a different command proves something else |
| No timeout | a hung test holds the project lock ([ADR 0016](0016-concurrency.md)) |

### Settled 2026-11-22

- Settled 2026-11-22: talita runs the frozen `verification`; `evidence override` is the marked bypass; no reported runs (the human's ruling).
- Settled 2026-11-22: Default timeout 10 min, `--timeout` per call.
- Settled 2026-11-22: Working directory: the project root.

## Consequences

### Positive

- An exit code on the record was produced by talita, from the approved
  command. Manufactured confidence needs an override, and the override shows.
- The mirror and the derived status cannot disagree.

### Negative

- talita runs arbitrary shell from the task file. The text was written by a
  creator, frozen at approval, and seen by a human; talita runs it with the
  caller's rights, no more.
- A long run blocks the caller. The project lock is **not** held while the
  command runs — only around the append.
- A procedure-style verification always needs an override. Such criteria
  should be rare (ADR 0006, Negative).

### Follow-up

- `Evidence` gains `manual: bool` (`#[serde(default)]`); raul shows it.
- `docs/skills/task-authoring.md`: write `verification` as a command
  whenever one exists.
