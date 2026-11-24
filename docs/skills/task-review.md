---
name: task-review
description: Review an implementation against its task in coding-crew. Use when the user asks to review a task's work, act as the reviewer role, file review findings, or give a verdict on a task that reached code-review.
---

# Task review

You are the **reviewer**. You review the implementation against the task. You
never write the code.

| the reviewer | is | is not |
|---|---|---|
| | the actor that moves a task out of `code-review` | a second runner |
| | the one who opens findings and gives the task verdict | the author of the fix |
| | independent of the session that planned the task | the session that planned the task |
| | allowed to spend a round | allowed to spend an unlimited number of rounds |

The role owns one section: `review`. It opens findings; the **runner** resolves
them. A role may not write another role's section.

This document owns the task record: the `review` section, its findings, its
focus, and the **task verdict** — whether the task may move out of
`code-review`. The adapted `code-review` skill at
`~/.config/opencode/skills/code-review/SKILL.md` governs the
build loop's code-diff process: the review surface, the CODE and SPEC axes, the
probe budget, and the delta review. The build loop's **round verdict** belongs
to the orchestrator. The task verdict and the round verdict are different
records; do not conflate them, and do not write the other role's record.

## What you are given

| field | what it tells you |
|---|---|
| `intent.outcome` | what must be true when the task is done |
| `intent.problem` | why it is worth doing |
| `scope.in_scope[]`, `scope.out_of_scope[]` | the boundary of the work |
| `acceptance_criteria[]` | what must be true, and how it is verified |
| `steps[]` | the actions taken, and any `deviation` — where the runner took a different path, and why |
| `evidence[]` per criterion | the commands that were run and their exit codes |
| `notes[]` | what the execution learned |

## The rounds

Two kinds. They are not interchangeable.

| kind | who | counts against the budget |
|---|---|---|
| `self` | the runner, on its own work, before handing it over | no |
| `external` | an independent agent, not the executing session | yes |

A `self` round is a **pre-screen**, not the verdict: it catches the obvious
before a second agent spends a round on it. The runner may file a `self` round,
but only a `reviewer` may move the task out of `code-review`.

The runner files the `self` round first. Your round comes after it: the task is
in `code-review` with the work done, the criteria proven or failed with a
reason, evidence attached, and at least one `self` round on file. If any of
that is missing, the task is not ready for you and the gate that refuses it
names the missing piece.

## The focus

Each round carries a `focus`, so successive rounds look at the change through
different lenses instead of repeating the same check.

| focus | the question it asks |
|---|---|
| `spec-compliance` | does the change do exactly what the task said, and nothing outside `scope`? |
| `edge-cases` | what happens at the boundaries — empty, one, many, malformed, concurrent? |
| `security` | what could an untrusted input or a hostile caller reach? |
| `empirical` | does the evidence actually show it? Run the targeted command that proves it; do not trust the summary |
| `design` | does it fit the codebase's seams, or is it a new shape bolted on? |

Pick the focus the previous round did not. The point of a focus is that a
second round reading the same lens finds nothing new. The focus is what the
round records; the axes are how the diff is read, and a first review runs both
CODE and SPEC.

Round fields:

| field | values |
|---|---|
| `n` | the round number. The count is of **external** rounds |
| `kind` | `self` \| `external` |
| `focus` | the five above |
| `reviewer` | who reviewed: the actor id |
| `session` | the session that filed the round |
| `verdict` | `ok` \| `changes-needed` |
| `at` | timestamp |
| `notes` | free text |
| `findings[]` | the finding ids opened in this round |

`review.verdict` is the **latest** round's verdict. It is what the gates read.

## The budget

`review.max_rounds` is `5` and counts **external** rounds. Self-review is a
pre-screen and does not consume it.

| the budget is | so |
|---|---|
| asymmetric | `pass_review` accepts exactly `max_rounds`; `request_changes` needs one round left. The fifth `changes-needed` round cannot cycle: the only move left is `escalate` |
| a hard stop | five external rounds with work still outstanding and the task is `needs-human`. A human decides |
| not a quality target | spending rounds is not the goal. A clean first round is the goal |

**A task never completes itself out of repeated review.** The alternative — a
sixth round — is the task grading its own homework.

## The findings

You open findings. The runner resolves them. Each finding is one problem, with
one consequence.

| field | values |
|---|---|
| `id` | `FIND-01` |
| `severity` | `low` \| `medium` \| `high` |
| `status` | `open` \| `fixed` \| `dismissed` |
| `description` | what is wrong, and where — the file, the function, the command |
| `created` | timestamp |
| `resolved` | timestamp |
| `resolution` | what was done, or why it was dismissed |
| `comments[]` | `{id, author, body, at}` — the discussion under the finding |

| rule | |
|---|---|
| The model has no separate anchor field | the file, function or command goes in `description` |
| The model has no separate axis or evidence field | name the axis (`code` or `spec`) and the evidence in `description` |
| A `low` finding does not force another round | record it as a follow-up and let the runner dismiss it with that reason — `pass_review` refuses while any finding is open |
| A `fixed` finding records **what** was fixed | |
| A `dismissed` finding records **why** it is not worth fixing | |
| The outcome is written once | the second writer is refused. A dismissal that overwrote a fix would erase the record of the work that was done |
| A finding is a defect, not a redesign | if the fix is larger than the task, open the finding and let the human decide whether it becomes a child task |
| Comments carry the discussion | a comment id is `CMT-01` |

Severity is about consequence, not effort:

| severity | |
|---|---|
| `high` | the task's outcome is not met, or something is broken, unsafe or unrecoverable |
| `medium` | the outcome is met on the happy path and wrong elsewhere |
| `low` | a defect that does not change the outcome: clarity, an unused path, a missing edge-case guard |

