use coding_lanes_core::{Status, Store, Task, parse_rfc3339};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn mock_store() -> Store {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("mock-data/coding-lanes/work");
    Store::new(root)
}

fn tasks() -> Vec<Task> {
    mock_store().list().expect("listing the mock tasks")
}

fn by_status(status: Status) -> Vec<Task> {
    tasks().into_iter().filter(|t| t.status == status).collect()
}

#[test]
fn every_fixture_loads() {
    let tasks = tasks();
    assert_eq!(tasks.len(), 29);

    let ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids.len(), tasks.len(), "task ids must be unique");
}

#[test]
fn every_fixture_has_a_status_since_between_created_and_updated() {
    for task in tasks() {
        assert!(
            !task.status_since.is_empty(),
            "{} has no status_since",
            task.id
        );
        let created = parse_rfc3339(&task.created)
            .unwrap_or_else(|| panic!("{} has an unreadable created", task.id));
        let since = parse_rfc3339(&task.status_since)
            .unwrap_or_else(|| panic!("{} has an unreadable status_since", task.id));
        let updated = parse_rfc3339(&task.updated)
            .unwrap_or_else(|| panic!("{} has an unreadable updated", task.id));
        assert!(
            created <= since,
            "{} entered its status before it was created",
            task.id
        );
        assert!(
            since <= updated,
            "{} entered its status after its last write",
            task.id
        );
    }
}

/// The moment a note records a status transition.
fn transition_note_at(task: &Task, prefix: &str) -> String {
    task.notes
        .iter()
        .rev()
        .find(|note| note.body.starts_with(prefix))
        .unwrap_or_else(|| panic!("{} has no {prefix} transition note", task.id))
        .at
        .clone()
}

/// The named event that entered the status a task is in.
fn status_transition_event(task: &Task) -> String {
    match task.status {
        Status::Draft => task.created.clone(),
        Status::Groomed => transition_note_at(task, "Groomed by"),
        Status::Approved => transition_note_at(task, "Approved by"),
        Status::Executing => task.claim.as_ref().expect("a claim").at.clone(),
        Status::CodeReview | Status::Remediating | Status::NeedsHuman | Status::Done => task
            .review
            .rounds
            .last()
            .expect("a review round")
            .at
            .clone(),
    }
}

/// Every lifecycle event, in the order the model has them: creation, the
/// transitions a note records, the claim, the evidence, and the review
/// rounds that assess it.
fn lifecycle_events(task: &Task) -> Vec<String> {
    let mut events = vec![task.created.clone()];
    for note in &task.notes {
        if note.body.starts_with("Groomed by") || note.body.starts_with("Approved by") {
            events.push(note.at.clone());
        }
    }
    if let Some(claim) = &task.claim {
        events.push(claim.at.clone());
    }
    for criterion in &task.acceptance_criteria {
        for evidence in &criterion.evidence {
            events.push(evidence.at.clone());
        }
    }
    for round in &task.review.rounds {
        events.push(round.at.clone());
    }
    events
}

#[test]
fn the_lifecycle_events_of_every_fixture_happen_in_order() {
    for task in tasks() {
        let events = lifecycle_events(&task);
        let mut previous: Option<i64> = None;
        for event in &events {
            let at = parse_rfc3339(event)
                .unwrap_or_else(|| panic!("{} has an unreadable event {event}", task.id));
            if let Some(previous) = previous {
                assert!(
                    previous <= at,
                    "{}: {event} happens before the event before it: {events:?}",
                    task.id
                );
            }
            previous = Some(at);
        }
    }
}

#[test]
fn every_fixture_is_stamped_at_the_event_that_entered_its_status() {
    for task in tasks() {
        let since = parse_rfc3339(&task.status_since)
            .unwrap_or_else(|| panic!("{} has an unreadable status_since", task.id));
        let named = status_transition_event(&task);
        let named = parse_rfc3339(&named).expect("the transition event");
        assert_eq!(
            since, named,
            "{} is not stamped at the event that entered {:?}",
            task.id, task.status
        );
    }
}

