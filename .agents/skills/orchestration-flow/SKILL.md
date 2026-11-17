---
name: Orchestration Flow
description: Three agents, one flow — the orchestrator session takes the human's requests and decides, dispatching builder and two-axis reviewer panes through Herdr, each an opencode instance on its own model. Load this when running a build loop in this repository.
metadata:
  opencode/autoinvoke: false
---

# Orchestration flow

Canonical for loop mechanics: hub-and-spoke, the caps, who commits. `AGENTS.md`
points here instead of restating them.

This is the flow for **building** this repository. It is not a description of
the project: the project has its own docs, and the agents read those.

## The state file

`TODO.txt` at the repository root is the orchestrator's memory. It is plain
text on purpose — no markup, so it stays readable in a terminal and in a diff.

It records three things: **what was done**, **what happened**, and **what is
open**. It exists so the work survives a session: a fresh orchestrator reads
`AGENTS.md`, the docs, and this file, and continues.

The orchestrator reads it at the start of a session and updates it when a unit
closes. It is committed, so the next session finds it.

```
TODO.txt
  HOW TO RESUME   the order to read things, and where the last findings are
  WHERE WE ARE    the unit in flight and the state of its rounds
  DONE            what is built
  HAPPENED        what went wrong, and the rule that came out of it
  OPEN            what is not built, each with enough detail to start
  NEXT            the order to do it in
```

The findings of a review round live in `/tmp`, which is not durable. What
matters from them is copied into `TODO.txt` before the round is closed.

## The three agents

| Agent | Where it runs | Does | Never does |
|---|---|---|---|
| **Orchestrator** | the human's session — where the request is typed, and where every agent reports back | Designs; decides every transition; opens the panes; runs the full gate; commits after a clean review | **Edits code, tests or scripts itself** — even small ones. Any code change goes to a builder, then a reviewer. Runs no product work |
| **Builder** | its own pane, in its own tab | Implements exactly the kickoff; runs the project's gate; reports to the orchestrator | Commits; starts other work; prompts other agents; decides next steps |
| **Reviewer** | its own pane, in its own tab — two panes for a first review (CODE, SPEC), one for a delta review | Reviews one axis (CODE or SPEC) against the kickoff and the project's rules, or re-checks the prior findings and the changed hunks in a delta review; writes its findings; runs **targeted probes** | Modifies code or docs; commits; prompts other agents; runs the full gate or long builds; gives the round verdict |

The orchestrator is the only agent the human talks to, and the only one the
others report to. It is the session that decides what happens next.

These three mirror the project's roles — `orchestrator`, `runner`, `reviewer`
— with the loop's own spelling: the loop's **builder** plays the project's
`runner`, and there is no separate fixer: a builder on its second round is
the fixer.

The review itself is [code-review](../code-review/SKILL.md): the CODE and SPEC
axes, the findings shape, the probe budget, and the delta review.

## Topology: hub-and-spoke

The orchestrator session is the hub. Builders and reviewers never talk to each
other — **every agent replies only to the orchestrator**, and the
orchestrator alone decides what happens next (open reviewer panes, send
findings to a builder, ask the human). An agent must never prompt another
agent's pane.

Each dispatch is a **fresh opencode session** — fresh context is intentional;
the repository's docs are the memory, not the conversation.

**Every dispatch is a fresh session: builder and reviewer alike.** Continuity
is the findings file and the diff — never an agent's memory. A builder that
wrote the code never fixes it (the session that produced the code is not the
session that repairs it), and a reviewer that reviews the change from a fresh
context re-derives it instead of accepting the builder's account of it.

The findings files and snapshots are the thread between the rounds:

| round | axis files | merged file | snapshot |
|---|---|---|---|
| 1 | `/tmp/opencode/review1/round1-code-findings.md`, `/tmp/opencode/review1/round1-spec-findings.md` | `/tmp/opencode/review1/round1-findings.md` | `/tmp/opencode/review1/round1-snapshot/` |
| fix N | `/tmp/opencode/review<N>/round<N>-delta-findings.md` | `/tmp/opencode/review<N>/round<N>-findings.md` | `/tmp/opencode/review<N>/round<N>-snapshot/`, delta `/tmp/opencode/review<N>/round<N>-delta.diff` |

The orchestrator takes the snapshot after the builder reports and before either
review pane opens: `scripts/review-snapshot.sh /tmp/opencode/review<N>/round<N>-snapshot`.
A fix round also generates
`scripts/review-snapshot.sh --diff <previous-snapshot> <current-snapshot> > /tmp/opencode/review<N>/round<N>-delta.diff`.
The snapshot copies the changed and untracked files and records the changed,
added, and deleted paths in `manifest.txt`; `--diff` prints the manifest
changes and the file-tree diff. The script never stages or edits anything.
That is the delta baseline: `git diff` still holds round 1's change alongside
the fix.

