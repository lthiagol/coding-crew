# 0018. The runtime record and the journal — who writes them, and when

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

raul reads the journal (`activity.json`); hugo reads only the runtime record
(`runtime.json`). Nothing writes either (TM § 11.1, § 11.2, § 15;
`TODO.md:204-217`). Both shapes are provisional and "the writer settles
them".

The journal: `{events: [{at, kind, task, by, model?, harness?, note?}]}`,
`task` required, nine kinds (`created … commented`,
`crates/core/src/activity.rs:12-22`). No kind exists for groom, start, a
filed round, finding resolution, resume, reopen or the overlays
(`inv-hugo.md` § 1b). The mock derives events from task history, partly from
notes that start "Approved by" (`mock-data/generate.py:1332-1407`) — a
generator convention, not the model.

The runtime record mixes two kinds of fields (`inv-hugo.md` gap 2):

| Derivable from the task files | Known only to the run |
|---|---|
| queue `status`, `claimable` | `started_at`, `stamp` |
| `round` (`task`, `stage`, `number`, `earlier`) | crew rows: `role`, `model`, `state` |
| `budget`, `findings`, `severities` | which tasks the run takes, in what order |

hugo trusts `claimable` as given (`inv-hugo.md` § 3). Nothing tells hugo an
agent or the orchestrator died: `stamp.runs` stays `true`.

## Decision

### The journal is a side effect, never a verb

Every task verb that changes a task appends its event in the same locked
write ([ADR 0016](0016-concurrency.md)). There is no `journal append`: a
free journal writer would be the escape hatch ADR 0013 forbids, and an event
nothing happened for. `journal show` reads it.

Kinds, one per act (bold = new; core's enum and raul's reader gain them):

| Kind | Written by |
|---|---|
| `created` | `task create`, `item promote` (note: `from ITM<nn>`) |
| **`groomed`** | `task groom` |
| `approved` | `task approve` (note: `instructed by <human>`, when the flag is given) |
| `claimed` | `claim take` (note on takeover: `took over from <actor>`) |
| **`released`** | `claim release`, `task cancel` with a live claim |
| **`started`** | `task start` |
| **`round_filed`** | `round file` (note: `self` or `external n`, focus, verdict) |
| `finding_raised` | `round file` (one per finding), `finding open` |
| **`finding_resolved`** | `finding resolve`, `finding dismiss` (note: fixed or dismissed) |
| `review_requested` | `task finish` |
| **`changes_requested`** | `task request-changes` |
| `remediated` | `task re-review` (note: `after round <n>`; the dated record is `review.re_reviews[]`, [ADR 0017](0017-review-record.md)) |
| `escalated` | `task escalate` |
| `completed` | `task pass-review` |
| **`resumed`** · **`reopened`** | `task resume`, `task reopen` (note: instructed by, reason) |
| **`blocked`** · **`unblocked`** · **`deferred`** · **`undeferred`** · **`cancelled`** | the overlay verbs (note: reason) |
| `commented` | `note add`, `comment add` |

Every event carries `at`, `kind`, `task` and `by`; a call that names
`--model` or `--harness` records them on each of its events, and a note when
the act has one. The model and the harness are optional: an event a call
appends without them carries neither, and an older journal reads unchanged.

Not journaled: content edits (intent, scope, criteria, steps, meta), questions,
evidence, item moves, run/crew/queue verbs. The task file is their record; the
journal is the feed of moves. `by` stays the role word (`runner`, `reviewer`,
`creator`, `orchestrator`, `human`); the actor id is in the task file.

talita writes no "Approved by" notes. The generator's convention should move
to journal events.

### talita owns `runtime.json`; derived fields are always recomputed

talita is the runtime record's only writer, as ADR 0007 asks of every file.

- **Written fields** come from explicit orchestrator verbs: `run start|stop`,
  `crew join|leave|assign|idle|stall`, `queue add|drop`.
- **Derived fields** are recomputed from the task files on **every** talita
  write while a record exists — a task verb by any agent, or a run verb. hugo
  never shows a `claimable` older than the last write.
- **Side effects on crew rows**, where the task verb makes them certain:

| Task verb | Crew row of the acting actor |
|---|---|
| `claim take` | holds the task, `since` now, `executing` |
| `task finish`, `task re-review` | `idle`, keeps the hold |
| `round file` (external) | `idle`, keeps the hold |
| `task pass-review`, `task escalate` | every row holding the task released, `idle` |
| `claim release`, `task cancel` | rows holding the task released |

