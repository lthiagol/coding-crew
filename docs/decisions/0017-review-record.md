# 0017. The review record — a round is filed whole

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

The review lives in the task: `review {max_rounds, rounds[], verdict,
findings[]}` (TM § 4). A round is `{n, kind, focus, reviewer, verdict, at,
notes, findings[]}`; `verdict` is required. `review.verdict` is the latest
round's. Nothing writes any of it (`inv-raul.md` gap 3).

Prose rules with no code (`docs/skills/task-review.md:193-194`): a verdict of
`ok` with an open finding is refused; `changes-needed` with no finding is
refused.

The reviewer skill today writes a round file with findings `D1…Dn`, each
`## D<n> [sev] title / where / defect / scenario / fix`, and a
`VERDICT: CLEAN|FINDINGS <n>` line (`hugo/skills/reviewer.md:41-51`). The
orchestrator merges axis findings into its round record. None of it reaches
the task.

The human floated `talita --task X --review add-round <json>`: a whole round
as structured input.

Two more gaps:

- ADR 0008 says `review.rounds[].reviewer` records "the session that filed
  the round"; fixtures and the field hold an actor id. `TODO.md:157` rules:
  add `session` to `Round` with the review writer.
- After `reopen` (done → executing) and `finish`, the latest round is still
  the old `ok`. `pass_review` would pass on it with no new review. The same
  stale verdict exists after any return to `code-review`.

## Decision

### A round is filed whole, in one call

```
talita round file TSK04 --input round.json          # external, any verdict, with findings
talita round file TSK04 --kind external --focus edge-cases --verdict ok
talita round file TSK04 --kind self --focus spec-compliance --verdict ok
```

The JSON input is a closed schema: `kind`, `focus`, `verdict`, `notes`,
`findings[] {ref, severity, description}`. Unknown keys are refused. The flag
form covers a round with no findings. talita:

1. checks status, role, session and budget;
2. checks the verdict rules against the round as a whole;
3. allocates `n` and one `FIND-nn` per finding;
4. writes the round, the findings and `review.verdict` in one atomic write;
5. returns `refs`: `{"D1": "FIND-03", …}`.

A round is never half-filed: raul and hugo never see a round without its
findings or a verdict that the findings contradict.

### Rules at filing

| Rule | self | external |
|---|---|---|
| Who | the working runner (claim holder in `executing`; a session other than `claim.session` in `remediating`) | reviewer |
| Status | executing, remediating | code-review |
| `n` | `0` (the pre-screen, `TODO.md:104`) | external rounds filed + 1 |
| Findings | refused — the builder fixes what it finds before `finish` | allowed |
| `ok` + any open finding on the task | — | `review.verdict_mismatch` |
| `changes-needed` + no finding in this round | — | `review.verdict_mismatch` |
| Budget | not counted | refused when `max_rounds` external rounds exist (`review.budget_spent`); the last round's way out is `task escalate`, its findings open (the human's ruling, 2026-11-23) |
| Session | recorded | recorded; must differ from `planned_by` (`review.independent`, checked now, not only at the transition) and from `claim.session` |

A self round's verdict is the builder's opinion; `finish` needs one to exist
(`self_round.filed`), not that it says `ok`.

### The D-style findings map to FIND-nn

| Reviewer file | Task finding |
|---|---|
| `D<n>` | `ref` in the input; not stored. The mapping is in the output for the orchestrator's round file |
| `[sev]` | `severity` (`low`/`medium`/`high`) |
| title, where, defect, scenario, fix | `description`, markdown, verbatim |
| `VERDICT: CLEAN` | `verdict: ok`, no findings |
| `VERDICT: FINDINGS n` | `verdict: changes-needed`, n findings |

The orchestrator's merge of axis findings into one round stays the
orchestrator's job (task-review.md, "The verdict"); talita files the merged
round.

### Round gains `session`

`Round.session` (optional on read, always written). `reviewer` stays the
actor id. `review.independent` reads the round's session for the filing
check.

### Two new gates on the reviewer's transitions