Each axis reviewer writes its own file. The orchestrator merges the axis files
into the round's merged file and decides the verdict; the next builder reads the
merged file, and the next delta reviewer verifies every finding in it. A `low`
finding is recorded there and does not open a round.

The files are better than memory: a reviewer with memory can be talked into
accepting a rename as a fix, while a reviewer reading the finding on disk
re-checks it.

For now the findings live in `/tmp`, outside the repository. That is a stopgap:
when coding-lanes runs its own work, the findings belong inside the task.

## Herdr mechanics

All multi-agent work runs through Herdr, in the orchestrator's own workspace.
Commands that start with `scripts/` are relative to the repository root; run
them from there.

| what | command |
|---|---|
| open the Orchestration workspace | `herdr workspace create --cwd <dir> --label "Orchestration" --no-focus` |
| list workspaces | `herdr workspace list` |
| close the Orchestration workspace | `herdr workspace close <ID>` |
| open a tab | `herdr tab create --workspace <ID> --label "<label>" --cwd <dir> --no-focus` |
| split a pane | `herdr pane split <PANE> --direction right --ratio 0.5 --cwd <dir>` |
| start an agent in a pane | `herdr pane run <PANE> env OPENCODE_CONFIG=<role-config> opencode --standalone` |
| dispatch a prompt and wake when the turn completes | `scripts/herdr-wake.sh <PANE> "<prompt>" --timeout 1800` — waits for a settled baseline, retries `agent_not_found`, submits one line, then waits for `completion_seq` to advance or a working turn to settle twice; the timeout bounds the whole command, including every herdr call and the final pane read |
| send a report without waiting | `herdr agent prompt <ORCHESTRATOR-PANE> "<report>"` — no `--wait`; text and Enter as one ordered submission |
| read a pane | `herdr pane read <PANE> --lines 40` |
| close a tab | `herdr tab close <TAB_ID>` |

**All agent tabs live in the Orchestration workspace**, not in the
orchestrator's own workspace. See the next section.

`herdr pane send-text` types literally: a newline inside the text submits
early and strands the rest in the input box. `scripts/herdr-wake.sh` flattens
CR, LF, tab and form feed to spaces, so the prompt it dispatches is always one
line and cannot move focus or clear the input.

### A kickoff that lands

Dispatch through the tested wake command:

```sh
scripts/herdr-wake.sh <PANE> "<the kickoff, one line>" --timeout 1800
```

It waits for herdr to recognise the agent (`agent_not_found` is *not ready*,
not a failure, and is retried until the timeout) and for two consecutive
`idle|done` reads before capturing the baseline. Two reads close the window
where the next turn has started but herdr still reports the previous state.
It then submits the prompt with `herdr agent prompt` (text and the encoded
Enter go as one ordered submission), flattens CR/LF/tab/form feed so the text
cannot submit early or move focus, and polls `herdr agent get`. It prints the
final pane output and exits:

| exit | meaning |
|---|---|
| 0 | the turn completed: `completion_seq` advanced, or a working turn settled twice without advancing it (fallback) |
| 2 | the agent is blocked |
| 3 | the whole-command bound expired |
| 4 | usage error, or a Herdr failure other than `agent_not_found` |

Exit 0 normally prints the final pane output on stdout. The final
`herdr pane read` runs under the budget left in the command. If that read
fails or no budget remains, the wake, blocked or timeout exit is kept, the
missing output is reported on stderr, and stdout stays empty — a failed or
skipped read does not change the exit.

Herdr's `unknown` status means an agent is present but Herdr cannot classify
it confidently; it does not prove completion. The command keeps waiting and
the whole-command bound ends the wait (exit 3); it never treats `unknown` as
a wake or as a settled baseline.

`--timeout` is a hard bound in seconds for the whole command, including a
hung `herdr agent get`, `herdr agent prompt`, or final `herdr pane read`; the
default is 1800. Every Herdr call runs under the budget left at that point.
`--interval` is the poll interval; the default is 1. If the pane is already
working, the command waits for that turn to settle before capturing the
baseline. It reads the pane with `herdr pane read --source recent-unwrapped`
so soft wraps do not break logical lines; `--lines N` changes how much it
prints. The command needs bash, python3, herdr, and `timeout` and `sleep`
from coreutils.

Do not replace the wake with `herdr agent wait --until idle`: on herdr 0.9.5
it can run to its full timeout after the agent already finished (see the defect
record below).

The agent reports in the other direction with `herdr agent prompt` and no
`--wait`, because the orchestrator is busy and the summary must not block it:

