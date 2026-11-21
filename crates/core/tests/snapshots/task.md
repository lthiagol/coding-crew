# Header

| field | value |
|---|---|
| id | `TSK07` |
| title | Add lane filtering to raul |
| status | `code-review` |
| kind | `task` |
| overlays | `blocked`: The upstream lane table is still being renamed. — by `creator-1` at `2026-11-16T15:00:00Z` |
| claim | by `runner-1` in `s-runner` at `2026-11-16T12:00:00Z` — lease `2026-11-16T18:00:00Z` |
| priority | `high` |
| effort | `L` |
| tags | `tui`, `lane-filter` |

# Intent

**Outcome**

raul can show one lane at a time, and the counts match the screen.

**Problem**

The Overview shows every lane at once, so a long list hides the
lane the human is working in.

# Scope

**In scope**

- A lane selector on the Overview.
- Counts that follow the selector.

**Out of scope**

- A saved default lane.
- Filtering the Backlog tab.

# Acceptance criteria

| id | description | verification | status | evidence | deferral | failure reason |
|---|---|---|---|---|---|---|
| `ACCEPT-01` | The selector hides a task that matches no selected lane. | `cargo test -p coding-crew-raul lane_filter` | `passed` | _quoted below_ | _none_ | _none_ |
| `ACCEPT-02` | The saved default lane is not in this task. | `cargo test -p coding-crew-raul saved_lane` | `deferred` | _none_ | The settings store does not exist yet. — follow up: `TSK09` | _none_ |
| `ACCEPT-03` | The empty lane announces that it is empty. | `cargo test -p coding-crew-raul empty_lane` | `failed` | _quoted below_ | _none_ | The lane count is drawn before the filter runs. |

**Evidence for `ACCEPT-01`**

> `cargo test -p coding-crew-raul lane_filter` — exit `0` — 12 passed; 0 failed — by `runner-1` in `s-runner` at `2026-11-16T16:10:00Z`

**Evidence for `ACCEPT-03`**

> `cargo test -p coding-crew-raul empty_lane` — exit `1` — 1 failed — by `runner-1` in `s-runner` at `2026-11-16T16:20:00Z`

# Steps

## `STP-01`

- **order:** `1`
- **action:** Read the Overview's lane mapping.
- **done when:** The mapping is named in the notes.
- **files:** `crates/raul/src/overview.rs`
- **status:** `done`
- **covers:** `ACCEPT-01`
- **depends on:** _none_
- **deviation:** _none_

## `STP-02`

- **order:** `2`
- **action:** Draw the selector above the list.
- **done when:** The selector is on the first row.
- **files:** `crates/raul/src/overview.rs`, `crates/raul/src/theme.rs`
- **status:** `in-progress`
- **covers:** `ACCEPT-01`, `ACCEPT-03`
- **depends on:** `STP-01`
- **deviation:** Used the existing slot table instead of a new one.

# Review

- **verdict:** `changes-needed`
- **budget:** `1/5` external rounds

**Rounds**

## Round `1` — `self`

- **focus:** `spec-compliance`
- **reviewer:** `runner-1`
- **verdict:** `ok`
- **at:** `2026-11-16T16:30:00Z`
- **notes:** Read the diff against the criteria.
- **findings:** _none_

## Round `2` — `external`

- **focus:** `edge-cases`
- **reviewer:** `reviewer-1`
- **verdict:** `changes-needed`
- **at:** `2026-11-16T17:00:00Z`
- **notes:** The empty lane is counted before the filter.

### `FIND-01`

- **severity:** `high`
- **status:** `open`
- **created:** `2026-11-16T17:00:00Z`
- **resolved:** _none_
- **description:** An empty lane counts the rows it does not show.
- **resolution:** _none_

**Comments**

- `CMT-01` — `reviewer-1` at `2026-11-16T17:01:00Z`
  The count is drawn from the unfiltered list.

**Findings not in a round**

## `FIND-02`

- **severity:** `low`
- **status:** `fixed`
- **created:** `2026-11-16T14:00:00Z`
- **resolved:** `2026-11-16T14:30:00Z`
- **description:** The selector's label is one column too wide.
- **resolution:** Cut the label at the column that fits.

**Comments**

_none_

# Questions

**Open**

## `QST-01`

- **question:** Does the selector remember its lane between runs?
- **raised by:** `creator-1`
- **raised at:** `2026-11-14T09:00:00Z`

**Resolved**

## `QST-02`

- **question:** Is the selector a tab or a row?
- **raised by:** `creator-1`
- **raised at:** `2026-11-14T09:05:00Z`
- **answer:** A row above the list; it is a filter, not a screen.
- **resolved by:** `human-1`
- **resolved at:** `2026-11-14T10:00:00Z`

# Notes

## `NOTE-01`

- **author:** `runner-1`
- **at:** `2026-11-16T12:30:00Z`

The lane mapping was already a table; reuse it instead of adding one.

# Dependencies

**Waits on**

- [`TSK01`](TSK01) — `done` — The lane table

**Waits on this**

- [`TSK02`](TSK02) — `groomed` — Sort the filtered list

# Children

- [ ] `TSK07.1` — `groomed` — Refine the selector's label — criteria `2/2` passed, steps `1/2` done