#[test]
fn every_fixture_entered_its_status_after_the_phase_before_it() {
    // `status_since` may not name a moment earlier than an event of the
    // phase that came first: the claim and the evidence precede the review
    // that assesses them, and a transition note precedes the date it makes.
    for task in tasks() {
        let since = parse_rfc3339(&task.status_since)
            .unwrap_or_else(|| panic!("{} has an unreadable status_since", task.id));
        let events = lifecycle_events(&task);
        let named = status_transition_event(&task);
        let at = events
            .iter()
            .rposition(|event| *event == named)
            .unwrap_or_else(|| panic!("{}: {named} is not in its lifecycle", task.id));
        for event in &events[..at] {
            let moment = parse_rfc3339(event)
                .unwrap_or_else(|| panic!("{} has an unreadable event {event}", task.id));
            assert!(
                moment <= since,
                "{} entered its status at {} before {event}",
                task.id,
                task.status_since
            );
        }
    }
}

#[test]
fn no_fixture_stamps_the_day_after_it_was_created() {
    // The fixture stamp used to be `created` plus a day, whatever the
    // fixture's history said. This check fails for that derivation.
    for task in tasks() {
        let created = parse_rfc3339(&task.created).expect("created");
        let since = parse_rfc3339(&task.status_since).expect("status_since");
        assert_ne!(
            since,
            created + 86_400,
            "{} still carries created plus a day",
            task.id
        );
    }
}

#[test]
fn the_self_review_round_put_tsk13_in_review() {
    let tasks = tasks();
    let task = tasks.iter().find(|task| task.id == "TSK13").expect("TSK13");
    assert_eq!(task.status, Status::CodeReview);
    assert_eq!(task.status_since, "2026-11-10T16:00Z");
    assert_eq!(
        task.status_since,
        task.review.rounds.last().expect("a round").at
    );
    // The old generator stamped the day after creation.
    assert_ne!(task.status_since, "2026-11-03T13:00Z");
}

#[test]
fn the_fifth_external_round_escalated_tsk19() {
    let tasks = tasks();
    let task = tasks.iter().find(|task| task.id == "TSK19").expect("TSK19");
    assert_eq!(task.status, Status::NeedsHuman);
    assert_eq!(task.external_rounds(), 5);
    assert_eq!(task.status_since, "2026-11-14T10:00Z");
    assert_eq!(
        task.status_since,
        task.review.rounds.last().expect("a round").at
    );
    // The old generator stamped the day after creation.
    assert_ne!(task.status_since, "2026-11-03T11:00Z");
}

#[test]
fn every_fixture_was_last_written_at_its_latest_event() {
    for task in tasks() {
        let updated = parse_rfc3339(&task.updated).expect("updated");
        let latest = event_stamps(&task)
            .iter()
            .map(|stamp| {
                parse_rfc3339(stamp)
                    .unwrap_or_else(|| panic!("{} has an unreadable event {stamp}", task.id))
            })
            .max()
            .expect("an event");
        assert_eq!(
            updated, latest,
            "{} was last written at {} but its latest event is later",
            task.id, task.updated
        );
    }
}

/// Every dated event in a fixture's history.
fn event_stamps(task: &Task) -> Vec<String> {
    let mut stamps = vec![task.created.clone(), task.status_since.clone()];
    if let Some(claim) = &task.claim {
        stamps.push(claim.at.clone());
    }
    for overlay in [
        &task.overlays.blocked,
        &task.overlays.deferred,
        &task.overlays.cancelled,
    ]
    .into_iter()
    .flatten()
    {
        stamps.push(overlay.at.clone());
    }
    for criterion in &task.acceptance_criteria {
        for evidence in &criterion.evidence {
            stamps.push(evidence.at.clone());
        }
    }
    for round in &task.review.rounds {
        stamps.push(round.at.clone());
    }
    for finding in &task.review.findings {
        stamps.push(finding.created.clone());
        if !finding.resolved.is_empty() {
            stamps.push(finding.resolved.clone());
        }
        for comment in &finding.comments {
            if !comment.at.is_empty() {
                stamps.push(comment.at.clone());
            }
        }
    }
    for note in &task.notes {
        stamps.push(note.at.clone());
    }
    for question in &task.questions {
        stamps.push(question.raised_at.clone());
        if !question.resolved_at.is_empty() {
            stamps.push(question.resolved_at.clone());
        }
    }
    stamps
}

