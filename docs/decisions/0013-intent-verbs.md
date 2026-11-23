# 0013. Intent verbs only — no field editing

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

ADR 0007 makes talita the one writer and says talita "must refuse or repair
what it cannot vouch for" (`0007-one-writer.md:80-82`). Many rules live only
in prose, with nothing in core to enforce them (inventory, `inv-raul.md` § 5):

- `intent`, `scope` and `acceptance_criteria` are frozen at approval
  (TM § 9); "an out-of-band edit … is not detected by the tool".
- `meta` is mutable before approval only (TM:163).
- A finding's outcome is written once; the second writer is refused (TM § 4.2).
- Evidence, notes, questions and rounds are append-only.
- A role may not write another role's section (roles.md, "Ownership by
  section").

A generic writer — `talita task set <id> <path> <value>`, or "apply this
JSON" — would make every one of these rules a check on a path pattern, and
any rule missed is a hole. Agents take the shortest route: if an escape hatch
exists, a stuck agent uses it.

## Decision

**Every write is a verb named for a domain act, and the verb carries its
rules.** There is no field setter, no `edit`, no raw-JSON import of a task.

| Rule | Enforced by |
|---|---|
| Lifecycle gates | core `transition()`; talita adds no gate of its own outside ADRs 0017, 0020 |
| Section ownership | the noun and the role: `criterion add` needs `creator`, `evidence run` needs `runner` |
| Phase | each verb lists the statuses it accepts (`criterion add`: draft, groomed) |
| Frozen sections | from `approved` on, every content verb on intent, scope and criteria returns `section.frozen`, with the remedy `talita task create --parent <TSK>` |
| Append-only | there is no verb to edit or delete evidence, notes, questions, rounds or comments |
| Written once | `finding resolve`/`dismiss` on a closed finding returns `finding.closed` |
| Derived stays derived | no verb writes a criterion's `status`, a lane, a review state or `claimable` |

### What stays writable after approval

The criteria are frozen as a **brief**: description and verification. What
the runner learns about them is not the brief and stays writable:
`deferral` (`criterion defer`), `failure_reason` (`criterion explain`),
`evidence[]`, and the stored status mirror talita writes from the evidence.
Steps stay open (roles.md); a step added or changed after approval must carry
a `deviation`.

### Repair

A file that fails to parse, or that breaks a rule talita can see, is refused
as `store.unreadable` for any write. `project check` reports it. There is no
repair verb yet; the human fixes the file with a reviewed hand-edit, which is
the one sanctioned exception, recorded in the commit (later: `project repair`).

### Alternatives considered

| Option | Lost because |
|---|---|
| A generic `set <path> <value>` with a rule table | every rule becomes a path pattern; a missed pattern is a silent hole; agents reach for it first |
| Content fingerprint on frozen sections | rejected already in TM § 9.1; refusing the write is cheaper than detecting it |
| Whole-task JSON import, validated | validation of a whole document cannot tell an intended change from a drive-by edit; it is the escape hatch with a schema |
| Few coarse verbs (`task groom` with every content flag) | merges intents; a partial failure leaves the agent unsure what was written |

### Settled 2026-11-22

- Settled 2026-11-22: Repair: a broken file is fixed by a reviewed hand-edit; `project check` reports it. No `project repair` verb.
- Settled 2026-11-22: Criterion after approval: no typo fix; a child task carries any change, as TM § 9 says.

## Consequences

### Positive

- A rule lives once, in the verb. A refusal names it and the way forward.
- The record cannot drift into a shape no verb produces, so raul's
  derivations stay valid.
- A frozen section is enforced, not only reviewed — TM § 9's stated cost
  ("not detected by the tool") goes away for writes through talita.

### Negative

- More verbs. Each needs its own help, tests and doc row.
- A missing verb blocks the agent. That is the intent: the agent says what
  is missing (as the skill docs already instruct) rather than hand-editing.
- A hand-edit outside talita is still possible on disk. talita cannot stop
  it; `project check` and review catch it.

### Follow-up

- TM § 9 gains a line: talita refuses content writes to frozen sections.
- core gains the ownership table as code, so talita and a future writer
  share it.
