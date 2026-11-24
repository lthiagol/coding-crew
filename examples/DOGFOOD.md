# Dogfood — what talita hurt

The five sandbox projects under `examples/` were written by talita alone, with
the binary built from this branch: Pantry and Ledgerline in the first run,
Tidewatch, Quillnote and Railyard in the second (`## Second run`). Every entry
below is something a caller would not expect, or a rule that cost a call to
find. A verb that writes was not re-run for this list; its entry quotes the
run that hit it.

## The review budget

### The fifth review round cannot be sent back
- command: `talita --project examples/pantry --as nadia --role reviewer --session ses-nadia-pantry task request-changes TSK05`
- happened: Not re-run (it writes). Refused, exit 4, `review.budget_left`: "the review round budget is exhausted"; remedy `talita task show TSK05`. The round's high finding stays open and the reviewer escalates it unfixed. Ledgerline's TSK03 ends the same way: five rounds filed, FIND-05 open, `needs-human`.
- expected: the fifth round to go back for a fix like the first four, or the refusal to name `task escalate` as the only move left. A caller following the cycle reads the budget's last round as one more round.
- ruled: the human, 2026-11-23 — when the budget is spent, the last round escalates with its findings open; that is the intended behaviour, and no code change follows.
- severity: low

## The human's name

### The approving human's name is accepted and dropped
- command: `talita --project examples/ledgerline --as thea --role creator --session ses-thea-ledgerline task approve TSK01 --instructed-by hugo`
- happened: Not re-run (it writes). Accepted, exit 0, `data.instructed_by: "hugo"`; the journal event is a bare `approved` with no note, and the task file records only `planned_by: ses-thea-ledgerline`. Every approval in both projects used `--instructed-by` — Pantry's ten, Ledgerline's nine — and all 19 `approved` events have an empty note.
- expected: ADR 0015 says the human's name is recorded in the journal event's note, so the approval should read "instructed by hugo" after the call. The command's own output says it was carried.
- fixed: unit T11a — `task approve --instructed-by <human>` now appends an `approved` event with the note `instructed by <human>`. The approvals written before it keep their empty notes.
- severity: high

## Remedies

### `task finish` names the wrong next call
- command: `talita --project examples/pantry --as ravi --role runner --session ses-ravi-pantry task finish TSK03`
- happened: Not re-run (it writes). Refused, exit 4, `criteria.proven`: "ACCEPT-01 failed without a reason"; remedy `talita evidence run TSK03 <ACCEPT>`. The evidence had just been run and failed; running it again records the same failure.
- expected: `talita criterion explain TSK03 ACCEPT-01 --failure-reason …`, the only call that clears the gate.
- fixed: unit DF1 — the remedy names the criterion the failing gate's detail names and the call that clears it: `criterion explain … --failure-reason` when the criterion failed without one, `evidence run` when it has no evidence, `criterion undefer` when it is deferred without a deferral. It never names `evidence run` for a criterion that already failed.
- severity: medium

### `task escalate` names the call it just refused
- command: `talita --project examples/pantry --as nadia --role reviewer --session ses-nadia-pantry task escalate TSK05`
- happened: Not re-run (it writes). After jo's fourth re-review, refused exit 4: `review.budget_exhausted` ("the review round budget is not exhausted") and `review.round_current` ("the latest external round was not filed since the task entered code-review"); remedy `talita task escalate TSK05`, the same call, refused again.
- expected: the remedy to name the round that must be filed first (`talita round file TSK05 --input <round>`), or `task show`; not the verb whose own gates just refused.
- fixed: unit DF1 — `review.budget_exhausted`, `review.verdict_changes_needed`, `review.round_current` and `review.round_owner` name `talita round file <TSK> --input <round>` for `escalate` as for `pass-review` and `request-changes`; no refusal names the verb it refused.
- severity: medium

## Identity

