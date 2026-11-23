# Task model

The unit of work in coding-crew. One task, one file, JSON, under the managed
project's own `coding-crew/work/` folder. A task is groomed by a creator
agent with a human, executed by a runner agent, reviewed by a reviewer agent,
and read by a human.

| doc | what it owns |
|---|---|
| [roles/roles.md](../roles/roles.md) | actors, roles, who may write which section, session boundaries |
| [tasks/smaller-tasks.md](smaller-tasks.md) | the work that is not a task yet (`ITM01`) |
| [vocabulary.md](../vocabulary.md) | the names of record |
| [decisions/](../decisions/) | why each choice was made |

The code is `crates/core` (`task.rs`, `lifecycle.rs`, `actor.rs`, `store.rs`);
`crates/core/tests/task_model.rs` carries the gates.

## 1. Design constraints

Every rule below traces to a failure the model has to survive. This is the
checklist the domain must satisfy.

| # | Constraint | Why |
|--:|---|---|
| C1 | **One source of truth per fact.** Nothing derived is stored. | A stored value that can be derived is a value that can disagree with the thing it was derived from |
| C2 | **No index file.** The folder *is* the index. | An index drifts from the records it indexes; the folder cannot |
| C3 | **Every field has a writer.** No dead flags. | A flag nothing writes is a flag the reader must still interpret |
| C4 | **Gates and transitions are one step.** | A transition that can be performed without its guards is a transition with no guards |
| C5 | **Every state is reachable and producible.** | A state nothing produces is a state the reader must still render |
| C6 | **One closed enum per concept.** | Three answers to one question in three places is three answers |
| C7 | **Evidence, not assertion.** | A result that matched nothing is not a result |
| C8 | **Evidence is append-only.** | Overwriting evidence erases the record of what was actually run |
| C9 | **The domain is UI-free.** | The two surfaces share the model; neither shells out to the other |
| C10 | **One writer.** talita writes, raul reads. | Two writers to one file is a merge conflict with no referee |
| C11 | **Unknown values are errors.** | A reader that tolerates what it does not know reports a task that is not there |
| C12 | **Material edits after approval are visible.** | An approved task rewritten in place leaves no trail |

## 2. Anatomy

### 2.1 The sections

```
Task
├── identity      id · title · slug · parent · children · created · updated · kind · promoted_from
├── status        status · status_since · overlays · claim · planned_by
├── meta          priority · effort · tags
├── intent        outcome · problem
├── scope         in_scope[] · out_of_scope[]
├── criteria      [AcceptanceCriterion]
├── steps         [Step]
├── review        max_rounds · rounds[] · verdict · findings[]
├── questions     [Question]
├── notes         [Note]
├── depends_on[]
```