```sh
herdr agent prompt <orchestrator-pane> "done: <one-line summary>"
```

### Versioned defect record

Re-check this on every herdr upgrade.

| component | version | observed |
|---|---|---|
| herdr | 0.9.5 | `agent wait <PANE> --until idle --timeout 120000` missed a turn that finished in 4–8 seconds and ran the full 120 seconds |
| opencode | 2.3.0 | the regular TUI lifecycle hook reported the pane idle or done while the wait still hung |

**`completion_seq` on herdr 0.9.5.** Herdr's schema describes it as the
current idle transition completed work, independently of who has viewed it.
It is absent while a turn runs and advances when an idle transition completes
work. The script captures the baseline from settled work, then treats a
present sequence greater than that baseline as the completion signal. A null
baseline means no earlier completed turn, so any later sequence is a wake.

**Working-to-settled fallback.** A turn can end early — the agent stops
mid-thought — and may not advance `completion_seq`. If the script observed
`working` after dispatch and the pane then settles `idle|done` twice in a row
without the sequence advancing, it exits 0 and says `settled after working
without completion_seq advancing` on stderr. Two reads match the baseline
rule: a single idle can be a transient mid-turn and must not wake. A pane
that never reached `working` is not a completion; it times out. `unknown`
resets the settled count because it does not prove completion.

**Blocked precedence.** `blocked` is checked before completion. An agent that
finishes a turn and then raises a question or approval satisfies both; the
script exits 2 and prints the pane output when the budget allows, because a
blocked agent needs the orchestrator. It never reports that turn as an
ordinary wake.

The script's test is `scripts/test-herdr-wake.sh`, run from the repository
root. It puts a fake `herdr` on `PATH`, needs no live session, and covers
readiness, baseline settling, blocked precedence, prompt flattening, the
one-line prompt, `unknown`, timeout bounds including a hanging `agent get`,
`agent prompt`, and final `pane read`, unwrapped pane reads, and the two-read
fallback. It bounds every case with `timeout` so a missing script timeout
fails instead of hanging. The fake writes herdr's error JSON to stderr and
exits 1, like herdr 0.9.5. `make test-scripts` runs it; `make ci` includes it,
and CI runs `make ci`.

### The pane recipe

The panes run the **regular opencode**, not `mini`. Mini does not run the TUI
plugin, so herdr cannot see its state at all — it falls back to reading the
screen, and the manifest's rules (`esc to interrupt`) do not match what mini
draws (`esc interrupt`). The regular TUI is the only client that reports.

The regular TUI takes no `--model` and no `--agent`. It reads them from a
config file, so each pane gets one:

```json
{
  "default_agent": "builder",
  "agents": {
    "builder": {
      "mode": "primary",
      "description": "Implements one unit of work",
      "model": "deepseek/deepseek-flash#high"
    }
  }
}
```

and the pane starts with it:

```sh
herdr pane run <PANE> env OPENCODE_CONFIG=flow/builder.json opencode --standalone
```

`--standalone` gives the pane its own server, so the config file is read; the
shared server ignores a client's `OPENCODE_CONFIG`. The cost is one server
per pane.

The config **merges** with the global one, and the role file names the agent
and its model directly, so each pane runs the model its role config holds.

**What this buys**: the state is authoritative, and each role runs the model
and the variant its config names.

## What herdr can tell us, and what it cannot

Herdr recognises an agent and watches its terminal output to decide whether
it is `working`, `idle`, `blocked`, `done` or `unknown`. The rules live in a
per-agent manifest — for opencode, they look for `esc to interrupt`,
`press esc to interrupt`, a line matching `.*opencode.*esc interrupt`, or a
progress bar of four or more blocks.

**`opencode mini` shows `esc interrupt`, and its status line never contains
the word `opencode`, so no rule matches.** Herdr falls back to
`default_known_agent_idle_fallback` and reports the agent as `idle` whether
it is working, waiting or finished.

| what | use it for |
|---|---|
| `scripts/herdr-wake.sh <PANE> "<text>" --timeout SECONDS` | the dispatch and wake path — waits for two settled reads, retries `agent_not_found`, flattens the prompt, polls `agent get` until `completion_seq` advances or a working turn settles twice, prints the final pane output; `SECONDS` bounds the whole command |
| `herdr agent prompt <PANE> "<text>"` | submit without waiting — one ordered submission, text and Enter together; used for reports to the orchestrator |
| `herdr agent get <PANE>` | the agent's state and `completion_seq`; what the wake command polls |
| `herdr agent wait <PANE> --until idle --timeout MS` | diagnosis only on herdr 0.9.5 — it can miss a fast turn; not the wake signal |
| `herdr pane read <PANE> --lines N` | what the agent is actually showing, and its report |
| `herdr agent explain <PANE>` | why herdr thinks an agent is in that state — the rule it matched, or the fallback |
| `herdr agent list` | the agents and their states, with a `state_change_seq` counter |
| `herdr notification show "<title>" --body "..." --sound done` | raising a toast for the human — use it when a unit closes |