### A human is told to act as an agent
- command: `talita --project examples/pantry --as hannah --role human task approve TSK09`
- happened: Not re-run (it writes). `actor.not_agent`, exit 5, "task approve is an agent's act"; remedy "act as an agent: --role creator, runner or reviewer".
- expected: the route ADR 0015 gives a human: ask the creator to run `task approve TSK09 --instructed-by hannah`. The remedy tells the human to be the thing talita just refused.
- fixed: unit DF1 — a human refused `task approve` is pointed at `talita task approve TSK09 --instructed-by hannah` (and the matching call for `resume`, `reopen` and `question answer`); every other role-gated verb names the role that owns it.
- severity: low

### `project init` asks for identity the table calls "any"
- command: `talita project init --path examples/pantry`
- happened: Not re-run (it writes). `identity.missing`, exit 5: "a write needs its actor (--as or TALITA_ACTOR), role (--role or TALITA_ROLE), session (--session or TALITA_SESSION)".
- expected: the folder. commands.md marks the verb "Who: any"; the conventions say every write carries identity, and the table does not repeat it.
- fixed: unit DF1 — `project init` needs no identity: it writes no task and no journal, so there is nothing to attribute (ADR 0015, the human's ruling of 2026-11-23).
- severity: low

## Status and overlays

### A cancelled task is not a cancelled status
- command: `talita --project examples/ledgerline task list --status cancelled`
- happened: `input.invalid`, exit 2, "status `cancelled` is not one of draft, groomed, approved, executing, code-review, remediating, needs-human, done". TSK11, cancelled by hugo, is listed as `groomed` with an `overlays.cancelled` entry, and raul's counts put it under groomed.
- expected: a filter for the terminal overlay — the model makes cancellation an overlay — so a caller tallying by status does not count a terminal task as live work.
- fixed: unit DF1 — `task list --overlay cancelled` lists it; `--overlay blocked` and `--overlay deferred` filter the other two, and `--status cancelled` names `--overlay cancelled` in its remedy.
- severity: low

### `start_readiness` does not see the overlay that stops the claim
- command: `talita --project examples/ledgerline task show TSK08`
- happened: `derived.claimable` is false, but `derived.start_readiness.gates` names only `task.claimed` ("task is not claimed"); the deferred overlay is absent, so the readiness list reads as "claim it and go". `claim take TSK08` then refuses `claim.not_claimable`: "TSK08 cannot be claimed: the task is deferred" (a write; not re-run).
- expected: the readiness list to name `overlay.clear`, the gate `claim take` reports, so the two derived answers agree.
- fixed: unit DF1 — `derived.start_readiness.gates` carries `overlay.clear` with core's detail ("the task is deferred"), from the same rule `claim take` reads.
- severity: low

## The journal

### The journal records the role, not the actor
- command: `talita --project examples/pantry journal show --task TSK06`
- happened: every event's `by` is `creator`, `runner`, `reviewer` or `human`; jo's four resolutions and four re-reviews read `"by": "runner"`, the same as ravi's, and `ses-jo-pantry` appears zero times in `activity.json`.
- expected: the actor id, as `claim.by`, `round.reviewer` and a note's `author` record it. ADR 0018 defers the actor and session to later; until then the record cannot say who fixed FIND-01.
- closed: unit DF2, by ruling (the human, 2026-11-23) — the actor is not recorded. The role and the model are what matter, and the harness is an extra: `by` stays the role word, and every journal event gained an optional `model` and `harness`, from `--model`/`--harness` or `TALITA_MODEL`/`TALITA_HARNESS`. The task file still carries the actor, so FIND-01's fixer is named there, not in the journal.
- severity: low

## Second run (2026-11-24)

`talita` below is `target/debug/talita` from `cargo build -p coding-crew-talita`,
run from the repository root. The projects are Tidewatch, Quillnote and Railyard.
Entries marked *re-run* are refusals or reads: they write nothing, and the
command works on the projects as they stand. The fifth-round refusal
(`review.budget_left`) reproduced in Quillnote's TSK01 and is the entry above;
it is not repeated.

### Work and evidence

#### `step done` straight from `failed` leaves no trace that the step failed
- command: `talita --project <scratch> --as ravi --role runner --session ses-ravi-scratch step done TSK02 STP-01`
- happened: Re-run on a scratch project where `step fail TSK02 STP-01 --deviation "broke"` had just run. Exit 0, `status: "done"`. The failure's deviation text stays, but the status no longer says the step failed, and `task finish` then reads it as settled (`steps.settled` accepts `done`).
- expected: `step done` on a failed step needs `step begin` first, the retry path in the next entry; or the status keeps a record that the step failed.
- ruled: the human, 2026-11-24 — `step done` on a failed step is refused; the remedy names `talita step begin <TSK> <STP>` (retry) or `talita step skip <TSK> <STP> --deviation <why>`.
- fixed: unit DF4 — `step done` on a failed step is refused with `status.wrong` (4), and the remedy names both calls.
- severity: medium

#### `task finish` refused for a failed step names `step done` with a placeholder
- command: `talita --project examples/tidewatch --as ravi --role runner --session ses-ravi-tw task finish TSK04`
- happened: Re-run (refused, nothing written). Exit 4, `gate.refused`: `steps.settled` ("every step must be settled"), `criteria.proven` ("ACCEPT-03 failed without a reason"), `self_round.filed`. Remedy `talita step done TSK04 <STP>`: the `<STP>` is not filled in, and `step done` is the call from the entry above. It names one gate's call; the failed criterion needs `criterion explain`.
- expected: the remedy names the failed step and what settles it (`step skip --deviation <why>`, or a fix and `step begin`), and names the criterion's call for `criteria.proven`.
- fixed: unit DF4 — `task finish` names one call per failing gate, with the failed step's id (`step begin` retry or `step skip --deviation`) and `criterion explain` for `criteria.proven`, with no `<STP>` when the id is known.
- severity: medium

#### A passing command with no output reads as failed, and the output gives no reason
- command: `talita --project examples/tidewatch --as ravi --role runner --session ses-ravi-tw evidence run TSK01 ACCEPT-01`
- happened: Not re-run (it writes). The verification `grep -q surge_limit …` exits 0 and prints nothing. Exit 0, `status: "failed"`, `output: []`. The stored evidence is `exit_code: 0`, `summary: ""`, read as a failure as task-model § 5 says. Nothing in the output says why; the next `task finish` gate says "ACCEPT-01 failed without a reason". Re-run the read: `talita --project examples/tidewatch task show TSK01 | jq '.data.task.acceptance_criteria[0].evidence[0]'`.
- expected: the rule stays. The output says why, for example "exit 0 with no summary: pass --summary", so a caller does not explain a pass.
- fixed: unit DF4 — the rule stays, and the run's output carries `warnings` `evidence.summary_empty` ("exit 0 with no summary is a failure: pass --summary …").
- severity: medium

#### A passing criterion keeps the failure reason from before it passed
- command: `talita --project examples/tidewatch task show TSK01 | jq '.data.task.acceptance_criteria[0] | {status, failure_reason}'`
- happened: Re-run (read). `status: "passed"` with `failure_reason: "grep printed nothing; exit 0 with no output counts as a failure"`. The reason came from `criterion explain`, before a later `evidence run --summary` passed the criterion.
- expected: a passing criterion carries no failure reason, or the reason clears when a later evidence passes.
- fixed: unit DF4 — a criterion whose passing evidence is written clears its `failure_reason`; a file already holding a reason keeps it until its next pass.
- severity: low

#### An override on a criterion that already passes marks its proof manual
- command: `talita --project examples/quillnote task show TSK03.1 | jq '.data.task.acceptance_criteria[] | select(.id=="ACCEPT-01") | {status, last: .evidence[-1]}'`
- happened: Re-run (read). ACCEPT-01 passed by its command. Then `evidence override TSK03.1 ACCEPT-01 --reason "manual check"` exited 0 and appended `manual: true`, `command: ""`. The criterion reads `passed`, and its last evidence is the bypass, which raul marks as manual.
- expected: an override refused for a criterion already proven by command, since the bypass is for a criterion that cannot be proven; or the command's proof stays last.
- ruled: the human, 2026-11-24 — an override on a criterion already passed by its command is refused; the refusal says it is already proven.
- fixed: unit DF4 — `evidence override` on a criterion a command has passed is refused with `status.wrong` (4), "already proven by its command".
- severity: low

#### `evidence run` on a criterion with no verification names the bypass
- command: `talita --project examples/railyard --as vic --role runner --session ses-vic-ry evidence run TSK02 ACCEPT-01` (run while ACCEPT-01 was the promoted placeholder; to reproduce, start a promoted task whose criterion still has `verification: ""`)
- happened: Refused, exit 2, `input.invalid`: "TSK02 ACCEPT-01 has no verification to run". Remedy `talita evidence override TSK02 ACCEPT-01 --reason <why>`: a manual bypass, which the spec calls debt. The fix is `criterion revise … --verification <cmd>`.
- expected: the remedy names `talita criterion revise TSK02 ACCEPT-01 --verification <cmd>`.
- fixed: unit DF4 — `evidence run` on a criterion with no verification names `talita criterion revise <TSK> <ACCEPT> --verification <cmd>`, not the override.
- severity: medium

#### A promoted task's placeholder criterion has no verification, and `groom` accepts it
- command: `talita --project examples/quillnote task show TSK07`
- happened: Re-run (read). `item promote` made TSK07 in draft with one criterion: `ACCEPT-01`, the item's title as its description, `verification: ""`. `task groom TSK07` exited 0 (`criteria.present`). Railyard's promoted tasks began the same way; each needed `criterion revise` on ACCEPT-01 before its evidence could run.
- expected: promotion writes no criterion, or groom refuses a criterion with no verification.
- ruled: the human, 2026-11-24 — `task groom` refuses a criterion with no verification; `item promote`'s placeholder criterion stays.
- fixed: unit DF4 — `task groom` refuses a criterion with no verification (`criteria.verified`), and the remedy names `talita criterion revise <TSK> <ACCEPT> --verification <cmd>`.
- severity: medium

### Claims and concurrency

#### `claim renew` by a non-holder points at `claim take`, which a live claim refuses
- command: `talita --project examples/tidewatch --as jo --role runner --session ses-jo-tw claim renew TSK04`
- happened: Re-run (refused, nothing written). Exit 5, `claim.not_holder`; remedy `talita claim take TSK04`. While ravi's lease runs, `claim take` by jo is refused `claim.held` ("the task is held by a live claim of ravi", as in the run's race). Today it is refused `claim.not_claimable` because TSK04 is blocked.
- expected: a remedy that names the holder, or `talita claim show TSK04`, not a take that a live lease refuses.
- fixed: unit DF5 — `claim renew` and `claim release` by a non-holder are `claim.not_holder` (5) naming the holder; the remedy is `talita claim show <TSK>`, or `claim take` when nothing holds the task.
- severity: low

