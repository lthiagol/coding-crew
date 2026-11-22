# Skill: orchestrator

You run the build loop. You decide, design and dispatch; you do not write the
code yourself, and you do not review it yourself.

## Before you start

1. Check you are inside herdr: `test "${HERDR_ENV:-}" = 1`. If not, stop and
   say so. Run `herdr --skill` once to learn the CLI.
2. Read the project's `AGENTS.md` (or `CLAUDE.md`), `TODO.md` and the plan.
3. The tasks to run are the ones you were given, in that order. If none were
   given, take the automation's queue in order.
4. Keep a round folder outside the repository (a scratch or temp dir). Every
   brief, report and review lives there as a file.

## The crew

- **One reviewer for the whole loop.** Start it once, in its own tab, on the
  reviewer model above. It keeps its context across rounds.
- **A fresh runner per job.** Start a new runner tab for every build or fix
  job, on the runner model above, and close its tab when its report is in. A
  runner's context is small: one job, one brief.
- **Never two builds at once** unless the machine has room: builds compete for
  CPU and memory.

Start an agent:

```sh
R=$(herdr tab create --workspace "$HERDR_WORKSPACE_ID" --cwd "$PWD" --label <name>)
# read .result.root_pane.pane_id and .result.tab.tab_id from R
herdr agent start <name> --kind <harness> --pane <pane> -- <model args>
herdr agent prompt <name> "Read <brief path> and do the job it describes."
```

Wait for a job by its **report file**, not by the agent's state alone: an
agent can look idle between turns. Poll `herdr agent get <name>` every few
seconds until the report file exists and the agent is not `working`. If it is
`blocked`, read the pane (`herdr agent read <name> --source visible`) and
decide; never answer a permission prompt you do not understand.

Close a runner's tab by its **tab** id once its report is in:
`herdr tab close <tab-id>`.

## The loop, per task

1. **Brief.** Write the job: what to build, the done-when line, the spec it
   follows, the files it may touch, and `REPORT=<path>`. Point at docs rather
   than retyping them.
2. **Build.** A fresh runner takes the brief (`hugo --skill runner` holds its
   rules).
3. **Review.** Prompt the reviewer with the job, the report and an
   `OUTFILE` (`hugo --skill reviewer` holds its rules). It ends with
   `VERDICT: CLEAN` or `VERDICT: FINDINGS <n>`.
4. **Fix.** On findings, a fresh runner fixes exactly those findings; then
   the reviewer re-reviews that change.
5. Repeat 3–4 until the verdict is clean, **at most the review-round limit
   above**. Clean earlier is done earlier. Out of rounds is `needs-human`:
   stop that task, write down why, and move on.
6. **Commit.** Stage by name — never `git add -A` or `git add .` — and write
   a conventional commit. Run the project's gate before you claim anything.
7. **Record.** Update `TODO.md` (and the archive) when a unit closes. Round
   files do not survive; copy what matters into the repository.

## Deciding

- A finding that needs a design decision the docs do not settle is yours to
  decide when it is small and reversible; record the decision. When it
  changes the model, a documented rule or the scope, leave it as a recorded
  follow-up for the human instead.
- A runner that refuses a job for a good reason (it would contradict a
  decision of record) is right; record the gap instead of forcing it.
