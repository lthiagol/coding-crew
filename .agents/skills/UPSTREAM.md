# Upstream provenance

| | |
|---|---|
| Repository | `github.com/mattpocock/skills` |
| License | MIT — see [LICENSE](LICENSE) |
| Pinned commit | `5beb35b040e3de97e54fa9e386d7e42d15c295c2` |
| Commit date | 2026-10-19 |
| Vendored on | 2026-11-13 |
| Adapted on | 2026-11-17 |

| Skill | Upstream path | Files | Mode |
|---|---|---|---|
| `codebase-design` | `skills/engineering/codebase-design/` | `SKILL.md`, `DEEPENING.md`, `DESIGN-IT-TWICE.md` | vendored |
| `diagnosing-bugs` | `skills/engineering/diagnosing-bugs/` | `SKILL.md`, `scripts/hitl-loop.template.sh` | vendored |
| `domain-modeling` | `skills/engineering/domain-modeling/` | `SKILL.md`, `GLOSSARY-FORMAT.md`, `ADR-FORMAT.md` | vendored |
| `writing-for-agents` | `skills/productivity/writing-for-agents/` | `SKILL.md`, `SKILL-MECHANICS.md` | vendored |
| `code-review` | `skills/engineering/code-review/` | `SKILL.md` | **adapted** — see [ledger](code-review/UPSTREAM.md) |

The upstream `agents/openai.yaml` files are **not** vendored — they configure a
different harness.

## Rules for vendored copies

1. **Pinned to a commit.** Never `main`. A vendored copy at a branch head drifts
   before anyone notices.
2. **Byte-identical to upstream.** No edits in place. A skill that needs changing
   for coding-lanes becomes ours, under `docs/skills/`.
3. **Re-sync by script.** `.agents/skills/sync.sh` prints a diff before
   overwriting.
4. **Record the commit.** This file is the provenance; the commit hash is the pin.

## The adapted copy

`code-review` is **adapted**, not vendored. It keeps the upstream source commit
and license, and adds a numbered deviation ledger:
[code-review/UPSTREAM.md](code-review/UPSTREAM.md). That ledger records the
source commit, the inspected upstream `main` commit, and the adaptation date.

`sync.sh` never overwrites the adapted skill. It fetches the pinned upstream
`SKILL.md`, diffs it against the local copy, and reports the drift so an
upstream change stays visible without erasing the coding-lanes adaptation.
`--bump` moves only the global pin: the adaptation keeps the source commit it
records and needs a manual rebase, which the script reports. The drift report
diffs the local copy against that recorded source commit, not the newly bumped
pin.

## Attribution

These skills are the work of Matt Pocock, MIT licensed. Used and redistributed
under that license. See [LICENSE](LICENSE).