#[test]
fn the_human_lane_fixtures_do_not_fall_back_to_created_or_updated() {
    // The Needs you tab measures the wait from status_since. In the human
    // lane the three stamps are distinct, so a rendered wait can only come
    // from status_since: not from the age of the task, not from the age of
    // the last write.
    let human_lane = [Status::Groomed, Status::CodeReview, Status::NeedsHuman];
    for task in tasks()
        .into_iter()
        .filter(|t| human_lane.contains(&t.status))
    {
        assert_ne!(
            task.status_since, task.created,
            "{} waits since it was created",
            task.id
        );
        assert_ne!(
            task.status_since, task.updated,
            "{} waits since its last write",
            task.id
        );
    }
}

#[test]
fn the_fixtures_of_a_phase_enter_it_at_different_moments() {
    // The rows of a group are read together: every fixture of a phase
    // carries its own status_since, so no two rows of a group show the
    // same wait.
    for status in [Status::Groomed, Status::CodeReview, Status::NeedsHuman] {
        let fixtures = by_status(status);
        let stamps: HashSet<&str> = fixtures
            .iter()
            .map(|task| task.status_since.as_str())
            .collect();
        assert_eq!(
            stamps.len(),
            fixtures.len(),
            "two fixtures of {status:?} entered it at the same moment"
        );
    }
}

#[test]
fn every_phase_is_covered() {
    for status in [
        Status::Draft,
        Status::Groomed,
        Status::Approved,
        Status::Executing,
        Status::CodeReview,
        Status::Remediating,
        Status::NeedsHuman,
        Status::Done,
    ] {
        assert!(
            !by_status(status).is_empty(),
            "no fixture for status {status:?}"
        );
    }
}

#[test]
fn phase_counts_match_the_fixtures() {
    assert_eq!(by_status(Status::Groomed).len(), 3);
    assert_eq!(
        by_status(Status::Approved).len(),
        4,
        "three in the flow, one approved after the last cycle finished"
    );
    assert_eq!(by_status(Status::Executing).len(), 3);
    assert_eq!(by_status(Status::CodeReview).len(), 3);
    assert_eq!(by_status(Status::Remediating).len(), 3);
    assert_eq!(by_status(Status::NeedsHuman).len(), 3);
    assert_eq!(by_status(Status::Done).len(), 5);
    assert_eq!(
        by_status(Status::Draft).len(),
        5,
        "three drafts, one child, one promoted bug"
    );
}

#[test]
fn fixtures_keep_their_gates() {
    for task in by_status(Status::Approved) {
        assert!(
            task.criteria_covered(),
            "{} has uncovered criteria",
            task.id
        );
        assert!(
            task.steps_have_done_when(),
            "{} has a step without done_when",
            task.id
        );
    }

    for task in by_status(Status::Executing) {
        assert!(task.claim.is_some(), "{} is not claimed", task.id);
        assert!(
            task.acceptance_criteria
                .iter()
                .any(|c| !c.evidence.is_empty()),
            "{} has no evidence",
            task.id
        );
    }

    for task in by_status(Status::CodeReview) {
        assert!(task.self_rounds() > 0, "{} has no self round", task.id);
        assert!(
            task.steps
                .iter()
                .all(|s| s.status == coding_lanes_core::StepStatus::Done),
            "{} has unsettled steps",
            task.id
        );
    }

    for task in by_status(Status::Remediating) {
        assert!(task.open_findings() > 0, "{} has no open finding", task.id);
        assert_eq!(
            task.review.verdict,
            Some(coding_lanes_core::Verdict::ChangesNeeded)
        );
    }

    for task in by_status(Status::NeedsHuman) {
        assert_eq!(
            task.external_rounds() as u32,
            task.review.max_rounds,
            "{} has not exhausted the round budget",
            task.id
        );
    }

    for task in by_status(Status::Done) {
        assert_eq!(task.open_findings(), 0, "{} has open findings", task.id);
        assert_eq!(task.review.verdict, Some(coding_lanes_core::Verdict::Ok));
    }
}

#[test]
fn dependencies_point_to_existing_tasks_and_form_no_cycle() {
    let tasks = tasks();
    let ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    for task in &tasks {
        for dependency in &task.depends_on {
            assert!(
                ids.contains(dependency.as_str()),
                "{} depends on unknown task {dependency}",
                task.id
            );
        }
    }
    if let Some(cycle) = dependency_cycle(&tasks) {
        panic!("the fixtures wait on each other in a cycle: {cycle}");
    }
}

