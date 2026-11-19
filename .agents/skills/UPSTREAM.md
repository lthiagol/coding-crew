# Upstream provenance

| | |
|---|---|
| Repository | `github.com/mattpocock/skills` |
| License | MIT — see [LICENSE](LICENSE) |
| Pinned commit | `5beb35b040e3de97e54fa9e386d7e42d15c295c2` |
| Commit date | 2026-10-19 |
| Vendored on | 2026-11-13 |

| Skill | Upstream path | Files | Mode |
|---|---|---|---|
| `codebase-design` | `skills/engineering/codebase-design/` | `SKILL.md`, `DEEPENING.md`, `DESIGN-IT-TWICE.md` | vendored |
| `diagnosing-bugs` | `skills/engineering/diagnosing-bugs/` | `SKILL.md`, `scripts/hitl-loop.template.sh` | vendored |
| `domain-modeling` | `skills/engineering/domain-modeling/` | `SKILL.md`, `GLOSSARY-FORMAT.md`, `ADR-FORMAT.md` | vendored |
| `writing-for-agents` | `skills/productivity/writing-for-agents/` | `SKILL.md`, `SKILL-MECHANICS.md` | vendored |

The upstream `agents/openai.yaml` files are **not** vendored — they configure a
different harness.

`code-review` and `orchestration-flow` are not here. Both live globally at
`~/.config/opencode/skills/`, loaded by their IDs. `code-review`'s provenance
is `~/.config/opencode/skills/code-review/UPSTREAM.md`, which records its own
source commit, the inspected upstream `main` commit, and the numbered deviation
ledger. `orchestration-flow` is ours and has no upstream ledger.

## Rules for vendored copies

1. **Pinned to a commit.** Never `main`. A vendored copy at a branch head drifts
   before anyone notices.
2. **Byte-identical to upstream.** No edits in place. A skill that needs changing
   for coding-lanes becomes ours: under `docs/skills/` while it is a draft, then
   loaded from `.agents/skills/` or `~/.config/opencode/skills/`.
3. **Re-sync by script.** `.agents/skills/sync.sh` prints a diff before
   overwriting.
4. **Record the commit.** This file is the provenance; the commit hash is the pin.

## The adapted list

`sync.sh` can diff an adapted skill against its own recorded source commit
without overwriting it. Each adapted entry names its own local directory and
reads the pin from `<directory>/UPSTREAM.md`, so an adapted skill may live
outside this repository.

The list is empty today: `code-review` left it when it moved to
`~/.config/opencode/skills/code-review` and became maintained by hand. The
mechanism stays for a future adapted skill.

When an entry exists, `--bump` moves only the global pin: the adaptation keeps
the source commit its own ledger records and needs a manual rebase, which the
script reports. The drift report diffs the local copy against that recorded
source commit, not the newly bumped pin.

## Attribution

These skills are the work of Matt Pocock, MIT licensed. Used and redistributed
under that license. See [LICENSE](LICENSE).
