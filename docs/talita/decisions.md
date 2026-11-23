# talita — decisions

The human ruled on 2026-11-22: 16, 28 and 9 explicitly, the rest as
recommended. The reasoning is in the ADRs.

| # | Question | Ruling | Where |
|---:|---|---|---|
| 1 | Grammar | noun, then verb | [0012](../decisions/0012-noun-verb-grammar.md) |
| 2 | builder / runner word | each file keeps its word; input accepts both | [0012](../decisions/0012-noun-verb-grammar.md) |
| 3 | Grooming input | one call per part; no batch input | [0012](../decisions/0012-noun-verb-grammar.md) |
| 4 | Repairing a broken file | reviewed hand-edit; `project check` reports | [0013](../decisions/0013-intent-verbs.md) |
| 5 | Criterion typo after approval | child task | [0013](../decisions/0013-intent-verbs.md) |
| 6 | Gate failures reported | every failing gate (core collect-all check) | [0014](../decisions/0014-output-and-errors.md) |
| 7 | Output format | JSON always; `--text` later | [0014](../decisions/0014-output-and-errors.md) |
| 8 | Identity source | `TALITA_*` env, flags override, refused when missing | [0015](../decisions/0015-identity.md) |
| 9 | A human's decision | creator agent with `--instructed-by <human>` (human's ruling) | [0015](../decisions/0015-identity.md) |
| 10 | Session registry | none; `session new` mints | [0015](../decisions/0015-identity.md) |
| 11 | `HUGO_SESSION` | renamed `HUGO_CREW` | [0015](../decisions/0015-identity.md) |
| 12 | When `planned_by` is set | at `task approve`, approving session | [0015](../decisions/0015-identity.md) |
| 13 | Locking | one `flock` per project | [0016](../decisions/0016-concurrency.md) |
| 14 | Revision check | `Task.revision`, `--expect-revision` optional | [0016](../decisions/0016-concurrency.md) |
| 15 | Journal format | one JSON object; JSON Lines later | [0016](../decisions/0016-concurrency.md) |
| 16 | How a round gets in | whole round, `round file --input` (human's ruling) | [0017](../decisions/0017-review-record.md) |
| 17 | New reviewer gates | `review.round_current`, `review.round_owner` added | [0017](../decisions/0017-review-record.md) |
| 18 | Reviewer vs execution session | external round refused from `claim.session` too | [0017](../decisions/0017-review-record.md) |
| 19 | Self rounds | no findings, any verdict, `n = 0` | [0017](../decisions/0017-review-record.md) |
| 20 | Repeated focus | warn | [0017](../decisions/0017-review-record.md) |
| 21 | Who closes a finding | remediating runner; reviewer withdraw later | [0017](../decisions/0017-review-record.md) |
| 22 | Budget after `resume` | `--grant-rounds <n>` with `--instructed-by` | [0017](../decisions/0017-review-record.md) |
| 23 | Who writes `runtime.json` | talita; derived fields on every write | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| 24 | Journal kinds | one per lifecycle and review act | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| 25 | Actor id in journal events | later; `by` stays the role | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| 26 | Dead runs | `run show` stale; `run start` refuses active | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| 27 | Crew `model` format | the model alone | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| 28 | Evidence | talita runs `verification`; `override` bypass (human's ruling) | [0019](../decisions/0019-evidence.md) |
| 29 | Verification timeout | 10 min, `--timeout` | [0019](../decisions/0019-evidence.md) |
| 30 | Claim lease | 2 h, renewable | [0020](../decisions/0020-claims-and-leases.md) |
| 31 | `task.claimed` on `finish` | yes | [0020](../decisions/0020-claims-and-leases.md) |
| 32 | Claim after `reopen`/`resume` | old claim stays; takeover if expired | [0020](../decisions/0020-claims-and-leases.md) |
| 33 | Child tasks | `task create --parent`, `TSK03.n`, inherits nothing | [commands](commands.md) |
| 34 | Ids past 99 | numeric sort in talita and raul | [commands](commands.md) |
| 35 | Items in the journal | later | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| 36 | `project init` | explicit; cwd fallback dropped | [commands](commands.md) |

## Drift and contradictions

What the inventories found, what the design does with each, and what is left.

| # | Drift | Where | This design | Status |
|---:|---|---|---|---|
| 1 | `evidence.attached` says "every criterion" vs "every criterion not deferred" | `docs/skills/task-authoring.md:119` vs TM:261 | follows TM; skill doc to fix | doc fix |
| 2 | `review.independent` lists `pass_review, request_changes`, omits `escalate` | task-authoring.md:130, task-review.md:191 vs TM:272, LC:315 | follows TM and LC; skill docs to fix | doc fix |
| 3 | fixture row "`status` `deferred`" is a criterion's status; the task is `executing` | TM:814 | exposed; TM to fix | doc fix |
| 4 | claimable = no live claim vs start refuses an expired lease | `generate.py:1472-1486` vs TM:864 | one rule ([0020](../decisions/0020-claims-and-leases.md)) | resolved by design; doc fix (TM:864) |
| 5 | queue holds held and done tasks vs "the queue holds unclaimed work" | TM § 11.2 vs `docs/hugo/design-record.md:99-100` | record holds all; the pane hides ([0018](../decisions/0018-runtime-and-journal-writers.md)) | resolved by design |
| 6 | `runner` in tasks and journal, `builder` in runtime | roles.md, activity.rs, runtime.rs | each file keeps its word ([0012](../decisions/0012-noun-verb-grammar.md)) | resolved by design |
| 7 | `rounds[].reviewer` is "the session" vs fixtures hold an actor id | ADR 0008 vs fixtures | `Round.session` added; `reviewer` stays the actor ([0017](../decisions/0017-review-record.md)) | follow-up for core |
| 8 | "a role may not write another's section" (review = reviewer) vs "the runner resolves findings" | roles.md vs TM § 4.2 | the finding outcome is the runner's part of the review section | resolved by design; doc fix (roles.md) |
| 9 | criterion status "never written" vs stored status "mirrors"; ADR 0006 has no `unproven` | ADR 0006 vs TM:158, § 5 | talita writes the mirror ([0019](../decisions/0019-evidence.md)); ADR 0006's table to gain `unproven` | resolved by design; doc fix (ADR 0006) |
| 10 | manual override "marked", no field | ADR 0006 vs `task.rs:376-389` | `Evidence.manual` ([0019](../decisions/0019-evidence.md)) | follow-up for core |
| 11 | `budget.spent` counts the round in flight vs self rounds not counted; generator asserts `spent == number` | TM § 11.2, § 4.3, `generate.py:1590` | self stage: `number 0`, spent = external filed ([0018](../decisions/0018-runtime-and-journal-writers.md)) | resolved by design; doc fix (generator) |
| 12 | fixtures record transitions as notes ("Approved by …"), journal derived from them | `generate.py:1114-1120, 1332-1407` | talita writes journal events, not such notes; generator to follow | doc fix (generator) |
| 13 | `HUGO_SESSION` holds the crew, not a session | `crates/hugo/src/config.rs:141` | talita ignores it; rename proposed ([0015](../decisions/0015-identity.md)) | follow-up for hugo |
| 14 | hugo and raul "write nothing" yet write `config.toml` | `hugo/src/main.rs:3-5`, `docs/tui/screens.md:3-4` | exposed, not talita's: the configuration's writer stays undefined (TM § 15) | open: configuration writer (TM § 15) |
| 15 | "not decided: whether autopilot runs at all" vs the ruling that hugo starts the orchestrator | `docs/hugo/design-record.md:271-274` vs :84-92 | exposed | doc fix (design-record) |
| 16 | the orchestrator skill records to `TODO.md` vs "the task folder is the memory once talita writes" | `hugo/skills/orchestrator.md:73` vs ADR 0011 | exposed; the flow skills move to talita when it exists | doc fix (flow skills, when talita exists) |
| 17 | `Store::list` skips a broken file silently; raul reports it | `store.rs:145-149` vs `raul/src/data.rs:105-118` | talita refuses writes to a broken file (`store.unreadable`), reports in `project check` | resolved by design |
| 18 | `save_item` does not remove the old file on rename | `store.rs:263-277` | exposed; no item verb renames today | follow-up for core |
| 19 | ids pad to 2 digits; string sort breaks past `TSK99` | `raul/src/data.rs:122`, `store.rs:151` | talita sorts numerically (decision 34) | resolved by design; follow-up for raul |
| 20 | fixtures write `HH:MMZ`; core writes `HH:MM:SSZ` | `generate.py:975-976`, `clock.rs:6-13` | talita writes seconds; both parse | resolved by design |
| 21 | `talita create` makes `./coding-crew` when none is found | `crates/talita/src/main.rs:66-71` | dropped; `project init` | resolved by design |
| 22 | nothing defines rounds, verdict, budget or claim after `reopen`/`resume`; a stale `ok` passes a reopened task | TM § 3.2, LC:322-325 | `review.round_current` ([0017](../decisions/0017-review-record.md)), claim rule ([0020](../decisions/0020-claims-and-leases.md)), decision 22 | resolved by design; follow-up for core |
| 23 | `planned_by` is never set; `review.independent` then refuses every review | `task.rs:102-105`, LC:398-411 | set at `approve` (decision 12) | resolved by design |
| 24 | `human.instructed` gate in TM, not in LC | TM § 3.2 vs LC:318-321 | talita checks `--instructed-by` ([0015](../decisions/0015-identity.md)) | resolved by design |
| 25 | talita's surface "not defined" | TM:842, § 15; plan.md G3 | defined here, proposed | resolved by design; doc fix (TM) |
| 26 | the skill docs' flat verbs | task-authoring.md:222-234, task-review.md:215-223 | renamed ([commands.md](commands.md#documented-verbs-renamed-or-dropped)) | doc fix (skill docs) |
