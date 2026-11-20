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

## The thirty sketches

The three variants below are the ones carried over from the Autopilot era.
The current comparison is thirty newer sketches, drawn as HTML pages so the
human could look at them: `docs/hugo/tui-proposals/`, one directory per
model family — `deepseek/`, `glm/`, `minimax/` — with the combined index at
its root. It groups the thirty into the eight readings the set contains:

1. the order leads — the policy is the screen
2. the crew leads — who is acting, right now
3. the findings lead — what is open decides the screen
4. the queue leads — the work as an index
5. the round leads — the review happening now
6. the budget leads — the scarce thing is the subject
7. the stamp leads — does it run, and why
8. the flow is the subject — every fact cites its cause

Readings 1–7 are reached by two or three families each; reading 8 only by
glm. The human picks one, and the pick becomes the screen that gets built.

The sketches are drawn as they are, defects included: a review round found
seven real ones and the human chose not to have them fixed, so the index
carries a known-issues block that names them. Read it before reading any
sketch as a specification.

## The pick

The human picked **the crew leads**: *who is acting, right now*. The agents
and what they hold are the first thing read in the main pane; the order and
the rest of the policy fold into the sidebar, which is where the
configuration lives anyway.

| reading | what leads the main pane |
|---|---|
| **the crew leads** | **the agents and what they hold — the pick** |
| the queue leads | the live work as an index |
| the findings lead | the open findings |
| the round leads | the review in flight |
| the budget leads | the external rounds spent |
| the order leads | the policy |
| the stamp leads | whether the automation runs |
| the flow is the subject | every fact cites the step that produced it |

The pick does not change the shape: one sidebar for the configuration, one
main pane for what is running, no tabs. It fixes what the main pane reads
**first** — the crew, then the work it holds.

## The rulings the review made

A review round read the thirty sketches against the rules above and settled
six points. They bind whatever gets built, so they are written down here:

- **A held task belongs to the crew, not the queue.** The queue holds
  unclaimed work; a task an agent holds is that agent's row.
- **No invented interactions.** A screen may not paint a key for an action
  the open question has not settled — whether hugo drives the loop or only
  shows it. Where a design wants one, it says so in a line instead.
- **The queue carries an id and a status**, plus derived review metadata —
  rounds, focus — when that metadata is not the task's content. The title,
  the notes and the intent stay out; they belong to raul.
- **A count must name its set.** Sixteen live tasks is the automation's
  queue; nineteen adds the three groomed. Both are computable, so the
  screen has to say which it counts.
- **The stamp says whether the automation runs *and* why.** A bare
  "running" is half the settled content.
- **The same fact has one home.** Not a summary and a detail of one count,
  not a composition stated in two blocks.

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
- **The configuration is centralized** under `~/.config/coding-crew/`.
- **The seam**: raul reads the work folder; hugo reads the runtime.
- **One interface, the build loop first**: the runtime is an abstraction,
  and the flow this repository runs itself is its first implementation.

## Not decided here

- Whether the autopilot runs at all, or hugo describes a policy the human
  applies by hand. That is the question behind the interface.
- The configuration's format and its file layout under `~/.config/`.