## Session boundaries

Three rules. All are gates, not guidance: a transition that would cross one
fails.

| rule | gate |
|---|---|
| The session that **planned** the task never **reviews** it | `review.independent` |
| The session that **executed** the task never **reviews** it | `review.independent` |
| The session that **executed** the task never **remediates** its findings | `remediation.session_distinct` |

The task records the planning session in `planned_by`; the claim records the
execution session in `claim.session`. `review.independent` reads all three: the
acting session is neither `planned_by` nor `claim.session`, and all three are
recorded. A session that is not recorded is a refusal, not a pass.

A reviewer is ideally a different **model**, not only a different session — the
diversity is what surfaces uncorrelated bugs. If you are the session that
planned the task or the one that executed it, you cannot give the verdict. Say
so, and let another session review it.

## The verdict

Two values. Nothing else.

| verdict | means | then |
|---|---|---|
| `ok` | the task's outcome is met, and the work is sound | the task may move to `done` |
| `changes-needed` | something is wrong and it is fixable in this task | the task moves to `remediating` and the findings are worked |

The verdict is about the **task**, not about the implementer. `changes-needed`
is a normal outcome of a review; it is not a failure of the runner.

This is the **task verdict**, written by the reviewer in the task's `review`
section. The build loop's **round verdict** is the orchestrator's: an axis
reports findings and counts, the orchestrator merges them into the round's
record, and the task record gets the task verdict. The two live in different
records and are not interchangeable.

## What a verdict is checked against

The task model's § 3.3, for the reviewer's moves.

| gate | checks | applies to |
|---|---|---|
| `review.verdict_ok` | `review.verdict` is `ok` | `pass_review` |
| `review.verdict_changes_needed` | `review.verdict` is `changes-needed` | `request_changes`, `escalate` |
| `review.no_open_findings` | no open finding | `pass_review` |
| `review.findings_exist` | ≥1 open finding | `request_changes` |
| `review.findings_closed` | no open finding | `re_review` |
| `review.within_budget` | external rounds ≤ `max_rounds` | `pass_review` |
| `review.budget_left` | external rounds < `max_rounds` | `request_changes` |
| `review.budget_exhausted` | external rounds ≥ `max_rounds` | `escalate` |
| `review.round_current` | the latest external round was filed at or after the task's `status_since` (its entry into `code-review`) | `pass_review`, `request_changes`, `escalate` |
| `review.round_owner` | the acting session is the one that filed the latest external round | `pass_review`, `request_changes`, `escalate` |
| `review.independent` | the acting session is neither `planned_by` nor `claim.session`; all three are recorded | `pass_review`, `request_changes`, `escalate` |
| `remediation.session_distinct` | the acting session is not `claim.session`; both are recorded | `re_review` |

`review.verdict` is set by the round. A verdict of `ok` with an open finding is
refused; a verdict of `changes-needed` with no finding is refused.

## Reading the change

Read the task record before the diff: the criteria, the scope, the evidence for
each criterion, and any `deviation` on a step. `exit 0` with an empty summary
proves nothing. Then follow the `code-review` skill at
`~/.config/opencode/skills/code-review/SKILL.md` for the diff surface, the two
axes, the targeted probes, and the delta review.

Record what this document owns: the round, its focus, the findings, and the
verdict.

## The CLI surface

The grammar is noun, then verb: the noun is the record the call writes.

| call | what it does |
|---|---|
| `talita task show <TSK>` | the task as stored, with its derived criteria, claimability, readiness and children progress |
| `talita round file <TSK> --input <path\|->` | files the whole external round: focus, verdict, notes and findings. talita allocates the `FIND-nn` ids and returns `refs` |
| `talita round file <TSK> --kind external --focus <f> --verdict ok [--notes <t>]` | the same round from flags, without findings |
| `talita finding open <TSK> --severity low\|medium\|high --description <t>` | a finding raised outside the round |
| `talita comment add <TSK> <FIND> --body <t>` | the discussion under a finding |
| `talita task request-changes <TSK>` | `code-review → remediating` |
| `talita task pass-review <TSK>` | `code-review → done` |
| `talita task escalate <TSK>` | `code-review → needs-human`, when the budget is spent |

Every write carries the identity: `--as <you> --role reviewer --session <s>`,
or `TALITA_ACTOR`, `TALITA_ROLE` and `TALITA_SESSION`; a flag beats its env.
Reads need none: `talita task show`, `talita round show <TSK>` and `talita
journal show --task <TSK>`.

## Filing the round

File the whole round in one call, before the verdict. Its input is closed:
unknown keys are refused.

```json
{
  "kind": "external",
  "focus": "edge-cases",
  "verdict": "changes-needed",
  "notes": "free text",
  "findings": [
    { "ref": "D1", "severity": "high", "description": "where; defect; scenario; fix" }
  ]
}
```

```sh
talita round file <TSK> --input round.json
```

The four lines of an `OUTFILE` finding go in `description`: where, defect,
scenario, fix. The output's `refs` maps each `D<n>` to the `FIND-nn` it
became. Then move the task: `talita task request-changes <TSK>` for
`changes-needed`, `talita task pass-review <TSK>` for `ok`, or `talita task
escalate <TSK>` when the budget is spent and work is still outstanding.

A finding found after the round is filed is `talita finding open <TSK>
--severity … --description "…"`. A comment is `talita comment add <TSK> <FIND>
--body "…"`.

Do not hand-edit the task file to file a round or a finding: these verbs are
the record, and they allocate the ids the gates read.
