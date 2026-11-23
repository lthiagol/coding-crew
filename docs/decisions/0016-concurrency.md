# 0016. Concurrency — one lock, atomic writes, optional revision check

- **Status:** ✅ accepted
- **Date:** 2026-11-22
- **Deciders:** coding-crew (the human, 2026-11-22)

## Context

The flow runs several agents on one project: a builder, a reviewer, an
orchestrator, sometimes a creator in a human's session. Each calls talita.

Today (`inv-existing.md` § 1):

- `Store::save` writes `<file>.json.tmp`, syncs, renames — atomic per file.
- There is no lock. Load–modify–save is last-writer-wins: two verbs on one
  task can lose an update.
- `next_id` scans the directory with no lock: two `task create`s can take
  the same id.
- `activity.json` is one JSON object (`{events: […]}`), so an append is a
  whole-file rewrite; two appenders lose events.
- hugo rereads `runtime.json` every second; it must never see half a file.

One verb can write up to three files: the task, the journal, the runtime
record.

## Decision

### One project lock

Every write takes an exclusive advisory lock (`flock`) on
`coding-crew/.talita.lock` for the whole read–check–write, across all files
it touches. Reads take no lock. The OS frees the lock when the process dies,
so there is no stale lock file to clean.

Wait up to 10 s, then `conflict.lock` (exit 6). Writes are small and fast;
one lock per project is simpler than one per file and makes the three-file
write and id allocation race-free.

### Atomic writes everywhere

The task rule (TM § 11) extends to `activity.json` and `runtime.json`:
write `.tmp`, sync, rename. raul and hugo only ever read a whole file.

### Write order

1. The task file (or item file) — the source of truth.
2. The journal.
3. The runtime record.

A crash between steps leaves the truth written and a feed behind it.
`run refresh` rebuilds the runtime's derived fields; a missing journal event
is not repaired (the journal is a feed, not the record). Item promotion keeps
its order: item first, then task (`store.rs:295-304`).

### Optional revision check

Gated verbs re-check their rules under the lock, so a stale read cannot
break a rule. Content verbs can still overwrite a change the caller did not
see (two creators revising one criterion). The task gains `revision`, a
counter incremented on every write. Any write verb takes
`--expect-revision <n>`; a mismatch is `conflict.revision` (exit 6), and the
caller re-reads. Not required: most calls do not need it.

### Journal stays one object, for now

Rewriting the journal under the lock is correct. Its cost grows with the
event count; at hundreds of events it is noise. When it matters, move to
JSON Lines (`activity.jsonl`), a reader change in raul — later.

### Alternatives considered

| Option | Lost because |
|---|---|
| No lock, atomic rename only | atomic per file, still lost updates and duplicate ids |
| A lock per file | three-file writes need ordered acquisition; ids need a lock on the folder anyway |
| Lock-file existence (`O_EXCL` create) | a crash leaves a stale lock someone must remove |
| `revision` required on every write | every call needs a read first; agents would pass whatever they last saw, defeating it |
| `updated` timestamp as the revision | one-second resolution; two writes in a second collide |
| JSON Lines journal now | changes raul's reader and the core type for a scale problem we do not have |

### Settled 2026-11-22

- Settled 2026-11-22: One `flock` per project for every write.
- Settled 2026-11-22: `revision` is optional on every verb (`--expect-revision`).
- Settled 2026-11-22: The journal stays one JSON object; JSON Lines is later.

## Consequences

### Positive

- No lost updates, no duplicate ids, no torn reads.
- Crash safety: the task is always consistent; the rest is rebuildable or a
  feed.

### Negative

- Writes serialize per project. Fine at a crew of three.
- `flock` is advisory: a hand-edit ignores it (as it ignores every rule).
- A lock file appears in `coding-crew/`. raul reads only `work/*.json`,
  `activity.json`; hugo only `runtime.json`.
- `flock` on network filesystems is unreliable. Not a target.

### Follow-up

- `Store` gains the lock and the generic atomic write for the two records.
- `revision` is added to `Task` with `#[serde(default)]`.
