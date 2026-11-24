# 0012. talita's grammar — noun, then verb

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

talita has three flat verbs today: `create`, `list`, `show`
(`crates/talita/src/main.rs:20-33`). The skill docs plan more flat verbs —
`groom`, `add-step`, `set-meta`, `finding`, `pass-review`
(`docs/skills/task-authoring.md:222-234`,
`docs/skills/task-review.md:215-223`). ADR 0001 already shows
`talita task approve TSK01` (`0001-naming.md:108`).

The surface is large. A task has eleven parts with their own ids and rules
(criteria, steps, questions, notes, evidence, rounds, findings, comments,
claim, overlays, meta), plus smaller tasks, the journal and the runtime
record. Flat verbs collide: `add` of what, `open` of what, `defer` a task or
a criterion. The callers are agents: they discover the surface from `--help`
and from the skill docs, and a regular grammar is cheaper to learn than a
list.

## Decision

**`talita <noun> <verb> <ids…> [flags]`**, Docker style.

```
talita task approve TSK04
talita criterion defer TSK04 ACCEPT-02 --reason "..." --follow-up TSK09
talita finding resolve TSK04 FIND-02 --resolution "..."
```

### Naming rules

| Rule | |
|---|---|
| **The noun is what the call writes** | `claim take` writes the claim; `task start` writes the status. A lifecycle event is a `task` verb because it writes the task's status |
| **Nouns are singular** | `task`, `finding`, never `tasks` |
| **Verbs are intents** | the act in the domain's words: `approve`, `resolve`, `dismiss`, `take`. Never `set`, `update`, `edit`, `patch` ([ADR 0013](0013-intent-verbs.md)) |
| **Lifecycle verbs are the core event names** | kebab-case of `Event`: `pass-review`, `request-changes`, `re-review`. One rename: `ResumeWork` → `resume`, as the skill docs already write it |
| **Ids are positional, parent first** | `TSK04 FIND-02`. Every other value is a flag |
| **talita allocates ids** | no verb takes the id of a part it creates |
| **One verb, one intent** | no verb does two acts. `groom` no longer also writes the intent and criteria |
| **No aliases** | one spelling per command, so transcripts and skills agree |
| **State words match the TUIs** | statuses, kinds and verdicts in output are the record's words (`docs/vocabulary.md:23`) |

### builder and runner

The flow calls the code-writing role **builder**; task files call it
`runner` (`roles.md`, "The order"). talita accepts both on input
(`--role builder` = `--role runner`). Each file keeps its own schema's word:
task files and the journal write `runner`; `runtime.json` writes `builder`.
Unifying the word is a schema change for later.

### Alternatives considered

| Option | Lost because |
|---|---|
| Flat verbs, as the skill docs planned | collide as the surface grows (`defer` a task vs a criterion); no place for item, run and crew verbs; `finding` alone reads as a noun used as a verb |
| Verb, then noun (`approve task TSK04`) | groups help by verb, so an agent asking "what can I do to a finding" reads the whole surface |
| `talita --task X <verb>` (the task as a flag) | the human's idea for rounds; makes the task id optional-looking and breaks "ids positional". Kept for the review input as `round file <TSK> --input` ([ADR 0017](0017-review-record.md)) |
| All lifecycle events under a `review` noun for the reviewer verbs | splits the status machine across nouns; the noun rule ("what it writes") puts every status change on `task` |

### Settled 2026-11-22

- Settled 2026-11-22: builder/runner: each file keeps its schema's word; input accepts both. A single migration to `builder`, with the journal's `by`, is later.
- Settled 2026-11-22: Grooming input: one call per part; no batch brief input. Revisit if call counts hurt.

## Consequences

### Positive

- `talita <noun> --help` lists everything an agent can do to that part.
- The noun says which section is written, so the ownership check reads the
  noun and the role.
- Transcripts read like the domain: `talita finding dismiss TSK04 FIND-02`.

### Negative

- Longer commands than flat verbs. Agents pay a few tokens per call.
- Today's flat `create`, `list` and `show` break. They are placeholders
  (`TODO.md:22`); nothing outside the repo calls them.
- Splitting `groom` makes grooming several calls instead of one.

### Follow-up

- Rewrite the CLI tables in `docs/skills/task-authoring.md` and
  `docs/skills/task-review.md` against `docs/talita/commands.md`.
- `docs/vocabulary.md` gains the noun list.