**Herdr's state is a signal only for the regular TUI.** It runs the
lifecycle plugin, so `working`, `idle` and `blocked` are the agent's own
reports. A **mini** pane has no plugin: herdr reads the screen, finds no
matching rule, and says `idle` whatever the agent is doing — so a mini pane
never goes `working`, and `--wait` on it returns `agent_prompt_stalled` even
though the prompt was delivered and answered.

Herdr's own agent-facing documentation is `herdr --skill`.

## The Orchestration workspace

One workspace per loop holds every agent pane, so the human can watch the
whole loop in one place — and so a tab can never land in the workspace that
happened to be focused.

1. At the start of a loop the orchestrator creates it:

   ```sh
   herdr workspace create --cwd <dir> --label "Orchestration" --no-focus
   ```

   The id comes from the command's output, or from `herdr workspace list`.
2. **Every** agent tab is created there, with `--workspace <orchestration-id>`:

   ```sh
   herdr tab create --workspace <orchestration-id> --label "build-<topic>" --cwd <dir> --no-focus
   ```

   `--cwd` sets the working directory, never the workspace. Omitting
   `--workspace` drops the tab into whatever workspace was last active, and
   the agent ends up somewhere the human is not watching.
3. The workspace lives as long as it has a tab: **closing the last tab
   closes the workspace with it.** So the loop creates it with the first
   agent pane and recreates it when the next round opens one — a round
   whose tab is closed is not a workspace that survives to the next round.
   At the end of the loop it is closed:

   ```sh
   herdr workspace close <orchestration-id>
   ```

The orchestrator's own session stays where it is. It receives the reports;
the Orchestration workspace holds the work.

## Naming

- Tabs: `build-<topic>`, `review-<topic>-code`, `review-<topic>-spec`,
  `review-<topic>-delta` (e.g. `build-m1-engine`,
  `review-m1-engine-code`, `review-m1-engine-spec`).
- Agents: `[a-z][a-z0-9_-]{0,31}` — e.g. `builder-m1`, `reviewer-m1-code`,
  `reviewer-m1-spec`, `reviewer-m1-delta`.

## Models

The reviewer must not run the builder's model. Different lenses surface
uncorrelated bugs — the same rule the project's own review rounds follow. The
two role models are different families; that difference, not a variant, is
what separates the lenses.

A first review's CODE and SPEC panes and a fix round's delta pane all run the
reviewer model. The axes are independent by context, not by model: each pane
gets a fresh session and its own findings file.

| role | model | variant |
|---|---|---|
| Orchestrator | the session's own model | — |
| Builder | `deepseek/deepseek-flash` | `high` |
| Reviewer | `minimax-coding-plan/MiniMax-M3` | `thinking` |

The two variants do not order against each other. `high` is an effort level on
`deepseek/deepseek-flash` (`none`, `low`, `high`, `max`); `thinking` is a
reasoning toggle on `minimax-coding-plan/MiniMax-M3` (`none`, `thinking`).
`high` says how hard the model works; `thinking` only that it reasons. Neither
name says the reviewer works deeper.

Both roles name a direct model ID and its variant: the builder runs
`deepseek/deepseek-flash#high` and the reviewer
`minimax-coding-plan/MiniMax-M3#thinking`. The variant travels in the model
string, so it is fixed by the role config and never has to be selected in a
session.

The role configs that name them live in the repository, under `flow/`:

```
flow/builder.json     default_agent builder  →  deepseek/deepseek-flash#high
flow/reviewer.json    default_agent reviewer →  minimax-coding-plan/MiniMax-M3#thinking
```

**Two things the config does not do**, each measured on the wire through a
logging proxy:

- A model's own `options.output_config.effort` is **ignored** — the catalog's
  default wins — so the variant has to be named in the model string.
- The provider's base-URL key in this config shape is **`api`**, not
  `baseURL`. The wrong key is ignored silently and the request goes to the
  real provider — which looks exactly like the variant being dropped.

**A model with several variants still carries the selection — measured on
opencode v2.3.0.** Both role models carry more than one variant, and each pane
ran the one its model string names: the builder recorded `variant: high`, the
reviewer `variant: thinking`. Re-check this when opencode is upgraded: an
earlier opencode dropped the selection for a multi-variant model — the TUI
showed `· medium` and the request sent `medium`, whatever the model string
said — which is why the roles once ran one-variant aliases. The aliases are
gone; the check is what remains.