#### A non-holder's `finish` is refused with the holder's gate, and the remedy asks for work it cannot do
- command: `talita --project examples/quillnote --as ines --role runner --session ses-ines-qn task finish TSK03.1`
- happened: Re-run (refused, nothing written). Exit 4, `gate.refused`. Gates: `self_round.filed`, and `task.claimed` twice ("the task is claimed by someone else"; "only the claiming session may finish the task"). Remedy `talita round file TSK03.1 --kind self --focus spec-compliance --verdict ok`. On the first run the remedy was `talita step done TSK03.1 <STP>`. A non-holder can file neither a self round nor a step, since both are the working runner's.
- expected: exit 5, `claim.not_holder`, naming the holder (tomas) as the other claim verbs do.
- fixed: unit DF5 — a `task finish` by someone other than the claim's holder, in its session, is `claim.not_holder` (5) naming the holder, with no gate list.
- severity: medium

### Adoption

#### `item promote` has no `--parent`, so an item cannot become a child task
- command: `talita --project examples/railyard --as nell --role creator --session ses-nell-ry item promote ITM10 --parent TSK02`
- happened: Re-run (usage error, nothing written). Exit 2: "unexpected argument '--parent'", with clap's tip "a similar argument exists: '--project'". Adopting TODO.md's "Write the config reader's tests", a child of the cap task, left ITM10 open: the child had to come from `task create --parent TSK02`, and nothing links it to the item.
- expected: `item promote --parent <TSK>`, or the child's `created` event names `from ITM10`, as a top-level promotion's does.
- fixed: unit DF6 — `item promote <ITM> --parent <TSK>` makes the task the child `TSK<nn>.<m>` under `task create --parent`'s gates (a cancelled parent is `task.cancelled`, a missing one `task.not_found`); the child's `created` event reads `from <ITM>`.
- severity: medium

