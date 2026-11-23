# Dogfood — what talita hurt

The two sandbox projects under `examples/` were written by talita alone, with
the binary built from this branch. Every entry below is something a caller
would not expect, or a rule that cost a call to find. A verb that writes was
not re-run for this list; its entry quotes the run that hit it.

## The review budget

### The fifth review round cannot be sent back
- command: `talita --project examples/pantry --as nadia --role reviewer --session ses-nadia-pantry task request-changes TSK05`
- happened: Not re-run (it writes). Refused, exit 4, `review.budget_left`: "the review round budget is exhausted"; remedy `talita task show TSK05`. The round's high finding stays open and the reviewer escalates it unfixed. Ledgerline's TSK03 ends the same way: five rounds filed, FIND-05 open, `needs-human`.
- expected: the fifth round to go back for a fix like the first four, or the refusal to name `task escalate` as the only move left. A caller following the cycle reads the budget's last round as one more round.
- ruled: the human, 2026-11-23 — when the budget is spent, the last round escalates with its findings open; that is the intended behaviour, and no code change follows.
- severity: low

## The human's name

### The approving human's name is accepted and dropped
- command: `talita --project examples/ledgerline --as thea --role creator --session ses-thea-ledgerline task approve TSK01 --instructed-by hugo`
- happened: Not re-run (it writes). Accepted, exit 0, `data.instructed_by: "hugo"`; the journal event is a bare `approved` with no note, and the task file records only `planned_by: ses-thea-ledgerline`. Every approval in both projects used `--instructed-by` — Pantry's ten, Ledgerline's nine — and all 19 `approved` events have an empty note.
- expected: ADR 0015 says the human's name is recorded in the journal event's note, so the approval should read "instructed by hugo" after the call. The command's own output says it was carried.
- fixed: unit T11a — `task approve --instructed-by <human>` now appends an `approved` event with the note `instructed by <human>`. The approvals written before it keep their empty notes.
- severity: high

## Remedies

### `task finish` names the wrong next call
- command: `talita --project examples/pantry --as ravi --role runner --session ses-ravi-pantry task finish TSK03`
- happened: Not re-run (it writes). Refused, exit 4, `criteria.proven`: "ACCEPT-01 failed without a reason"; remedy `talita evidence run TSK03 <ACCEPT>`. The evidence had just been run and failed; running it again records the same failure.
- expected: `talita criterion explain TSK03 ACCEPT-01 --failure-reason …`, the only call that clears the gate.
- severity: medium

### `task escalate` names the call it just refused
- command: `talita --project examples/pantry --as nadia --role reviewer --session ses-nadia-pantry task escalate TSK05`
- happened: Not re-run (it writes). After jo's fourth re-review, refused exit 4: `review.budget_exhausted` ("the review round budget is not exhausted") and `review.round_current` ("the latest external round was not filed since the task entered code-review"); remedy `talita task escalate TSK05`, the same call, refused again.
- expected: the remedy to name the round that must be filed first (`talita round file TSK05 --input <round>`), or `task show`; not the verb whose own gates just refused.
- severity: medium

## Identity

### A human is told to act as an agent
- command: `talita --project examples/pantry --as hannah --role human task approve TSK09`
- happened: Not re-run (it writes). `actor.not_agent`, exit 5, "task approve is an agent's act"; remedy "act as an agent: --role creator, runner or reviewer".
- expected: the route ADR 0015 gives a human: ask the creator to run `task approve TSK09 --instructed-by hannah`. The remedy tells the human to be the thing talita just refused.
- severity: low

### `project init` asks for identity the table calls "any"
- command: `talita project init --path examples/pantry`
- happened: Not re-run (it writes). `identity.missing`, exit 5: "a write needs its actor (--as or TALITA_ACTOR), role (--role or TALITA_ROLE), session (--session or TALITA_SESSION)".
- expected: the folder. commands.md marks the verb "Who: any"; the conventions say every write carries identity, and the table does not repeat it.
- severity: low

## Status and overlays

### A cancelled task is not a cancelled status
- command: `talita --project examples/ledgerline task list --status cancelled`
- happened: `input.invalid`, exit 2, "status `cancelled` is not one of draft, groomed, approved, executing, code-review, remediating, needs-human, done". TSK11, cancelled by hugo, is listed as `groomed` with an `overlays.cancelled` entry, and raul's counts put it under groomed.
- expected: a filter for the terminal overlay — the model makes cancellation an overlay — so a caller tallying by status does not count a terminal task as live work.
- severity: low

### `start_readiness` does not see the overlay that stops the claim
- command: `talita --project examples/ledgerline task show TSK08`
- happened: `derived.claimable` is false, but `derived.start_readiness.gates` names only `task.claimed` ("task is not claimed"); the deferred overlay is absent, so the readiness list reads as "claim it and go". `claim take TSK08` then refuses `claim.not_claimable`: "TSK08 cannot be claimed: the task is deferred" (a write; not re-run).
- expected: the readiness list to name `overlay.clear`, the gate `claim take` reports, so the two derived answers agree.
- severity: low

## The journal

### The journal records the role, not the actor
- command: `talita --project examples/pantry journal show --task TSK06`
- happened: every event's `by` is `creator`, `runner`, `reviewer` or `human`; jo's four resolutions and four re-reviews read `"by": "runner"`, the same as ravi's, and `ses-jo-pantry` appears zero times in `activity.json`.
- expected: the actor id, as `claim.by`, `round.reviewer` and a note's `author` record it. ADR 0018 defers the actor and session to later; until then the record cannot say who fixed FIND-01.
- severity: low
