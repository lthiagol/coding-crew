# Vendored and adapted skills

Third-party skills we depend on, pinned in this repo so they can be shipped
with the tool.

**Upstream.** `github.com/mattpocock/skills` — MIT. See [LICENSE](LICENSE) and
[UPSTREAM.md](UPSTREAM.md) for provenance.

**Pinned to a commit, not `main`.** The pin is the same commit maitre vendored,
so the vendored copies match maitre's byte for byte and both can be re-synced to
the same place. A branch head drifts before anyone notices — see
[sync.sh](sync.sh).

## What we took, and why

| Skill | How | Why we need it |
|---|---|---|
| [codebase-design](codebase-design/SKILL.md) | vendored | The `crates/core`-vs-surface seam is our central architectural commitment: the domain is UI-free, talita writes and raul reads, and neither shells out to the other. This is the vocabulary for it: module, interface, depth, seam, adapter, leverage, locality. **Our own `core` package gets judged by the deletion test.** |
| [writing-for-agents](writing-for-agents/SKILL.md) | vendored | Most of what we write is for agents. This governs skills and `AGENTS.md` — including the positive-voice rule our house style follows. |
| [domain-modeling](domain-modeling/SKILL.md) | vendored | Keeps [docs/vocabulary.md](../../docs/vocabulary.md) and the ADRs in [docs/decisions/](../../docs/decisions/) sharp. It is the discipline behind our names of record: one word per state, one spelling per id. |
| [diagnosing-bugs](diagnosing-bugs/SKILL.md) | vendored | Build a feedback loop before hypothesising. No TDD conflict: its regression-test phase is regression testing, not test-first authoring. |
| [code-review](code-review/SKILL.md) | **adapted** | The reviewer's two axes: CODE for correctness and regressions, SPEC for the unit's requirements and scope, and a fix round's delta over round snapshots. It reads our uncommitted diff and the unit kickoff, not an issue tracker, and the orchestrator owns the gate and the verdict. |

The four vendored skills are byte-identical to upstream. `code-review` is
adapted: its local copy is authoritative, and
[its provenance](code-review/UPSTREAM.md) records the source commit, the
inspected main commit, and the numbered deviation ledger.

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
| [orchestration-flow](orchestration-flow/SKILL.md) | `.agents/skills/` | the build loop: three agents, the Herdr workspace, the two reviewer axes, the delta review, the rounds, the budget |
| [task-authoring](../../docs/skills/task-authoring.md) | `docs/skills/` | the `creator` role — writing a task through talita |
| [task-review](../../docs/skills/task-review.md) | `docs/skills/` | the `reviewer` role's task-record, finding, verdict, focus, and role model; the code-diff process lives in [code-review](code-review/SKILL.md) |

A skill we want loaded lives in `.agents/skills/`; one that is still a draft
lives in `docs/skills/` until it is promoted.

## One note on the vendored files

The vendored skills refer to `GLOSSARY.md` and to ADRs. Our equivalents are
[docs/vocabulary.md](../../docs/vocabulary.md) and [docs/decisions/](../../docs/decisions/).
The vendored files stay byte-identical to upstream, so they are **not** edited
to say this — read the mapping here. The adapted `code-review` skill uses
coding-lanes paths directly.

## Re-syncing

```bash
.agents/skills/sync.sh          # copy the vendored skills at the pin; report adapted drift
.agents/skills/sync.sh --bump   # move the global pin, then sync; code-review keeps its source commit
```

The script never edits a vendored file by hand. If a vendored skill needs
changing for coding-lanes, it becomes **our** skill — under `docs/skills/`
while it is a draft, and under `.agents/skills/` once it is ready to load. The
vendored copy stays pristine so the diff against upstream stays readable.
`code-review` is already adapted, so the script never overwrites it: it fetches
the skill's own recorded source commit, prints the diff as drift, and leaves
the local copy in place. `--bump` does not move that source commit; the
adaptation needs a manual rebase, and the script says so.
