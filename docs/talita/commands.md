# talita — command reference

Accepted 2026-11-22. Built: `session`, `project` (T5); `task` create and content, `show` and `list`, `scope`, `criterion add|revise|remove`, `step add|revise|remove`, `question`, `note` (T6); the `task` lifecycle verbs and overlays (T7); the `claim` verbs (T8); the `step` and `criterion` work verbs and the `evidence` verbs (T9); the `round`, `finding` and `comment` verbs (T10); the journal writer and `journal show` (T11); the runtime writer, `run`, `crew` and `queue` (T12); the `item` verbs (T13). The grammar is [ADR 0012](../decisions/0012-noun-verb-grammar.md);
the output and errors [ADR 0014](../decisions/0014-output-and-errors.md);
identity [ADR 0015](../decisions/0015-identity.md). Gate names are task-model
§ 3.3's. "TM" is `docs/tasks/task-model.md`, "LC" `crates/core/src/lifecycle.rs`.

## Conventions

```
talita [global flags] <noun> <verb> <ids…> [flags]
```

| Global flag | Env | Meaning |
|---|---|---|
| `--project <path>` | `TALITA_PROJECT` | project root. Default: walk up to the nearest `coding-crew/` (ADR 0010) |
| `--as <actor>` | `TALITA_ACTOR` | actor id, e.g. `builder-1`, `luiz` |
| `--role <role>` | `TALITA_ROLE` | `creator` · `runner` (alias `builder`) · `reviewer` · `orchestrator` · `human` |
| `--session <id>` | `TALITA_SESSION` | the working session. Required for every agent write |
| `--model <model>` | `TALITA_MODEL` | optional. The model that runs the call; its journal events carry it |
| `--harness <harness>` | `TALITA_HARNESS` | optional. The harness that runs the call; its journal events carry it |
| `--expect-revision <n>` | — | refuse with `conflict.revision` unless the task's revision is `n` (ADR 0016) |

- Flags beat env. Read verbs (`show`, `list`, `journal show`) need no identity.
- `human` is not a role: it marks the actor as `kind: human` (ADR 0015).
- **Text values.** Any text flag takes `@<path>` to read a file, `@-` for stdin.
  Text is stored verbatim (ADR 0009). A title is one line.
- **Durations.** `30m`, `2h`, `1d`.
- **Ids.** Task first, then the part: `TSK04 FIND-02`. Parts are per task:
  `ACCEPT-nn`, `STP-nn`, `QST-nn`, `NOTE-nn`, `FIND-nn`, `CMT-nn`. talita
  allocates every id; no verb takes an id for a part it creates.
- **Output.** One JSON object on stdout (ADR 0014):
  `{"ok":true,"command":"task approve","data":{…},"wrote":[…],"journal":[…]}`
  or `{"ok":false,"command":…,"error":{"code","message","gates"?,"remedy"?}}`.
- **Side effects.** A verb that writes a task also, in the same call, appends
  its journal event (the **J** column) and, when `runtime.json` exists,
  refreshes its derived fields (ADR 0018).
- **Who.** `creator`, `runner`, `reviewer`, `orchestrator` are agent roles;
  `any` includes humans. Role-gated verbs refuse a human (`actor.not_agent`).
- **A human's decision.** A human decides; a creator agent writes it down. The
  four verbs that take `--instructed-by` — `task approve`, `task resume`,
  `task reopen`, `question answer` — point a refused human at that call. Every
  other role-gated verb names the role that owns it.
- **No identity.** `project init` is the one write that needs no identity: it
  writes no task and no journal (ADR 0015). Every other write carries it.
- **Ownership** (roles.md): intent, scope, criteria, meta = creator;
  questions: any role asks, the creator answers; steps = creator before
  approval, runner after; claim, evidence = runner; review = reviewer, except a finding's outcome = runner; notes =
  any; status = transitions only. A write outside the role's section is
  `section.not_owned`.
- **Working runner.** In `executing`: the runner holding the claim (same
  actor and session; `claim.not_holder` otherwise). In `remediating`: a runner
  whose session is not `claim.session` (`remediation.session_distinct`).
  Step, criterion, evidence and self-round verbs need the working runner.
