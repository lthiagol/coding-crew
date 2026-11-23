# talita

talita is coding-crew's CLI. Agents call it; humans read its results in raul
and hugo. It is the **only writer** of a project's `coding-crew/` folder: the
task files, the smaller tasks, the journal and the runtime record.

Status: **accepted design; nothing is built**. The full reference is
[commands.md](commands.md); the decisions are ADRs 0012–0020; the human's rulings
are in [decisions.md](decisions.md); the build order is [roadmap.md](roadmap.md).

## Principles

| Principle | Rule | ADR |
|---|---|---|
| One writer | Every file raul or hugo reads has exactly one writer: talita. No verb, no file: a field nothing writes is a gap, not a hand-edit | [0007](../decisions/0007-one-writer.md) |
| Noun, then verb | `talita <noun> <verb> <ids…> [flags]`. The noun is the part the call writes | [0012](../decisions/0012-noun-verb-grammar.md) |
| Intent verbs only | A verb names a domain act (`approve`, `resolve`, `claim`), never a field. There is no `set`, `edit` or raw-JSON escape hatch | [0013](../decisions/0013-intent-verbs.md) |
| Rules enforced, not trusted | Every write runs the lifecycle gates, section ownership, the frozen sections and the session boundaries. A refusal names the rule and the remedy | [0013](../decisions/0013-intent-verbs.md) |
| Agent-first output | JSON on stdout, one result shape, stable error codes, a fixed exit-code table | [0014](../decisions/0014-output-and-errors.md) |
| Identity is part of acting | Every write carries an actor, a role and a session. A write without them is refused | [0015](../decisions/0015-identity.md) |
| Side effects, not second calls | A verb that changes a task appends its journal event and refreshes the runtime record in the same call | [0018](../decisions/0018-runtime-and-journal-writers.md) |

## Nouns

| Noun | Is | Writes | Read by |
|---|---|---|---|
| `project` | the `coding-crew/` folder | the folder, `work/` | — |
| `task` | a task and its lifecycle | `work/TSK*.json` | raul |
| `scope` | a task's in- and out-of-scope lists | task | raul |
| `criterion` | an acceptance criterion | task | raul |
| `step` | a step | task | raul |
| `question` | an open decision on a task | task | raul |
| `note` | a learning on a task | task | raul |
| `evidence` | proof attached to a criterion | task | raul |
| `claim` | a runner's hold on a task, with a lease | task | raul, hugo (via `claimable`) |
| `round` | a review round, self or external | task | raul, hugo |
| `finding` | a defect a round raised | task | raul, hugo (counts) |
| `comment` | discussion under a finding | task | raul |
| `item` | a smaller task (`ITM*`) | `work/ITM*.json` | raul |
| `session` | an actor's working session | nothing (mints an id) | — |
| `run` | the flow's run | `runtime.json` | hugo |
| `crew` | the agents of the run | `runtime.json` | hugo |
| `queue` | the run's tasks, in order | `runtime.json` | hugo |
| `journal` | what happened, oldest first | read only — written as a side effect | raul |

Nouns are singular. Ids are positional, parent first: `talita finding resolve
TSK01 FIND-02`.

## Command map

Three areas, one graph each. Solid arrows: the file a verb writes. Dotted: a
side effect. The last column is who reads the file.

### The work

```mermaid
flowchart LR
    subgraph verbs["talita: the work"]
        project["project<br/>init · show · check"]
        task["task<br/>create · show · list · retitle · describe<br/>prioritize · estimate · tag · depend · undepend<br/>groom · approve · start · finish<br/>resume · reopen<br/>block · unblock · defer · undefer · cancel"]
        parts["scope · include · exclude · drop<br/>criterion · add · revise · remove · defer · undefer · explain<br/>step · add · revise · remove · begin · done · skip · fail<br/>question · ask · answer<br/>note · add<br/>evidence · run · override<br/>claim · take · renew · release"]
        item["item<br/>create · show · list · shape · dismiss · promote"]
    end
    TF[("work/TSK*.json")]
    IF[("work/ITM*.json")]
    J[("activity.json")]
    R[("runtime.json")]
    raul(["raul"])
    hugo(["hugo"])
    project --> TF
    task --> TF
    parts --> TF
    item --> IF
    item -- promote --> TF
    task -. lifecycle event .-> J
    parts -. "claim, note" .-> J
    task -. "status, claimable" .-> R
    parts -. "claim to claimable" .-> R
    TF --> raul
    IF --> raul
    J --> raul
    R --> hugo
```

