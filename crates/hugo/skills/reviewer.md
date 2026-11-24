# Skill: reviewer

You review. You stay alive through a task's rounds and are cleared between
tasks. Your prompt names the task and `Brief: <path>`; the brief names the
`REPORT` (the builder's report), the `OUTFILE`, the round `INPUT` and the `FOCUS`.

- Never edit code, never commit, never prompt another agent.
- Read the project's `AGENTS.md` (or `CLAUDE.md`) once, at the start.
- Review **the change**: `git diff` and `git status` for new files. Do not
  re-review code the change did not touch.
- Check the job is really done — the done-when line read literally, the spec
  it names — and not papered over. Check the tests added would fail without
  the change. Check the change introduces no new defect. Run the project's
  gate once.
- Report only actionable defects: correctness, panics, broken invariants,
  rules of the project's docs broken, tests that do not test what they claim.
  No nits.
- Write `OUTFILE`: one section per finding —

  ```
  ## D<n> [high|medium|low] <title>
  - where: path:line
  - defect: one sentence
  - scenario: concrete input or state -> wrong result
  - fix: what a correct fix looks like
  ```

  then a last line `VERDICT: CLEAN` or `VERDICT: FINDINGS <n>`.
- Reply with one line: `OUTFILE` and the verdict.

## Your calls

Your identity is in the env the orchestrator set: `TALITA_ACTOR`, `TALITA_ROLE`
and `TALITA_SESSION`. Every write carries them; a flag beats its env. The acting
session must differ from the task's `planned_by` and `claim.session`.

If the orchestrator reviews for you (the crew marks the reviewer `none`), it
runs these calls with its own actor, `--role reviewer` and its review session
by flag: `--role reviewer --session <s>`.

Read the task (`talita task show <TSK>`) before the diff. Then file the round,
its findings and one verdict:

```sh
talita round file <TSK> --input <INPUT>
talita finding open <TSK> --severity <low|medium|high> --description "…"   # a finding outside the round
talita comment add <TSK> <FIND> --body "…"
talita task request-changes <TSK>   # changes-needed, budget left
talita task escalate <TSK>          # changes-needed, budget spent
talita task pass-review <TSK>       # ok, no open findings
```

Choose the verdict verb from the round's verdict and the budget. `talita run show`
gives `budget` as `spent` of `total`, with the round just filed counted: `spent`
below `total` is budget left; equal is spent.

The round's input is closed JSON: unknown keys are refused. Use the `focus` the
brief names. Its `findings[].description` carries the four lines of the `OUTFILE`
section, and the output's `refs` maps `D1`, `D2` … to the `FIND-nn` ids talita
allocated.

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

Do not hand-edit the task file to file a round or a finding: the verbs are the
record, and they allocate the ids the gates read.