- **Frozen.** From `approved` on, `intent`, `scope` and
  `acceptance_criteria` refuse content writes with `section.frozen`; the
  remedy is a child task (`task create --parent`). Deferral, failure reason
  and evidence stay writable (ADR 0013).

Error codes and exit codes ([ADR 0014](../decisions/0014-output-and-errors.md)).
Pinned to talita's code enum by a test:

| Exit | Class | Codes |
|---:|---|---|
| 1 | internal | `internal` |
| 2 | usage or input | `usage.invalid`, `input.invalid` |
| 3 | not found | `project.not_found`, `task.not_found`, `part.not_found`, `item.not_found`, `run.absent`, `crew.unknown` |
| 4 | refused by a rule | `status.wrong`, `gate.refused`, `section.frozen`, `task.cancelled`, `review.verdict_mismatch`, `review.budget_spent`, `finding.closed`, `question.resolved`, `claim.held`, `claim.not_claimable`, `criterion.covered`, `criterion.not_failed`, `criterion.deferred`, `depends.cycle`, `overlay.absent`, `item.not_open`, `item.is_idea`, `item.already_shaped`, `evidence.timeout`, `project.exists`, `run.active`, `crew.exists` |
| 5 | identity | `identity.missing`, `role.wrong`, `actor.not_agent`, `section.not_owned`, `claim.not_holder` |
| 6 | conflict | `conflict.lock`, `conflict.revision` |
| 7 | damaged store | `store.unreadable` |

Common errors, on every verb: `usage.invalid` (2), `input.invalid` (2),
`project.not_found` (3), `task.not_found` (3), `part.not_found` (3),
`identity.missing` (5), `role.wrong` (5), `actor.not_agent` (5),
`task.cancelled` (4), `conflict.lock` (6), `conflict.revision` (6),
`store.unreadable` (7). Per-verb errors below add to these.

---

## project

| Verb | Synopsis | Who | Does |
|---|---|---|---|
| `init` | `project init [--path <dir>]` | any | creates `<dir>/coding-crew/work/` with no identity. Refuses `project.exists` if a `coding-crew/` is already there. Writes no task, no journal |
| `show` | `project show` | read | paths, counts by status and kind, whether journal and runtime exist, schema version |
| `check` | `project check` | read | parses every file; reports unreadable files, duplicate ids, `parent`/`children[]` mismatches, dangling `depends_on`, stored criterion status ≠ derived. Writes nothing. Repair: later |

No write verb creates `coding-crew/` implicitly: today's `talita create`
falls back to the cwd (inv-existing § 1); that fallback goes.

## task

### Read

| Verb | Synopsis | Output |
|---|---|---|
| `show` | `task show <TSK>` | `{task: <file as stored>, derived: {criteria: {ACCEPT-01: "passed"…}, review_state, lane, claimable, start_readiness: {ok, gates[]}, children_progress}}` |
| `list` | `task list [--status <s>…] [--overlay blocked\|deferred\|cancelled] [--kind <k>] [--claimable] [--lane human\|agent] [--needs-you] [--tag <t>]` | `[{id, title, status, overlays, kind, claim?, claimable}]`, id order (numeric, not string) |

Derived values are computed with core's one rule each (TM § 12), so talita
and raul cannot disagree.

`--overlay` may be given more than once; a task matches when it carries any
of the named overlays, as `--status` does. Cancellation is an overlay, not a
status: `--status cancelled` is `input.invalid`, and its remedy names
`talita task list --overlay cancelled`.

### Create and content (creator, before approval)

