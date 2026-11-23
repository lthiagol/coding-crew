# 0015. Identity — how actor, role and session reach a call

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

Core's `Actor` is `{id, kind, role, session}` (`crates/core/src/actor.rs`).
The gates need it: role-gated events refuse a human (`NotAnAgent`, LC:157-164)
and the wrong role; `review.independent` and `remediation.session_distinct`
compare sessions and refuse when one is missing (TM § 3.3). ADR 0008 says
"recording a session id must be part of acting, not an optional field" and
leaves open "how a session identity is minted and carried across talita
invocations" (`0008-session-boundaries.md`, Follow-up).

Three facts shape the answer:

- Agents call talita from shells that often do not keep environment
  variables between tool calls. A subagent's shell is fresh each call.
- In herdr, the orchestrator starts each agent in a pane, and can set that
  pane's environment — hugo already does it for the orchestrator's tab with
  `HUGO_SESSION` (`crates/hugo/src/config.rs:141`).
- `HUGO_SESSION` is **not** an actor session. It is the crew configuration
  (agents, round limit, order, raul) as one-line JSON. The name invites the
  confusion.

And one gap: some acts are a human's decision — approval (ADR 0005), and
`resume_work`, "a human instructed it" (TM § 3.2) — yet core refuses a human
actor on every role-gated event.

## Decision

### Where identity comes from

| Part | Flag | Env | Required |
|---|---|---|---|
| actor id | `--as` | `TALITA_ACTOR` | every write |
| role | `--role` | `TALITA_ROLE` | every write: `creator`, `runner` (or `builder`), `reviewer`, `orchestrator`, or `human` |
| session | `--session` | `TALITA_SESSION` | every agent write |

Flags beat env. A write missing a part is `identity.missing` (exit 5), never
a default. `talita session show` prints what a call would resolve to and
from where. Reads need no identity.

`--role human` means `kind: human` with no role — humans carry no role
(roles.md). A human's session is optional and not recorded.

### Minting a session

`talita session new --as <actor> --role <role>` returns a fresh id,
`ses-<role>-<actor>-<12 random base32>`, and the `env` line to export. It
writes nothing: talita keeps no session registry.

Who mints, by context:

| Context | Who mints | How it travels |
|---|---|---|
| herdr flow | the orchestrator, per job | set as the agent pane's environment at launch; the brief repeats it |
| subagent flow (no persistent env) | the orchestrator, per job | in the brief; the agent passes `--session` on every call |
| interactive grooming | the creator agent, once per conversation | the agent passes it, or exports it in a shell that persists |

A fresh builder job is a fresh session. That is what makes
`remediation.session_distinct` hold by construction in the flow.

### Where the sessions land

| Field | Written by |
|---|---|
| `planned_by` | `task approve`: the approving creator's session (ADR 0008: "wrote and approved"). Nothing sets it today, so `review.independent` refuses every review; promoted items get it here too |
| `claim.session` | `claim take` ([ADR 0020](0020-claims-and-leases.md)) |
| `Round.session` | `round file` ([ADR 0017](0017-review-record.md)) |
| evidence `session` | `evidence run`, `evidence override` |

### Trust

talita cannot verify that a caller is who it says. The session gates stop
**accidental** crossings — an agent continuing its own work — not a hostile
agent that invents ids. That matches ADR 0008's purpose: the boundary is a
property of the record, and an agent following its brief cannot cross it.

### A human's decision

A human decides; an agent writes it down. The creator agent in the
interactive session performs the event as `creator`, and names the human:

```
talita task resume TSK04 --instructed-by luiz --reason "scope cut to ACCEPT-01"
talita task reopen TSK04 --instructed-by luiz --reason "regression in login"
talita task approve TSK04 --instructed-by luiz
```

`--instructed-by` is required on `resume` and `reopen` (gate
`human.instructed`, TM § 3.2, today unchecked in LC:318-321) and optional on
`approve` and `question answer`. It is recorded in the journal event's note.
Core keeps refusing a human actor on role-gated events: the human's route is
the creator, as ADR 0007 already implies.

A human may also call talita directly (`--role human`) for what core lets
any actor do: overlays, notes, questions, comments. That is raul's future
"route to talita" (ADR 0007:52-54).

### `HUGO_SESSION`

talita never reads `HUGO_SESSION`. hugo renames it `HUGO_CREW` so
"session" means one thing in the project: an actor's working session.

### Alternatives considered

| Option | Lost because |
|---|---|
| Flags only | every call repeats three flags; easy in a brief, noisy in herdr where env works |
| Env only | subagent shells lose env between calls; a silent fallback would then be a missing session |
| A session registry (`sessions.json`) with `session start/end` | a write per session for no gate that needs it; talita still could not prove who calls |
| Derive the session from the process (PID, tty, herdr pane id) | not stable across tool calls, not portable outside herdr |
| Let a human actor perform `resume_work`/`approve` directly | changes core's `NotAnAgent` rule and ADR 0005's "agent roles only"; the creator relay already exists |
| Default role from the noun | an agent with a wrong env would act in a role it did not choose |

### Settled 2026-11-22

- Settled 2026-11-22: Identity: env `TALITA_ACTOR`/`TALITA_ROLE`/`TALITA_SESSION`, flags override, refused when missing.
- Settled 2026-11-22: A human's decision: the creator agent acts with `--instructed-by <human>`, required on `resume` and `reopen` (the human's ruling).
- Settled 2026-11-22: No session registry; `session new` only mints.
- Settled 2026-11-22: `HUGO_SESSION` is renamed `HUGO_CREW`.
- Settled 2026-11-22: `planned_by` is set at `task approve`, to the approving session.
- Settled 2026-11-22: The human's name lives in the journal note only; a task field waits until raul needs it.

## Consequences

### Positive

- Every write records who, in what role, in which session. The session
  gates have data.
- The human's decision is on the record by name, not implied.
- One meaning for "session".

### Negative

- An agent that drops `--session` is refused, not helped. Intended.
- No proof of identity. Accepted: talita guards against mistakes, not abuse.

### Follow-up

- `docs/roles/roles.md` gains the env table.
- The flow skills mint a session per job and put it in the brief.
- hugo renames `HUGO_SESSION` to `HUGO_CREW` and documents it in
  `docs/hugo/` (owed already, `TODO.md:153`).