### Runtime and reads

#### `run start` takes one `--task` per id; the doc says a list
- command: `talita --project examples/railyard --as oz --role orchestrator --session ses-oz-ry run start --task TSK01 TSK02`
- happened: Re-run (usage error, nothing written). Exit 2, "unexpected argument 'TSK02'"; remedy `talita --help`. commands.md § run says `run start [--task <TSK>…]`. The working form is `--task TSK01 --task TSK02`.
- expected: the list the doc gives, or the doc shows the repeated flag.
- fixed: unit DF5 — commands.md § run shows the repeated flag, `run start [--task <TSK>]…`, one id each (`--task TSK01 --task TSK02`); the binary is unchanged.
- severity: medium

#### `project show` from a subfolder suggests a nested `project init`
- command: `talita --project examples/quillnote/src project show`
- happened: Re-run (read). Exit 3, `project.not_found`: "examples/quillnote/src holds no coding-crew folder". Remedy `talita project init --path examples/quillnote/src`, which would make a second `coding-crew/` inside the first project's source tree.
- expected: a remedy that names the project root, such as `--project examples/quillnote`.
- fixed: unit DF5 — a `--project` root that holds no `coding-crew/` is `project.not_found` with a remedy naming the enclosing project's root as `--project`; `project init` is named only when no project encloses it.
- severity: medium

