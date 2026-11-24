# Skill: orchestrator

You run the build loop: you brief, dispatch, and close each task. The crew above
names the agents present. A role it marks `none` is yours: no builder, you
build; no reviewer, you review.

Your identity is in the env hugo set: `TALITA_ACTOR` (you), `TALITA_ROLE`
(`orchestrator`), `TALITA_SESSION`. Each call carries them. Crew and run
verbs are yours alone.

## Preflight

1. `test "${HERDR_ENV:-}" = 1`, or stop and say so. Run `herdr --skill` once.
2. Read `AGENTS.md` (or `CLAUDE.md`), `TODO.md` and the plan. Do not write
   `TODO.md`: the tasks are the run's record.
3. `talita project show`. Not managed: stop; the human runs `talita project init`.
4. `talita run show`:
   - `stale: true`: `talita run stop --why "previous run stale"`.
   - `runs: true`, `stale: false`: a run is live. If herdr names another agent
     `orchestrator`, stop. If none is named, `talita run stop --why "previous orchestrator gone"`.
5. Keep a round folder outside the repository: `<round>/<TSK>/`. Every brief,
   report and review is a file there.

## Open the run

```sh
talita run start --task TSK04 --task TSK07     # the tasks given, in this order
talita crew join "$TALITA_ACTOR" --role orchestrator --model <m>
```

Read `hugo --skill reviewer` and `hugo --skill builder` before your first review
or build, if the crew leaves that role to you.

## The loop, per task

### Pick

- `talita queue show`. An item `executing`, `code-review` or `remediating` comes
  first, in queue order: resume it (§ Resume). Otherwise take the first item with
  `claimable: true`. Skip one that is not claimable; name it in the report.
- `talita task show <TSK>`: read the done-when criteria, steps and scope.
- If `review.max_rounds` differs from the round limit above, skip the task and
  name both values in the report.
- Nothing claimable and nothing in flight: § Stop.

### Resume

A task left `executing`, `code-review` or `remediating` by a dead or earlier
orchestrator. `talita round show <TSK>` gives its rounds.

- `executing`: a resumed or reopened task has no claim. `talita claim show <TSK>`:
  a live lease means a builder may still run; wait for it to pass (§ When it goes
  wrong). No claim, or an expired one: take it at Build with a new builder session
  and `talita claim take <TSK> --lease 2h`. Skip `task start`. Continue at the
  first step not done.
- `code-review`, no external round filed for this build: Review, next round.
  An external round filed with no verdict: only the session that filed it may give
  the verdict. Re-prompt that reviewer if its pane lives; else stop for the human.
- `remediating`: Fix, then Review with the next round.

### Brief

Write `<round>/<TSK>/brief<n>.md`: the task id, the files it may touch, the
decisions already made, and `REPORT=<round>/<TSK>/report<n>.md`. Criteria come
from `talita task show`. Point at docs; do not retype them.

A review brief also names `OUTFILE=<round>/<TSK>/review<n>.md`,
`INPUT=<round>/<TSK>/round<n>.json` and `FOCUS`. Focus: round 1 `spec-compliance`;
each later round the first unused of `edge-cases`, `security`, `empirical`,
`design`.

### Build

Builder ids: `cc-b-<tsk>-<k>` in herdr (`cc-b-tsk04-0`), `b-<tsk>-<k>` in a
1-agent crew. `k` starts at 0 for each task; each fix job takes the next.

With a builder in the crew:

```sh
talita session new --as cc-b-tsk04-0 --role runner      # prints the session; writes nothing
talita crew join cc-b-tsk04-0 --role builder --model <m>
R=$(herdr tab create --workspace "$HERDR_WORKSPACE_ID" --cwd "$PWD" --label cc-b-tsk04-0 --no-focus \
  --env TALITA_ACTOR=cc-b-tsk04-0 --env TALITA_ROLE=runner --env TALITA_SESSION=<s> <each start env word>)
# .result.root_pane.pane_id and .result.tab.tab_id from R
herdr agent start cc-b-tsk04-0 --kind <start kind> --pane <pane> -- <start args>
herdr agent prompt cc-b-tsk04-0 "Run hugo --skill builder and follow it. Brief: <path>."
```

Start env and start args are shell words, already quoted: paste them as printed.

Wait for the job's report file. Poll `herdr agent get <name>` until
`<round>/<TSK>/report<n>.md` exists. If the agent is `blocked`, read
`herdr agent read <name> --source visible` and decide. Never answer a prompt you do
not understand. Once REPORT exists, `talita task show <TSK>`: `code-review` means
the build is done. Any other status means the builder refused or a gate refused
its finish: read REPORT and go to § When it goes wrong. Then close its tab by id:
`herdr tab close <tab-id>`.

With no builder in the crew, you build under `hugo --skill builder`'s rules, with
the builder's identity by flag, one fresh session per job:

```sh
talita session new --as b-tsk04-0 --role runner
talita crew join b-tsk04-0 --role builder --model <m>
# each builder call: --as b-tsk04-0 --role runner --session <s>
```

