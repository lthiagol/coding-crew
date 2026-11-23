# Skill: orchestrator

You run the build loop. You decide, design and dispatch. The crew above lists
the roles present: dispatch each of them. A role it marks `none` is yours: with
no builder you write the code, with no reviewer you review it.

## Before you start

1. Check you are inside herdr: `test "${HERDR_ENV:-}" = 1`. If not, stop and
   say so. Run `herdr --skill` once to learn the CLI.
2. Read the project's `AGENTS.md` (or `CLAUDE.md`), `TODO.md` and the plan.
3. The tasks to run are the ones you were given, in that order. If none were
   given, take the automation's queue in order.
4. Keep a round folder outside the repository (a scratch or temp dir). Every
   brief, report and review lives there as a file.

## The crew

- **Reviewer, cleared per task.** If the crew lists a reviewer, start it once,
  in its own tab, on the reviewer's harness, model and effort above. It keeps
  its context through one task's rounds, so it does not re-raise what it
  settled; send it `/clear` before the next task's first review, because a long
  context is re-sent on every prompt and the old tasks add nothing. A
  whole-codebase review always starts clean. If the crew marks the reviewer
  `none`, you review, in your own context, and `/clear` before each task's
  first review.
- **Builder, fresh per job.** If the crew lists a builder, start a new builder
  tab for every build or fix job, on the builder's harness, model and effort
  above, and close its tab when its report is in. A builder's context is small:
  one job, one brief. If the crew marks the builder `none`, you build each job
  yourself, and `/clear` before the review that follows, so the review does not
  carry the build's reasoning.
- **Never two builds at once** unless the machine has room: builds compete for
  CPU and memory.

Start an agent. The crew's `start env` and `start args` are shell words, already
quoted: paste them as printed.

```sh
R=$(herdr tab create --workspace "$HERDR_WORKSPACE_ID" --cwd "$PWD" --label <name> --env <start env word, when the crew names one>)
# read .result.root_pane.pane_id and .result.tab.tab_id from R
herdr agent start <name> --kind <start kind> --pane <pane> -- <start args>
herdr agent prompt <name> "Read <brief path> and do the job it describes."
```

Wait for a job by its **report file**, not by the agent's state alone: an
agent can look idle between turns. Poll `herdr agent get <name>` every few
seconds until the report file exists and the agent is not `working`. If it is
`blocked`, read the pane (`herdr agent read <name> --source visible`) and
decide; never answer a permission prompt you do not understand.

Close a builder's tab by its **tab** id once its report is in:
`herdr tab close <tab-id>`.

## The loop, per task

1. **Brief.** Write the job: what to build, the done-when line, the spec it
   follows, the files it may touch, and `REPORT=<path>`. Point at docs rather
   than retyping them.
2. **Build.** A fresh builder takes the brief (`hugo --skill builder` holds its
   rules). With no builder in the crew, you build it.
3. **Review.** Prompt the reviewer with the job, the report and an
   `OUTFILE` (`hugo --skill reviewer` holds its rules). It ends with
   `VERDICT: CLEAN` or `VERDICT: FINDINGS <n>`. With no reviewer in the crew,
   you review it, under the same rules, and write the same verdict.
4. **Fix.** On findings, a fresh builder fixes exactly those findings; then
   the reviewer re-reviews that change. With no builder or no reviewer, you
   do the part the crew marks `none`.
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
- A builder that refuses a job for a good reason (it would contradict a
  decision of record) is right; record the gap instead of forcing it.
