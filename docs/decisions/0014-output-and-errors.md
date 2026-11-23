# 0014. talita's output and errors — machine first

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

talita's callers are agents (AGENTS.md: "not designed for humans"). Today
`create` and `show` print pretty JSON, `list` prints a text table, and every
error is anyhow's `Error: …` on stderr with exit 1; clap usage errors exit 2
(`inv-existing.md` § 1). A gate refusal, a missing task and a disk error are
indistinguishable to the caller.

An agent needs to know three things after a call: did it apply, what changed,
and if not, why and what to do next. It parses stdout; it often loses stderr
or sees it mixed in.

## Decision

### JSON is the default and only format

One JSON object on stdout per call, for success and failure alike. No
`--json` flag. A human-readable form is later (`--text`), for humans
running talita through raul's route.

```json
{ "ok": true, "command": "task approve",
  "data": { "id": "TSK04", "status": "approved", "revision": 12 },
  "wrote": ["work/TSK04-login-form.json", "activity.json", "runtime.json"],
  "journal": ["approved"] }
```

```json
{ "ok": false, "command": "task finish",
  "error": { "code": "gate.refused",
             "message": "finish is refused by 2 gates",
             "gates": [ { "gate": "evidence.attached", "detail": "ACCEPT-02 has no evidence" },
                        { "gate": "self_round.filed", "detail": "no self round" } ],
             "remedy": "talita evidence run TSK04 ACCEPT-02; talita round file TSK04 --kind self ..." } }
```

| Field | Rule |
|---|---|
| `ok` | always present |
| `command` | noun and verb as called |
| `data` | the verb's result: what was created or changed, with new ids. Read verbs: the object read |
| `wrote` | paths written, relative to `coding-crew/`, in write order |
| `journal` | the kinds appended |
| `warnings` | optional, list of `{code, message}`; never a refusal |
| `error.code` | stable, dotted, from the table below |
| `error.gates` | for `gate.refused`: every failing gate, by its task-model name |
| `error.remedy` | when one exists: the talita command that unblocks |

stderr carries nothing on a normal run; a panic is the only thing that lands
there.

### Error codes and exit codes

| Exit | Class | Codes |
|---:|---|---|
| 0 | applied | — |
| 1 | internal | `internal` (I/O, bug) |
| 2 | usage or input | `usage.invalid`, `input.invalid` (bad JSON input, enum, empty required text) |
| 3 | not found | `project.not_found`, `task.not_found`, `part.not_found`, `run.absent`, `crew.unknown` |
| 4 | refused by a rule | `status.wrong`, `gate.refused`, `section.frozen`, `task.cancelled`, `review.verdict_mismatch`, `review.budget_spent`, `finding.closed`, `question.resolved`, `claim.held`, `claim.not_claimable`, `criterion.covered`, `criterion.not_failed`, `criterion.deferred`, `depends.cycle`, `overlay.absent`, `item.not_open`, `item.is_idea`, `evidence.timeout`, `project.exists`, `run.active`, `crew.exists` |
| 5 | identity | `identity.missing`, `role.wrong`, `actor.not_agent`, `section.not_owned`, `claim.not_holder` |
| 6 | conflict | `conflict.lock`, `conflict.revision` — safe to retry after re-reading |
| 7 | damaged store | `store.unreadable` — a file the verb needs does not parse |

Codes are added, never renamed. Gate names are not codes: they ride in
`error.gates`, so a new gate adds no code.

### Every failing gate, not the first

Core's `transition()` returns the first failing gate (LC:151). talita needs
all of them, so an agent fixes everything in one pass. Core gains a
`check(task, event, actor, ctx) -> Vec<GateFailure>`; `transition()` calls
it and refuses if non-empty.

### Alternatives considered

| Option | Lost because |
|---|---|
| Text by default, `--json` opt-in | the caller is an agent; a forgotten flag yields text the agent then parses by guess |
| Errors on stderr, data on stdout | agents' tools often merge or drop stderr; one stream with `ok` is unambiguous |
| One exit code for every failure | the caller cannot tell "retry" (6) from "fix the work" (4) from "wrong identity" (5) without parsing |
| Error codes per gate (`gate.criteria_proven`) | grows the code table with every gate; the gate names are already stable in TM § 3.3 |
| JSON Lines streaming | no verb streams; `evidence run` returns the output tail in `data` |

### Settled 2026-11-22

- Settled 2026-11-22: Every failing gate is reported; core gains the collect-all check.
- Settled 2026-11-22: JSON always; `--text` for humans is later.

## Consequences

### Positive

- One parse path for every call. Exit codes let a shell script branch
  without parsing.
- A refusal carries its remedy; the skill docs can point at it instead of
  repeating rules.

### Negative

- Humans reading raw talita output see JSON. Acceptable: humans read raul.
- Core's lifecycle needs the collect-all gate check.

### Follow-up

- The code table lives in `docs/talita/commands.md` and in one Rust enum; a
  test pins them together, as `roles.md` is pinned to core's defaults.