The reviewer's `reviewing` state and a remediation builder's hold have no
task verb behind them; the orchestrator sets them with `crew assign`.

Crew rows gain `actor` (optional; hugo ignores it) so a task verb finds the
caller's row.

### How the derived fields are computed

| Field | Rule |
|---|---|
| `queue[].status` | the task file's status |
| `queue[].claimable` | `approved` · no overlay · no live claim · every `depends_on` done — the same core function raul's Claimable uses |
| `round` | present when a crew row holds a task. `task` = that task. `stage` = `external` while a reviewer row is `reviewing` it or it is `remediating`; else `self`. `number` = external rounds filed + 1 for `external`; `0` for `self` |
| `earlier[]` | the external rounds filed, oldest first; `raised` = findings that round opened |
| `budget` | `spent` = `number` for `external`, external rounds filed for `self`; `total` = `review.max_rounds` |
| `findings`, `severities` | open findings of the task in flight, by severity |

Self rounds never enter `earlier` or `budget` (TM § 4.3: self rounds do not
consume the budget). This settles `inv-hugo.md` gap 7; the generator's
invariant `budget.spent == round.number` holds for `external` only.

The queue keeps every task the run took, held and done included (TM § 11.2);
the pane hides held ones. design-record's "the queue holds unclaimed work"
describes the pane, not the record.

### Staleness

No heartbeat. `run show` reports `stale: true` when the record's last write
is older than the longest live lease of any task the crew holds (default 2 h).
The orchestrator calls `run stop --why` on every exit it controls. A crashed
run stays `runs: true` until the next `run start`, which refuses
(`run.active`) and names `run stop` as the remedy. Liveness in hugo is later.

### Alternatives considered

| Option | Lost because |
|---|---|
| A `journal append` verb | an unchecked writer; events with no act behind them |
| Journal kinds kept at nine | groom, start, resume, reopen and overlays would be invisible in raul's activity |
| The orchestrator writes `runtime.json` itself | a second writer (ADR 0007); its derived fields would drift from the tasks |
| Derive the whole record on read, no file | hugo reads a file by design, and crew, order and stamp are not in the tasks |
| Recompute derived fields only on run verbs | `claimable` would go stale between orchestrator calls |
| A heartbeat field, renewed by the orchestrator | a write every few seconds, and hugo does not read it yet |

### Settled 2026-11-22

- Settled 2026-11-22: talita owns `runtime.json`: orchestrator verbs for written fields, derived fields recomputed on every write.
- Settled 2026-11-22: Journal kinds: one per lifecycle and review act, as listed above.
- Settled 2026-11-23: The journal's `by` stays the role word, and the actor is
  not recorded. The role and the model are what matter, and the harness is an
  extra: an event carries the model and the harness when the call named them
  (the human's ruling, replacing the line of 2026-11-22 that deferred the
  actor and the session).
- Settled 2026-11-22: Items are not journaled; later.
- Settled 2026-11-22: Dead runs: `run show` reports `stale`, `run start` refuses an active record; a heartbeat is later.
- Settled 2026-11-22: Crew `model` is the model alone; harness and effort stay in the configuration.

## Consequences

### Positive

- Both records have one writer, and every fact in them traces to a verb.
- hugo's `claimable` and counts match the task files after every write.
- The journal covers every lifecycle move, so raul's activity is complete.

### Negative

- Every task write also rewrites `runtime.json` while a run exists.
- A journal write that fails after the task file is written refuses the call
  with the task already on disk. The write order
  ([ADR 0016](0016-concurrency.md)) puts the truth first: the journal never
  claims a move that did not happen, and the reverse window — a task written
  with no event — is accepted. It takes a full disk or a vanished permission
  between the two writes.
- New journal kinds are a core enum change; an older raul counts them as
  unreadable.
- A crash leaves `runs: true` until a human or the next run clears it.

### Follow-up

- TM § 11.1 settles the kinds; § 11.2 settles the derivation rules above.
- `mock-data/generate.py` emits the new kinds and stops deriving `approved`
  from notes.
- The flow skills call the run, crew and queue verbs at the points the
  sequence diagram in `docs/talita/README.md` shows.
