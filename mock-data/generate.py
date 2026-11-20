#!/usr/bin/env python3
"""Generate the mock task fixtures for coding-crew.

Every fixture satisfies the gates of the phase it is in. The dependency graph
is acyclic, and a task past approved is claimed only after the done work it
waits on has finished, so the set is valid as a whole.

Run from the repo root:

    python3 mock-data/generate.py
"""

import json
import pathlib
import shutil
from datetime import datetime, timedelta

ROOT = pathlib.Path(__file__).resolve().parent
TASKS = ROOT / "coding-crew" / "work"
ACTIVITY = ROOT / "coding-crew" / "activity.json"
RUNTIME = ROOT / "coding-crew" / "runtime.json"

# The models the flow's configuration names, in flow/builder.json and
# flow/reviewer.json. The orchestrator reviews when the crew is two, so it
# runs the reviewer's family: the reviewer is never the model that wrote the
# code.
ORCHESTRATOR_MODEL = "opencode-go/glm-5.3#high"
RUNNER_MODEL = "opencode-go/deepseek-v4.1-flash#high"
REVIEWER_MODEL = "opencode-go/glm-5.3#high"

TITLES = [
    "Lorem ipsum dolor sit amet",
    "Consectetur adipiscing elit sed do eiusmod",
    "Tempor incididunt ut labore et dolore",
    "Magna aliqua ut enim ad minim veniam",
    "Iusto odio dignissimos ducimus qui blanditiis",
    "Praesentium voluptatum deleniti atque corrupti",
    "Quaerat voluptatem consequatur sunt in culpa",
    "Officiis debitis aut rerum necessitatibus",
    "Sed ut perspiciatis unde omnis iste natus",
    "Quis autem vel eum iure reprehenderit",
    "Nam libero tempore cum soluta nobis est",
    "Eligendi optio cumque nihil impedit quo minus",
    "Et harum quidem rerum facilis est et expedita",
    "Nemo enim ipsam voluptatem quia voluptas sit",
    "At vero eos et accusamus et iusto odio",
    "Totam rem aperiam eaque ipsa quae ab illo",
    "Ut enim ad minim veniam quis nostrud",
    "Duis aute irure dolor in reprehenderit",
    "Nulla pariatur excepteur sint occaecat",
    "Temporibus autem quibusdam et aut officiis",
]

OUTCOMES = [
    "Tempor incididunt ut labore et dolore magna aliqua, ut enim ad minim veniam.",
    "Quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat.",
    "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque.",
    "Totam rem aperiam eaque ipsa quae ab illo inventore veritatis et quasi architecto.",
    "Praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi.",
    "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit.",
    "Et harum quidem rerum facilis est et expedita distinctio nam libero tempore.",
    "Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe.",
    "Ut et voluptates repudiandae sint et molestiae non recusandae itaque earum rerum.",
]

PROBLEMS = [
    "Quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    "Occaecati cupiditate non provident sed ut perspiciatis unde omnis iste natus error.",
    "At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium.",
    "Ea voluptate velit esse quam nihil molestiae consequatur vel illum qui dolorem eum.",
    "Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet.",
    "Soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat.",
    "Itaque earum rerum hic tenetur a sapiente delectus ut aut reiciendis voluptatibus.",
    "Nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia.",
]

ACTIONS = [
    "Sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.",
    "Neque porro quisquam est qui dolorem ipsum quia dolor sit amet consectetur.",
    "At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium.",
    "Itaque earum rerum hic tenetur a sapiente delectus ut aut reiciendis voluptatibus.",
    "Et harum quidem rerum facilis est et expedita distinctio nam libero tempore cum soluta.",
    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat.",
    "Nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia.",
    "Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet.",
]

DONE_WHEN = [
    "The lorem module is read and its behaviour is described in the notes.",
    "The lorem tests exist and pass.",
    "The lorem lorem lorem is covered by a lorem test.",
    "The lorem behaviour is verified by the lorem command.",
]

CRITERIA = [
    "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque.",
    "Totam rem aperiam eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae.",
    "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit sed quia.",
    "Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet.",
    "Ut et voluptates repudiandae sint et molestiae non recusandae itaque earum rerum hic.",
    "Soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat.",
]

FINDINGS = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor.",
    "Incididunt ut labore et dolore magna aliqua, ut enim ad minim veniam, quis nostrud.",
    "Quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    "Consequat duis aute irure dolor in reprehenderit in voluptate velit esse cillum.",
    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu.",
    "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit sed.",
]

NOTES = [
    "Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe.",
    "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip.",
    "Et harum quidem rerum facilis est et expedita distinctio nam libero tempore cum.",
    "Quaerat voluptatem consequatur sunt in culpa qui officia deserunt mollit anim id.",
]

VERIFICATIONS = [
    "cargo test --workspace",
    "cargo test -p coding-crew-core",
    "cargo clippy --all-targets -- -D warnings",
    "make check",
]

SEVERITIES = ["low", "medium", "high"]
PRIORITIES = ["urgent", "high", "normal", "low"]
EFFORTS = ["S", "M", "L"]
FOCI = ["spec-compliance", "edge-cases", "security", "empirical", "design"]

# The done fixtures take ids 22 to 26: they are generated after the seven
# phases up to needs-human, and before the promoted bug, the child and the
# late approved task. A task past approved may only wait on done work, and
# the approved tasks may wait on work that is not done.
DONE_IDS = [f"TSK{number:02d}" for number in range(22, 27)]


def slugify(title):
    out = []
    dash = False
    for ch in title.lower():
        if ch.isalnum():
            if dash and out:
                out.append("-")
            dash = False
            out.append(ch)
        else:
            dash = True
    return "".join(out)