#### `task list --lane` and `--needs-you` are not in the binary
- command: `talita --project examples/tidewatch task list --lane human --needs-you`
- happened: Re-run (read). Exit 2, "unexpected argument '--lane'". Clap's tip: "a similar argument exists: '--harness'". `talita --help` shows no `lane` noun, and `talita task list --help` no such flags. The § task table (commands.md:99) lists `--lane` and `--needs-you` without saying they are not built; the header at commands.md:3 says so. A caller who reads only the table reaches for them. The gap is recorded in TODO.md's follow-ups (2026-11-23, T6 review D2).
- expected: the § task table marks `--lane`, `--needs-you` and the `lane` noun as not built, as the header does.
- fixed: unit F2 — `task list --lane` and `--needs-you` are built, and `task show` fills `derived.lane` and `derived.review_state`; the command above exits 0.
- severity: low

#### `talita re-review` at the top level
- command: `talita --project examples/quillnote --as ines --role runner --session ses-ines-qn re-review TSK04`
- happened: Re-run (usage error, nothing written). Exit 2, "unrecognized subcommand 're-review'"; remedy `talita --help`, which does not show the verb. The verb is `talita task re-review`.
- expected: a remedy that names `talita task re-review <TSK>`.
- fixed: unit DF5 — a task verb typed at the top level is `usage.invalid` (2) with the remedy `talita task <verb> <TSK>` (no `<TSK>` for `create` and `list`), for every task verb.
- severity: low

#### `queue add --before` reports a task that exists as not found
- command: `talita --project examples/quillnote --as kai --role orchestrator --session ses-kai-qn queue add TSK02 --before TSK05`
- happened: Re-run (refused, nothing written). Exit 3, `task.not_found`: "the queue does not hold TSK05". TSK05 exists (approved, not queued). `queue add TSK02 --before TSK99` gets the same code and message for a task that does not exist.
- expected: a message that tells "not in the project" from "not in the queue", for example a code of its own for the queue case.
- fixed: unit DF5 — `queue add --before` on a task the project holds but the queue does not is `queue.not_queued` (3), remedy `talita queue add <TSK>`; a task the project does not hold is `task.not_found`, "is not in the project".
- severity: low
