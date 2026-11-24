# The orchestrator's job

Accepted, unit O1, 2026-11-24: the human took every recommendation of § Open questions. O2 builds from it. Nothing
here is built yet: the shipped skills (`crates/hugo/skills/`) still describe the
herdr-crew loop.

Sources: [roles](../roles/roles.md), [task model](../tasks/task-model.md) §§ 3–4
and 11.2, [talita commands](../talita/commands.md), ADRs
[0011](../decisions/0011-flow-files.md), [0015](../decisions/0015-identity.md),
[0017](../decisions/0017-review-record.md),
[0018](../decisions/0018-runtime-and-journal-writers.md),
[0020](../decisions/0020-claims-and-leases.md), [screens](screens.md#starting-the-orchestrator).

## What the agent gets

- `s` opens a herdr tab, puts the session's crew in `HUGO_CREW` (agents with
  harness, model and effort; `round_limit`; `order`; `raul`), and prompts:
  `Run hugo --skill orchestrator and follow it. Project: <path>. Tasks, in order: <TSK>…`
- `hugo --skill orchestrator` prints the crew: each agent's start kind, args and
  env, and `none` for a role the crew leaves out. Then the skill.
- hugo claims nothing, runs no round and writes no record. The loop below is the
  orchestrator's.

## Identity

- The orchestrator mints its session once with `talita session new --as orch-1 --role orchestrator`
  (writes nothing). A shell keeps no env between calls, so every talita call
  carries `TALITA_ACTOR`, `TALITA_ROLE` and `TALITA_SESSION`, as env or flags. Q7
  asks whether hugo mints it instead.
- **`crew join --role` names the crew's role** (`builder`, `reviewer`,
  `orchestrator`), not the caller's. The caller's role comes from `TALITA_ROLE`.
  Do not pass the caller's role as `--role` on a `crew` call.