Every section has one owner (see
[roles/roles.md § Ownership by section](../roles/roles.md#ownership-by-section)).
A role may not write another role's section.

### 2.2 The file

```json
{
  "schema_version": 1,
  "id": "TSK01",
  "title": "Add lane filtering to raul",
  "slug": "add-lane-filtering-to-raul",
  "parent": null,
  "children": [],
  "created": "2026-11-13T10:00:00Z",
  "updated": "2026-11-13T12:30:00Z",
  "revision": 4,
  "kind": "task",
  "promoted_from": null,
  "status": "groomed",
  "status_since": "2026-11-13T12:00:00Z",
  "overlays": { "blocked": null, "deferred": null, "cancelled": null },
  "claim": null,
  "planned_by": "s-creator-1",
  "meta": { "priority": "normal", "effort": "M", "tags": [] },
  "intent": { "outcome": "", "problem": "" },
  "scope": { "in_scope": [], "out_of_scope": [] },
  "acceptance_criteria": [
    { "id": "ACCEPT-01", "description": "", "verification": "",
      "status": "pending", "evidence": [], "deferral": null,
      "failure_reason": "" }
  ],
  "steps": [
    { "id": "STP-01", "order": 1, "action": "", "done_when": "", "files": [],
      "status": "pending", "covers": [], "depends_on": [], "deviation": "" }
  ],
  "review": {
    "max_rounds": 5,
    "rounds": [],
    "verdict": null,
    "findings": [],
    "re_reviews": []
  },
  "questions": [],
  "notes": [],
  "depends_on": []
}
```

### 2.3 Fields

| field | type | meaning |
|---|---|---|
| `schema_version` | `u32` | the shape of this file. Currently `1` |
| `id` | string | `TSK01`, child `TSK03.1` |
| `title` | string | what the task is |
| `slug` | string | the title, lowercased and hyphenated. Drives the file name |
| `parent` | id or null | the task this one refines |
| `children[]` | ids | the tasks this one spawned |
| `created`, `updated` | timestamp | RFC 3339. `updated` moves on every write |
| `revision` | `u32` | a write counter, moved on by every write. A caller that read revision `n` and writes with `--expect-revision n` is refused when the file has moved on ([ADR 0016](../decisions/0016-concurrency.md)). `0` on a file written before the field existed |
| `kind` | `task` \| `bug` \| `feature` | `bug`/`feature` is a **small task**: the step rules do not apply to it. See [smaller-tasks.md](smaller-tasks.md) |
| `promoted_from` | id or null | the smaller task this task became |
| `status` | enum | one canonical status. Never a combined string |
| `status_since` | timestamp | RFC 3339. When the task entered the status it is in. Moves on every status transition; it does not move on any other write. A file written before the field existed falls back to `created` — the wait it names is then a **lower bound**, not the wait |
| `overlays` | three slots | orthogonal to the status. Each slot holds `{reason, by, at}` or null |
| `claim` | object or null | `{by, session, at, lease_expires_at}` — who holds the task and in which session |
| `planned_by` | session or null | the creator session that wrote the task. Used by the review-independence gate |
| `meta.priority` | `urgent` \| `high` \| `normal` \| `low` | |
| `meta.effort` | `S` \| `M` \| `L` | |
| `meta.tags[]` | strings | free |
| `intent.outcome` | string | what must be true when the task is done |
| `intent.problem` | string | why it is worth doing |
| `scope.in_scope[]` | strings | the work |
| `scope.out_of_scope[]` | strings | the work that is explicitly **not** in this task. At least one is required to groom |
| `depends_on[]` | ids | tasks that must be `done` before this one starts |
| `questions[]` | `[Question]` | `{id, question, status, answer, raised_by, raised_at, resolved_by, resolved_at}` |
| `notes[]` | `[Note]` | `{id, author, body, at}` — what the execution learned |

### 2.4 Identifiers

| entity | form | scope |
|---|---|---|
| task | `TSK01`, child `TSK03.1` | project |
| smaller task | `ITM01` | project |
| acceptance criterion | `ACCEPT-01` | task |
| step | `STP-01` | task |
| question | `QST-01` | task |
| finding | `FIND-01` | task |
| comment | `CMT-01` | task |
| note | `NOTE-01` | task |
| review round | `n`, 0.. — `0` is the self pre-screen, and the count the budget carries is of **external** rounds, numbered `1..` | task |

Ids are allocated by **scanning what exists**, never from a count of files: the
highest plain `TSK` number in the folder plus one, zero-padded to two digits.
A deleted task must not hand its id to a new one. Child ids (`TSK03.1`) are
not plain numbers and are skipped by the scan.

### 2.5 Mutability

| Section | Mutability | Written by |
|---|---|---|
| `intent`, `scope`, `acceptance_criteria` | **Frozen at approval.** A change ⇒ a child task | creator, pre-approval |
| `acceptance_criteria[].evidence` | Append-only | runner |
| `acceptance_criteria[].status` | Derived from the latest evidence; the stored value mirrors it | runner (mirrors), domain (derives) |
| `steps` | Mutable, always. A different path is recorded, not hidden | runner |
| `claim`, `status`, `status_since` | Written by transitions only | domain |
| `review` | Append-only for rounds and findings; `verdict` moves | reviewer |
| `notes`, `questions` | Append-only | any role |
| `meta` | Mutable, pre-approval | creator |
| `overlays` | Mutable, each with a reason, a `by` and an `at` | any actor |

## 3. The lifecycle

### 3.1 States

Eight, one canonical status at a time. Overlays are orthogonal and are not
states.

| State | Meaning |
|---|---|
| `draft` | Being written. Intent may be incomplete |
| `groomed` | Outcome, criteria and out-of-scope written; open questions still allowed |
| `approved` | **Locked.** Every criterion covered, every step has a `done_when`. The frozen sections are frozen |
| `executing` | Claimed and being implemented. Steps may be revised |
| `code-review` | Work done, criteria proven or failed with a reason, evidence attached, self-review filed |
| `remediating` | External findings open; being fixed |
| `needs-human` | The autonomous review budget is spent and work is still outstanding. A human decides |
| `done` | Reviewed clean. Terminal until reopened |

`done` is the only terminal state, and it is not final: a creator may reopen it.

### 3.2 Transitions

```mermaid
stateDiagram-v2
    direction TB

    [*] --> draft : create

    draft --> groomed : groom<br/>intent.outcome_present · criteria.present · scope.out_of_scope_declared
    groomed --> approved : approve<br/>questions.resolved · criteria.covered · steps.done_when_present
    approved --> executing : start<br/>task.claimed · dependencies.satisfied
    executing --> code-review : finish<br/>steps.settled · evidence.attached · criteria.proven · self_round.filed
    code-review --> done : pass_review<br/>review.verdict_ok · review.no_open_findings · review.within_budget · review.independent
    code-review --> remediating : request_changes<br/>review.verdict_changes_needed · review.findings_exist · review.budget_left · review.independent
    remediating --> code-review : re_review<br/>review.findings_closed · remediation.session_distinct
    code-review --> needs-human : escalate<br/>review.verdict_changes_needed · review.budget_exhausted
    needs-human --> executing : resume_work<br/>human.instructed
    done --> executing : reopen

    note right of remediating
      The review loop.
      Findings open → remediate →
      findings closed → back to code-review.
    end note

    note right of needs-human
      The budget is spent.
      The task does not review
      itself into completion.
    end note
```

Each edge is labelled with the event and the gates that guard it. The gates are
defined once, in [§ 3.3](#33-gates).

| from → to | event | role | must hold |
|---|---|---|---|
| — → `draft` | create | creator | a title |
| `draft` → `groomed` | `groom` | creator | `intent.outcome` set, ≥1 acceptance criterion, ≥1 `out_of_scope` entry |
| `groomed` → `approved` | `approve` | creator | no open questions, every criterion covered by a step, every step has `done_when` — **the step rules do not apply to a small task** (`kind` `bug` or `feature`) |
| `approved` → `executing` | `start` | runner | task claimed, dependencies satisfied |
| `executing` → `code-review` | `finish` | runner | all steps settled, every criterion proven, failed with a reason, or deferred; evidence attached to every criterion not deferred; ≥1 `self` round filed — **a small task has no steps to settle** |
| `code-review` → `done` | `pass_review` | reviewer | verdict `ok`, no open findings, within the external-round budget, reviewer session is not the planning session |
| `code-review` → `remediating` | `request_changes` | reviewer | verdict `changes-needed`, ≥1 finding opened, budget not exhausted, reviewer session is not the planning session |
| `remediating` → `code-review` | `re_review` | runner | all open findings fixed, and the remediation session differs from the execution session |
| `code-review` → `needs-human` | `escalate` | reviewer | 5 external rounds exhausted and still `changes-needed`, reviewer session is not the planning session |
| `needs-human` → `executing` | `resume_work` | creator | a human instructed it |
| `done` → `executing` | `reopen` | creator | — |

**A task never completes itself out of repeated review.** After five autonomous
external rounds with work still outstanding, the loop stops and the task
escalates to `needs-human`. The alternative — a sixth round — is the task
grading its own homework.

**A cancelled task refuses every event, overlays included.** `overlays.cancelled` is terminal
by overlay: there is no un-cancel event, because a cancelled task is a record
of a decision, not a state to be walked back from.

### 3.3 Gates

One table, one definition each. The gates live in `crates/core/src/lifecycle.rs`;
the transitions run them internally, so a transition and its guards cannot be
applied apart.

| Gate | Checks | Applies to |
|---|---|---|
| `intent.outcome_present` | `intent.outcome` is non-empty | `groom` |
| `criteria.present` | ≥1 acceptance criterion | `groom` |
| `scope.out_of_scope_declared` | ≥1 `scope.out_of_scope` entry | `groom` |
| `questions.resolved` | no open question | `approve` |
| `criteria.covered` | every criterion has ≥1 covering step | `approve` |
| `steps.done_when_present` | every step has a non-empty `done_when` | `approve` |
| `task.claimed` | a claim exists and `claim.by` is the acting runner | `start` |
| `dependencies.satisfied` | every `depends_on` task is `done` | `start` |
| `steps.settled` | every step is `done` or `skipped` | `finish` |
| `evidence.attached` | every criterion not deferred carries ≥1 evidence entry | `finish` |
| `criteria.proven` | every criterion is `passed`, `failed` **with a `failure_reason`**, or `deferred` **with a `deferral`** | `finish` |
| `self_round.filed` | ≥1 round of kind `self` | `finish` |
| `review.verdict_ok` | `review.verdict` is `ok` | `pass_review` |
| `review.verdict_changes_needed` | `review.verdict` is `changes-needed` | `request_changes`, `escalate` |
| `review.no_open_findings` | no open finding | `pass_review` |
| `review.findings_exist` | ≥1 open finding | `request_changes` |
| `review.findings_closed` | no open finding | `re_review` |
| `review.within_budget` | external rounds ≤ `max_rounds` | `pass_review` |
| `review.budget_left` | external rounds < `max_rounds` | `request_changes` |
| `review.budget_exhausted` | external rounds ≥ `max_rounds` | `escalate` |
| `review.independent` | the acting session is not `planned_by`; both are recorded | `pass_review`, `request_changes`, `escalate` |
| `remediation.session_distinct` | the acting session is not `claim.session`; both are recorded | `re_review` |
| `overlay.reason_present` | a non-empty reason | `block`, `defer`, `cancel` |

**The budget is asymmetric on purpose.** `pass_review` accepts exactly
`max_rounds`; `request_changes` needs one round left. So the fifth
`changes-needed` round cannot cycle: the only move left is `escalate`.

**Dependencies are checked, not assumed.** `dependencies.satisfied` needs the
store — a check that cannot be performed is a refusal, not a pass.

**Sessions are checked, not assumed.** A session-boundary gate needs both
sessions recorded. A session that is not recorded is a refusal, not a pass.

### 3.4 Overlays

Overlays ride alongside the status and are not edges on the lifecycle graph.
Any actor may set one; a reason is always required.

| Overlay | Fields | Notes |
|---|---|---|
| `blocked` | `reason`, `by`, `at` | Something outside the task's control stops it. A blocked task can be half-executed |
| `deferred` | `reason`, `by`, `at` | Parked on purpose. Re-openable |
| `cancelled` | `reason`, `by`, `at` | Terminal by overlay. Refuses every transition |

### 3.5 Session boundaries

Two rules, both enforced as gates:

| Rule | Gate |
|---|---|
| The session that **planned** a task never **reviews** it | `review.independent` |
| The session that **executed** a task never **remediates** its findings | `remediation.session_distinct` |

The task records the planning session in `planned_by`; the claim records the
execution session in `claim.session`. A reviewer is ideally a different
*model*, not only a different session — the diversity is what surfaces
uncorrelated bugs.

## 4. Review

### 4.1 Rounds

Two kinds, and they are not interchangeable.

| kind | who | counts against the budget |
|---|---|---|
| `self` | the runner, on its own work, before handing it over | no |
| `external` | an independent agent, not the executing session | yes |

A `self` round is a **pre-screen**, not the verdict: it catches the obvious
before a second agent spends a round on it. The runner may file a `self`
round, but only a `reviewer` may move the task out of `code-review` — the
`pass_review` and `request_changes` events are reviewer-only.

Each round carries a `focus`, so successive rounds look at the change through
different lenses instead of repeating the same check.

| field | values |
|---|---|
| `n` | the round number |
| `kind` | `self` \| `external` |
| `focus` | `spec-compliance` \| `edge-cases` \| `security` \| `empirical` \| `design` |
| `reviewer` | who reviewed: the actor id |
| `session` | the session that filed the round, or null on a file written before the field existed ([ADR 0017](../decisions/0017-review-record.md)) |
| `verdict` | `ok` \| `changes-needed` |
| `at` | timestamp |
| `notes` | free text |
| `findings[]` | the finding ids opened in this round |

`review.verdict` is the **latest** round's verdict. It is what the gates read.

### 4.4 Hand-backs

`review.re_reviews[]` is the dated record of each return to review. `task
re-review` appends one entry in the same write as the move, so a `code-review`
task after an external round shows whether it was remediated, and when.

```json
{ "at": "2026-11-11T17:30Z", "after_round": 1, "by": "runner-02", "session": "session-runner-02-remediation-1" }
```

| field | rule |
|---|---|
| `at` | when the task went back to `code-review` |
| `after_round` | the external round whose findings were worked |
| `by`, `session` | the remediating actor and session. The session is the one `remediation.session_distinct` checks: it differs from `claim.session` |

Append-only: one entry per hand-back, oldest first. Each entry's `at` is the
latest move into `code-review`.

### 4.2 Findings

| field | values |
|---|---|
| `id` | `FIND-01` |
| `severity` | `low` \| `medium` \| `high` |
| `status` | `open` \| `fixed` \| `dismissed` |
| `description` | what is wrong |
| `created` | timestamp |
| `resolved` | timestamp |
| `resolution` | what was done, or why it was dismissed |
| `comments[]` | `{id, author, body, at}` — the discussion under the finding |

The reviewer opens findings; the runner resolves them. A finding that is
`fixed` records **what** was fixed; a finding that is `dismissed` records **why**
it is not worth fixing. The outcome is written once — the second writer is
refused, because a dismissal that overwrote a fix would erase the record of
the work that was done.

### 4.3 The round budget

`review.max_rounds` is `5` and counts **external** rounds. Self-review is a
pre-screen and does not consume it.

Five external rounds with work still outstanding and the task is `needs-human`.
The loop is bounded so that a task cannot be approved by the same kind of
agent indefinitely.

## 5. Evidence

Evidence is attached to a criterion and is the only thing that proves it.

```json
{ "command": "cargo test --workspace", "exit_code": 0, "summary": "10 passed; 0 failed",
  "at": "2026-11-10T12:00Z", "by": "runner-01", "session": "session-runner-01", "manual": false }
```

| rule | detail |
|---|---|
| **Attached, never prose** | `{command, exit_code, summary, at, by, session, manual}` |
| **Append-only** | Re-running appends. It never overwrites |
| **Exit code is the truth** | `exit 0` with a non-empty summary is a pass; any other exit code is a failure; no exit code at all is `unproven` |
| **Status is derived** | A criterion's status is the outcome of its **latest** evidence: `passed` \| `failed` \| `unproven`. No evidence ⇒ the stored `status` stands (`pending`) |
| **A failure needs a reason** | `failure_reason` is required when a criterion is `failed`; `criteria.proven` refuses a failure without one |
| **A manual override is evidence** | An agent may override a derived status, and the override is recorded as an evidence entry with `manual: true`. A bypass is **explicit debt**, never a silent one |
| **Evidence is per criterion** | The command proves one thing; it does not prove the next criterion |

`exit 0` with an **empty** summary is a failure, not a pass: a command that ran
and said nothing has proved nothing.

## 6. Steps and criteria

**Acceptance criteria are the outcomes.** What must be true, and how it is
verified.

| field | meaning |
|---|---|
| `id` | `ACCEPT-01` |
| `description` | what must be true |
| `verification` | a command, or a reproducible procedure. **Not prose** |
| `status` | derived: `pending` \| `passed` \| `failed` \| `unproven` \| `deferred` |
| `evidence[]` | append-only |
| `deferral` | `{reason, follow_up}` — required when the criterion is deferred |
| `failure_reason` | why it failed, when it did |

**Steps are the actions that get there.**

| field | meaning |
|---|---|
| `id` | `STP-01` |
| `order` | position |
| `action` | what to do |
| `done_when` | **Observable** — how you will know it worked |
| `files[]` | where. A hint, not a contract |
| `status` | `pending` \| `in-progress` \| `done` \| `skipped` \| `failed` |
| `covers[]` | the acceptance criteria ids this step advances |
| `depends_on[]` | other step ids |
| `deviation` | set when the step was done differently than described, with why |

Steps are **guidance, not a blueprint**. The runner may add, split or drop
steps while executing; what changed goes on the step as `deviation`, where it
stays as part of the record.

The rule of thumb: a criterion is what must be true, a step is one way of
making it true, and `covers[]` is the line between them.

## 7. Scope-down visibility

Partial scope is legitimate. Hidden scope is not.

```json
"deferral": { "reason": "needs an upstream API change", "follow_up": "TSK03" }
```

A deferred criterion does not block completion — `criteria.proven` accepts it —
but it stays visible in raul and on the task. The work that was not done is
recorded, with a reason and a pointer to where it went.

## 8. Notes and questions

**Notes** (`NOTE-nn`) carry what the execution learned — the part that is not a
step and not a finding, but that the next task of the same shape should not
have to rediscover. Any role may write one.

**Questions** (`QST-nn`) carry what is still undecided. An open question blocks
approval (`questions.resolved`); a resolved one keeps its answer.

## 9. Immutability after approval

Once approved, `intent`, `scope` and the acceptance criteria are **frozen**.
The execution approach stays open: steps, evidence, findings, notes and rounds
keep changing.

**A refinement is not a silent edit.** It becomes a child task — `TSK03.1` —
with its own brief, criteria, evidence and review. The parent keeps the record
of the intent it was approved against, and the child carries the change.

### 9.1 Why there is no content fingerprint

An obvious alternative is to hash the frozen sections at approval and reject
any later change. We deliberately do not.

| | fingerprint | child task |
|---|---|---|
| What it catches | a silent edit to a frozen section | nothing on its own — it *prevents* the edit by giving it another home |
| What it costs | a digest to compute, store, compare and invalidate; a hash that must be re-derived by every reader, and a plan that a rename or a field reordering can invalidate | a child id in the parent, and a task file |
| What it can tell a reader | "this file was tampered with" | "this is what the task became" — the *refinement*, not the suspicion |
| Failure mode | a false accusation: a task nobody edited is refused because the digest moved | an extra task, which is visible and cheap |

The trail the fingerprint was meant to preserve is carried by the **child-task
path** instead: the parent's frozen sections stay frozen, the change is a
first-class artifact with its own lifecycle, and the link between them is a
pair of ids a human can read. A suspicion is not a trail.

The cost of the choice is stated plainly: an out-of-band edit to a frozen
section is **not detected by the tool**. It is caught by review, and the
child-task path is the sanctioned way to make the change.

## 10. Subtasks

`TSK03.1` is a **full task**: its own status, criteria, steps, evidence,
review and findings. It is not a checklist item on the parent.

| rule | |
|---|---|
| The parent lists the child in `children[]` | |
| The child names the parent in `parent` | |
| Progress is **derived** from the children's statuses | never stored, never aggregated automatically |
| The parent is not blocked by an open child | a refinement is not a debt of the original |

## 11. Storage

```
<project>/coding-crew/work/
  TSK01-<slug>.json     # a task
  ITM01-<slug>.json     # a smaller task — see smaller-tasks.md
```

One folder holds every work unit. The id prefix says which is which
([ADR 0002](../decisions/0002-work-folder.md)). No central store: the data
lives in the project it describes, next to the code it is about.

| rule | |
|---|---|
| **One file per artifact** | one task per file |
| **The folder is the index** | there is no summary file and no manifest. Listing is reading the directory |
| **JSON, one object per file** | pretty-printed, one trailing newline |
| **Atomic writes** | the writer writes `<file>.json.tmp`, syncs it, and renames it over the target. A crash leaves the old file intact |
| **The file name is `<id>-<slug>.json`** | a rename writes a new file; the writer removes the old one, so one id never has two files |
| **Ids come from a scan** | the highest `TSK` number present plus one. Never `len()+1` |
| **The reader is tolerant of a broken file** | a file that does not parse is skipped, not fatal — one corrupt file does not hide the folder |

The store is **discovered**, not configured: from any path, walk up until a
`coding-crew` folder appears.

### 11.1 The journal (provisional)

```
<project>/coding-crew/
  activity.json         # the journal
  work/
    TSK01-<slug>.json   # a task
```

The journal sits beside `work/`, not inside it: the work folder holds one file
per work unit, and a journal in there would be read as a task. It is optional —
a project without one has no journal, not a broken one.

The file is one object with an `events` array, oldest first.

| field | rule |
|---|---|
| `at` | when it happened, RFC 3339 |
| `kind` | one of the kinds below |
| `task` | the task id it happened to — `TSK01` |
| `by` | who did it: the actor's role or kind — `creator`, `runner`, `reviewer`, `human`, `orchestrator` |
| `note` | optional, one line of free text |

The kinds are settled by [ADR 0018](../decisions/0018-runtime-and-journal-writers.md).
Nothing writes the journal yet; talita will, as a side effect of each verb.

| kind | what happened |
|---|---|
| `created` | a task was created |
| `groomed` | a creator wrote the task's intent, scope and criteria |
| `approved` | a task left the human lane and became claimable |
| `claimed` | an agent took a task |
| `released` | a claim was released, or the task cancelled while claimed |
| `started` | the runner began executing the claimed task |
| `round_filed` | a review round was filed, self or external |
| `review_requested` | a runner sent the work to review |
| `finding_raised` | a reviewer raised a finding |
| `finding_resolved` | a runner fixed or dismissed a finding |
| `changes_requested` | a reviewer sent the work back with `changes-needed` |
| `remediated` | a runner answered a finding and sent the work back |
| `escalated` | the work needs a human again |
| `completed` | the task is done |
| `resumed` | a `needs-human` task went back to work, instructed by a human |
| `reopened` | a `done` task went back to work |
| `blocked` · `unblocked` | the blocked overlay was set or cleared |
| `deferred` · `undeferred` | the deferred overlay was set or cleared |
| `cancelled` | the cancelled overlay was set |
| `commented` | someone left a note on the task |

Each event names a task and covers a step the lifecycle already has; the kinds
add no state.

### 11.2 The runtime record (provisional)

```
<project>/coding-crew/
  activity.json         # the journal
  runtime.json          # the flow's runtime
  work/
    TSK01-<slug>.json   # a task
```

The runtime record sits beside the journal, outside `work/`, for the journal's
reason. It is **provisional**: nothing writes it yet, and the flow's writer
settles the shape when it lands. Its types live in
`crates/core/src/runtime.rs`, so the writer and the reader share them.

The file is one object.

| field | rule |
|---|---|
| `started_at` | when this run began, RFC 3339 |
| `crew` | one entry per agent of the flow |
| `queue` | the run's tasks in queue order: an `id`, a `status` and a `claimable` each, never a task's title, note or intent. A held task and a done one are in it; the pane draws the waiting ones, and offers the waiting ones that are `claimable` |
| `round` | the task in flight: the `task`, the `stage` (`self` or `external`), the round's `number`, and `earlier`, the rounds before it on the same task, oldest first |
| `earlier` | each finished round: its `number` and the `raised` findings its review raised, open or since resolved |
| `budget` | the rounds the task in flight has used, as `spent` (the round in flight counts), of its round limit `total` |
| `findings` | the task in flight's open findings, as a count |
| `severities` | the task in flight's open findings by `high`, `medium` and `low`, when the writer holds them; they sum to `findings` |
| `stamp` | whether the automation `runs`, and `why` when it does not |

The flow is sequential: while a task is in flight the builder and the reviewer
both hold it. The builder is `idle` between its rounds, and the reviewer is
`reviewing` while its review runs.

A crew entry:

| field | rule |
|---|---|
| `role` | `orchestrator`, `builder` or `reviewer`; `runner` reads as `builder` |
| `model` | the model the agent runs |
| `actor` | the actor id it acts as in task files, when known. A task verb finds the caller's row by it. hugo does not read it |
| `holds` | the task it holds, or none |
| `since` | when it took it, RFC 3339 |
| `state` | `idle`, `executing`, `reviewing` or `blocked`. A blocked agent holds its task and cannot go on; it is not working |

The crew's size is a **composition**, not a number: the crew is the roles
present. Three agents are one per role and the reviewer reviews; an
orchestrator and a builder, so the orchestrator reviews; the orchestrator
alone, which builds and reviews, so the orchestrator reviews. A record with no
orchestrator has the human reviewing; the configuration cannot produce that
crew, since the orchestrator is always present. The rule is
`runtime::composition`, derived from the roles, and the pane paints it in
words. A crew with more than one agent in a
role has no rule that says who reviews; it is named as what it is, never as
one per role. A held task belongs to the crew, not the queue.

The queue is the run's, not the unclaimed work's alone: a writer lists the
tasks in the order the run takes them, so the pane can show what waits and how
much is done. A mark in hugo lasts the session only; nothing about it reaches the record.

The record is optional like the journal: a project without one has no runtime,
not a broken one.

### 11.3 The configuration (provisional)

```
~/.config/coding-crew/
  config.toml           # the flow's configuration
  config.json           # the old shape: not read any more
```

The configuration sits on the tool's side, under `~/.config/coding-crew/`,
because it is policy rather than work. It is **provisional**: the flow has not
written one yet, and the format may change when it does. Its types live in
`crates/core/src/config.rs`, so the writer and the reader share them.

The format is **TOML**, because the configuration is the one file a human
edits, and a comment is how a configuration explains itself. The defaults hugo
writes carry a short note beside each part. The records — the journal and the
runtime record — are machine-written and stay JSON.

The file is one TOML document. When it is absent, hugo or raul writes the
defaults into the home it creates on demand. A configuration that parses is used as it is, and a file that reads but holds what the flow does not run is named with a warning (below). A
configuration that does not parse is named with its error, and the defaults are
shown beside it, so the human sees both what is wrong and what the tool would
use; the file is never replaced. The old `config.json` is not read: when one is
found beside the configuration, hugo says so and converts nothing.

| field | rule |
|---|---|
| `agents` | the crew: one table per role, each with its harness, model and effort |
| `round_limit` | the build-and-fix cycles a task gets: five |
| `order` | the flow's steps, in the order they run |
| `[raul]` | raul's own settings: the palette and the icons |
| `[raul.tasks]` | the order the Tasks tab opens in |

The TOML shape:

```toml
# The build-and-fix cycles a task gets.
round_limit = 5

# The crew: one table per role. The orchestrator is always present; a builder
# and a reviewer are optional. Each names its harness, model and effort.
[agents.orchestrator]
harness = "claude"
model = "claude-opus-5-5"
effort = "medium"

[agents.builder]
harness = "claude"
model = "claude-haiku-5-5"
effort = "high"

[agents.reviewer]
harness = "claude"
model = "claude-opus-5-5"
effort = "medium"

# The order the flow follows: one step per entry, in the order they run.
[[order]]
step = "groom"
role = "creator"
gates = [{ from = "groomed", to = "approved" }]

# raul's own settings: theme is dark, nord, catppuccin, dracula or
# terminal; icons is nerd or unicode. raul reads them at startup.
[raul]
theme = "dark"
icons = "nerd"

# The order the Tasks tab opens in: the key it sorts by, and whether it
# runs backwards.
[raul.tasks]
sort = "id"
reverse = false
```

An agent's table is named by its role. The table holds no `name` and no
`role` key:

| table | rule |
|---|---|
| `[agents.orchestrator]` | always present: the orchestrator runs the loop |
| `[agents.builder]` | writes the code; optional |
| `[agents.reviewer]` | reviews the code; optional |

The crew is the roles present. Three tables are one per role, and the reviewer
reviews. Two are the orchestrator and the builder, and the orchestrator
reviews. One is the orchestrator alone, which builds and reviews. Two tables
of one role are unreadable: the file holds one table per role.

A file that reads but holds what the flow does not run is **used as it is and
named**: a warning on hugo's sidebar and on raul's Settings, one line each, on
the warning slot. Two cases:

- a table `[agents.<name>]` whose name is no role of the crew is not read, and
  the warning names the table;
- a crew of roles the flow does not run (the orchestrator and the reviewer, with
  no builder) loads, and the warning names the crew.

Nothing is dropped silently. A file with no orchestrator, or a harness the table
does not hold, is still unreadable.

Each agent table holds:

| field | rule |
|---|---|
| `harness` | the harness it runs in: `claude` · `opencode` · `cursor-agent`, the names of the harness table in `crates/core/src/harness.rs` |
| `model` | the model it runs, as its harness names it — `claude-haiku-5-5`, `opencode-go/glm-5.3` |
| `effort` | the effort it runs at, one of its model's levels; absent, it is not yet picked and hugo will not start until it is, unless its model takes none. For a model its harness does not list, it may be any level the harness knows, a typed word, or `""` — none: started with no effort flag |

The harness table holds, for each harness, its program, the agent kind herdr
starts it as, how its models are listed, and how a model and an effort become
the arguments that start it. The levels belong to the model, not the harness,
and come from the same listing: `claude` holds a curated table, with five
levels for Opus, Sonnet and Fable and three for Haiku; `opencode` reads
`opencode api --standalone model.list`, whose variants are the levels;
`cursor-agent` reads `cursor-agent --list-models`, where a model's level is the
suffix of its id. A model that lists no level takes no effort, and so does a
base the listing also names bare (`gpt-5.3-codex`). Nothing else is a harness.

A file written before the crew was one table per role has an `[[agents]]`
array, each entry naming its `role`; `runner` is the builder. It still loads.
An older file has one `[harness]` table for all of them and a model that
carries its effort as `#effort`: an agent with no `harness` takes the old
table's `tool`, and the `#effort` is split off into `effort`. Nothing is
rewritten until a save. A save that changes an agent writes the crew as tables,
and keeps the rest of the file as it was.

An order step:

| field | rule |
|---|---|
| `step` | the step's name — `groom`, `build` |
| `role` | the role that does it: `creator`, `builder` or `reviewer` |
| `gates` | the transitions it passes: each a `from` and a `to` status |

raul's table:

| table | field | rule |
|---|---|---|
| `[raul]` | `theme` | the palette: `dark` · `nord` · `catppuccin` · `dracula` · `terminal`; default `dark` |
| `[raul]` | `icons` | the icon set: `nerd` · `unicode`; default `nerd` |
| `[raul.tasks]` | `sort` | the key: `id` · `created` · `updated` · `status` · `priority` · `effort`; default `id` |
| `[raul.tasks]` | `reverse` | `true` · `false`; default `false` |

An absent key, or an absent table, means its default, so a file written before
`[raul]` loads unchanged. A save writes only the values that changed: a file
without `[raul]` is not rewritten to add it. For `theme` and `icons` the order
is a flag, then the environment (`CODING_CREW_ICONS`, icons only), then the
file, then the default.

The defaults are the project's own flow: the builder on `claude-haiku-5-5` at
`high`, the reviewer and the orchestrator on `claude-opus-5-5` at `medium`, all
in the `claude` harness; five build-and-fix cycles; and the order
[roles.md](../roles/roles.md#the-order) records. A test reads that table, and
the two role configs name their agents only: the models are the defaults'
alone, pinned by their own test.

**hugo's picks are the session's.** hugo picks each agent's harness, model and
effort in its sidebar, and its crew-size selector picks which roles the crew
runs. The picks last the session and are never written to this file: the file's
values are the defaults, and raul's Settings edits them. The models offered are
the ones the agent's harness lists, so no pick can name a model that does not
exist. See [hugo's design record](../hugo/design-record.md#ruling-2026-11-22-hugos-picks-are-session-only).

## 12. Derived, never stored

| view | derived from |
|---|---|
| criterion status | the criterion's latest evidence |
| review state | `pending-review` · `open-findings` · `clean` — from the rounds and the findings |
| lanes | the human lane and the agent lane — from the status, the overlays and the claims (see [vocabulary.md](../vocabulary.md#the-lanes-are-derived)) |
| board views | the lanes, grouped and sorted |
| children progress | the children's own statuses |
| readiness to claim | would `start` be accepted right now, and if not, which gate refuses it |
| who reviews | the crew's roles — § 11.2: a reviewer, else the orchestrator, else the human |

A stored lane is a lane that can disagree with the status it was derived from.
These are views, and they are recomputed on read.

## 13. Worked examples

`mock-data/coding-crew/work/` holds one task per lifecycle stage, plus the
edge cases. Every fixture satisfies the gates of the phase it is in.

The table names the **role** of a fixture, not its id: the generator renumbers
when it changes, so an id in a doc is a fact that goes stale.

| fixture | what it shows |
|---|---|
| a draft task | title only |
| a blocked task | a `blocked` overlay with a reason, a `by` and an `at` |
| an approved task | criteria covered by steps, every step with a `done_when` |
| an approved task whose dependencies are done | **claimable** — the one thing an agent could take right now |
| an executing task | claimed with a lease, evidence attached, a `deviation` on the step, a note |
| a task with a deferred criterion | `status` `deferred`, a `deferral` with a `follow_up` |
| a task in code review | a `self` round and one open finding |
| a task in remediation | a `self` round (`ok`) then an `external` round (`changes-needed`) |
| a task needing a human | five external rounds spent, work still outstanding |
| a done task | verdict `ok`, no open findings |
| a child task | `parent` set, still `draft` |

The fixture graph has no cycles and every `depends_on` points at a task that
exists. Edges may point forwards or backwards in id order: the claim gate is
what makes a wait real. `dependencies.satisfied` requires every dependency to
be `done` before an `approved` task can start, so a task past `approved` waits
only on done work, and the `done` set is closed under its own waits-on edges.
An `approved` task may still wait on unfinished work — that is what holds it.
The approved task whose dependencies are done can be claimed.

`crates/core/tests/mock_data.rs` loads the set and checks that it covers every
phase and keeps every gate.

## 13a. Where the screens are

The TUI's screens are specified in [tui/screens.md](../tui/screens.md): the
window, the five tabs, the detail pane, the derived views. The state of the
work is in `TODO.md`.

## 14. What this doc is not

| | |
|---|---|
| **Not talita's command surface** | the verbs, their flags and their output are not defined yet |
| **Not raul's screens** | the tabs, the columns and the key bindings are not defined yet |
| **Not the smaller-task model** | that is [smaller-tasks.md](smaller-tasks.md) |
| **Not the role model** | that is [roles/roles.md](../roles/roles.md) |
| **Not a settled project config** | the configuration is provisional — § 11.3 |
| **Not autopilot** | the orchestrator role exists; what it drives does not |

## 15. Known gaps

**Not defined yet:**

- talita's operation surface
- autopilot
- the runtime record's writer — the record is provisional (§ 11.2)
- the configuration's writer — the configuration is provisional (§ 11.3)

**Specified here, now in `crates/core`:**

| what | where |
|---|---|
| `kind` and `promoted_from` on the task | `crates/core/src/task.rs` — `TaskKind`, `Task::is_small_task` |
| the small-task step exemption | `crates/core/src/lifecycle.rs` — `criteria.covered`, `steps.done_when_present` and `steps.settled` are skipped for a small task |
| the claim lease | `Claim::lease_expired` compares RFC 3339 instants; an empty lease never expires, a valid lease expires at or after its instant, and a malformed nonempty lease is treated as expired. The `approved → executing` gate refuses an expired lease |
| the rename cleanup | `Store::save` removes the file the task had under an older name |
| the work folder path | the store reads `coding-crew/work/`, as [ADR 0002](../../decisions/0002-work-folder.md) decides; [ADR 0010](../../decisions/0010-project-boundary.md) stops discovery at the nearest project's `coding-crew/` folder |
| smaller tasks (`ITM*`) | `crates/core/src/item.rs` — shaping, dismissal, promotion; `Store::promote_item` writes the item first |
| the schema additions: `revision`, `review.re_reviews[]`, `Round.session`, `Evidence.manual`, crew `actor`, the journal kinds | `crates/core/src/task.rs`, `runtime.rs`, `activity.rs`. Optional on read; the fixtures carry them; no verb writes them yet |
