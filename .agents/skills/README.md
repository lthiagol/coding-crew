# Vendored skills

Third-party skills we depend on, pinned in this repo so they can be shipped
with the tool.

**Upstream.** `github.com/mattpocock/skills` — MIT. See [LICENSE](LICENSE) and
[UPSTREAM.md](UPSTREAM.md) for provenance.

**Pinned to a commit, not `main`.** The pin is the same commit maitre vendored,
so the vendored copies match maitre's byte for byte and both can be re-synced to
the same place. A branch head drifts before anyone notices — see
[sync.sh](sync.sh).

## What we keep here, and why

| Skill | How | Why we need it |
|---|---|---|
| [codebase-design](codebase-design/SKILL.md) | vendored | The `crates/core`-vs-surface seam is our central architectural commitment: the domain is UI-free, talita writes and raul reads, and neither shells out to the other. This is the vocabulary for it: module, interface, depth, seam, adapter, leverage, locality. **Our own `core` package gets judged by the deletion test.** |
| [writing-for-agents](writing-for-agents/SKILL.md) | vendored | Most of what we write is for agents. This governs skills and `AGENTS.md` — including the positive-voice rule our house style follows. |
| [domain-modeling](domain-modeling/SKILL.md) | vendored | Keeps [docs/vocabulary.md](../../docs/vocabulary.md) and the ADRs in [docs/decisions/](../../docs/decisions/) sharp. It is the discipline behind our names of record: one word per state, one spelling per id. |
| [diagnosing-bugs](diagnosing-bugs/SKILL.md) | vendored | Build a feedback loop before hypothesising. No TDD conflict: its regression-test phase is regression testing, not test-first authoring. |

The four vendored skills are byte-identical to upstream, and
[sync.sh](sync.sh) copies them from the pinned commit.

## Skills that live globally

Two skills are not in this folder. They live under
`~/.config/opencode/skills/`, are loaded by ID from there, and their
provenance sits next to them:

| Skill | Where | What it covers |
|---|---|---|
| `code-review` | `~/.config/opencode/skills/code-review/` | The reviewer's two axes: CODE for correctness and regressions, SPEC for the unit's requirements and scope, and a fix round's delta over round snapshots. It reads our uncommitted diff and the unit kickoff, not an issue tracker, and the orchestrator owns the gate and the verdict. |
| `orchestration-flow` | `~/.config/opencode/skills/orchestration-flow/` | the build loop: three agents, the Herdr workspace, the two reviewer axes, the delta review, the rounds, the budget |

`code-review`'s provenance is
`~/.config/opencode/skills/code-review/UPSTREAM.md`: it records the source
commit, the inspected main commit, and the numbered deviation ledger.
`code-review` is adapted from upstream and maintained by hand — no script
syncs it. `orchestration-flow` is ours and has no upstream ledger.

## What we did not take

| Skill | Why not |
|---|---|
| `wayfinder` | Tracker-coupled and decision-shaped. We took its *ideas* — map-as-index (the work folder is the index), fog of war (the lanes are derived views), HITL-vs-AFK (the human lane versus the agent lane) — not the skill. We kept ids, because ids are of record here. |
| `prototype` | The method transfers; the artifacts assume a web stack. We already have raul for the equivalent. |
| `implement`, `implement-spec` | They assume an orchestrator driving multi-agent worktrees. Our orchestrator role exists, but what it drives does not yet (task-model §14). |
| `improve-codebase-architecture` | Nothing to scan in a small codebase. Its HTML report is a natural fit for raul later. |

## Our own skills

Not vendored, and not upstream: the skills coding-lanes defines.

| Skill | Where | What it covers |
|---|---|---|
| `task-authoring` | `docs/skills/` | the `creator` role — writing a task through talita |
| `task-review` | `docs/skills/` | the `reviewer` role's task-record, finding, verdict, focus, and role model; the code-diff process lives in the global `code-review` skill |

A skill OpenCode loads lives either globally at
`~/.config/opencode/skills/` or in a project's `.agents/skills/` when it is
vendored. A draft stays in `docs/skills/` until it is promoted.

## One note on the vendored files

The vendored skills refer to `GLOSSARY.md` and to ADRs. Our equivalents are
[docs/vocabulary.md](../../docs/vocabulary.md) and [docs/decisions/](../../docs/decisions/).
The vendored files stay byte-identical to upstream, so they are **not** edited
to say this — read the mapping here. The adapted `code-review` skill uses
coding-lanes paths directly and lives globally, not here.

## Re-syncing

```bash
.agents/skills/sync.sh          # copy the vendored skills at the pin; report adapted drift
.agents/skills/sync.sh --bump   # move the global pin, then sync
```

The script never edits a vendored file by hand. If a vendored skill needs
changing for coding-lanes, it becomes **our** skill — under `docs/skills/`
while it is a draft, and loaded from `.agents/skills/` or
`~/.config/opencode/skills/` once it is ready. The vendored copy stays
pristine so the diff against upstream stays readable.

The adapted list is empty today: `code-review` left it when it moved to
`~/.config/opencode/skills/code-review` and became maintained by hand. The
mechanism stays for a future adapted skill. An adapted entry names its own
local directory and reads the pin from that directory's `UPSTREAM.md`, so it
may live outside this repository; the script fetches that commit, prints the
diff as drift, and leaves the local copy in place. `--bump` does not move the
entry's source commit; the adaptation needs a manual rebase, and the script
says so.