### The review

```mermaid
flowchart LR
    subgraph verbs["talita: the review"]
        round["round<br/>file · show"]
        finding["finding<br/>open · resolve · dismiss"]
        comment["comment<br/>add"]
        verdict["task<br/>pass-review · request-changes<br/>re-review · escalate"]
    end
    TF[("work/TSK*.json<br/>review.rounds · review.findings")]
    J[("activity.json")]
    R[("runtime.json<br/>round · budget · findings · severities")]
    raul(["raul"])
    hugo(["hugo"])
    round --> TF
    finding --> TF
    comment --> TF
    verdict --> TF
    round -. "round_filed, finding_raised" .-> J
    finding -. "finding_raised, finding_resolved" .-> J
    comment -. commented .-> J
    verdict -. "completed, changes_requested, remediated, escalated" .-> J
    round -. refresh .-> R
    finding -. refresh .-> R
    verdict -. refresh .-> R
    TF --> raul
    J --> raul
    R --> hugo
```

### The flow

```mermaid
flowchart LR
    subgraph verbs["talita: the flow"]
        session["session<br/>new · show"]
        run["run<br/>start · stop · show · refresh"]
        crew["crew<br/>join · leave · assign · idle · stall"]
        queue["queue<br/>add · drop · show"]
        journal["journal<br/>show"]
    end
    R[("runtime.json")]
    J[("activity.json")]
    TF[("work/TSK*.json")]
    hugo(["hugo"])
    raul(["raul"])
    agent(["the calling agent"])
    session -- "prints an id" --> agent
    run --> R
    crew --> R
    queue --> R
    TF -- "derived fields" --> R
    J -- read --> journal
    R --> hugo
    J --> raul
```

## The task status machine

Each edge: the talita command, then the role allowed. The gates are task-model
§ 3.3's; the commands are in [commands.md](commands.md#task).

```mermaid
stateDiagram-v2
    direction TB

    state "code-review" as code_review
    state "needs-human" as needs_human

    [*] --> draft : task create (creator)
    draft --> groomed : task groom (creator)
    groomed --> approved : task approve (creator)
    approved --> executing : task start (runner, holds the claim)
    executing --> code_review : task finish (runner)
    code_review --> done : task pass-review (reviewer)
    code_review --> remediating : task request-changes (reviewer)
    remediating --> code_review : task re-review (runner, new session)
    code_review --> needs_human : task escalate (reviewer)
    needs_human --> executing : task resume (creator, human-instructed)
    done --> executing : task reopen (creator, human-instructed)

    note right of approved
        claim take (runner) precedes start.
        It changes no status.
    end note

    note right of remediating
        finding resolve or dismiss (runner),
        then re-review.
    end note
```

Overlays ride beside the status. Any actor may set or clear them, humans
included; a reason is required to set. `cancelled` is terminal and refuses
every later command on the task.

```mermaid
stateDiagram-v2
    direction LR

    state "no overlay" as clear
    state "blocked" as blocked
    state "deferred" as deferred
    state "cancelled" as cancelled

    [*] --> clear
    clear --> blocked : task block with a reason (any actor)
    blocked --> clear : task unblock (any actor)
    clear --> deferred : task defer with a reason (any actor)
    deferred --> clear : task undefer (any actor)
    clear --> cancelled : task cancel with a reason (any actor)
    blocked --> cancelled : task cancel with a reason (any actor)
    deferred --> cancelled : task cancel with a reason (any actor)
    cancelled --> [*]

    note right of blocked
        blocked and deferred are independent.
        A task may carry both.
    end note
```

## A full unit

One task, `TSK04`, already `approved`. The orchestrator runs the flow; a fresh
builder session builds and another remediates; one reviewer session reviews
both rounds. Every talita call, in order. Each task-changing call appends its
journal event and refreshes `runtime.json` as part of the same call.