| Verb | Synopsis | Who | When | Writes | J |
|---|---|---|---|---|---|
| `create` | `task create --title <t> [--kind task\|bug\|feature] [--parent <TSK>]` | creator | — | new task in `draft`; id from the scan; with `--parent`, id `TSK03.n` and both `parent` and the parent's `children[]` | `created` |
| `retitle` | `task retitle <TSK> --title <t>` | creator | draft, groomed | title, slug, file rename (old file removed) | — |
| `describe` | `task describe <TSK> [--outcome <t>] [--problem <t>]` | creator | draft, groomed | `intent` | — |
| `prioritize` | `task prioritize <TSK> --priority urgent\|high\|normal\|low` | creator | before `approved` | `meta.priority` | — |
| `estimate` | `task estimate <TSK> --effort S\|M\|L` | creator | before `approved` | `meta.effort` | — |
| `tag` | `task tag <TSK> [--add <t>…] [--remove <t>…]` | creator | before `approved` | `meta.tags` | — |
| `depend` | `task depend <TSK> --on <TSK>…` | creator | draft, groomed, approved (unclaimed) | `depends_on`; refuses a missing task or a cycle (`depends.cycle`) | — |
| `undepend` | `task undepend <TSK> --on <TSK>…` | creator | same | `depends_on` | — |

`meta` is "mutable, pre-approval" (TM:163); talita enforces it. A small task's
`kind` is set at create and not changed (inv-raul gap 18).

### Lifecycle

One verb per core `Event`. The gates run inside core's `transition()`; talita
reports every failing gate, not only the first ([decision 6](decisions.md)).

| Verb | Synopsis | From → to | Who | Gates | Also writes | J |
|---|---|---|---|---|---|---|
| `groom` | `task groom <TSK>` | draft → groomed | creator | `intent.outcome_present` · `criteria.present` · `scope.out_of_scope_declared` | — | `groomed` |
| `approve` | `task approve <TSK> [--instructed-by <human>]` | groomed → approved | creator | `questions.resolved` · `criteria.covered` · `steps.done_when_present` (step rules skipped for a small task) | `planned_by` = acting session; freezes intent, scope, criteria | `approved` (note: `instructed by <human>`, when given) |
| `start` | `task start <TSK>` | approved → executing | runner | `task.claimed` (holder, lease live) · `dependencies.satisfied` | — | `started` |
| `finish` | `task finish <TSK>` | executing → code-review | runner, claim holder | `steps.settled` · `evidence.attached` · `criteria.proven` · `self_round.filed` · `task.claimed` (new on finish) | crew: builder idle | `review_requested` |
| `pass-review` | `task pass-review <TSK>` | code-review → done | reviewer | `review.verdict_ok` · `review.no_open_findings` · `review.within_budget` · `review.independent` · `review.round_current` (new) · `review.round_owner` (new) | crew: holds cleared; queue: done | `completed` |
| `request-changes` | `task request-changes <TSK>` | code-review → remediating | reviewer | `review.verdict_changes_needed` · `review.findings_exist` · `review.budget_left` · `review.independent` · `review.round_current` · `review.round_owner` | — | `changes_requested` |
| `re-review` | `task re-review <TSK>` | remediating → code-review | runner | `review.findings_closed` · `remediation.session_distinct` | appends `review.re_reviews[] {at, after_round, by, session}`; crew: builder idle | `remediated` |
| `escalate` | `task escalate <TSK>` | code-review → needs-human | reviewer | `review.verdict_changes_needed` · `review.budget_exhausted` · `review.independent` · `review.round_current` · `review.round_owner` | crew: holds cleared | `escalated` |
| `resume` | `task resume <TSK> --instructed-by <human> --reason <t> [--grant-rounds <n>]` | needs-human → executing | creator | `human.instructed` (flag present) | `--grant-rounds` raises `review.max_rounds` by `n` ([decision 22](decisions.md)) | `resumed` |
| `reopen` | `task reopen <TSK> --instructed-by <human> --reason <t>` | done → executing | creator | `human.instructed` | — | `reopened` |

`review.round_current` and `review.round_owner` are defined in
[ADR 0017](../decisions/0017-review-record.md): the latest external round was
filed at or after the task's last entry into `code-review`, and by the acting
session.
`task.claimed` on `finish` is defined in
[ADR 0020](../decisions/0020-claims-and-leases.md). `human.instructed` is in
TM § 3.2 but not in LC; talita checks it as "the flag names a human".

Errors: `status.wrong` (4), `gate.refused` (4, `gates[]` lists each failing
gate), `role.wrong` (5).

### Overlays