Other catalog models, when a role needs a different lens:
`deepseek/deepseek-v4-pro#high`, and the `opencode/*` family
(`deepseek-v4.1-flash-free`, `glm-5.3-flash-free`, `longcat-2.5-free`,
`mimo-v2.6-flash-free`, `nemotron-3.5-super-vl-free`, `iceman-free`,
`inkling-2-small-free`, `flint-40b-free`, `minimax-m3.1-flash-free`,
`beam-free`, `big-pickle`). These come from the catalog, so the flow depends
on no entry in a global opencode config.

## Loop mechanics

1. **The orchestrator designs and decides.** It decides what the unit of work
   is, creates the Orchestration workspace (once per loop), and opens a tab in
   it for the builder.
2. **The orchestrator submits the kickoff and waits with the tested command:**
   `scripts/herdr-wake.sh <PANE> "<kickoff>" --timeout 1800`. Exit 0 means
   `completion_seq` advanced or a working turn settled, and the final pane
   output is printed when the budget allows; exit 2 is blocked, 3 is the
   whole-command bound, 4 is a usage or Herdr failure. `agent_not_found` is
   retried as not-ready, two
   consecutive settled reads set the baseline, and `unknown` never counts as
   completion. Do not use
   `herdr agent wait --until idle` as the wake signal (defect record above).
   **Waking mechanism:** every working agent ends by reporting to **the
   orchestrator pane only** —
   `herdr agent prompt <orchestrator-pane-id> "done: <one-line summary>"`, with
   no `--wait`; the pane ID comes from the kickoff. The literal name
   `orchestrator` is not registered, so addressing it by name fails. The
   orchestrator's wait ends when the wake command returns, not when the report
   arrives.
   **Fallback:** if the report prompt fails because the pane or session
   churned, look the orchestrator's pane up again with `herdr pane current`
   and retry once; if that also fails, idle with the summary visible in the
   terminal output — the orchestrator recovers verdicts from terminal output
   on the next check-in.
3. **On wake**, the orchestrator reads the agent's summary, then decides.
   Before opening any review pane it snapshots the round:
   `scripts/review-snapshot.sh /tmp/opencode/review<N>/round<N>-snapshot`. On a
   fix round it also writes the delta:
   `scripts/review-snapshot.sh --diff <previous-snapshot> <current-snapshot> > /tmp/opencode/review<N>/round<N>-delta.diff`.
   The script reads git and copies files; it never stages or edits.
4. **The orchestrator verifies** by running the full `make ci` gate before the
   review and again before the commit — an agent's claim that the gate passed is
   not evidence. Reviewers run only targeted probes. The review brief carries
   the command, its exit status, and the log path
   (`/tmp/opencode/review<N>/gate.log`), so the reviewer reads the result
   without rerunning it.
5. **The orchestrator opens the next role** in a **fresh tab and pane**. A first
   review opens **two reviewer panes at once** — one CODE, one SPEC — each with
   its own findings file. A fix round opens **one delta reviewer**. Only the
   orchestrator decides transitions — never the agents.
6. **On both axis reports**, the orchestrator merges them into the round's
   findings file and decides one `ok` / `changes-needed` verdict. `low` findings
   are recorded; they do not open another round.
7. **After a clean review** the orchestrator commits and updates the project's
   records. On findings, it opens a fresh builder tab in the same workspace with
   the merged findings. Close the tab when the round is done; keep the workspace
   for the next round.

**Every round is builder → review.** A first review has two reviewers, CODE and
SPEC, in parallel; a fix round has one delta reviewer. Nothing a builder
produces is reviewed by the builder, and nothing is committed without a review
round — not a one-line fix, not a rename. The review is the gate, not a
courtesy.

**The budget is five build-and-fix cycles.** Round 1 is builder → first review;
its two axis panes spend one round together. Each `changes-needed` verdict
starts a new round: builder → delta reviewer. Round 5 is the last one: if it
comes back `changes-needed`, the orchestrator stops, keeps the workspace open,
and asks the human — with the findings, the rounds, and what it would change
next.

A round the builder ends `BLOCKED` does not spend one of the five: it ended
before anything was built. Only real build-and-fix cycles count.

## Prompt templates

The orchestrator submits each kickoff below with the tested command, run
from the repository root:

```sh
scripts/herdr-wake.sh <PANE> "<the template text, substitutions made>" --timeout 1800
```

The command flattens CR, LF, tab and form feed to spaces, so a block-quoted
template below is dispatched as one line. It returns when `completion_seq`
advances or a working turn settles twice, and prints the final pane output
when the budget allows, so the orchestrator reads the report after it returns.
The report line at the end of each template is sent
by the agent with `herdr agent prompt <orchestrator-pane> "<report>"` and no
`--wait`; do not use the wake command for a report to the orchestrator,
because it would wait for the orchestrator's turn to complete.