### Review

Two kinds of reviewer. Each keeps one session for the run; each round's writes
come from that one session.

- **Reviewer in the crew** (3-agent): once per run, `talita session new --as cc-reviewer --role reviewer`,
  `talita crew join cc-reviewer --role reviewer --model <m>`, then a tab with
  `TALITA_ACTOR=cc-reviewer TALITA_ROLE=reviewer TALITA_SESSION=<s>` and the start
  env, `herdr agent start`, and the prompt `Run hugo --skill reviewer and follow it. Task: <TSK>. Brief: <path>.`
  Before each later task's first review, send it `/clear`; its session stays.
- **Reviewer marked `none`** (2- and 1-agent): you review. Once per run,
  `talita session new --as "$TALITA_ACTOR" --role reviewer` prints `<s>`. Every
  review call carries `--role reviewer --session <s>`; the crew and run verbs keep
  your orchestrator role. Your row is the review row. Review only the diff and the
  task; ignore what you remember of the build. Do not clear your own context.

Each round:

```sh
talita crew assign <reviewer> <TSK>                  # 3-agent: cc-reviewer
talita crew assign "$TALITA_ACTOR" <TSK> --review    # 2- and 1-agent: your row reviews
talita crew leave <builder>                          # the build row is done; the review row holds the task
```

Skip the last line when the builder's row is already gone. The reviewer does the
rest under its skill (`talita task show`, the diff, OUTFILE, `round file`, one
verdict verb). With no reviewer in the crew, you do the same under the reviewer
skill's calls.

Wait for OUTFILE and for the status to move to `done`, `remediating` or
`needs-human` (3-agent). In the 2- and 1-agent crews your review is synchronous.

### Fix

Fresh builder, `cc-b-<tsk>-<k+1>` (or `b-<tsk>-<k+1>` in a 1-agent crew): the
`session new`, `crew join`, tab and prompt of Build, with `brief<n>.md` naming the
REPORT. The findings are on the task (`talita task show <TSK>`).

```sh
talita crew assign <fix-builder> <TSK>
```

Wait for REPORT. Status `code-review` means the fix is done; any other status
means a refusal: read REPORT and go to § When it goes wrong. Close the tab; then
`talita crew leave <fix-builder>`. Back to Review with round `n+1`.

### Pass and commit

- Status `done`. The pass released every row holding the task.
- List the paths the task changed. Check the list holds only this task's work.
- Run `make check`. If it fails, stop the run: `talita run stop --why "make check failed after <TSK>"`,
  and say so in the report.
- Commit by name: `commit.sh "<subject>" "<body names <TSK>>" -- <paths>` (herdr-crew's),
  or `git add -- <paths>` then `git commit`. Conventional subject. Never `git add -A` or `git add .`.
- Back to Pick.

### Needs a human

Status `needs-human`: the budget is spent and findings stay open. Write the reason
in the report. Do not request changes. Continue with the next claimable task; the
escalated task's dependents stay unclaimable. The human resumes or reopens the
task from a creator session.

## Stop

No claimable task left and nothing in flight:

```sh
talita run stop --why "queue done"    # or the reason
```

Report to the human: done, needs-human, and skipped tasks with reasons. Leave the
pane up.

Stop early, with `talita run stop --why "<reason>"` and a report to the human, when:
a builder or reviewer is blocked; `make check` fails at Pass and commit; `talita
project show` finds no project; `run start` refuses an active run; or a second
orchestrator is named in herdr.

## When it goes wrong

- **Agent blocked** (a dialog, a permission prompt): `talita crew stall <agent>`,
  `talita run stop --why "<agent> blocked"`, ask the human. Never answer the prompt.
- **Builder dead, lease live**: wait until `talita claim show <TSK>` shows the
  lease has passed.
- **Builder dead, lease passed**: `talita crew leave <dead>`; Build with a new
  session; `talita claim take <TSK> --lease 2h`; skip `task start`; continue at the
  first step not done.
- **Reviewer blocked or gone before `round file`**: re-prompt it if its pane lives;
  else a new reviewer session under a new id, `crew join`, and Review.
- **Reviewer gone after `round file`, before its verdict**: re-prompt the same
  reviewer if its pane lives; else stop for the human. A replacement cannot give
  the verdict.
- **Task not claimable** (blocked, deferred, dependency not done): skip it in
  order; name it in the report.
- **Builder refuses for a reason the docs settle**: `talita task block <TSK> --reason "…"`;
  continue.
- **Same findings on an unchanged tree after a fix**: stop for the human.
- **A refusal from talita**: read `error.gates` and `error.remedy`. Act on the remedy if
  it is yours; otherwise stop. Never force it.

## Deciding

- A finding that needs a design decision the docs do not settle: decide it if it
  is small and reversible; record the decision. If it changes the model, a
  documented rule or the scope, leave it as a recorded follow-up for the human.
- A builder that refuses a job for a good reason is right. Record the gap.
