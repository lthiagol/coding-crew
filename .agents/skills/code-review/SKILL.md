---
name: code-review
description: Review coding-lanes changes on two axes — CODE (correctness and regressions) and SPEC (the unit's requirements and scope) — against the uncommitted diff. Use when acting as a reviewer, filing findings, or re-reviewing a fix round; the orchestrator owns the round verdict.
---

# Code review

Two independent axes over the uncommitted diff. **CODE** asks whether the
change is sound. **SPEC** asks whether it is the unit that was asked for. The
orchestrator opens both at once, each in a fresh context, so neither masks the
other and neither trusts the builder's account.

Each axis writes its own findings file. The orchestrator merges them and gives
the round's single verdict.

## Inputs

| input | what it gives you |
|---|---|
| review brief | the unit, the round number, the axis, and the path to the findings file |
| unit kickoff and done-when | what was asked, the scope boundary, and what counts as done |
| gate result | the orchestrator's `make ci` command, its exit status, and the log path, as carried in the brief |
| diff surface | the uncommitted working tree — `git status`, `git diff --stat`, `git diff`; the change under review is never a committed range |
| round snapshot | the snapshot directory `scripts/review-snapshot.sh` wrote after the builder round; the brief names its path |
| previous snapshot and delta | on a fix re-review only: the previous snapshot directory, the current one, and the diff of their file trees and manifests |
| standards | `AGENTS.md` and the docs it points to; the repository's own rules and patterns |
| prior findings | the previous round's findings file — present only on a fix re-review |

Stop if the diff is empty: an empty diff is a missing-change problem, not a
clean change.

## The axes

| axis | question | looks for |
|---|---|---|
| **CODE** | is the change sound? | correctness defects, regressions, unsafe behaviour, data loss, mishandled errors, inadequate tests, concrete maintainability defects |
| **SPEC** | is this the unit that was asked for? | requirements missing or partial, scope creep, requirements that look implemented but whose implementation looks wrong |

The axes stay separate: a change can follow every standard and do the wrong
thing, or do exactly what the brief asked and break the project's conventions.
One axis never reviews the other's findings. The smell baseline below belongs
to **CODE**; SPEC does not report smells. A SPEC finding quotes the kickoff.

## Standards and the CODE smell baseline

Repository standards override generic advice. Cite the rule — the file and the
section — when a finding leans on one. Skip anything tooling already enforces.

The baseline is five code smells. Each is a judgement call, never a hard
violation, and a finding only when tied to a concrete defect:

| smell | the defect it points at | the move |
|---|---|---|
| Mysterious Name | a name that does not reveal what the thing does or holds | rename it; a name that will not come is a design problem |
| Duplicated Code | the same logic shape in more than one hunk | extract the shape and call it from both |
| Feature Envy | a method reaching into another object's data more than its own | move it onto the data it envies |
| Primitive Obsession | a primitive standing in for a domain concept | give the concept a small type |
| Speculative Generality | an abstraction, parameter, or hook the brief does not need | delete it; inline until a real need shows |

Name the defect the smell causes. A smell on its own is taste, not a finding.

## Findings

| field | holds |
|---|---|
| axis | `code` or `spec` |
| severity | `high`, `medium`, or `low` |
| location | `file:line`, or the requirement quoted |
| consequence | what breaks, or which requirement goes unmet |
| evidence | the hunk, the command and its output, or the brief line |

A finding is **blocking** when any of these is true:

- severity is `high` or `medium`;
- a requirement is unmet or only partly met;
- the change regresses existing behaviour;
- the gate is failing.

`low` findings are recorded in the round. They do not force another round; the
orchestrator carries them as follow-ups.

Optional architecture is not a finding. It becomes one only when it breaches a
project rule or carries a concrete risk. A finding is a defect, not a redesign.

## The gate and probes

The orchestrator runs one full `make ci` before the review and again before the
commit. The brief carries the command, its exit status, and the log path; read
the result there and do not duplicate the run.

Run only the **targeted probe** a concrete suspicion needs: one test, one
command, one edge case. No exhaustive mutation sweeps, no size-space sweeps, no
repeated upstream comparisons, no speculative attacks. A check worth repeating
belongs in the repository as its own unit.

## The axis report

Write to the path the brief names, about 400 words:

1. One line naming the axis and the diff reviewed.
2. The findings, strongest first, each with the five fields.
3. A closing count: `<n> blocking, <n> low`.

Report to the orchestrator with that count. The orchestrator owns the
`ok` / `changes-needed` verdict; an axis reports findings, not the round's
verdict.

## The delta review

A fix round is a **delta review**, not a second first review. The orchestrator
snapshots every builder round with `scripts/review-snapshot.sh <dir>`, and
generates the delta with `scripts/review-snapshot.sh --diff <previous> <current>`.
One fresh reviewer:

1. Reads the prior findings and marks each `closed` or `open`. A rename is not
   a fix; check the behaviour the finding named.
2. Reads the generated delta: the manifest changes and the diff of the
   previous and current snapshot file trees, plus the lines those hunks touch.
   `git diff` still holds the first round's change, so it cannot separate the
   fix; the snapshots can. Unchanged code that was clean stays clean.
3. Checks the gate result and runs the targeted probe that proves each fix.

New defects in the changed hunks are findings like any other. A full rescan of
untouched code is a new review, not a delta.

A delta finding keeps its origin axis — `code` or `spec` — in the `axis`
field. **Delta** is the report section, not a third axis. Its report is:

1. One line naming the delta review and the two snapshot directories.
2. The prior findings, each `closed` or `open`.
3. The new findings, strongest first, each with the five fields.
4. A closing count: `<n> prior open, <n> new blocking, <n> low`.