A first review submits the CODE and SPEC kickoffs to two panes started
together. A fix round submits the delta kickoff to one fresh pane. The round
snapshot supplies `<path-to-round-snapshot>` in the two first-review kickoffs and
`<path-to-current-snapshot>` in the delta kickoff.

**Builder kickoff:**

> Your role: builder.
> Autonomous: never use the ask tool; if blocked, end with
> `BLOCKED: <question>; options: ...; recommendation: ...` and report to the orchestrator.
> Read AGENTS.md and the project's docs it points to, and the round's
> findings at `<path-to-findings>`. TODO.txt is the orchestrator's: read it,
> do not write it.
> If a finding needs data the mock data does not have, you may add or edit
> it: change mock-data/generate.py, regenerate with
> `python3 mock-data/generate.py`, and keep the tree reproducible - the
> generator must write the same bytes the committed fixtures hold.
> The spec is the orchestrator's. If the implementation needs a rule the spec
> does not have, say so in your report - do not add or change a normative
> sentence in the spec yourself.
> Your task: `<the unit of work>`. Do not start other work. Finish the whole
> unit before you report — if you are done with a part, move to the next part
> rather than stopping. Run the project's
> gate before you report. Finish by reporting to the orchestrator pane with
> `herdr agent prompt <orchestrator-pane> "done: <one-line summary>"` (no
> `--wait`), and check with `herdr pane read <orchestrator-pane> --lines 5`
> that the text left the input box before you finish. The orchestrator is
> already waiting on your `completion_seq`.

**CODE reviewer kickoff** (one of the two first-review panes):

> Your role: reviewer on the CODE axis.
> Autonomous: never use the ask tool; if blocked, end with
> `BLOCKED: <question>; options: ...; recommendation: ...` and report to the orchestrator.
> Read AGENTS.md and the project's docs it points to, and
> .agents/skills/code-review/SKILL.md. TODO.txt is the orchestrator's: read it,
> do not write it. This is review round <N> of at most 5.
> Read the unit kickoff and done-when at `<path-to-kickoff>`.
> The round snapshot is at `<path-to-round-snapshot>`.
> Review the uncommitted diff (`git status`, `git diff --stat`, `git diff`)
> against the kickoff and the project's rules — not against the builder's own
> summary.
> Your axis is CODE: correctness, regressions, unsafe behaviour, data loss,
> error handling, test adequacy, and concrete maintainability defects. The
> repository's standards override generic smells; a baseline smell is a finding
> only when it names a concrete defect.
> The orchestrator ran the gate: `make ci`, exit <status>, log at
> `<path-to-gate-log>`. Read that result; do not run the gate again. Run only
> the targeted probe a concrete suspicion needs — no mutation or size-space
> sweeps, no repeated upstream comparisons, no speculative attacks.
> Findings carry axis, severity, location, consequence, and evidence. `low`
> findings are recorded but do not block. Write about 400 words to
> `<path-to-code-findings>`. Do not modify code.
> Report to the orchestrator pane with
> `herdr agent prompt <orchestrator-pane> "CODE: <n> blocking, <n> low"` and no
> `--wait`; check with `herdr pane read <orchestrator-pane> --lines 5` that the
> text left the input box before you finish. The orchestrator is already waiting
> on your `completion_seq`.

**SPEC reviewer kickoff** (the other first-review pane, opened at the same
time):

> Your role: reviewer on the SPEC axis.
> Autonomous: never use the ask tool; if blocked, end with
> `BLOCKED: <question>; options: ...; recommendation: ...` and report to the orchestrator.
> Read AGENTS.md and the project's docs it points to, and
> .agents/skills/code-review/SKILL.md. TODO.txt is the orchestrator's: read it,
> do not write it. This is review round <N> of at most 5.
> Read the unit kickoff and done-when at `<path-to-kickoff>`, and the scope
> boundary with it.
> The round snapshot is at `<path-to-round-snapshot>`.
> Review the uncommitted diff (`git status`, `git diff --stat`, `git diff`)
> against the kickoff and the project's rules — not against the builder's own
> summary.
> Your axis is SPEC: requirements missing or partial, scope creep, and
> requirements that look implemented but whose implementation looks wrong.
> Quote the kickoff line for each finding.
> The orchestrator ran the gate: `make ci`, exit <status>, log at
> `<path-to-gate-log>`. Read that result; do not run the gate again. Run only
> the targeted probe a concrete suspicion needs — no mutation or size-space
> sweeps, no repeated upstream comparisons, no speculative attacks.
> Findings carry axis, severity, location, consequence, and evidence. `low`
> findings are recorded but do not block. Write about 400 words to
> `<path-to-spec-findings>`. Do not modify code.
> Report to the orchestrator pane with
> `herdr agent prompt <orchestrator-pane> "SPEC: <n> blocking, <n> low"` and no
> `--wait`; check with `herdr pane read <orchestrator-pane> --lines 5` that the
> text left the input box before you finish. The orchestrator is already waiting
> on your `completion_seq`.