/// The first waits-on cycle in the fixtures, if there is one.
fn dependency_cycle(tasks: &[Task]) -> Option<String> {
    fn visit<'a>(
        id: &'a str,
        tasks: &'a [Task],
        visiting: &mut Vec<&'a str>,
        settled: &mut HashSet<&'a str>,
    ) -> Option<String> {
        if settled.contains(id) {
            return None;
        }
        if let Some(start) = visiting.iter().position(|seen| *seen == id) {
            let mut cycle: Vec<&str> = visiting[start..].to_vec();
            cycle.push(id);
            return Some(cycle.join(" -> "));
        }
        visiting.push(id);
        let task = tasks.iter().find(|task| task.id == id)?;
        for dependency in &task.depends_on {
            if let Some(cycle) = visit(dependency, tasks, visiting, settled) {
                return Some(cycle);
            }
        }
        visiting.pop();
        settled.insert(id);
        None
    }

    let mut settled = HashSet::new();
    for task in tasks {
        if let Some(cycle) = visit(&task.id, tasks, &mut Vec::new(), &mut settled) {
            return Some(cycle);
        }
    }
    None
}

#[test]
fn every_task_past_approved_waits_only_on_done_work() {
    // `dependencies.satisfied` lets an agent take an approved task only when
    // every dependency is done, and a status never goes back. So a task past
    // approved can only exist if its dependencies were done when it was
    // claimed, and they are still done now. This is the rule the Path tab is
    // drawn on, in structure and in time.
    let tasks = tasks();
    let by_id: HashMap<&str, &Task> = tasks.iter().map(|task| (task.id.as_str(), task)).collect();
    let past_approved = [
        Status::Executing,
        Status::CodeReview,
        Status::Remediating,
        Status::NeedsHuman,
        Status::Done,
    ];

    let mut checked = 0;
    for task in tasks
        .iter()
        .filter(|task| past_approved.contains(&task.status))
    {
        checked += 1;
        for dependency in &task.depends_on {
            let dependency = by_id
                .get(dependency.as_str())
                .unwrap_or_else(|| panic!("{} waits on unknown task {dependency}", task.id));
            assert_eq!(
                dependency.status,
                Status::Done,
                "{} is {:?} but waits on {}, which is {:?}",
                task.id,
                task.status,
                dependency.id,
                dependency.status
            );
            // The claim gate needs both: a claim, and dependencies done. A
            // fixture claimed before the work it waits on finished would tell
            // a chronology the model does not allow.
            let claim_at = &task.claim.as_ref().expect("a claim").at;
            let claimed = parse_rfc3339(claim_at)
                .unwrap_or_else(|| panic!("{} has an unreadable claim", task.id));
            let finished = parse_rfc3339(&dependency.status_since)
                .unwrap_or_else(|| panic!("{} has an unreadable status_since", dependency.id));
            assert!(
                claimed > finished,
                "{} was claimed at {claim_at}, before {} finished at {}",
                task.id,
                dependency.id,
                dependency.status_since
            );
        }
    }
    assert!(checked > 0, "no fixture is past approved");

    // The done set is closed under waits-on.
    for task in tasks.iter().filter(|task| task.status == Status::Done) {
        for dependency in &task.depends_on {
            let dependency = by_id
                .get(dependency.as_str())
                .unwrap_or_else(|| panic!("{} waits on unknown task {dependency}", task.id));
            assert_eq!(
                dependency.status,
                Status::Done,
                "{} is done but waits on {}, which is {:?}",
                task.id,
                dependency.id,
                dependency.status
            );
        }
    }

    // An approved task may still wait on work that is not done: that is the
    // Path tab's blocked branch, not a contradiction.
    assert!(
        tasks
            .iter()
            .filter(|task| task.status == Status::Approved)
            .flat_map(|task| task.depends_on.iter())
            .any(|dependency| by_id[dependency.as_str()].status != Status::Done),
        "no approved task is legitimately blocked by unfinished work"
    );
}