| Verb | Synopsis | Who | Gate | J |
|---|---|---|---|---|
| `block` | `task block <TSK> --reason <t>` | any | `overlay.reason_present` | `blocked` |
| `unblock` | `task unblock <TSK>` | any | blocked overlay present | `unblocked` |
| `defer` | `task defer <TSK> --reason <t>` | any | `overlay.reason_present` | `deferred` |
| `undefer` | `task undefer <TSK>` | any | deferred overlay present | `undeferred` |
| `cancel` | `task cancel <TSK> --reason <t>` | any | `overlay.reason_present` | `cancelled`; with a live claim also `released` |

Overlay writes set `{reason, by, at}`; `by` is the actor id. A cancelled task
refuses every later write (`task.cancelled`). Cancel releases a live claim.
Clearing an overlay that is not set: `overlay.absent` (4).

## scope

Creator, draft or groomed. Frozen after.

| Verb | Synopsis | Writes |
|---|---|---|
| `include` | `scope include <TSK> --item <t>` | appends to `scope.in_scope` |
| `exclude` | `scope exclude <TSK> --item <t>` | appends to `scope.out_of_scope` |
| `drop` | `scope drop <TSK> --in <n>` or `--out <n>` | removes entry `n` (1-based, as `show` lists them) |

## criterion

| Verb | Synopsis | Who | When | Writes |
|---|---|---|---|---|
| `add` | `criterion add <TSK> --description <t> --verification <t>` | creator | draft, groomed | new `ACCEPT-nn`, status `pending` |
| `revise` | `criterion revise <TSK> <ACCEPT> [--description <t>] [--verification <t>]` | creator | draft, groomed | the two fields |
| `remove` | `criterion remove <TSK> <ACCEPT>` | creator | draft, groomed | removes it; refuses if a step `covers` it (`criterion.covered`) — revise the step first |
| `defer` | `criterion defer <TSK> <ACCEPT> --reason <t> --follow-up <TSK>` | working runner | executing, remediating | `deferral`, stored status `deferred`. `--follow-up` must name an existing task |
| `undefer` | `criterion undefer <TSK> <ACCEPT>` | working runner | executing, remediating | clears `deferral`; status re-derived |
| `explain` | `criterion explain <TSK> <ACCEPT> --failure-reason <t>` | working runner | executing, remediating | `failure_reason`; refused unless the derived status is `failed` (`criterion.not_failed`) |

No verb writes a criterion's `status` directly. talita writes the stored
mirror from `effective_status()` on every evidence or deferral write (TM:158).

## step

| Verb | Synopsis | Who | When | Writes |
|---|---|---|---|---|
| `add` | `step add <TSK> --action <t> --done-when <t> [--covers <ACCEPT>…] [--files <p>…] [--after <STP>…] [--deviation <t>]` | creator before approval; working runner after | not done | new `STP-nn`, `order` last. After approval `--deviation` is required: a new step is a deviation |
| `revise` | `step revise <TSK> <STP> [same flags]` | same | same | the given fields; after approval `--deviation` required |
| `remove` | `step remove <TSK> <STP>` | creator | draft, groomed | removes. After approval use `skip`: the record stays |
| `begin` | `step begin <TSK> <STP>` | working runner | executing, remediating | `in-progress` |
| `done` | `step done <TSK> <STP> [--deviation <t>]` | working runner | executing, remediating | `done` |
| `skip` | `step skip <TSK> <STP> --deviation <t>` | working runner | executing, remediating | `skipped`; why is required |
| `fail` | `step fail <TSK> <STP> --deviation <t>` | working runner | executing, remediating | `failed` |

Reorder: later. `--covers` and `--after` refuse unknown ids.

## question

| Verb | Synopsis | Who | Writes | J |
|---|---|---|---|---|
| `ask` | `question ask <TSK> --question <t>` | any | new `QST-nn`, `open`, `raised_by`, `raised_at` | — |
| `answer` | `question answer <TSK> <QST> --answer <t> [--instructed-by <human>]` | creator | `resolved`, `answer`, `resolved_by`, `resolved_at`, `instructed_by` (the human `--instructed-by` names; a question is not journaled). Second answer: `question.resolved` (4) | — |

Append-only (TM:163): no withdraw. A question that no longer applies is
answered with why.

## note