def pick(seq, index):
    return seq[index % len(seq)]


def ts(day, hour, minute=0):
    return f"2026-11-{day:02d}T{hour:02d}:{minute:02d}Z"


def evidence(index, by, session, day, hour, passed=True, minute=0):
    return {
        "command": pick(VERIFICATIONS, index),
        "exit_code": 0 if passed else 1,
        "summary": f"{10 + index} passed; 0 failed" if passed else f"1 failed; {9 + index} passed",
        "at": ts(day, hour, minute),
        "by": by,
        "session": session,
    }


def criterion(index, with_evidence=None, deferral=None, cid="ACCEPT-01"):
    item = {
        "id": cid,
        "description": pick(CRITERIA, index),
        "verification": pick(VERIFICATIONS, index),
        "status": "pending",
        "evidence": with_evidence or [],
        "deferral": deferral,
        "failure_reason": "",
    }
    return item


def step(index, order, status, covers, files, deviation="", depends_on=None):
    return {
        "id": f"STP-{order:02d}",
        "order": order,
        "action": pick(ACTIONS, index),
        "done_when": pick(DONE_WHEN, index),
        "files": files,
        "status": status,
        "covers": covers,
        "depends_on": depends_on or [],
        "deviation": deviation,
    }


def finding(index, created, status="open", resolution="", resolved=""):
    return {
        "id": f"FIND-{index:02d}",
        "severity": pick(SEVERITIES, index),
        "status": status,
        "description": pick(FINDINGS, index),
        "created": created,
        "resolved": resolved,
        "resolution": resolution,
        "comments": [],
    }


def round_(n, kind, focus, reviewer, verdict, index, at, findings=None):
    return {
        "n": n,
        "kind": kind,
        "focus": focus,
        "reviewer": reviewer,
        "verdict": verdict,
        "at": at,
        "notes": pick(NOTES, index),
        "findings": findings or [],
    }


def transition_note(index, author, body, at, note_id="NOTE-01"):
    """A note dated at the moment the task entered its status.

    The model has no separate event log, so a status transition records
    its moment in a note.
    """
    return {
        "id": note_id,
        "author": author,
        "body": body,
        "at": at,
    }


def transition_moment(item):
    """The moment the fixture entered the status it is in."""
    status = item["status"]
    if status == "draft":
        return item["created"]
    if status in ("groomed", "approved"):
        prefix = "Groomed by" if status == "groomed" else "Approved by"
        for note in reversed(item["notes"]):
            if note["body"].startswith(prefix):
                return note["at"]
        raise ValueError(f"{item['id']} has no {prefix} transition note")
    if status == "executing":
        return item["claim"]["at"]
    # Every other status is entered by its last review round.
    return item["review"]["rounds"][-1]["at"]


def event_stamps(item):
    """Every dated event in the fixture's history."""
    stamps = [item["created"], item["status_since"]]
    claim = item.get("claim")
    if claim:
        stamps.append(claim["at"])
    for overlay in item.get("overlays", {}).values():
        if overlay:
            stamps.append(overlay["at"])
    for criterion in item.get("acceptance_criteria", []):
        for evidence in criterion.get("evidence", []):
            stamps.append(evidence["at"])
    review = item.get("review", {})
    for finding in review.get("findings", []):
        stamps.append(finding["created"])
        if finding["resolved"]:
            stamps.append(finding["resolved"])
        for comment in finding.get("comments", []):
            stamps.append(comment["at"])
    for round in review.get("rounds", []):
        stamps.append(round["at"])
    for question in item.get("questions", []):
        stamps.append(question["raised_at"])
        if question["resolved_at"]:
            stamps.append(question["resolved_at"])
    for note in item.get("notes", []):
        stamps.append(note["at"])
    return stamps


def task(task_id, title, status, index, **overrides):
    # `created` and `updated` are the two ends of the fixture's timeline:
    # `updated` is moved to the latest event in `finalize` below.
    created = ts(2 + index % 6, 9 + index % 8)
    item = {
        "schema_version": 1,
        "id": task_id,
        "title": title,
        "slug": slugify(title),
        "parent": None,
        "children": [],
        "created": created,
        "updated": "",
        "status": status,
        "status_since": "",
        "overlays": {},
        "claim": None,
        "planned_by": None,
        "meta": {"priority": pick(PRIORITIES, index), "effort": pick(EFFORTS, index), "tags": ["mock"]},
        "intent": {"outcome": "", "problem": ""},
        "scope": {"in_scope": [], "out_of_scope": []},
        "acceptance_criteria": [],
        "steps": [],
        "review": {"max_rounds": 5, "rounds": [], "verdict": None, "findings": []},
        "questions": [],
        "notes": [],
        "depends_on": [],
    }
    item.update(overrides)
    return item


def finalize(item):
    """Stamp the status from the history and move the last write with it."""
    item["status_since"] = transition_moment(item)
    # The last write is the latest event in the fixture's history.
    item["updated"] = max(event_stamps(item))
    assert item["created"] <= item["status_since"] <= item["updated"], item["id"]
    return item


def check_claims_follow_dependencies(tasks):
    """Every task past approved was claimed after its dependencies finished.

    `dependencies.satisfied` gates `start`: the task must be claimed and every
    dependency done. A fixture past approved therefore tells a chronology the
    model allows only if its claim sits after the completion of the done work
    it waits on.
    """
    by_id = {item["id"]: item for item in tasks}
    past_approved = {"executing", "code-review", "remediating", "needs-human", "done"}
    for item in tasks:
        if item["status"] not in past_approved:
            continue
        for dependency in item["depends_on"]:
            done = by_id[dependency]
            assert done["status"] == "done", (item["id"], dependency, done["status"])
            assert item["claim"]["at"] > done["status_since"], (
                item["id"],
                item["claim"]["at"],
                dependency,
                done["status_since"],
            )


