# talita — roadmap

The build order for talita, in small units. Design only: nothing here is
scheduled until the human says to build talita (AGENTS.md, Scope).

Core changes come first, so every talita verb calls rules that already exist.
The nouns then come in the order the flow uses them. Each unit ends with
`make check` green; a unit that changes a schema also changes
`mock-data/generate.py` and regenerates the fixtures.

| # | Unit | Size | Done when |
|--:|---|:-:|---|
| T1 | **Core schema additions**: `Round.session`, `review.re_reviews[]`, `Evidence.manual`, `Task.revision`, `crew[].actor`, the new journal kinds ([ADR 0018](../decisions/0018-runtime-and-journal-writers.md)); generator writes them | M | raul and hugo render the regenerated fixtures unchanged except for the new fields; TSK14 carries a `re_reviews[]` entry |
| T2 | **Core gates**: collect-all `check()`; `review.round_current`, `review.round_owner`, `task.claimed` on `finish`, external round refused from `claim.session` | M | a lifecycle test per new gate, and one showing `check()` returns every failing gate |
| T3 | **Core operations**: claim take/renew/release with the takeover rule; the ownership table; the stored criterion-status mirror | M | unit tests: claimable equals the generator's rule; an expired lease is taken over; an owner-less write is refused |
| T4 | **Core store**: project `flock`, atomic writes for `activity.json` and `runtime.json`, numeric id sort, `save_item` removes an old file | S | a test runs two writers in parallel and sees no lost update and no duplicate id |
| T5 | ✓ **talita skeleton**: noun–verb parser, result envelope, error-code enum with exit codes, identity resolution, `session new|show`, `project init|show|check`; old `create|list|show` removed | M | `talita session show` resolves flag over env; every error path prints the envelope with its exit code; a test pins the code table to commands.md |
| T6 | ✓ **Task content**: `task create|show|list|retitle|describe|prioritize|estimate|tag|depend|undepend`, `scope`, `criterion add|revise|remove`, `step add|revise|remove`, `question`, `note` | M | a task is groomed end to end through talita; a write to a frozen section returns `section.frozen` with the child-task remedy |
| T7 | ✓ **Task lifecycle**: every `Event` as a `task` verb, overlays, `planned_by` at approve, `--instructed-by` on resume/reopen, `--grant-rounds` | M | each fixture's status is reachable from `draft` through talita alone; each refusal names its gates |
| T8 | ✓ **Claim**: `claim take|renew|release|show` | S | a runner claims, starts, lets the lease expire, and another runner takes over |
| T9 | **Steps at work and evidence**: `step begin|done|skip|fail`, `criterion defer|undefer|explain`, `evidence run|override` | M | `evidence run` records the real exit code and summary; a timeout records `unproven`; an override shows `manual` in raul |
| T10 | ✓ **Review**: `round file` (flags and `--input`), `finding open|resolve|dismiss`, `comment add`, `re_reviews[]` on `task re-review` | M | the unit in README's sequence diagram runs through talita; the verdict rules refuse both mismatches; `refs` maps D-numbers to FIND ids |
| T11 | ✓ **Journal writer**: every task verb appends its kind; `journal show` | S | raul's activity shows each move of the T10 run, in order, none unreadable |
| T12 | **Runtime writer**: `run`, `crew`, `queue` verbs; derived fields recomputed on every write; crew side effects | M | hugo, pointed at a project driven only by talita, shows the crew, round, budget and findings the mock shows for the same state |
| T13 | ✓ **Items**: `item create|show|list|shape|dismiss|promote` | S | an idea is shaped, promoted, and the new task is in `draft` with `promoted_from` |
| T14 | **Docs and skills**: skill docs' CLI tables, TM § 3.3/§ 4/§ 11, roles.md env table, the drift fixes in [decisions.md](decisions.md); the flow skills call talita | S | no doc names a flat verb; the drift table's "doc fix" rows are closed |

### Carried into the units (grooming, 2026-11-23)

Follow-ups from the core units, placed where they are built:

- **T5:** `project check` calls `Store::duplicate_ids()`, which core has and nothing calls.
- **T6:** `questions` splits into ask (any role) and answer (the creator): a `Section` split and a task-model § 2.5 change (ruling, 2026-11-23).
- **T7:** `task cancel` releases a live claim (ADR 0020); `transition()` takes `now` as the claim operations do.
- **T9:** a mirror with neither evidence nor deferral writes `pending`, so a cleared deferral cannot stick; settle it with the criterion verbs.
- **T10:** `round file` refuses a round filed by the claim's own session (ADR 0017), calling core's check; a fixture holds a code-review task before its first external round.
- **T8:** `claim take` journals `claimed` — on a takeover the note is `took over from <actor>` — and `claim release` journals `released` with its required reason. T11 wrote the writer; the verbs are T8's.
- **T12:** the crew rows the claim verbs touch — `claim take` holds the task and is `executing`, `claim release` releases the row — and the derived fields the runtime record recomputes on every write. T8 built the verbs; the rows are T12's.
- **T11:** `release` stores its required reason in the journal; `item promote` appends the `created` event for the task it creates, with the note `from ITM<nn>`; the review verbs append `round_filed`, `finding_raised`, `finding_resolved` and `commented`.

T6–T10 can each ship alone once T5 is in. T11 and T12 can start after T7,
since every later verb then gains its side effects as it lands.