| Verb | Synopsis | Who | Writes | J |
|---|---|---|---|---|
| `add` | `note add <TSK> --body <t>` | any | `NOTE-nn {author, body, at}` | `commented` |

## evidence

[ADR 0019](../decisions/0019-evidence.md). The working runner, in
`executing` or `remediating`. Append-only.

| Verb | Synopsis | Does |
|---|---|---|
| `run` | `evidence run <TSK> <ACCEPT> [--timeout <dur>] [--summary <t>]` | runs the criterion's `verification` with `sh -c` in the project root; records `command`, `exit_code`, `summary`, `at`, `by`, `session`. Summary: `--summary` if given, else the last non-empty line of output, cut to 200 chars. Output returns the exit code, the derived status and the output's last 40 lines |
| `override` | `evidence override <TSK> <ACCEPT> --reason <t>` | appends `{command: "", exit_code: 0, summary: <reason>, manual: true, …}`. The criterion reads `passed`, marked manual in raul |

Errors: `evidence.timeout` (4; the run is still recorded, exit code absent →
`unproven`), `criterion.deferred` (4).

## claim

[ADR 0020](../decisions/0020-claims-and-leases.md). Runner.

| Verb | Synopsis | When | Writes | J |
|---|---|---|---|---|
| `take` | `claim take <TSK> [--lease <dur>]` | approved and claimable (no overlay, deps done, no live claim); or executing with an expired lease and no `blocked` overlay (takeover) | `claim {by, session, at, lease_expires_at}`; default lease 2h; crew row of this actor: holds, executing | `claimed` |
| `renew` | `claim renew <TSK> [--lease <dur>]` | holder (same actor and session) | `lease_expires_at` | — |
| `release` | `claim release <TSK> --reason <t>` | holder; approved or executing | removes `claim`; crew row released | `released` |
| `show` | `claim show <TSK>` | read | the claim and whether it is live | — |

Errors: `claim.held` (4, a live claim by another), `claim.not_claimable`
(4, with the failing condition), `claim.not_holder` (5).

## round

[ADR 0017](../decisions/0017-review-record.md).

### `round file`

```
talita round file <TSK> --kind self --focus <f> --verdict ok|changes-needed [--notes <t>]
talita round file <TSK> --kind external --focus <f> --verdict ok [--notes <t>]
talita round file <TSK> --input <path|->
```

