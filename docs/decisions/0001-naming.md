# 0001. Naming — the project, the interfaces, the binaries

- **Status:** ✅ accepted
- **Date:** 2026-11-19
- **Deciders:** coding-crew

## Context

coding-crew has three interfaces and they are not interchangeable. One is written
by agents. Two are read by humans: one shows the work, one governs the automation
that runs it. All three act on the same project.

Names surface everywhere: a binary name, a package name, a crate path, a help line,
a window title, a command in a transcript a human reads later. A name is therefore
not decoration — it is a word an agent types and a human reads.

The risk to avoid is names that describe a mechanism rather than a purpose. "The CLI
tool" and "the TUI" are true and useless: they tell a reader what shape the program
has, not what it is for, and they leave the human with no word for the tool they read.
The second risk is naming both surfaces after the same thing, so that a transcript
cannot say which one did what.

The project needs names that carry an acronym's worth of meaning — the expansion has
to say what the tool does — while staying short enough to type and to read in a
terminal.

The expansions also name **the audience of each surface**, which is what makes a
reader's first guess right: the tool the agent drives, the tool the human reads to
learn the work's state, and the tool the human uses to govern the automation.

## Decision

### The project

| | |
|---|---|
| Project | **coding-crew** |
| Tagline | *An agentic todo list with superpowers* |
| Workspace layout | `crates/core`, `crates/talita`, `crates/raul`, `crates/hugo` (when built) |

The name says what the project has become: a crew of agents and humans working one
queue, under a human's governance. The work is no longer only lanes that a single
TUI groups tasks by; it is a crew that claims tasks, reviews them, remediates them,
and a governance interface — `hugo` — that configures and watches that crew. The
project names the whole, and the whole is the crew and its governance.

The **lanes** stay in the model: the human lane and the agent lane are the task's
own structure, and `docs/tasks/task-model.md` says so. The project's name and the
interfaces' expansions simply no longer spell that word — the name is for the crew,
and the crew is who does the work.

### The interfaces

| | talita | raul | hugo |
|---|---|---|---|
| What it is | the CLI tool | the TUI for the work | the TUI for the flow |
| Audience | agent | human | human |
| Acronym | **T**ask **A**dministration, **L**ifecycle & **I**nteraction **T**erminal **A**pplication | **R**eview, **A**ssess, **U**nderstand & **L**earn | **H**uman-guided **U**nified **G**overnance & **O**rchestration |
| Binary | `talita` | `raul` | `hugo` |
| Package | `coding-crew-talita` | `coding-crew-raul` | `coding-crew-hugo` (when built) |
| Crate | `crates/talita` | `crates/raul` | `crates/hugo` (when built) |
| Writes task data | yes | no — see [ADR 0007](0007-one-writer.md) | no |

The seam between the interfaces is the data: raul reads the work folder, hugo reads
the runtime. raul draws task content; hugo draws tasks only as items of a queue.

### Why talita

It is the tool that **administers the task**: it creates, edits, claims, approves,
records evidence, and it drives the task through its **lifecycle** — draft, groomed,
approved, and the rest of the transitions in
[the task model](../tasks/task-model.md). *Task Administration* names that job,
*Lifecycle* names the thing it administers, and *Interaction* names that it is used
in a conversation with a human.

It is also the surface an agent uses without a human present, so it is the one that
must read as a tool first: a short command word, a stable verb set, no display
charm.

### Why raul

It is the tool that makes the work **legible to a human**. It does not create work
and it does not approve it — it shows what is waiting, what is waiting on whom, and
what the human is being asked to decide. *Review, Assess, Understand & Learn* is
that job: you review the tasks, assess the state, understand the work, and learn
where the project stands.

The expansion names the **human's** side of the surface: raul is where the human
learns, and the tool presents. That is why it says *Learn* and not *Lead* — a
window does not lead the work; and the work's flow is `hugo`'s to govern, not
raul's.

### Why hugo

It is the tool that makes the **crew governable**. It does not do the work and does
not show the documents; it configures the agents, the models, the harness, the
review-round limit and the order, and it shows the crew, the round, the budget and
the findings. *Human-guided Unified Governance & Orchestration* is that job, and
*Human-guided* says who holds the governance.

### Why the interfaces keep their own names

`raul`, `talita` and `hugo` are not job titles and not roles. They are the names of
surfaces, and surfaces wear their own short names. Keeping them means a transcript
reads unambiguously:

```
talita task approve TSK01
raul
hugo
```

Three different verbs, three different names, one line of history to read. Renaming
a surface would lose that, and would lose a short, pronounceable word that fits a
terminal title bar.

### The alternatives, and why they lost

| Option | Lost because |
|---|---|
| "The CLI tool" and "the TUI" | mechanism, not purpose; no word for a human to hold in their head, and nothing to type |
| `coding-crew-cli` and `coding-crew-tui` | true and flat. The suffix says what the program is shaped like, and carries no meaning beyond the package name it duplicates |
| Naming the surfaces after roles — `coder` and `reviewer` | a role is a job, and a surface is not a job. It also collides with the roles in [ADR 0005](0005-roles.md) |
| Naming the CLI after the lane and the TUI after the board | two metaphors for one idea; the TUI shows lanes, not boards, and "board" would need a second definition |
| A name chosen for a two-interface tool | the project now has a third interface for governing the crew, and a name must describe what the project is, not the shape it had |
| Spelling *Lead* into raul's expansion | raul is a window, not an actor: "lead" reads as authority over the work, which is `hugo`'s to govern and the human's to decide. *Learn* names the same relation without claiming the job |
| `talita` renamed later | the name is in package names, crate paths and every transcript. Renaming costs more than it ever earns, so the name is chosen once |

## Consequences

### Positive

- A human has a word for each tool, and a transcript never needs a gloss.
- The acronym expansions say what each tool is for and who it is for, so the names
  survive being read months later by someone who has forgotten them.
- Package and crate names follow the binary names, so there is one name per
  surface.
- The project's name now covers the crew and its governance, so `hugo` fits under
  `coding-crew` rather than beside it as an exception.
- The lanes keep their place in the model without being the project's name.

### Negative

- **Three invented names have to be learned.** A new reader meets `talita`, `raul`
  and `hugo` before meeting any tool. The expansions in the docs and the help text
  carry the weight.
- The surfaces do not describe a workflow. `raul` in particular hides the fact that
  it is read-only today; that is the cost of naming a surface by its audience
  rather than by its features.
- Two of raul's four words — *Understand* and *Learn* — sit close together. The
  distinction is the work and its state; a reader who hears only one of them loses
  little.

### Follow-up

- The help text and window titles use the same names, decided in
  [docs/vocabulary.md](../vocabulary.md) § *What is strings only*.
- The data folder is `coding-crew/`, the package prefix is `coding-crew-`, and the
  project's configuration home is `~/.config/coding-crew/`.