- Each agent gets its identity in its tab: `herdr tab create … --env TALITA_ACTOR=… --env TALITA_ROLE=… --env TALITA_SESSION=…`
  (herdr-crew's `crew_tab` in `lib.sh` takes these).
- **A fresh session per act**, so the session gates hold by construction: a build
  job, a fix job, a review, and in the one-agent crew each act of the orchestrator.
- Actor ids: the herdr agent's name (`cc-b-tsk04-0`, `cc-reviewer`). A builder
  job is a new actor, because `crew join` refuses an id it already holds.

| agent | mints | lives |
|---|---|---|
| builder, build job | `session new --as <b> --role runner` | one job |
| builder, fix job | `session new --as <b> --role runner` | one job; differs from the claim's session by construction |
| reviewer | `session new --as <r> --role reviewer` | the run; one session files every round and gives every verdict, as `review.round_owner` needs |

## The crew

| agents | builds | reviews | crew rows the orchestrator joins |
|---|---|---|---|
| 3 (one per role) | builder | reviewer | orchestrator; reviewer once per run; a builder per job |
| 2 (orchestrator and builder) | builder | orchestrator, as reviewer (Q5) | orchestrator; builder per job; a reviewer row for its review session |
| 1 (orchestrator alone) | a builder actor per job, acted for by the orchestrator | a reviewer actor, acted for by the orchestrator | orchestrator; a builder row per job; a reviewer row |

A role the crew leaves out is `none`, and the orchestrator does that part itself.

In the 1-agent crew no herdr agent is started for either role. The orchestrator
acts under each role's identity by flag, one fresh session per act:
- Build job: `talita session new --as b-tsk04-0 --role runner`; `talita crew join
  b-tsk04-0 --role builder --model <m>`; the builder's calls with
  `TALITA_ACTOR=b-tsk04-0 TALITA_ROLE=runner TALITA_SESSION=<s>`; `crew leave` at S5.
  A fix job is `b-tsk04-1`, and so on: `crew join` refuses an id it already holds.
- Review: `rev-1`, once per run, as in S5.
- The orchestrator's own row is never the builder or the reviewer (Q5), so the
  pane shows the stage the same way as in the 3- and 2-agent crews.

Row side effects (ADR 0018, `runtime.rs`): `claim take` holds the task and sets
the row executing; `task finish`, `task re-review` and an external `round file`
set it idle and keep the hold; `task pass-review` and `task escalate` release
every row holding the task. A reviewer row is `reviewing` only after
`crew assign`. The orchestrator's row is never a reviewer (Q5).

## The loop

Each step names who acts, the talita calls, what the pane shows from
`runtime.json`, and where it stops.

### S0 Preflight — orchestrator

- `test "${HERDR_ENV:-}" = 1`; read `AGENTS.md`, `TODO.md`, the plan.
- `talita project show`. Not managed: stop; the human runs `talita project init`.
- `talita run show`. `runs: true`, `stale: false`: a run is live. Herdr names
  another orchestrator agent: stop. It names none: the run's orchestrator is
  gone, so `talita run stop --why "previous orchestrator gone"` (Q14), then S1.
  A record with no held lease is never stale.
- `stale: true`: `talita run stop --why "previous run stale"`, then S1.

### S1 Open the run — orchestrator

- `talita run start --task TSK04 --task TSK07` — the tasks hugo handed over, in order.
  It starts with an empty crew: a dead run's rows and holds go; its claims stay in the task files.
- `talita crew join orch-1 --role orchestrator --model <m>`.
- Pane: queue waiting, orchestrator idle, stamp running.

### S2 Pick — orchestrator

- `talita queue show`: an in-flight item (status `executing`, `code-review` or
  `remediating`) comes first, in queue order; resume it (§ Resume). Otherwise take
  the first item with `claimable: true`. A task that is not claimable is skipped,
  not waited for. `claimable` is `approved`-only, so an in-flight task never shows it.
- `talita task show TSK04`: read the criteria (the done-when), the steps and the scope.
- None claimable and nothing in flight: stop (see § Stopping).

### Resume — orchestrator

Reached from S2 for an in-flight task, left by a dead or earlier orchestrator.
`talita task show TSK07` gives the status; `talita round show TSK07` gives the
rounds filed so far.

- `executing`: `talita claim show TSK07`. A live lease: the builder may still run;
  wait for the lease to pass, as § When it goes wrong says. Expired or absent: a
  takeover as S4, with a new builder session and `talita claim take TSK07 --lease 2h`.
  Skip `task start`; continue at the first step not done.
- `code-review`, no external round filed for this build: S5 from the next round.
  An external round filed with no verdict: only the session that filed it may give
  the verdict (`review.round_owner`). Re-prompt that reviewer if its pane lives;
  else Q9.
- `remediating`: S6 with a fresh fix builder, then S5 with round `n+1`.

### S3 Brief — orchestrator

- Write `<round>/TSK04/brief0.md`: the task id, the files it may touch, the
  decisions already made, and `REPORT=<round>/TSK04/report0.md`. The criteria come
  from `talita task show`, not from the brief.

### S4 Build — builder

Orchestrator:
- `talita session new --as cc-b-tsk04-0 --role runner`, then
  `talita crew join cc-b-tsk04-0 --role builder --model <m>`.
- Start the agent with its env (see Identity; `spawn.sh` cannot carry it, § From herdr-crew), then
  `herdr agent prompt <name> "Run hugo --skill builder and follow it. Brief: <path>."`

Builder (`hugo --skill builder` holds these):
- `talita claim take TSK04 --lease 2h`
- `talita task start TSK04`
- `talita step begin TSK04 STP-01`, then `talita step done TSK04 STP-01` (or `skip`/`fail` with `--deviation`)
- `talita evidence run TSK04 ACCEPT-01` for each criterion
- `talita note add TSK04 --body "…"`
- `talita round file TSK04 --kind self --focus <f> --verdict ok|changes-needed`
- `talita task finish TSK04`, then its REPORT, then one reply line.

Orchestrator: wait for the REPORT and `talita task show TSK04` reading `code-review`
(S5 for the next step). Close the builder's tab by id.

Pane: building, `crew` builder executing with TSK04 held, round self `0`, budget `0 of 5`.
After finish: builder idle, still holding.

### S5 Review — reviewer

Orchestrator:
- Once per run: `talita session new --as rev-1 --role reviewer`, then
  `talita crew join rev-1 --role reviewer --model <m>`; start the reviewer agent
  and prompt it with `Run hugo --skill reviewer and follow it. Task: TSK04. Brief: <path>.`
  Between tasks, `/clear` the reviewer, as herdr-crew does; its session stays.
- Each round: `talita crew assign rev-1 TSK04`, then `talita crew leave cc-b-tsk04-0`
  (the build row is no longer needed; the reviewer's row now holds the task).

Reviewer:
- `talita task show TSK04`, then the diff. Writes `review<n>.md` (the OUTFILE, in
  scratch) and `round<n>.json` (the input).
- `talita round file TSK04 --input <path>/round<n>.json` — `kind: external`, the
  focus, the verdict, and `findings[]` with a `ref` each.
- Exactly one verdict:
  - `talita task pass-review TSK04` — `ok`, no open findings → S7.
  - `talita task request-changes TSK04` — `changes-needed`, budget left → S6.
  - `talita task escalate TSK04` — `changes-needed`, budget spent → S8.
- Focus: round 1 `spec-compliance`; each later round the first unused of
  `edge-cases`, `security`, `empirical`, `design`.

Orchestrator: wait for the OUTFILE and for the status to move (`done`,
`remediating` or `needs-human`).

Pane: `crew assign` → reviewer reviewing, round external `n`, budget `n of 5`
(the round in flight counts). After `round file`, the reviewer is idle and the
round reads `self` until the next assignment, while the budget still counts it.

### S6 Fix — fresh builder

Orchestrator: `talita session new --as cc-b-tsk04-1 --role runner`,
`talita crew join cc-b-tsk04-1 --role builder --model <m>`,
`talita crew assign cc-b-tsk04-1 TSK04`, the tab with env, and the prompt
`Run hugo --skill builder and follow it. Brief: <path>.` with `brief<n>.md`. The findings are on the task: `talita task show TSK04`.

Builder:
- per finding: `talita finding resolve TSK04 FIND-01 --resolution "…"`, or
  `talita finding dismiss TSK04 FIND-01 --reason "…"`
- `talita evidence run TSK04 ACCEPT-nn` for each criterion the fix touches
- `talita task re-review TSK04` (refused if the session is the claim's)
- REPORT, one reply line.

Orchestrator: wait for REPORT and status `code-review`; close the tab; `talita crew leave cc-b-tsk04-1`;
`talita crew assign rev-1 TSK04`; back to S5 with round `n+1`.

Pane: `remediating` shows the external stage, numbered for the next round; the
findings count falls as they close.

### S7 Pass and commit — orchestrator

- Status `done`. The pass released every row holding TSK04.
- List the paths the task changed (scratch snapshot, as `changed.sh` does). Check
  the list holds only this task's work.
- Run `make check` — the commit's claim (Q13).
- `scripts/commit.sh "<subject>" "<body names TSK04>" -- <paths>`, conventional, stage by name (Q1).
- Back to S2.

Pane: the done count rises.

### S8 Needs a human — orchestrator

- Status `needs-human`: the budget is spent and the findings stay open. Write the
  reason in the report. Do not request changes; the escalation is the reviewer's.
- Back to S2 for the next claimable task (Q10).
- The human resumes or reopens from a creator session, not from the orchestrator:
  `talita task resume TSK04 --instructed-by <human> --reason "…" [--grant-rounds <n>]`
  or `talita task reopen TSK04 --instructed-by <human> --reason "…"`. Q8 covers
  what happens next.

### S9 Stop — orchestrator

- No claimable task left: `talita run stop --why "queue done"` (or the reason).
- Report to the human, and leave the pane up (Q11).

## When it goes wrong

| event | seen by | the orchestrator | talita state |
|---|---|---|---|
| builder blocks (a dialog, a permission prompt) | `await.sh` exit 2 | `talita crew stall <b>`; `talita run stop --why "<b> blocked"`; ask the human. Never answer the prompt | claim live; no takeover until the lease passes |
| builder dies, lease live | exit 3, or exit 5 with no REPORT | wait for `talita claim show TSK04` to pass `lease_expires_at`; then the next row | executing, claim held |
| builder dies, lease passed | `claim show` not live | `talita crew leave <dead>`; S4 with a new session: `talita claim take TSK04 --lease 2h`. Skip `task start`: it is executing. Continue at the first step not done | executing; takeover moves the execution session |
| reviewer blocked or gone before `round file` | exit 2 or 3 | re-prompt if its pane lives; else a new reviewer session, `crew join` under a new id, and S5 | no round filed; re-filing is allowed |
| reviewer gone after `round file`, before its verdict | exit 3 | stop; Q9 | code-review; the round is filed |
| task blocked or deferred, or a dependency not done | `queue` shows `claimable: false` | skip it in order; name it in the report | unchanged |
| builder refuses for a reason the docs settle | REPORT says so | `talita task block TSK04 --reason "…"` so it is not picked again; continue | blocked overlay |
| orchestrator died: `runs: true`, `stale: false`, no other orchestrator in herdr | `run show` at S0 | `run stop --why "previous orchestrator gone"`, S1, then § Resume for in-flight tasks. Q14 | record cleared; claims stay in the task files |
| a talita refusal | the envelope's `error.gates` and `error.remedy` | act on the remedy if it is the orchestrator's; otherwise stop. Do not force it | unchanged |

## Stopping

The run stops (S9) when no task is claimable. It stops early when a builder or a
reviewer is blocked (the loop does not dispatch past an unanswered dialog, as
herdr-crew's rule has it), when `make check` fails at S7, when `talita project show`
finds no project, when `run start` refuses an active run, or when a second
orchestrator is named in herdr. `needs-human` does not stop the run (Q10).

## From herdr-crew to O2

herdr-crew's scripts are read, not edited. Each step maps to one of: kept as it
is, replaced by a talita call or a hugo action, or dropped.

| herdr-crew | in O2 |
|---|---|
| `SKILL.md`: herdr check, `herdr --skill` | kept |
| `init.sh`, `crew.env` | replaced: `HUGO_CREW` is the crew; `CREW_ROUNDS` is the task's `review.max_rounds` (Q6); `CREW_IDLE_LIMIT`, `CREW_WORKSPACE`, `CREW_TRAILER` stay as the round folder's settings |
| `builder-rules.md`, `delta-rules.md` | replaced by `hugo --skill builder` and `--skill reviewer`, which hold the rules. Project rules stay in `AGENTS.md` |
| `templates/job.md` | the brief: task id, paths, REPORT. Criteria come from `talita task show` |
| `start-reviewer.sh` | replaced: `session new`, `crew join` once per run, `crew_tab`, `herdr agent start` |
| `unit.sh`: `/clear` the reviewer | kept, between tasks |
| `unit.sh`: `build` → `spawn.sh` | replaced by S4's start and prompt: `crew_tab` with the job's `TALITA_*` env, `herdr agent start`, `herdr agent prompt` with the skill prompt, then `await.sh`. The talita session and `crew join` before it, `crew leave` after (S5, S6) |
| `unit.sh`: `review` → prompt, `await.sh`, `VERDICT` | kept as the prompt and the wait; the verdict is the task's status (Q2) |
| `unit.sh`: `verdict()` grep of `VERDICT:` | replaced: the status and `talita round show TSK04 <n>`. The OUTFILE's line stays for the human |
| `unit.sh`: `NO_PROGRESS` (same findings, same tree) | kept as a rule: the same finding descriptions on an unchanged tree, from `round show` and the tree digest |
| `unit.sh`: the `CREW_ROUNDS` loop and fix briefs | replaced by S4–S6; the findings are `FIND-nn` on the task |
| `unit.sh`: setting old files aside on re-run | dropped: the rounds are the task's record (ADR 0011) |
| `await.sh` | kept. Done means the file and the status (Q2). Exits 2, 3 and 5 map to the stall and dead paths above |
| `spawn.sh` | replaced. It passes the static `BUILDER_ENV` from `crew.env` and a fixed prompt, so it cannot carry a job's `TALITA_*` or the skill prompt. Not edited; the step is S4 |
| `changed.sh`, `commit.sh` | kept; the orchestrator runs them at S7 |
| `lib.sh` (`crew_tab`, `crew_status`, `crew_owner`, `crew_tree_state`) | kept |
| `templates/full-review.md` | kept outside the loop: a whole-project review has no task |
| round folder: `review<n>.md`, `report<n>.md`, `log`, tree digests | kept as scratch. Findings and verdicts live in the task (ADR 0011). The folder does not survive the machine |

## Choices made here

- Sequential: one task in flight, one build at a time (herdr-crew's rule).
- The reviewer keeps one session for the run; the builder gets a fresh one per job.
- Focus rotates through the five; a repeat is a warning in talita, not a refusal.
- The builder runs the gate as evidence; the reviewer runs it once (`delta-rules`).

## Open questions

**Ruled 2026-11-24 by the human: every recommended option.** Q1 A · Q2 C · Q3 A ·
Q4 A · Q5 C, with A until talita changes, and ADR 0005's line amended to the
crew rule (the orchestrator reviews in the two- and one-agent crews) · Q6 A ·
Q7 B · Q8 A · Q9 A then B · Q10 A · Q11 A · Q12 A · Q13 A · Q14 A. The text
below keeps the options as they were weighed.

**Q1 — Who commits, and on which branch?**
- (A) **Orchestrator**: one commit per task at `done`, on the checked-out branch, by name (`commit.sh`). *Recommended.*
- (B) Builder: contradicts the builder's rule of never committing.
- (C) talita or hugo: talita does not touch git; hugo writes nothing.

**Q2 — How does the orchestrator learn a job is done?**
- (A) The REPORT file alone (`await.sh` today).
- (B) The task's status alone, polled.
- (C) **Both**: the file, then the status it must show (`code-review` after a build; `done`, `remediating` or `needs-human` after a review). *Recommended.* The status is the record; the file says why.

**Q3 — Are the agents briefed by `hugo --skill <role>` alone?**
No: the skill has no task id, no REPORT or OUTFILE path. Options:
- (A) **Prompt: "Run `hugo --skill <role>` and follow it. Brief: <path>."** The brief holds the ids and paths; the content comes from talita. *Recommended.*
- (B) Paste the brief into the prompt.
- (C) Keep herdr-crew's per-project rules files.

**Q4 — How does `TODO.md` give way to the task folder?**
- (A) **Now (O2)**: the orchestrator stops writing `TODO.md`; the tasks are the run's memory; `TODO.md` stays the human's notes. *Recommended for O2.*
- (B) At DF: the plan rows become tasks and `TODO.md` is retired (ADR 0011).

**Q5 — Does the orchestrator review in the 2- and 1-agent crews?**
Three things conflict. ADR 0005 says the orchestrator "does not execute, review or approve". The crew rule (task model § 11.2, unit 70) says it reviews. talita's review verbs need the `reviewer` role (`role.wrong` otherwise), and a row the orchestrator holds in `executing` never shows an external round (`runtime.rs:157`).
- (A) A reviewer row with its own actor and session, on the orchestrator's model. The rounds are right; the pane counts an extra row and says "one per role" for a two-agent crew.
- (B) The orchestrator row only. The pane never shows the review stage.
- (C) **A talita change**: `crew assign` takes a review state for an orchestrator row. Needs talita's word. *Recommended.* Until then, (A). The human also rules whether ADR 0005's line is amended.

**Q6 — Which round limit binds?**
hugo's `round_limit` is 5 and the task's `review.max_rounds` is 5; only `--grant-rounds` raises the task's, and nothing sets it at create.
- (A) **The task's value binds. The orchestrator refuses to start a task whose `max_rounds` differs from `round_limit`, and names both.** *Recommended.*
- (B) talita sets `max_rounds` at create (a new flag; talita's word).

**Q7 — Who mints the orchestrator's session?**
- (A) The orchestrator, at S0, and prefixes every call. Today's skill.
- (B) **hugo, at `s`**: `session new` writes nothing, and the id goes into the tab's env. The agent never handles it. Needs a hugo change in O2 and a line in ADR 0015. *Recommended.*

**Q8 — What happens to a resumed or reopened task?**
It returns to `executing` with its old claim. Neither the queue nor a takeover picks it up: claimable is `approved`-only, and a takeover needs an expired lease. A resume within two hours of the take is stuck.
- (A) **`task resume` and `task reopen` release the claim**, so the next run takes the task fresh. A core gate change, an ADR 0020 amendment. *Recommended.*
- (B) The queue offers executing tasks with an expired lease as takeovers.
- (C) The human starts a new run with the task.

**Q9 — The reviewer is gone between `round file` and its verdict.**
Only the session that filed the round may give the verdict (`review.round_owner`). A replacement cannot move the task.
- (A) **Re-prompt the same reviewer if its pane lives.** *Recommended first.*
- (B) **Stop for the human** if it does not. *Recommended second.*
- (C) A talita verb to re-file under a new session: a talita change, later.

**Q10 — After `needs-human`, stop the run or continue?**
The shipped skill moves on; the design's words say "stop for the human".
- (A) **Continue with the next claimable task.** The escalated task waits, and its dependents stay unclaimable. A blocked agent still stops the run. *Recommended.*
- (B) Stop the run at the escalation.

**Q11 — The orchestrator's pane after the run stops.**
herdr names the orchestrator, so `s` refuses while its pane is up.
- (A) **The pane stays up; the human closes the tab to start again.** *Recommended.*
- (B) The orchestrator closes its own tab at `run stop`; the report is then gone from view.

**Q12 — Is `talita` on PATH?**
The skills call `talita`. `make install` installs hugo and raul, not talita.
- (A) **Add talita to `make install`, with a PATH note as hugo's has.** *Recommended.* A Makefile change; the human's word, or O2's.
- (B) Leave it to the human's own install.

**Q13 — Who runs the gate, and what if it fails after `done`?**
The builder runs it as evidence, the reviewer once, the orchestrator again before the commit. A failure after `done` has no talita path back without the human (`reopen`).
- (A) **Keep the three runs; a failure at S7 stops the run for the human.** *Recommended.*
- (B) Run the gate only at S7.

**Q14 — Who clears a run whose orchestrator died?**
A record with no held lease is never stale (`stale()` in `crates/talita/src/runtime.rs`;
ADR 0018), and `run stop` is the orchestrator's act. A dead run with no holds
therefore blocks every later `s` until someone clears it.
- (A) **The next orchestrator, at S0, when herdr names no other orchestrator agent:
  `run stop --why "previous orchestrator gone"`.** *Recommended.* The human's path is
  to close the old tab and press `s`.
- (B) A talita change: a record with no held lease goes stale after a fixed time.
  Amends ADR 0018 and its stale rule; talita's word.