| | self | external |
|---|---|---|
| Who | working runner | reviewer |
| Status | executing, remediating | code-review |
| `n` | `0` | external rounds filed + 1 |
| Findings | none | via `--input` only |
| Budget | not counted | counted; refused past `max_rounds` (`review.budget_spent`); the last round escalates with its findings open (the human's ruling, 2026-11-23) |
| Session check | — | acting session ≠ `planned_by` (`review.independent`, checked at filing too) and ≠ `claim.session` |

Writes the round `{n, kind, focus, reviewer: actor id, session, verdict, at,
notes, findings[]}`, allocates `FIND-nn` for each input finding with status
`open` and `created`, and sets `review.verdict` to this round's verdict.

Verdict rules, checked against the whole round before anything is written:

| Rule | Code |
|---|---|
| external `ok` with an open finding on the task | `review.verdict_mismatch` |
| external `changes-needed` with no finding in the round | `review.verdict_mismatch` |
| `--focus` equal to the previous external round's | `review.focus_repeated` (warning in output, not a refusal — [decision 20](decisions.md)) |

Input schema (`--input`), closed — unknown keys refused:

```json
{
  "kind": "external",
  "focus": "edge-cases",
  "verdict": "changes-needed",
  "notes": "free text",
  "findings": [
    { "ref": "D1", "severity": "high", "description": "markdown: where, defect, scenario, fix" }
  ]
}
```

Output: the round, and `refs`: `{"D1": "FIND-03"}`. J: `round_filed`, plus
`finding_raised` per finding.

| Verb | Synopsis | Output |
|---|---|---|
| `show` | `round show <TSK> [<n>]` | rounds (or one), each with its findings |

## finding

| Verb | Synopsis | Who | When | Writes | J |
|---|---|---|---|---|---|
| `open` | `finding open <TSK> --severity low\|medium\|high --description <t>` | reviewer, the session that filed the latest round | code-review; latest round is external, `changes-needed` | `FIND-nn`, linked into the latest round's `findings[]` | `finding_raised` |
| `resolve` | `finding resolve <TSK> <FIND> --resolution <t>` | runner, session ≠ `claim.session` | remediating | `fixed`, `resolved`, `resolution` | `finding_resolved` |
| `dismiss` | `finding dismiss <TSK> <FIND> --reason <t>` | runner, session ≠ `claim.session` | remediating | `dismissed`, `resolved`, `resolution` = reason | `finding_resolved` |

Outcome written once: a second `resolve` or `dismiss` is
`finding.closed` (4). `resolve`/`dismiss` check
`remediation.session_distinct` at write time, so the executing session cannot
close its own findings and then hand them to another session to `re-review`.

## comment

| Verb | Synopsis | Who | Writes | J |
|---|---|---|---|---|
| `add` | `comment add <TSK> <FIND> --body <t>` | any agent role; human | `CMT-nn {author, body, at}` under the finding. Allowed on a closed finding | `commented` |

## item

Smaller tasks (`docs/tasks/smaller-tasks.md`). Creator. Items are not
journaled: a journal event needs a task id (later).

| Verb | Synopsis | When | Writes |
|---|---|---|---|
| `create` | `item create --kind idea\|feature\|bug --title <t> [--body <t>] [--scope <t>…] [--tag <t>…]` | — | new `ITM<nn>`, `open` |
| `show` / `list` | `item show <ITM>` · `item list [--status] [--kind]` | read | |
| `shape` | `item shape <ITM> --kind feature\|bug [--scope <t>…]` | open, kind idea | kind and scope; id unchanged |
| `dismiss` | `item dismiss <ITM> --reason <t>` | open | `dismissed`, `dismissed_reason` |
| `promote` | `item promote <ITM>` | open, kind feature or bug | item `promoted`, `promoted_to`; new task in `draft` with one criterion (item first, then task). J on the new task: `created`, note `from ITM<nn>` |

Errors: `item.not_open` (4), `item.is_idea` (4, remedy: shape it first),
`item.already_shaped` (4), `item.not_found` (3).

## session

| Verb | Synopsis | Output |
|---|---|---|
| `new` | `session new --as <actor> --role <role>` | `{session: "ses-<role>-<actor>-<random>", env: "TALITA_ACTOR=… TALITA_ROLE=… TALITA_SESSION=…"}`. Writes nothing |
| `show` | `session show` | the identity this call resolves to, and where each part came from (flag or env) |

## run, crew, queue — the runtime record

[ADR 0018](../decisions/0018-runtime-and-journal-writers.md). Orchestrator:
a write with another agent role is `role.wrong`; a human is `actor.not_agent`.
The reads (`run show`, `queue show`) need no identity.

Written fields are set by these verbs; derived fields (queue `status`,
`claimable`, `round`, `budget`, `findings`, `severities`) are recomputed from
the task files on every write, a task verb's included. Every write of the
record stamps `updated_at` with the instant it uses. None of these verbs
touches a task file or the journal.

| Verb | Synopsis | Writes |
|---|---|---|
| `run start` | `run start [--task <TSK>…]` | new `runtime.json`: `started_at` now, the queue from `--task` in order, an empty crew, `stamp.runs: true`. Refuses `run.active` if a record with `runs: true` exists (remedy `talita run stop --why <t>`); `task.not_found` for an unknown id |
| `run stop` | `run stop --why <t>` | `stamp {runs: false, why}`; every crew row `idle`, keeping its hold |
| `run show` | `run show` | the record, plus `stale` against its `updated_at` (read) |
| `run refresh` | `run refresh` | recomputes every derived field and writes the record. Repair after a crash |
| `crew join` | `crew join <actor> --role builder\|reviewer\|orchestrator --model <m>` | new crew row `{role, model, actor, state: idle}` |
| `crew leave` | `crew leave <actor>` | removes the row |
| `crew assign` | `crew assign <actor> <TSK>` | `holds`, `since` now, state `executing` (builder, orchestrator) or `reviewing` (reviewer) |
| `crew idle` | `crew idle <actor>` | state `idle`, keeps `holds` |
| `crew stall` | `crew stall <actor>` | state `blocked` (the agent cannot go on). Word chosen so it is not read as the task overlay |
| `queue add` | `queue add <TSK>… [--before <TSK>]` | appends (or inserts) ids, each once; every id the run took stays |
| `queue drop` | `queue drop <TSK>…` | removes the ids it holds; the rest changes nothing |
| `queue show` | `queue show` | the queue with derived status and claimable (read) |

Outputs: `run start|stop|refresh` and `run show` carry `{"runtime": <record>}`
(`run show` adds `"stale"`); the crew verbs `{"crew": [rows]}`; the queue verbs
`{"queue": [items]}`. A verb that writes the record names `runtime.json` in
`wrote`, and so does every task write while the record exists.

**Stale.** `run show` reports `stale: true` when the time since the record's
`updated_at` — the instant of its last write — passes the longest lease the
crew holds. A lease's length is its claim's `lease_expires_at` minus its `at`;
a lease that has already expired still counts, and a record with no `updated_at`
or no held lease is never stale. One whose `updated_at` does not parse is
stale, as a malformed lease reads expired.

Errors: `run.absent` (3), `run.active` (4), `crew.unknown` (3),
`crew.exists` (4). A record that does not parse refuses the write with
`store.unreadable` (7); a project without one has no runtime, not a broken one.

## journal

Read only. Written as a side effect of task verbs (ADR 0018).

An event carries `at`, `kind`, `task`, `by` (the role word) and, when the
call named them, `model` and `harness`; plus its `note` when the act has one.

| Verb | Synopsis | Output |
|---|---|---|
| `show` | `journal show [--task <TSK>] [--since <ts>] [--kind <k>…] [--limit <n>]` | events, oldest first; `unreadable` count |

---

## Core `Event` → command

Every `Event` in LC:86-107 maps to exactly one command.

| Event | Command | Note |
|---|---|---|
| `Groom` | `task groom` | content moved to `task describe`, `scope`, `criterion` |
| `Approve` | `task approve` | also sets `planned_by` |
| `Start` | `task start` | |
| `Finish` | `task finish` | |
| `PassReview` | `task pass-review` | |
| `RequestChanges` | `task request-changes` | |
| `ReReview` | `task re-review` | |
| `Escalate` | `task escalate` | |
| `ResumeWork` | `task resume` | **renamed**: `resume_work` → `resume`, as the skill docs already say |
| `Reopen` | `task reopen` | |
| `Block` · `Unblock` · `Defer` · `Undefer` · `Cancel` | `task block` · `unblock` · `defer` · `undefer` · `cancel` | |

No `Event` exists for: create (a `Store` call), claim (new core operation,
ADR 0020). No Event is dropped.

## Documented verbs, renamed or dropped

The flat verbs in `docs/skills/task-authoring.md:222-234` and
`docs/skills/task-review.md:215-223`:

| Skill doc | Here | Change |
|---|---|---|
| `talita create "<t>"` | `task create --title` | noun added; title is a flag |
| `talita list` / `show <id>` | `task list` / `task show` | noun added; JSON output |
| `talita groom <id> --outcome --problem --out-of-scope --criterion` | `task describe` + `scope exclude` + `criterion add` + `task groom` | **split**: one verb, one intent |
| `talita add-step` | `step add` | |
| `talita approve` | `task approve` | |
| `talita ask` / `answer` | `question ask` / `answer` | |
| `talita set-meta --priority --effort --tags` | `task prioritize` / `estimate` / `tag` | **split**: no field setter |
| `talita reopen` / `resume` | `task reopen` / `resume` | now need `--instructed-by` and `--reason` |
| `talita block` … `undefer` | `task block` … `undefer` | |
| `talita review <id> --focus --verdict --note` | `round file --kind external` | `--note` → `--notes`, the field's name |
| `talita finding <id> --severity --description` | `round file --input` or `finding open` | |
| `talita comment <id> <FIND> "<body>"` | `comment add <TSK> <FIND> --body` | |
| `talita pass-review` / `request-changes` / `escalate` | `task pass-review` / `request-changes` / `escalate` | |