**Delta reviewer kickoff** (a fix round, one fresh pane):

> Your role: delta reviewer.
> Autonomous: never use the ask tool; if blocked, end with
> `BLOCKED: <question>; options: ...; recommendation: ...` and report to the orchestrator.
> Read AGENTS.md and the project's docs it points to, and
> .agents/skills/code-review/SKILL.md. TODO.txt is the orchestrator's: read it,
> do not write it. This is review round <N> of at most 5.
> Read the unit kickoff and done-when at `<path-to-kickoff>`, and the scope
> boundary with it.
> Read the previous round's findings at `<path-to-previous-findings>` and check
> that every one of them is closed — a renamed fix is not a closed finding.
> The previous builder round is snapshotted at `<path-to-previous-snapshot>`,
> this round at `<path-to-current-snapshot>`, and the delta (manifest changes
> plus file-tree diff) is at `<path-to-delta-diff>`. `git diff` still holds the
> first round's change, so read the delta instead. Read only the hunks in that
> delta, plus the lines they touch. Unchanged code that was clean stays clean.
> The orchestrator ran the gate: `make ci`, exit <status>, log at
> `<path-to-gate-log>`. Read that result; do not run the gate again. Run the
> targeted probe that proves each fix.
> New defects in the changed hunks are findings. Each finding keeps its origin
> axis (`code` or `spec`); `delta` is the report section, not a third axis.
> Quote the kickoff line for a SPEC finding.
> Findings carry axis, severity, location, consequence, and evidence. `low`
> findings are recorded but do not block. Write about 400 words to
> `<path-to-delta-findings>`. Do not modify code.
> Report to the orchestrator pane with
> `herdr agent prompt <orchestrator-pane> "DELTA: <n> prior open, <n> new blocking, <n> low"`
> and no `--wait`; check with `herdr pane read <orchestrator-pane> --lines 5`
> that the text left the input box before you finish. The orchestrator is
> already waiting on your `completion_seq`.

**Fixer kickoff** (a builder on its second round):

> Your role: builder.
> Autonomous: never use the ask tool; if blocked, end with
> `BLOCKED: <question>; options: ...; recommendation: ...` and report to the orchestrator.
> Read AGENTS.md, the project's docs, and the merged findings at
> `<path-to-findings>`. Fix every finding — the closed ones stay closed. The
> next round is a delta review over these findings and the hunks you change, not
> a fresh scan, so fix the defect rather than the area around it.
> Re-run the project's gate. Finish by reporting to the orchestrator pane with
> `herdr agent prompt <orchestrator-pane> "done: <one-line summary>"` (no
> `--wait`), and check with `herdr pane read <orchestrator-pane> --lines 5`
> that the text left the input box before you finish. The orchestrator is
> already waiting on your `completion_seq`.

## Clarifications and stalls

Dispatched agents run **autonomously**. Nobody watches their panes, so an
ask-the-user prompt stalls the agent forever.

- **Never use the ask-the-user tool** in a dispatched agent. If something is
  ambiguous, stop and end the reply with a single line:
  `BLOCKED: <the question>; options: <A / B / C>; my recommendation: <X>`.
  Then report it to the orchestrator pane with
  `herdr agent prompt <orchestrator-pane> "<the line>"` and no `--wait`.
- **The orchestrator decides.** It answers from the docs, or asks the human
  through its own ask tool — always stating its recommended option first —
  then re-prompts the same agent with the answer, or opens a fresh one with
  the answer in the kickoff.
- **Stall check:** a stall is the **absence of a report**, and there are two
  kinds:

  | what the pane shows | what it means | what to do |
  |---|---|---|
  | reasoning that stops mid-thought, files half changed | the turn ended early | **nudge it to continue** — `scripts/herdr-wake.sh <PANE> "Continue the unit. ... Do not stop mid-work: if you are done with a part, move to the next part before reporting." --timeout 1800` |
  | nothing at all, no files touched | the turn never started | restart it with the task |

  A turn that ends on a design decision — "let me settle on this
  definitively" — and then stops is the first kind, not a crash. The wake
  command's fallback returns that early-ended turn promptly (exit 0,
  `settled after working`); the nudge then starts a new turn.

## Rules for the orchestrator

- Never run product builds in the orchestrator pane; delegate. Its one heavy
  command is the full `make ci` gate.