# The journal's event kinds, in the order one task's history has them when
# they share an instant. The kinds are provisional: the writer settles them.
EVENT_KINDS = [
    "created",
    "approved",
    "claimed",
    "review_requested",
    "finding_raised",
    "remediated",
    "commented",
    "escalated",
    "completed",
]
EVENT_RANK = {kind: rank for rank, kind in enumerate(EVENT_KINDS)}


def stamp(text):
    return datetime.strptime(text, "%Y-%m-%dT%H:%MZ")


def at_minute(moment):
    return moment.replace(second=0, microsecond=0).strftime("%Y-%m-%dT%H:%MZ")


def minutes_before(text, minutes):
    return at_minute(stamp(text) - timedelta(minutes=minutes))


def between(first, second):
    """The whole minute midway between two events."""
    return at_minute(stamp(first) + (stamp(second) - stamp(first)) / 2)


def event(at, kind, task_id, by, note=""):
    record = {"at": at, "kind": kind, "task": task_id, "by": by}
    if note:
        record["note"] = note
    return record


def actor(author):
    """The role or kind an event names from a fixture's author."""
    if author.startswith("runner"):
        return "runner"
    if author.startswith("reviewer"):
        return "reviewer"
    return "human"


def approval_at(item):
    """When the task left the human lane, from its own history."""
    for note in reversed(item["notes"]):
        if note["body"].startswith("Approved by"):
            return note["at"]
    if item["status"] == "approved":
        return item["status_since"]
    if item.get("claim"):
        # An older fixture records the claim but not the approval: the
        # claim is the next event, and the approval is shortly before it.
        return minutes_before(item["claim"]["at"], 30)
    return None


def activity_events(tasks):
    """The journal of the tasks' own histories, oldest first."""
    events = []
    for item in tasks:
        task_id = item["id"]
        events.append(event(item["created"], "created", task_id, "human"))
        approved = approval_at(item)
        if approved:
            events.append(
                event(approved, "approved", task_id, "human", "left the human lane")
            )
        claim = item.get("claim")
        if claim:
            events.append(
                event(claim["at"], "claimed", task_id, "runner", f"claimed by {claim['by']}")
            )
        rounds = item.get("review", {}).get("rounds", [])
        if rounds:
            events.append(
                event(rounds[0]["at"], "review_requested", task_id, "runner", "sent to review")
            )
        for finding in item.get("review", {}).get("findings", []):
            events.append(
                event(
                    finding["created"],
                    "finding_raised",
                    task_id,
                    "reviewer",
                    f"raised {finding['id']}",
                )
            )
        # A re-review is the runner answering a finding and sending the work
        # back: it sits between two external rounds.
        external = [round for round in rounds if round["kind"] == "external"]
        for first, second in zip(external, external[1:]):
            events.append(
                event(
                    between(first["at"], second["at"]),
                    "remediated",
                    task_id,
                    "runner",
                    "answered the findings",
                )
            )
        if item["status"] == "needs-human":
            events.append(
                event(
                    item["status_since"],
                    "escalated",
                    task_id,
                    "reviewer",
                    "the review budget is spent",
                )
            )
        if item["status"] == "done":
            events.append(
                event(item["status_since"], "completed", task_id, "reviewer", "reviewed clean")
            )
        for note in item.get("notes", []):
            if note["body"].startswith("Groomed by") or note["body"].startswith(
                "Approved by"
            ):
                continue
            # The journal's note is one line, so a multi-line note gives its
            # first line.
            body = note["body"].splitlines()[0] if note["body"] else ""
            events.append(
                event(note["at"], "commented", task_id, actor(note["author"]), body)
            )
    events.sort(key=lambda record: (record["at"], EVENT_RANK[record["kind"]], record["task"]))
    return events


def check_activity(tasks, events):
    """The journal tells the chronology the fixtures already tell."""
    by_id = {item["id"]: item for item in tasks}
    for record in events:
        assert record["kind"] in EVENT_RANK, record
        assert record["task"] in by_id, record
        item = by_id[record["task"]]
        assert record["at"] >= item["created"], (record, item["created"])
    for task_id, item in by_id.items():
        task_events = [record for record in events if record["task"] == task_id]
        assert task_events, f"{task_id} has no activity"
        assert task_events[0]["kind"] == "created", (task_id, task_events[0])
        stamps = [record["at"] for record in task_events]
        assert stamps == sorted(stamps), (task_id, stamps)
        kinds = [record["kind"] for record in task_events]
        if "claimed" in kinds:
            assert "approved" in kinds, task_id
            assert kinds.index("approved") < kinds.index("claimed"), task_id
            if "review_requested" in kinds:
                assert kinds.index("claimed") < kinds.index("review_requested"), task_id
        if item["status"] == "done":
            assert "completed" in kinds, task_id
        if item["status"] == "needs-human":
            assert "escalated" in kinds, task_id
    # The dependency rule holds in time as it does in the fixtures: a claim
    # comes after the completion of every done task it waits on.
    for record in events:
        if record["kind"] != "claimed":
            continue
        item = by_id[record["task"]]
        for dependency in item["depends_on"]:
            completed = [
                done["at"]
                for done in events
                if done["task"] == dependency and done["kind"] == "completed"
            ]
            assert completed, (record["task"], dependency, "not completed")
            assert record["at"] > completed[0], (
                record["task"],
                record["at"],
                dependency,
                completed[0],
            )
    # The block shows the last few: the source has more than it shows.
    assert len(events) > 5, len(events)