#[test]
fn every_phase_after_the_first_depends_on_something() {
    for status in [
        Status::Approved,
        Status::Executing,
        Status::CodeReview,
        Status::Remediating,
        Status::NeedsHuman,
        // A done task may wait on nothing: the done graph has a root. The
        // done tasks that do wait are covered by
        // every_task_past_approved_waits_only_on_done_work.
    ] {
        for task in by_status(status) {
            assert!(!task.depends_on.is_empty(), "{} has no dependency", task.id);
        }
    }
}

#[test]
fn child_task_points_at_its_parent() {
    let tasks = tasks();
    let child = tasks
        .iter()
        .find(|t| t.parent.is_some())
        .expect("a child task");
    let parent = tasks
        .iter()
        .find(|t| t.id == child.parent.clone().unwrap())
        .expect("the parent task");
    assert!(parent.children.contains(&child.id));
}

#[test]
fn deferred_criterion_names_its_follow_up() {
    let tasks = tasks();
    let deferred: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.acceptance_criteria.iter().any(|c| c.deferral.is_some()))
        .collect();
    assert!(!deferred.is_empty(), "no fixture with a deferred criterion");

    for task in deferred {
        for criterion in &task.acceptance_criteria {
            if let Some(deferral) = &criterion.deferral {
                assert!(
                    tasks.iter().any(|t| t.id == deferral.follow_up),
                    "{} defers to unknown task {}",
                    task.id,
                    deferral.follow_up
                );
            }
        }
    }
}

#[test]
fn dependencies_fan_in_and_chain() {
    let tasks = tasks();

    let fan_in = tasks.iter().filter(|t| t.depends_on.len() >= 3).count();
    assert!(fan_in > 0, "no task waits on three others");

    let same_phase_chain = tasks.iter().any(|task| {
        task.depends_on.iter().any(|dependency| {
            tasks
                .iter()
                .any(|other| &other.id == dependency && other.status == task.status)
        })
    });
    assert!(same_phase_chain, "no task waits on a task in its own phase");
}

#[test]
fn a_task_waits_on_a_blocked_task() {
    let tasks = tasks();
    let blocked: Vec<&str> = tasks
        .iter()
        .filter(|t| t.overlays.blocked.is_some())
        .map(|t| t.id.as_str())
        .collect();
    // One groomed task and one approved task: the approved one is held by
    // an overlay rather than by a dependency that is not done.
    assert_eq!(blocked.len(), 2);

    let waiting = tasks.iter().any(|task| {
        task.depends_on
            .iter()
            .any(|dependency| blocked.contains(&dependency.as_str()))
    });
    assert!(waiting, "no task waits on the blocked one");
}

#[test]
fn the_fixtures_are_blocked() {
    let blocked = tasks()
        .into_iter()
        .filter(|t| t.overlays.blocked.is_some())
        .count();
    assert_eq!(blocked, 2);
}

#[test]
fn a_claim_is_held_by_another_session() {
    // The case of a task that cannot be claimed because another session
    // holds the claim: the lease is empty, so the claim never expires.
    let held = tasks()
        .into_iter()
        .filter(|task| {
            task.claim
                .as_ref()
                .is_some_and(|claim| claim.lease_expires_at.is_empty())
        })
        .count();
    assert_eq!(held, 1, "no fixture for a claim held elsewhere");
}

#[test]
fn smaller_tasks_are_fixtures_too() {
    let store = mock_store();
    let items = store.list_items().expect("listing the mock items");
    assert_eq!(items.len(), 5);

    let kinds: HashSet<&str> = items.iter().map(|i| i.kind.as_str()).collect();
    for kind in ["idea", "feature", "bug"] {
        assert!(kinds.contains(kind), "no item of kind {kind}");
    }

    let promoted = items
        .iter()
        .find(|i| i.status == coding_lanes_core::ItemStatus::Promoted);
    let promoted = promoted.expect("a promoted item");
    let task_id = promoted.promoted_to.clone().expect("the task it became");
    let task = store.load(&task_id).expect("the promoted task");
    assert_eq!(task.promoted_from.as_deref(), Some(promoted.id.as_str()));
    assert!(task.is_small_task());

    let dismissed = items
        .iter()
        .find(|i| i.status == coding_lanes_core::ItemStatus::Dismissed);
    let dismissed = dismissed.expect("a dismissed item");
    assert!(!dismissed.dismissed_reason.is_empty());
}