| Gate | Checks | Applies to |
|---|---|---|
| `review.round_current` | the latest external round was filed at or after `status_since` (the task's latest entry into `code-review`) | `pass_review`, `request_changes`, `escalate` |
| `review.round_owner` | the acting session filed the latest external round | the same |

The first closes the stale-verdict hole. The second keeps the verdict and the
move it causes in one session.

### A re-review is a dated record

After `task re-review` the task is back in `code-review`, and nothing on it
says when or by whom remediation handed it back: `status_since` is
overwritten by the next move, and the rounds only say what the reviewer
filed. A fixture in `code-review` after an external round (TSK14) cannot
show it was remediated. So `task re-review` appends, in the same write:

```json
"review": { "re_reviews": [ { "at": "2026-11-11T17:30:00Z", "after_round": 1,
                              "by": "builder-1", "session": "ses-runner-builder-1-…" } ] }
```

| Field | Rule |
|---|---|
| `at` | when the task went back to `code-review` |
| `after_round` | the external round whose findings were worked |
| `by`, `session` | the remediating actor and session — the session `remediation.session_distinct` checked |

Append-only, one entry per `re_review`. The journal gets `remediated` with
the note `after round <n>`. raul can place each re-review between its rounds;
the runtime's `round.stage` reads it.

### Transitions stay separate calls

`round file` does not move the status. The reviewer then calls
`task pass-review`, `request-changes` or `escalate`. One verb, one intent
(ADR 0012); a refused transition does not undo a filed round.

### Findings after the round

`finding open` adds a finding to the latest external round, only when it is
`changes-needed` and the caller filed it. Resolution is the runner's, in
`remediating`, in a session other than `claim.session`; the outcome is
written once.

### Alternatives considered

| Option | Lost because |
|---|---|
| One call per part: `round open`, `finding open` × n, `round close --verdict` | a round without a verdict cannot be represented (verdict is required); raul would see half-rounds; the verdict rules cannot be checked until the end anyway |
| Repeated flags (`--finding "high:…"`) | multi-line markdown descriptions in flags are fragile |
| `round file` also performs the transition | merges two intents; a gate refusal would leave the agent unsure whether the round was written |
| Store `ref` (`D3`) on the finding | the D number is the orchestrator's scratch label; the FIND id is the record's |
| Self rounds may carry findings | an open self finding would block `pass_review` and only the runner could close it, in a session `re_review` forbids |

### Settled 2026-11-22

- Settled 2026-11-22: A round is filed whole: `round file <TSK> --input` (the human's ruling).
- Settled 2026-11-22: `finish` needs a self round to exist, not an `ok` one.
- Settled 2026-11-22: `review.round_current` and `review.round_owner` are added to core.
- Settled 2026-11-22: An external round is refused from `claim.session` as well as from `planned_by`.
- Settled 2026-11-22: Self rounds all carry `n = 0`; position orders them.
- Settled 2026-11-22: A repeated focus warns; it does not refuse.
- Settled 2026-11-22: The remediating runner resolves or dismisses findings; a reviewer withdraw is later.
- Settled 2026-11-22: `task re-review` records `review.re_reviews[]`.
- Settled 2026-11-22: After `resume`, `--grant-rounds <n>` raises `max_rounds`, only with `--instructed-by`.

## Consequences

### Positive

- One atomic write per round. The verdict rules become code.
- The reviewer's existing output maps one-to-one onto the input.
- A stale `ok` can no longer pass a reopened task.

### Negative

- Two new gates in core's lifecycle; `Round.session` and `review.re_reviews[]`
  in the schema.
- Self rounds all number `0`; several self rounds on one task share `n`.
  raul orders them by position.

### Follow-up

- TM § 3.3 gains `review.round_current` and `review.round_owner`; § 4.1
  gains `session`.
- The reviewer skill writes its round as the input schema, or the
  orchestrator converts it.
- `mock-data/generate.py` adds `session` to rounds (`TODO.md:157`) and
  `re_reviews[]` to remediated fixtures (TSK14).
- TM § 4 gains `review.re_reviews[]`.