# The instant the fixtures are read with, as in crates/core/tests/mock_data.rs:
# it decides whether a claim's lease is still live.
FIXTURE_NOW = "2026-11-19T08:00:00Z"


def lease_is_live(claim):
    """Whether a claim still holds its task at the fixtures' clock."""
    if not claim:
        return False
    lease = claim.get("lease_expires_at") or ""
    if not lease:
        return True
    try:
        expires = datetime.fromisoformat(lease.replace("Z", "+00:00"))
    except ValueError:
        return False
    return expires > datetime.fromisoformat(FIXTURE_NOW.replace("Z", "+00:00"))


def claimable(item, by_id):
    """Whether an agent could claim the task at the fixtures' clock.

    The claim gate is the model's: approved, no overlay, every dependency
    done, and no live claim held elsewhere. It is the queue's one rule.
    """
    if item["status"] != "approved":
        return False
    if any(item["overlays"].get(kind) for kind in ("blocked", "deferred", "cancelled")):
        return False
    for dependency in item["depends_on"]:
        if by_id[dependency]["status"] != "done":
            return False
    return not lease_is_live(item.get("claim"))


def runtime_record(tasks):
    """The flow's runtime, from the fixtures' own state.

    The crew holds the live work: the runner the first executing task it
    claimed, the reviewer the first task waiting for its external round.
    The queue is every claimable task nobody holds. The round is the review
    the reviewer is on. The budget counts the external rounds of the live
    tasks that have rounds; the findings count the open ones.
    """
    by_id = {item["id"]: item for item in tasks}
    runner_task = next(item for item in tasks if item["status"] == "executing")
    reviewer_task = next(item for item in tasks if item["status"] == "code-review")
    queue = [item for item in tasks if claimable(item, by_id)]
    earliest = min(runner_task["claim"]["at"], reviewer_task["status_since"])
    live = [
        item
        for item in tasks
        if item["review"]["rounds"] and item["status"] != "done"
    ]
    return {
        "started_at": minutes_before(earliest, 30),
        "crew": [
            {
                "role": "orchestrator",
                "model": ORCHESTRATOR_MODEL,
                "state": "idle",
            },
            {
                "role": "runner",
                "model": RUNNER_MODEL,
                "holds": runner_task["id"],
                "since": runner_task["claim"]["at"],
                "state": "executing",
            },
            {
                "role": "reviewer",
                "model": REVIEWER_MODEL,
                "holds": reviewer_task["id"],
                "since": reviewer_task["status_since"],
                "state": "reviewing",
            },
        ],
        "queue": [{"id": item["id"], "status": item["status"]} for item in queue],
        "round": {
            "task": reviewer_task["id"],
            "stage": "external",
            "number": max(round["n"] for round in reviewer_task["review"]["rounds"]) + 1,
        },
        "budget": {
            "spent": sum(
                1
                for item in live
                for round in item["review"]["rounds"]
                if round["kind"] == "external"
            ),
            "total": sum(item["review"]["max_rounds"] for item in live),
        },
        "findings": sum(
            1
            for item in tasks
            for finding in item["review"]["findings"]
            if finding["status"] == "open"
        ),
        "stamp": {"runs": True},
    }


def check_runtime(record, tasks, events):
    """The runtime record is computable from the fixtures, in their time."""
    by_id = {item["id"]: item for item in tasks}
    held = [member["holds"] for member in record["crew"] if member.get("holds")]
    assert len(held) == len(set(held)), held
    for task_id in held:
        assert task_id in by_id, task_id
    queued = [item["id"] for item in record["queue"]]
    assert not set(queued) & set(held), (queued, held)
    for task_id in queued:
        assert claimable(by_id[task_id], by_id), task_id

    # The stamp runs, and a record that stops says why.
    if record["stamp"]["runs"]:
        assert "why" not in record["stamp"], record["stamp"]
    else:
        assert record["stamp"].get("why"), record["stamp"]

    # The numbers are the fixtures' own.
    live = [
        item
        for item in tasks
        if item["review"]["rounds"] and item["status"] != "done"
    ]
    assert record["budget"] == {
        "spent": sum(
            1
            for item in live
            for round in item["review"]["rounds"]
            if round["kind"] == "external"
        ),
        "total": sum(item["review"]["max_rounds"] for item in live),
    }, record["budget"]
    assert record["findings"] == sum(
        1
        for item in tasks
        for finding in item["review"]["findings"]
        if finding["status"] == "open"
    ), record["findings"]

    # The round in flight is the reviewer's task, at the next round.
    round_ = record["round"]
    task = by_id[round_["task"]]
    assert task["status"] == "code-review", task["id"]
    assert round_["number"] == max(r["n"] for r in task["review"]["rounds"]) + 1
    reviewer = next(
        member for member in record["crew"] if member["role"] == "reviewer"
    )
    assert reviewer["holds"] == task["id"], reviewer
    assert reviewer["since"] == task["status_since"], reviewer

    # The crew is one per role, each entry naming the live work it holds.
    roles = [member["role"] for member in record["crew"]]
    assert sorted(roles) == ["orchestrator", "reviewer", "runner"], roles
    runner = next(member for member in record["crew"] if member["role"] == "runner")
    runner_task = by_id[runner["holds"]]
    assert runner_task["status"] == "executing", runner_task["id"]
    assert runner["since"] == runner_task["claim"]["at"], runner
    assert runner_task["claim"]["by"].startswith("runner-"), runner_task["claim"]

    # The run began before the first hold, and after every event before it.
    started = stamp(record["started_at"])
    earliest = min(stamp(member["since"]) for member in record["crew"] if member.get("since"))
    assert started < earliest, (record["started_at"], earliest)
    for event_ in events:
        assert (
            event_["at"] <= record["started_at"] or stamp(event_["at"]) >= earliest
        ), event_
    for member in record["crew"]:
        if member.get("since"):
            assert stamp(member["since"]) >= started, member