- One active builder at a time, unless the units of work are independent — then
  parallel tabs, each with its own unit of work.
- **Everything a builder produces gets a review.** A first review is two fresh
  panes, CODE and SPEC, opened together with separate findings files; a fix
  round is one fresh delta reviewer. No exceptions.
- **One finding per round when the findings touch different layers.** A round
  that changes the fixtures, the model, a test and a rendering at once is
  three rounds wearing one coat — the builder spends the round arguing with
  its own half-finished refactor and lands the unit broken. Give it the
  smallest finding first.
- **At most 5 build-and-fix rounds** per unit of work. The two axis panes of a
  first review spend one round together. Round 5 comes back `changes-needed` →
  stop and ask the human.
- **Low findings do not open a round.** Record them in the merged findings file
  and carry them as follow-ups; a round that costs a build-and-fix cycle is for
  a blocking finding.
- **The orchestrator runs the full `make ci` itself** before opening the review
  and again before the commit. An agent's claim that the gate passed is not
  evidence. The review brief carries the command, its exit status, and the log
  path (`/tmp/opencode/review<N>/gate.log`).
- **Snapshot every builder round.** `scripts/review-snapshot.sh
  /tmp/opencode/review<N>/round<N>-snapshot` after the builder reports and
  before any review pane opens. A fix round also writes
  `scripts/review-snapshot.sh --diff <previous> <current> > /tmp/opencode/review<N>/round<N>-delta.diff`.
  The delta reviewer reads that diff; `git diff` cannot separate the fix from
  round 1's change.
- **Reviewers run targeted probes only** — the one test, command, or edge case a
  concrete suspicion needs — and read the code. No full gate, no ad-hoc
  harnesses, no exhaustive sweeps, no long builds, no edits. A check worth
  repeating belongs in the repo as its own unit.
- No code ships unreviewed: every changed file is covered by a review round
  before the unit is done.
- The orchestrator commits after a clean review. Agents never commit or push.
- **Close a tab when its round is done.** A finished agent left open is an
  idle process holding a slot, and it is easy to mistake for a live one.
- **Raise a toast when a unit closes:** `herdr notification show "coding-lanes" --body "<unit> is committed" --sound done`.
- **Stage by name.** Never `git add -A`, never `git add .` — a blanket stage
  sweeps another agent's in-flight work into the commit. Stage the paths the
  reviewed unit touched, and check `git status` before committing.
- **The working tree is the review surface.** The orchestrator commits only
  after a clean verdict, so the change under review is uncommitted.
- **Every dispatch is a fresh session.** Continuity is the findings file and the
  diff. No agent is reused across the rounds of one unit of work.
- **`TODO.txt` is the orchestrator's memory.** Read it when a session starts,
  update it when a unit closes. No agent edits it.

## Human workflow

The human types in the orchestrator session and nowhere else. Everything flows
through it.

1. **Request** — the human tells the orchestrator what they want. The
   orchestrator designs, decides, and delegates.
2. **Orchestrator's role** — designs first (the human approves when the change
   is architectural), opens the Orchestration workspace, starts a builder in a
   fresh tab there, runs the full gate, then opens two fresh-context reviewer
   panes in parallel — CODE and SPEC — on a different model family with its own
   reasoning variant. It merges their findings, decides the verdict, and sends
   a clean round to commit. A `changes-needed` verdict starts a fix round: a
   fresh builder, then one fresh delta reviewer. Up to 5 build-and-fix rounds;
   it commits after a clean review and updates the project's records.
3. **Escalation ladder** — review/fix capped at 5 rounds per unit;
   unresolved findings after that go to the human. Ambiguous product decisions
   go to the human immediately; the orchestrator decides within the approved
   principles and records the decision.
4. **Batching** — human-reported friction is batched into one loop per batch,
   not one agent per annoyance.
5. **Resumption** — any fresh session reads `AGENTS.md`, the docs it points to,
   and `TODO.txt`, and continues from the current unit of work. Herdr runtime
   state is ephemeral and rebuilt from these documents; `TODO.txt` is the part
   that survives.
6. **Commit policy** — the orchestrator commits locally after a clean review;
   pushing is the human's decision.

## Direct human requests in an agent pane

The human *should* route everything through the orchestrator, but messages
sometimes land directly in an agent pane. Then that pane is a **reporter, not
a builder**:

1. Diagnose **read-only** — reads, greps and test runs are fine, no edits.
2. Reply with the root cause, the realistic fix options, and their trade-offs.
3. Stop. Do not edit code or docs, and do not commit.

The orchestrator owns the design decision, the delegation and the review loop.
"But the human asked me directly" is not authorization to build — a direct
request is a *report trigger*, and the report goes back through the
orchestrator pane even when the human is watching live.
