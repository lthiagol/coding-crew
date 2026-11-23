# Dogfood — what talita hurt

The two sandbox projects under `examples/` were written by talita alone, with
the binary built from this branch. Every entry below is something a caller
would not expect, or a rule that cost a call to find.

## Pantry

### The fifth review round cannot be sent back
- command: `talita --as nadia --role reviewer --session ses-nadia-pantry task request-changes TSK05`
- happened: refused, exit 4, `review.budget_left`: "the review round budget is exhausted" (5 of 5 rounds). The round's high finding stays open and the reviewer escalates it unfixed.
- expected: the fifth round to go back for a fix like the first four, or the refusal to name `task escalate` as the only move left. A caller following the cycle reads the budget's last round as one more round.
- severity: high

### A re-review makes the reviewer's verdict stale
- command: `talita --as nadia --role reviewer --session ses-nadia-pantry task escalate TSK05`
- happened: refused, exit 4, `review.budget_exhausted` and `review.round_current`: "the latest external round was not filed since the task entered code-review". The remedy names the same command again.
- expected: the changes-needed verdict to hold while the runner hands the task back. `review.round_current` treats the hand-back as a new entry into code-review, so the round is stale; together with the entry above, an escalate can only happen on an unremediated finding.
- severity: medium

### The finish refusal names the wrong next call
- command: `talita --as ravi --role runner --session ses-ravi-pantry task finish TSK03`
- happened: refused, exit 4, `criteria.proven`: "ACCEPT-01 failed without a reason"; remedy `talita evidence run TSK03 <ACCEPT>`.
- expected: `talita criterion explain TSK03 ACCEPT-01 --failure-reason …`, the only call that clears the gate. Rerunning the evidence changes nothing.
- severity: medium

### A human is told to act as an agent
- command: `talita --as hannah --role human task approve TSK09`
- happened: `actor.not_agent`, exit 5, "task approve is an agent's act"; remedy "act as an agent: --role creator, runner or reviewer".
- expected: the route ADR 0015 gives a human: ask the creator to run `task approve TSK09 --instructed-by hannah`. The remedy tells the human to be the thing talita just refused.
- severity: low

### `project init` asks for identity the docs call "any"
- command: `talita project init --path examples/pantry`
- happened: `identity.missing`, exit 5: a write needs its actor, role and session.
- expected: the folder. commands.md marks the verb "Who: any"; the conventions say every write carries identity, and the table does not repeat it.
- severity: low

### The journal records the role, not the actor
- command: `talita journal show --task TSK06`
- happened: every event's `by` is `creator`, `runner`, `reviewer` or `human`; ravi's and jo's moves both read `"by": "runner"`.
- expected: the actor id, as `claim.by`, `round.reviewer` and a note's `author` record it. ADR 0018 defers the actor and session to later; until then the record cannot say who fixed FIND-01.
- severity: low