def build():
    tasks = []
    counter = 0

    def next_id():
        nonlocal counter
        counter += 1
        return f"TSK{counter:02d}"

    ids = {}

    # draft — title only, plus one child of an approved task.
    for i in range(3):
        tid = next_id()
        ids.setdefault("draft", []).append(tid)
        tasks.append(task(tid, pick(TITLES, i + 14), "draft", i))

    # groomed — intent, scope, criteria; one of them blocked.
    for i in range(3):
        tid = next_id()
        ids.setdefault("groomed", []).append(tid)
        groom_at = ts(6 + 2 * i, 9 + i)
        notes = [
            transition_note(
                i + 3,
                f"session-creator-{i + 1:02d}",
                f"Groomed by session-creator-{i + 1:02d}: intent, scope and criteria written.",
                groom_at,
            )
        ]
        if i != 1:
            # A later write, after the transition: the wait starts at the
            # transition, not at the last write.
            notes.append(
                {
                    "id": "NOTE-02",
                    "author": f"session-creator-{i + 1:02d}",
                    "body": pick(NOTES, i),
                    "at": ts(7 + 2 * i, 10 + i),
                }
            )
        item = task(
            tid,
            pick(TITLES, i + 8),
            "groomed",
            i + 3,
            planned_by=f"session-creator-{i + 1:02d}",
            intent={"outcome": pick(OUTCOMES, i), "problem": pick(PROBLEMS, i)},
            scope={
                "in_scope": [pick(ACTIONS, i)],
                "out_of_scope": [pick(PROBLEMS, i + 4)],
            },
            acceptance_criteria=[criterion(i)],
            notes=notes,
        )
        if i == 1:
            item["overlays"] = {
                "blocked": {
                    "reason": pick(FINDINGS, i),
                    "by": f"creator-{i + 1:02d}",
                    "at": ts(8, 14),
                }
            }
        tasks.append(item)

    # approved — criteria covered by steps with done_when.
    for i in range(3):
        tid = next_id()
        ids.setdefault("approved", []).append(tid)
        item = task(
            tid,
            pick(TITLES, i + 2),
            "approved",
            i + 6,
            planned_by=f"session-creator-{i + 1:02d}",
            intent={"outcome": pick(OUTCOMES, i + 3), "problem": pick(PROBLEMS, i + 2)},
            scope={
                "in_scope": [pick(ACTIONS, i + 1)],
                "out_of_scope": [pick(PROBLEMS, i + 5)],
            },
            acceptance_criteria=[criterion(i, cid="ACCEPT-01"), criterion(i + 3, cid="ACCEPT-02")],
            steps=[
                step(i, 1, "pending", ["ACCEPT-01"], ["crates/core/src/task.rs"]),
                step(i + 1, 2, "pending", ["ACCEPT-02"], ["crates/core/tests/task_model.rs"], depends_on=["STP-01"]),
            ],
            notes=[
                transition_note(
                    i + 6,
                    f"session-creator-{i + 1:02d}",
                    f"Groomed by session-creator-{i + 1:02d}: intent, scope and criteria written.",
                    ts(4 + i, 9),
                ),
                transition_note(
                    i + 6,
                    f"session-creator-{i + 1:02d}",
                    f"Approved by session-creator-{i + 1:02d}: criteria covered, steps planned.",
                    ts(5 + i, 16 + i),
                    note_id="NOTE-02",
                ),
            ],
            # The overlay and the claim fixtures wait only on done work: each
            # is held by exactly one thing, so the tests can tell the two
            # reasons apart. The dependency fixture waits on two groomed
            # tasks, both still unfinished.
            depends_on=(
                [ids["groomed"][0], ids["groomed"][1]]
                if i == 0
                else [DONE_IDS[i - 1]]
            ),
        )
        if i == 1:
            # An overlay holds the task: this is the case of a task blocked
            # by something other than a dependency.
            item["overlays"] = {
                "blocked": {
                    "reason": pick(PROBLEMS, 2),
                    "by": "creator-02",
                    "at": ts(13, 15),
                }
            }
        if i == 2:
            # A claim another session holds: the case of a task blocked by
            # a live claim. The lease has an expiry, far enough out that the
            # case stays held whenever the fixtures are read; it does not
            # lean on an empty lease, which the model reads as never ending.
            item["claim"] = {
                "by": "runner-09",
                "session": "session-runner-09",
                "at": ts(14, 9),
                "lease_expires_at": "2099-12-31T00:00:00Z",
            }
        tasks.append(item)

    # executing — claimed, evidence on the first criterion; one defers a criterion.
    for i in range(3):
        tid = next_id()
        ids.setdefault("executing", []).append(tid)
        claim_day = 9 + i
        criteria = [
            criterion(
                i,
                with_evidence=[
                    evidence(
                        i,
                        f"runner-{i + 1:02d}",
                        f"session-runner-{i + 1:02d}",
                        claim_day,
                        12,
                    )
                ],
            ),
            criterion(i + 3),
        ]
        if i == 1:
            criteria[1]["status"] = "deferred"
            criteria[1]["deferral"] = {
                "reason": pick(PROBLEMS, i + 1),
                "follow_up": ids["draft"][2],
            }
        item = task(
            tid,
            pick(TITLES, i + 11),
            "executing",
            i + 9,
            planned_by=f"session-creator-{i + 1:02d}",
            claim={
                "by": f"runner-{i + 1:02d}",
                "session": f"session-runner-{i + 1:02d}",
                "at": ts(claim_day, 9),
                "lease_expires_at": ts(claim_day, 17),
            },
            intent={"outcome": pick(OUTCOMES, i + 5), "problem": pick(PROBLEMS, i + 3)},
            scope={
                "in_scope": [pick(ACTIONS, i + 2)],
                "out_of_scope": [pick(PROBLEMS, i + 6)],
            },
            acceptance_criteria=criteria,
            steps=[
                step(
                    i,
                    1,
                    "in-progress" if i == 0 else "done",
                    ["ACCEPT-01"],
                    ["crates/core/tests/task_model.rs"],
                    deviation=pick(NOTES, i) if i == 0 else "",
                )
            ],
            review={"max_rounds": 5, "rounds": [], "verdict": None, "findings": []},
            notes=[
                {
                    "id": "NOTE-01",
                    "author": f"runner-{i + 1:02d}",
                    "body": pick(NOTES, i),
                    "at": ts(claim_day, 13),
                }
            ],
            depends_on=[DONE_IDS[i]],
        )
        tasks.append(item)

    # code-review — steps settled, evidence, self round, one open finding.
    for i in range(3):
        tid = next_id()
        ids.setdefault("code-review", []).append(tid)
        review_day = 10 + i
        claim_at = ts(review_day, 9)
        round_at = ts(review_day, 16)
        findings = [finding(i + 1, round_at)]
        item = task(
            tid,
            pick(TITLES, i + 5),
            "code-review",
            i + 12,
            planned_by=f"session-creator-{i + 1:02d}",
            claim={
                "by": f"runner-{i + 1:02d}",
                "session": f"session-runner-{i + 1:02d}",
                "at": claim_at,
                "lease_expires_at": ts(review_day, 18),
            },
            intent={"outcome": pick(OUTCOMES, i + 7), "problem": pick(PROBLEMS, i + 4)},
            scope={
                "in_scope": [pick(ACTIONS, i + 3)],
                "out_of_scope": [pick(PROBLEMS, i)],
            },
            acceptance_criteria=[
                criterion(
                    i,
                    with_evidence=[
                        evidence(
                            i,
                            f"runner-{i + 1:02d}",
                            f"session-runner-{i + 1:02d}",
                            review_day,
                            14,
                        )
                    ],
                )
            ],
            steps=[step(i, 1, "done", ["ACCEPT-01"], ["crates/core/tests/task_model.rs"])],
            review={
                "max_rounds": 5,
                "rounds": [
                    round_(
                        1,
                        "self",
                        "spec-compliance",
                        f"runner-{i + 1:02d}",
                        "changes-needed",
                        i,
                        round_at,
                        ["FIND-01"],
                    )
                ],
                "verdict": None,
                "findings": findings,
            },
            notes=[
                {
                    "id": "NOTE-01",
                    "author": f"runner-{i + 1:02d}",
                    "body": pick(NOTES, i),
                    "at": ts(review_day + 1, 9),
                }
            ],
            depends_on=[DONE_IDS[i], DONE_IDS[(i + 1) % len(DONE_IDS)]],
        )
        if i == 0:
            # The detail pane's fixture. Its note carries a ==mark==, so the
            # document paints text.selected on free text; it carries a fenced
            # code block, so the document paints syntect's code tokens; and
            # its criterion's verification carries a shell pipe, so the
            # criteria table holds a code span with an escaped pipe in it.
            item["notes"][0]["body"] += (
                " The ==key finding== is marked for the next round.\n\n"
                "```rust\n"
                "let answer = 42;\n"
                "```"
            )
            item["acceptance_criteria"][0]["verification"] = (
                "cargo test -p coding-crew-core | grep detail"
            )
        tasks.append(item)

    # remediating — external round with changes-needed, findings open.
    for i in range(3):
        tid = next_id()
        ids.setdefault("remediating", []).append(tid)
        claim_day = 13 + i // 2
        claim_hour = 9 + i % 2
        evidence_hour = claim_hour + 4
        self_hour = claim_hour + 6
        external_hour = claim_hour + 8
        external_at = ts(claim_day, external_hour)
        item = task(
            tid,
            pick(TITLES, i + 9),
            "remediating",
            i + 15,
            planned_by=f"session-creator-{i + 2:02d}",
            claim={
                "by": f"runner-{i + 2:02d}",
                "session": f"session-runner-{i + 2:02d}",
                "at": ts(claim_day, claim_hour),
                "lease_expires_at": ts(claim_day, external_hour),
            },
            intent={"outcome": pick(OUTCOMES, i + 2), "problem": pick(PROBLEMS, i + 5)},
            scope={
                "in_scope": [pick(ACTIONS, i + 4)],
                "out_of_scope": [pick(PROBLEMS, i + 1)],
            },
            acceptance_criteria=[
                criterion(
                    i,
                    with_evidence=[
                        evidence(
                            i,
                            f"runner-{i + 2:02d}",
                            f"session-runner-{i + 2:02d}",
                            claim_day,
                            evidence_hour,
                        )
                    ],
                )
            ],
            steps=[step(i, 1, "done", ["ACCEPT-01"], ["crates/core/tests/task_model.rs"])],
            review={
                "max_rounds": 5,
                "rounds": [
                    round_(
                        1,
                        "self",
                        "spec-compliance",
                        f"runner-{i + 2:02d}",
                        "ok",
                        i,
                        ts(claim_day, self_hour),
                    ),
                    round_(
                        2,
                        "external",
                        pick(FOCI, i + 1),
                        f"reviewer-{i + 1:02d}",
                        "changes-needed",
                        i,
                        external_at,
                        ["FIND-01"],
                    ),
                ],
                "verdict": "changes-needed",
                "findings": [finding(i + 1, external_at)],
            },
            depends_on=[DONE_IDS[(i + 1) % len(DONE_IDS)]],
        )
        tasks.append(item)

    # needs-human — five external rounds exhausted.
    for i in range(3):
        tid = next_id()
        ids.setdefault("needs-human", []).append(tid)
        claim_day = 14 + i
        rounds = [
            round_(
                1,
                "self",
                "spec-compliance",
                f"runner-{i + 3:02d}",
                "ok",
                i,
                ts(claim_day, 5),
            )
        ]
        findings = []
        for n in range(2, 7):
            at = ts(claim_day, 4 + n)
            findings.append(finding(n - 1, at))
            rounds.append(
                round_(
                    n,
                    "external",
                    pick(FOCI, n),
                    f"reviewer-{n - 1:02d}",
                    "changes-needed",
                    n,
                    at,
                    [f"FIND-{n - 1:02d}"],
                )
            )
        item = task(
            tid,
            pick(TITLES, i + 17),
            "needs-human",
            i + 18,
            planned_by=f"session-creator-{i + 3:02d}",
            claim={
                "by": f"runner-{i + 3:02d}",
                "session": f"session-runner-{i + 3:02d}",
                "at": ts(claim_day, 1),
                "lease_expires_at": ts(claim_day, 11),
            },
            intent={"outcome": pick(OUTCOMES, i + 4), "problem": pick(PROBLEMS, i + 6)},
            scope={
                "in_scope": [pick(ACTIONS, i + 5)],
                "out_of_scope": [pick(PROBLEMS, i + 2)],
            },
            acceptance_criteria=[
                criterion(
                    i,
                    with_evidence=[
                        evidence(
                            i,
                            f"runner-{i + 3:02d}",
                            f"session-runner-{i + 3:02d}",
                            claim_day,
                            3,
                        )
                    ],
                )
            ],
            steps=[step(i, 1, "done", ["ACCEPT-01"], ["crates/core/tests/task_model.rs"])],
            review={
                "max_rounds": 5,
                "rounds": rounds,
                "verdict": "changes-needed",
                "findings": findings,
            },
            notes=[
                {
                    "id": "NOTE-01",
                    "author": f"reviewer-{5:02d}",
                    "body": pick(NOTES, i),
                    "at": ts(claim_day, 12),
                }
            ],
            depends_on=[DONE_IDS[i], DONE_IDS[(i + 2) % len(DONE_IDS)]],
        )
        tasks.append(item)

    # done — clean verdict, no open findings. Five of them, chained only
    # among themselves: the done set is closed under its waits-on edges.
    # The chain is scheduled first in the month, before every task that
    # waits on it, so each of those can be claimed after its dependencies
    # finished.
    done_dependencies = [
        [],
        [DONE_IDS[0]],
        [DONE_IDS[0], DONE_IDS[1]],
        [DONE_IDS[2]],
        [DONE_IDS[1], DONE_IDS[2], DONE_IDS[3]],
    ]
    done_schedule = [
        (ts(5, 15), (5, 16, 0), ts(5, 17), ts(5, 18)),
        (ts(6, 16), (6, 17, 0), ts(6, 18), ts(6, 19)),
        (ts(7, 17), (7, 18, 0), ts(7, 19), ts(7, 20)),
        (ts(7, 20, 30), (7, 20, 40), ts(7, 20, 50), ts(7, 21)),
        (ts(8, 9), (8, 10, 0), ts(8, 11), ts(8, 12)),
    ]
    for i in range(5):
        tid = next_id()
        assert tid == DONE_IDS[i], (tid, DONE_IDS[i])
        ids.setdefault("done", []).append(tid)
        claim_at, (evidence_day, evidence_hour, evidence_minute), self_at, external_at = done_schedule[i]
        item = task(
            tid,
            pick(TITLES, i + 12),
            "done",
            i + 21,
            planned_by=f"session-creator-{i + 4:02d}",
            claim={
                "by": f"runner-{i + 4:02d}",
                "session": f"session-runner-{i + 4:02d}",
                "at": claim_at,
                "lease_expires_at": external_at,
            },
            intent={"outcome": pick(OUTCOMES, i + 6), "problem": pick(PROBLEMS, i)},
            scope={
                "in_scope": [pick(ACTIONS, i + 6)],
                "out_of_scope": [pick(PROBLEMS, i + 3)],
            },
            acceptance_criteria=[
                criterion(
                    i,
                    with_evidence=[
                        evidence(
                            i,
                            f"runner-{i + 4:02d}",
                            f"session-runner-{i + 4:02d}",
                            evidence_day,
                            evidence_hour,
                            minute=evidence_minute,
                        )
                    ],
                )
            ],
            steps=[step(i, 1, "done", ["ACCEPT-01"], ["crates/core/tests/task_model.rs"])],
            review={
                "max_rounds": 5,
                "rounds": [
                    round_(
                        1,
                        "self",
                        "spec-compliance",
                        f"runner-{i + 4:02d}",
                        "ok",
                        i,
                        self_at,
                    ),
                    round_(
                        2,
                        "external",
                        pick(FOCI, i),
                        f"reviewer-{i + 5:02d}",
                        "ok",
                        i,
                        external_at,
                    ),
                ],
                "verdict": "ok",
                "findings": [],
            },
            depends_on=done_dependencies[i],
        )
        tasks.append(item)

    done_ids = ids["done"]

    # smaller tasks — ideas, bugs and small features, plus the trail of one
    # that became a task.
    def item(item_id, title, kind, status, body="", scope=None, promoted_to=None,
             dismissed_reason=""):
        return {
            "schema_version": 1,
            "id": item_id,
            "title": title,
            "slug": slugify(title),
            "kind": kind,
            "status": status,
            "body": body,
            "scope": scope or [],
            "tags": ["mock"],
            "created": ts(2, 8),
            "updated": ts(6, 15),
            "promoted_to": promoted_to,
            "dismissed_reason": dismissed_reason,
        }

    items = [
        item("ITM01", "Lorem ipsum dolor sit amet", "idea", "open",
             body="Consectetur adipiscing elit, sed do eiusmod tempor incididunt."),
        item("ITM02", "Tempor incididunt ut labore et dolore", "feature", "open",
             body="Magna aliqua, ut enim ad minim veniam.",
             scope=["the lorem pane"]),
        item("ITM03", "Quis nostrud exercitation ullamco laboris", "bug", "open",
             body="Nisi ut aliquip ex ea commodo consequat.",
             scope=["the lorem list"]),
        item("ITM04", "Sed do eiusmod tempor incididunt", "feature", "dismissed",
             body="Ut labore et dolore magna aliqua.",
             dismissed_reason="Lorem ipsum dolor sit amet, consectetur."),
    ]

    # A bug that was promoted: the task it became is a small task.
    promoted_title = "Duis aute irure dolor in reprehenderit"
    promoted_id = next_id()
    promoted_task = task(promoted_id, promoted_title, "draft", 4)
    promoted_task["kind"] = "bug"
    promoted_task["promoted_from"] = "ITM05"
    promoted_task["intent"] = {
        "outcome": promoted_title,
        "problem": "Velit esse cillum dolore eu fugiat.",
    }
    promoted_task["scope"] = {"in_scope": ["the lorem button"], "out_of_scope": []}
    promoted_task["acceptance_criteria"] = [
        {
            "id": "ACCEPT-01",
            "description": promoted_title,
            "verification": "",
            "status": "pending",
            "evidence": [],
            "deferral": None,
            "failure_reason": "",
        }
    ]
    tasks.append(promoted_task)
    items.append(
        item("ITM05", promoted_title, "bug", "promoted",
             body="Velit esse cillum dolore eu fugiat.",
             scope=["the lorem button"],
             promoted_to=promoted_id)
    )

    # parent / child: the first approved task owns a draft child.
    child = next_id()
    ids.setdefault("draft", []).append(child)
    child_task = task(child, pick(TITLES, 19), "draft", 2)
    child_task["parent"] = ids["approved"][0]
    tasks.append(child_task)
    for item in tasks:
        if item["id"] == ids["approved"][0]:
            item["children"] = [child]

    # approved after the last cycle finished: its dependencies are done, so
    # an agent can claim it right now. It is generated last, so it takes the
    # next id and the ids of the older fixtures keep their meaning.
    tid = next_id()
    ids.setdefault("approved", []).append(tid)
    tasks.append(
        task(
            tid,
            pick(TITLES, 20),
            "approved",
            6,
            planned_by="session-creator-05",
            intent={"outcome": pick(OUTCOMES, 8), "problem": pick(PROBLEMS, 7)},
            scope={
                "in_scope": [pick(ACTIONS, 7)],
                "out_of_scope": [pick(PROBLEMS, 3)],
            },
            acceptance_criteria=[
                criterion(2, cid="ACCEPT-01"),
                criterion(5, cid="ACCEPT-02"),
            ],
            steps=[
                step(2, 1, "pending", ["ACCEPT-01"], ["crates/core/src/task.rs"]),
                step(
                    5,
                    2,
                    "pending",
                    ["ACCEPT-02"],
                    ["crates/core/tests/task_model.rs"],
                    depends_on=["STP-01"],
                ),
            ],
            notes=[
                transition_note(
                    6,
                    "session-creator-05",
                    "Groomed by session-creator-05: intent, scope and criteria written.",
                    ts(13, 10),
                ),
                transition_note(
                    6,
                    "session-creator-05",
                    "Approved by session-creator-05: criteria covered, steps planned.",
                    # After the dependency it waits for is done (TSK22).
                    ts(16, 12),
                    note_id="NOTE-02",
                ),
            ],
            depends_on=[done_ids[0]],
        )
    )

    for task_item in tasks:
        finalize(task_item)
    check_claims_follow_dependencies(tasks)

    return tasks, items


def main():
    if TASKS.exists():
        shutil.rmtree(TASKS)
    TASKS.mkdir(parents=True)
    tasks, items = build()
    for record in tasks + items:
        path = TASKS / f"{record['id']}-{record['slug']}.json"
        path.write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n")
        print(path.relative_to(ROOT))
    events = activity_events(tasks)
    check_activity(tasks, events)
    ACTIVITY.write_text(
        json.dumps({"events": events}, indent=2, ensure_ascii=False) + "\n"
    )
    print(ACTIVITY.relative_to(ROOT))
    runtime = runtime_record(tasks)
    check_runtime(runtime, tasks, events)
    RUNTIME.write_text(json.dumps(runtime, indent=2, ensure_ascii=False) + "\n")
    print(RUNTIME.relative_to(ROOT))


if __name__ == "__main__":
    main()
