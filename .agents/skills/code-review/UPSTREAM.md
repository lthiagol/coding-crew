# Upstream provenance: code-review

| | |
|---|---|
| Repository | `github.com/mattpocock/skills` |
| License | MIT — see [../LICENSE](../LICENSE) |
| Upstream path | `skills/engineering/code-review/SKILL.md` |
| Source commit | `5beb35b040e3de97e54fa9e386d7e42d15c295c2` |
| Inspected main commit | `5863806dd4e15c63174bee4e7515a4256792ecfa` |
| Adapted on | 2026-11-17 |

This skill is **adapted**, not vendored. `.agents/skills/sync.sh` never
overwrites it: it fetches the source commit below, diffs it against the local
copy, and reports the drift. The local adaptation stays in place.

`--bump` moves the global upstream pin but never edits this file or its Source
commit: the adaptation remains based on `5beb35b…` until it is rebased onto a
newer upstream by hand. The script reports that manual step, and it diffs the
local skill against this recorded source commit, not the bumped pin.

## Deviation ledger

1. **Review target.** Upstream reviews `git diff <fixed-point>...HEAD` against a
   user-supplied commit, branch, tag, or merge-base. coding-lanes reviews the
   uncommitted working tree, because the orchestrator commits only after a
   clean verdict.
2. **Fixed point.** Upstream pins and validates a ref before dispatching.
   coding-lanes drops that step; the diff surface is `git status` and `git
   diff`, and an empty diff is a missing change.
3. **Spec source.** Upstream hunts issue tracker references, a user path, or a
   spec file under `docs/`, `specs/`, or `.scratch/`. coding-lanes takes the
   unit kickoff and its done-when line from the review brief, and the scope
   boundary with it.
4. **Standards source.** Upstream searches for `CODING_STANDARDS.md`,
   `CONTRIBUTING.md`, and every standards file it can find. coding-lanes points
   to `AGENTS.md` and the project docs it names.
5. **Axis names and scope.** Upstream's Standards axis becomes **CODE**, with
   correctness, regressions, unsafe behaviour, data loss, error handling, and
   test adequacy added from the `reviewer` role. **SPEC** keeps the requirement,
   scope-creep, and wrong-implementation checks.
6. **Smell baseline.** Upstream carries twelve Fowler smells. coding-lanes
   keeps five coding-focused ones — Mysterious Name, Duplicated Code, Feature
   Envy, Primitive Obsession, Speculative Generality — each a judgement call
   tied to a concrete defect and scoped to the CODE axis. SPEC does not report
   smells. Data Clumps, Repeated Switches, Shotgun Surgery,
   Divergent Change, Message Chains, Middle Man, and Refused Bequest are
   dropped.
7. **Parallel axes.** Upstream spawns parallel sub-agents and aggregates their
   reports. coding-lanes opens two fresh reviewer panes, each with its own
   findings file; the orchestrator merges the axis files into the round's
   findings file.
8. **Verdict.** Upstream reports the axes side by side and refuses to rerank
   them. coding-lanes merges them into one `ok` / `changes-needed` round verdict,
   because the task record, the round budget, and the transition gates need a
   single answer. The axes still never review each other.
9. **Finding shape.** Upstream reports per file or hunk with a quoted standard
   or spec line. coding-lanes findings carry axis, severity, location,
   consequence, and evidence, and a blocking rule the upstream report did not
   have.
10. **Low findings.** coding-lanes records `low` findings without forcing
    another round; the orchestrator carries them as follow-ups. Upstream has no
    severity model.
11. **Gate ownership.** The orchestrator runs one full `make ci` before the
    review and again before the commit, and the review brief carries the
    command, its exit status, and the log path. Reviewers read that result and
    run only targeted probes. Upstream assumes the reviewing agent runs
    whatever commands it needs.
12. **Probe limits.** coding-lanes bans exhaustive mutation sweeps, size-space
    sweeps, repeated upstream comparisons, and speculative attacks. A check
    worth repeating becomes its own unit of work.
13. **Delta review.** Upstream reviews once. coding-lanes snapshots every
    builder round with `scripts/review-snapshot.sh`, gives a fix reviewer the
    previous and current snapshot directories plus the diff of their file
    trees and manifests, and adds a delta review over prior findings, changed
    hunks, and the targeted probe that proves each fix. The uncommitted tree
    holds round 1's change too, so `git diff` cannot supply the baseline.
14. **Report length.** Each axis report stays at about 400 words, as upstream
    asks. The orchestrator's merged round findings file is the durable record.
15. **Harness and tracker setup.** Upstream's setup command for its issue
    tracker is removed, along with its branch-and-commit framing. References
    point at coding-lanes paths and roles.
