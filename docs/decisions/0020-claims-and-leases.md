# 0020. Claims and leases

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

`start` requires a claim held by the acting runner with a live lease
(`task.claimed`, LC:215-223). Nothing creates a claim: there is no `Event`
for claiming (LC:84-113), and the skill docs list no claim verb
(`inv-raul.md` gap 1). The claim is `{by, session, at, lease_expires_at}`;
an empty lease never expires, a malformed one counts as expired
(`task.rs:239-250`).

The inventories flag a contradiction:

- `mock-data/generate.py:1472-1486` treats an approved task as claimable when
  it has no **live** claim — an expired lease frees it.
- TM:864 says the `approved → executing` gate "refuses an expired lease".

`claim.session` must outlive `finish`: `remediation.session_distinct` reads it
(LC:413-428). After `reopen` or `resume` the task is `executing` again with
no rule for who holds it (`inv-raul.md` gap 7).

## Decision

### The two rules are one rule

There is no contradiction once claiming is a verb:

- **Claimable** = `approved`, no overlay, every dependency done, and no live
  claim. An expired claim does not hold the task.
- **`start`** refuses an expired lease **of the claim it reads**. The holder
  of an expired claim renews before starting; anyone else takes it over.

### Claim is an operation, not a lifecycle event

It changes no status, so it is not an `Event`. Core gains a claim operation
with its own checks; talita's `claim` noun calls it.

| Verb | Who | Allowed when | Writes |
|---|---|---|---|
| `claim take <TSK> [--lease 2h]` | runner | `approved` and claimable; or `executing` with no claim or an expired lease, unless blocked (takeover) | `claim {by, session, at, lease_expires_at}` |
| `claim renew <TSK> [--lease 2h]` | the holder: same actor and session | claim present | `lease_expires_at` |
| `claim release <TSK> --reason <t>` | the holder | `approved` or `executing` | removes the claim |

- Default lease 2 h; no lease is "never expires" and is not offered.
- Takeover replaces `by` and `session`. The new session becomes the
  execution session: it may not remediate later.
- `task cancel` releases a live claim.
- A claim stays through `code-review`, `remediating` and `done`;
  `remediation.session_distinct` needs it. It is not renewed then: the lease
  only guards the time before `finish`.

### The holder hands the work over

`finish` gains `task.claimed`: only the claim holder (actor and session) moves
the task to review. Today any runner can finish another's work. The lease is
not checked at `finish` — an expired lease with no takeover still belongs to
its holder.

### After `reopen` and `resume`

The task returns to `executing` with no claim: `resume` and `reopen` remove
it, whatever its lease ([amended 2026-11-24](#amendment-2026-11-24-resume-and-reopen-release-the-claim)).
A runner takes the task over (`executing` with no claim). The new session is
the execution session from then on.

### Alternatives considered

| Option | Lost because |
|---|---|
| `start` claims implicitly | a claim on an `approved` task, before work starts, is a real state (fixture TSK09) and what makes a task unclaimable for others |
| `task claim` (task noun) | the noun names what is written ([ADR 0012](0012-noun-verb-grammar.md)); the claim is its own part with renew and release |
| An expired lease revokes the claim automatically | needs a writer on a timer; an expired claim with no taker harms no one |
| Claims never expire | a dead runner would hold a task until a human edits the file |
| Takeover allowed in `approved` only | a crashed runner's `executing` task would need a human |

### Settled 2026-11-22

- Settled 2026-11-22: Default lease 2 h, renewable.
- Settled 2026-11-22: `finish` requires `task.claimed`.
- Settled 2026-11-22: After `reopen`/`resume` the old claim stays; takeover covers a dead holder. *Superseded 2026-11-24: `resume` and `reopen` now release the claim (see the amendment below).*
- Settled 2026-11-22: The orchestrator does not release a dead builder's claim; an expired lease frees it.

### Amendment 2026-11-24: `resume` and `reopen` release the claim

Decided by the human, 2026-11-24 (O1 Q8). The rule above left a resumed task
`executing` with its old claim: a live claim blocked the takeover until it
lapsed, so a resume within two hours of the take could not be taken.

- `task resume` and `task reopen` remove the task's claim, live or expired,
  and release the crew rows holding the task. The removal is journaled as
  `released` after the move's own event ([ADR 0018](0018-runtime-and-journal-writers.md)).
- `claim take` in `executing` takes over a task with **no claim** as well as
  one with an expired lease. A blocked task is still not taken over.
- The queue's `claimable` is unchanged: it stays `approved`-only. An
  `executing` task is taken through the orchestrator's resume path, not the
  queue.

## Consequences

### Positive

- Every claim has a writer, a lease and a way out.
- The generator's rule and TM:864 read as one rule.
- `finish` cannot be done by a stranger.

### Negative

- A new gate on `finish` and a new core operation.
- Takeover in `executing` moves the execution session; the first runner may
  then remediate its own work. Accepted: it did not finish it.

### Follow-up

- TM § 3.3 gains `task.claimed` on `finish`; TM:864 gains a line: "an
  expired claim is claimable by others; its holder renews before `start`".
- The journal gains `released` ([ADR 0018](0018-runtime-and-journal-writers.md)).