```mermaid
sequenceDiagram
    autonumber
    participant O as orchestrator
    participant B as builder
    participant V as reviewer
    participant T as talita

    Note over O,T: run setup
    O->>T: talita session new --as orch-1 --role orchestrator
    O->>T: talita run start --task TSK04
    O->>T: talita crew join builder-1 --role builder --model M1
    O->>T: talita crew join reviewer-1 --role reviewer --model M2
    O->>T: talita session new --as builder-1 --role runner
    Note over O,B: orchestrator starts the builder with TALITA_SESSION set

    Note over B,T: build
    B->>T: talita task show TSK04
    B->>T: talita claim take TSK04 --lease 2h
    Note right of T: journal claimed, crew builder-1 holds TSK04, executing
    B->>T: talita task start TSK04
    B->>T: talita step begin TSK04 STP-01
    B->>T: talita step done TSK04 STP-01
    B->>T: talita step skip TSK04 STP-02 --deviation "..."
    B->>T: talita evidence run TSK04 ACCEPT-01
    B->>T: talita evidence run TSK04 ACCEPT-02
    B->>T: talita note add TSK04 --body "..."
    B->>T: talita round file TSK04 --kind self --focus spec-compliance --verdict ok
    B->>T: talita task finish TSK04
    Note right of T: journal review_requested, builder-1 idle

    Note over O,V: external round 1
    O->>T: talita crew assign reviewer-1 TSK04
    Note right of T: reviewer-1 reviewing, round external 1 in flight
    V->>T: talita task show TSK04
    V->>T: talita round file TSK04 --input round.json
    Note right of T: round 1 changes-needed, FIND-01 FIND-02 opened, journal round_filed and finding_raised x2
    V->>T: talita task request-changes TSK04
    Note right of T: journal changes_requested, budget 1 of 5

    Note over O,B: remediation, new builder session
    O->>T: talita session new --as builder-1 --role runner
    O->>T: talita crew assign builder-1 TSK04
    B->>T: talita finding resolve TSK04 FIND-01 --resolution "..."
    B->>T: talita finding dismiss TSK04 FIND-02 --reason "..."
    B->>T: talita evidence run TSK04 ACCEPT-01
    B->>T: talita task re-review TSK04
    Note right of T: gate remediation.session_distinct, journal remediated

    Note over O,V: external round 2
    O->>T: talita crew assign reviewer-1 TSK04
    V->>T: talita round file TSK04 --kind external --focus edge-cases --verdict ok
    V->>T: talita task pass-review TSK04
    Note right of T: journal completed, holds cleared, queue TSK04 done

    O->>T: talita run stop --why "queue done"
```

## Model changes this design asks for

The design stays inside the task model except where listed. Each is an
additive, optional field (raul ignores unknown fields) or a new journal kind.

| Change | Where | Why | ADR |
|---|---|---|---|
| `review.re_reviews[]` | task file | `task re-review` leaves a dated record of each hand-back; a `code-review` task after a round could not show it (TSK14) | [0017](../decisions/0017-review-record.md) |
| `Round.session` | task file | `review.independent` needs the filing session; `reviewer` stays the actor id | [0017](../decisions/0017-review-record.md) |
| `Evidence.manual` | task file | ADR 0006's "marked as a manual override" has no field | [0019](../decisions/0019-evidence.md) |
| `Task.revision` | task file | optimistic check for content verbs | [0016](../decisions/0016-concurrency.md) |
| `crew[].actor` | runtime record | match a crew row to the actor a verb acts as | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| new journal kinds | journal | every lifecycle event and review act gets one | [0018](../decisions/0018-runtime-and-journal-writers.md) |
| gate `review.round_current` | core lifecycle | a reopened task could pass review on a stale `ok` | [0017](../decisions/0017-review-record.md) |
| gate `review.round_owner` | core lifecycle | the session that filed the verdict makes the move | [0017](../decisions/0017-review-record.md) |
| all failing gates reported | core lifecycle | one round trip per fix | [0014](../decisions/0014-output-and-errors.md) |
| `planned_by` set at `approve` | talita | nothing sets it today; `review.independent` refuses without it | [0015](../decisions/0015-identity.md) |
| gate `task.claimed` on `finish` | core lifecycle | only the claim holder hands the work to review | [0020](../decisions/0020-claims-and-leases.md) |
| claim as an operation | core | no `Event` exists for claiming | [0020](../decisions/0020-claims-and-leases.md) |
