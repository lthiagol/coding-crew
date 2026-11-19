# Hugo — the design record

hugo is the TUI for the flow: **one sidebar for configuration and one main
pane for what is running. No tabs.** The Autopilot tab that used to live in
raul is gone; the automation's questions live here.

This file is the record of what was compared before any of it was built:
three sketches of the flow's screen, drawn in terminal text. Their *content*
model carried over — the order, the queue, the crew, the stamp. Their
*layout* did not: the sketches arranged those blocks vertically, because the
Autopilot tab was a tab. hugo splits them instead:

| the sketch's block | where it lives in hugo |
|---|---|
| the order — the policy | the sidebar, as configuration |
| the queue | the main pane, as items — id and status, never content |
| the crew | the main pane |
| the stamp | the main pane |

## The rules the sketches obey

- **The order is policy and sits on top; the queue is progress under it;
  the stamp at the foot says whether the automation runs.** Policy above,
  progress below, and **the same fact never twice**.
- **The agent count is a composition, not a number.** Three agents means one
  per role; two means the orchestrator reviews; one means the human
  reviews, because an agent cannot review itself.
- **The stamp prints no number it cannot compute.**
- **The order lives on the tool's side**, not in the project's data.
- **The crew is runtime state**, and only the runtime has it.

The order the sketches show is the project's own flow — claim an approved
task whose dependencies are done, write the steps and the evidence, a
reviewer who did not write the code gives the verdict, findings go back to
the runner — read from `docs/roles/roles.md`.

---

## Variant A — the order first

maitre's Sous-Chef shape: policy, then progress, then the stamp.

```text
AUTOPILOT ───────────────────────────────── running

ORDER
  1  claim an approved task whose dependencies are done
  2  the runner writes the steps and the evidence
  3  a reviewer who did not write it gives the verdict

QUEUE
  ●  TSK29   approved    claimable now
  ○  TSK07   approved    waits on TSK04
  ○  TSK08   approved    waits on TSK05

CREW
  runner-1     TSK29   executing
  reviewer-2   idle

STAMP
  running since 09:14
```

**Reads as:** the rule, then what the rule is doing. The reader learns
*why* a task is queued before they learn which one.
**Costs:** the top third is the same on every screen, and the live work —
the crew — sits at the bottom, below a queue that changes faster.

---

## Variant B — the crew first

What is happening now, at the top; the rule and the stamp below.

```text
AUTOPILOT ───────────────────────────────── running

CREW
  runner-1      executing  TSK29    12m
  reviewer-2    reviewing  TSK14     4m
  reviewer-3    idle

QUEUE
  ●  TSK29   approved    claimable now
  ○  TSK07   approved    waits on TSK04

STAMP
  running since 09:14
```

**Reads as:** a dashboard of the live run. The first line answers "is
anything happening".
**Costs:** the order is a footnote, and the composition rule has no home —
three agents in a list is just a list until the stamp says what three
agents *mean*.

---

## Variant C — the composition first

The stamp and what the crew's size means, then the order, the queue, and
the crew.

```text
AUTOPILOT ─────────────────── 3 agents — one per role

ORDER
  claim → execute → review (by someone else)

CREW
  runner-1     TSK29   executing
  reviewer-2   idle
  reviewer-3   idle

QUEUE
  ●  TSK29   approved    claimable now
  ○  TSK07   approved    waits on TSK04

STAMP
  running since 09:14
```

**Reads as:** the shape of the automation first — how many agents, what
that means, what they will do — then the detail.
**Costs:** the order is compressed to a phrase, and the header must change
when the crew changes size, because the count is a *composition*: two
agents would read "2 agents — the orchestrator reviews", one would read
"1 agent — the human reviews".

---

## The questions the winner must answer

1. **What the sidebar configures** — the agents, the models, the harness,
   the review-round limit, and the order, in what order they are offered
   and what each one means.
2. **What the main pane shows** — the crew, the current round, the budget,
   the findings, and the queue; and what it shows when nothing is running.
3. **How the composition is stated** — as the header's own line, or only
   when it matters (a stamp that names who reviews).
4. **What the queue shows** — the claimable task and the ones waiting, or
   the whole Path tree raul already has. hugo draws tasks only as items of
   a queue: id and status, never their content.

## Decided since

- **The layout**: one sidebar, one main pane, no tabs. The sidebar is
  configuration; the main pane is the running state.
- **The configuration is centralized** under `~/.config/coding-lanes/`.
- **The seam**: raul reads the work folder; hugo reads the runtime.
- **One interface, the build loop first**: the runtime is an abstraction,
  and the flow this repository runs itself is its first implementation.

## Not decided here

- Whether the autopilot runs at all, or hugo describes a policy the human
  applies by hand. That is the question behind the interface.
- The configuration's format and its file layout under `~/.config/`.
