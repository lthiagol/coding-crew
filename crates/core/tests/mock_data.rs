use coding_crew_core::lifecycle::{Event, TransitionContext, transition};
use coding_crew_core::{Actor, Role, RoundKind, Status, Store, Task, Verdict, parse_rfc3339};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn mock_store() -> Store {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("mock-data/coding-crew/work");
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
    assert_eq!(tasks.len(), 30);

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
        Status::CodeReview => {
            // A hand-back to review is the latest move into code-review, when
            // it comes after the round it answers. Otherwise `finish` put the
            // task there with its self round, the first.
            let round = &task.review.rounds.last().expect("a review round").at;
            match task.review.re_reviews.last() {
                Some(hand_back) if parse_rfc3339(&hand_back.at) > parse_rfc3339(round) => {
                    hand_back.at.clone()
                }
                _ => task.review.rounds[0].at.clone(),
            }
        }
        Status::Remediating | Status::NeedsHuman | Status::Done => task
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
    // A hand-back follows the round whose findings it answers.
    for round in &task.review.rounds {
        events.push(round.at.clone());
        for hand_back in task
            .review
            .re_reviews
            .iter()
            .filter(|h| h.after_round == round.n)
        {
            events.push(hand_back.at.clone());
        }
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
fn tsk14_is_handed_back_after_its_findings_are_fixed() {
    // TSK14's one external round raised three findings. The runner fixed
    // them, and the task went back to review: the hand-back is stamped after
    // the last fix and names the remediating session, which is not the
    // session that executed the task.
    let task = tasks()
        .into_iter()
        .find(|task| task.id == "TSK14")
        .expect("TSK14");
    let [hand_back] = task.review.re_reviews.as_slice() else {
        panic!("TSK14 has one hand-back: {:?}", task.review.re_reviews);
    };
    assert_eq!(hand_back.after_round, 1);
    assert_eq!(
        hand_back.session.as_deref(),
        Some("session-runner-02-remediation-1")
    );
    assert_ne!(
        hand_back.session,
        task.claim.as_ref().and_then(|claim| claim.session.clone())
    );
    let last_fix = task
        .review
        .findings
        .iter()
        .filter_map(|finding| parse_rfc3339(&finding.resolved))
        .max()
        .expect("TSK14's findings are fixed");
    assert!(parse_rfc3339(&hand_back.at).expect("a stamp") > last_fix);
    assert_eq!(task.status_since, hand_back.at);
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
    // `finish` moved the task here with its self round, the first. The
    // external round came after, and raised the findings.
    let [self_round, external] = task.review.rounds.as_slice() else {
        panic!(
            "TSK13 has a self and an external round: {:?}",
            task.review.rounds
        );
    };
    assert_eq!(self_round.kind, RoundKind::SelfReview);
    assert!(
        self_round.findings.is_empty(),
        "a self round carries no findings"
    );
    assert_eq!(task.status_since, self_round.at);
    assert_eq!(external.kind, RoundKind::External);
    assert!(
        parse_rfc3339(&external.at) > parse_rfc3339(&self_round.at),
        "the external round follows the self round"
    );
    // The old generator stamped the day after creation.
    assert_ne!(task.status_since, "2026-11-03T13:00Z");
}

#[test]
fn no_self_round_carries_a_finding() {
    // ADR 0017: a self round with findings is refused, so no fixture may hold
    // one. The findings of a code-review task sit on its external round.
    for task in tasks() {
        for round in task
            .review
            .rounds
            .iter()
            .filter(|round| round.kind == RoundKind::SelfReview)
        {
            assert!(
                round.findings.is_empty(),
                "{} self round {} carries findings: {:?}",
                task.id,
                round.n,
                round.findings
            );
        }
    }
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

#[test]
fn a_closure_note_comes_after_the_round_that_closed_it() {
    // A note that says the review is closed is written when the review
    // closes: it cannot precede the round that made it. The TSK23 fixture
    // used to stamp its closure from the schedule's nominal external round,
    // fifty minutes before the round that actually closed it.
    for task in tasks() {
        let closures: Vec<&coding_crew_core::Note> = task
            .notes
            .iter()
            .filter(|note| note.body.starts_with("Reviewed clean"))
            .collect();
        if closures.is_empty() {
            continue;
        }
        let closed = task
            .review
            .rounds
            .iter()
            .rev()
            .find(|round| round.verdict == coding_crew_core::Verdict::Ok)
            .unwrap_or_else(|| panic!("{} claims a closure with no ok round", task.id));
        let closed_at = parse_rfc3339(&closed.at)
            .unwrap_or_else(|| panic!("{} has an unreadable round", task.id));
        for note in closures {
            let at = parse_rfc3339(&note.at)
                .unwrap_or_else(|| panic!("{} has an unreadable closure note", task.id));
            assert!(
                at > closed_at,
                "{}: the closure note at {} precedes the round that closed it at {}",
                task.id,
                note.at,
                closed.at
            );
        }
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
        6,
        "three plain drafts, one promoted bug, two children"
    );
}

/// Replay `event` on `task`, put back to `from`, through the real gate.
///
/// The fixture must pass the gates of the phase it sits in, so the event that
/// carries it into that phase must be allowed from the phase before it.
fn must_pass(task: &Task, from: Status, event: Event, actor: &Actor, done: &HashSet<String>) {
    let mut replayed = task.clone();
    replayed.status = from;
    if from == Status::CodeReview {
        // The fixture's stamp dates the phase it is in now, not the review it
        // replays. A task in review entered it no later than its latest
        // external round was filed, so the replay dates it there.
        replayed.status_since = latest_external(task)
            .expect("a task in review has a filed external round")
            .at
            .clone();
    }
    let is_done = |id: &str| done.contains(id);
    let ctx = TransitionContext {
        dependency_done: Some(&is_done),
        reason: None,
    };
    if let Err(error) = transition(&mut replayed, event, actor, &ctx) {
        panic!("{} does not pass {event:?} from {from:?}: {error}", task.id);
    }
}

fn creator() -> Actor {
    Actor::agent("creator", Role::Creator).in_session("session-creator")
}

/// The reviewer who filed the task's latest external round: the review move
/// rests on that round, so its reviewer and session make the move.
fn reviewer_of(task: &Task) -> Actor {
    let round = latest_external(task).expect("a filed external round");
    let session = round.session.as_deref().expect("a recorded round session");
    Actor::agent(round.reviewer.as_str(), Role::Reviewer).in_session(session)
}

fn latest_external(task: &Task) -> Option<&coding_crew_core::Round> {
    task.review
        .rounds
        .iter()
        .filter(|round| round.kind == RoundKind::External)
        .max_by_key(|round| round.n)
}

fn runner_of(task: &Task) -> Actor {
    let claim = task.claim.as_ref().expect("a claimed task");
    let session = claim.session.as_deref().expect("a recorded claim session");
    Actor::agent(claim.by.as_str(), Role::Runner).in_session(session)
}

#[test]
fn fixtures_keep_their_gates() {
    let done: HashSet<String> = by_status(Status::Done).into_iter().map(|t| t.id).collect();

    for task in by_status(Status::Groomed) {
        must_pass(&task, Status::Draft, Event::Groom, &creator(), &done);
    }

    for task in by_status(Status::Approved) {
        must_pass(&task, Status::Groomed, Event::Approve, &creator(), &done);
    }

    for mut task in by_status(Status::Executing) {
        // The lease is a clock fact, and these fixtures were written on dates
        // that have passed. Its gate is tested on its own in `task.rs`.
        task.claim
            .as_mut()
            .expect("a claimed task")
            .lease_expires_at
            .clear();
        must_pass(
            &task,
            Status::Approved,
            Event::Start,
            &runner_of(&task),
            &done,
        );
    }

    for task in by_status(Status::CodeReview) {
        must_pass(
            &task,
            Status::Executing,
            Event::Finish,
            &runner_of(&task),
            &done,
        );
    }

    for task in by_status(Status::Remediating) {
        must_pass(
            &task,
            Status::CodeReview,
            Event::RequestChanges,
            &reviewer_of(&task),
            &done,
        );
    }

    for task in by_status(Status::NeedsHuman) {
        must_pass(
            &task,
            Status::CodeReview,
            Event::Escalate,
            &reviewer_of(&task),
            &done,
        );
    }

    for task in by_status(Status::Done) {
        must_pass(
            &task,
            Status::CodeReview,
            Event::PassReview,
            &reviewer_of(&task),
            &done,
        );
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
    // holds the claim: the lease has an expiry, and the claim is still live
    // at the clock the fixtures are read with.
    const NOW: &str = "2026-11-19T08:00:00Z";
    let tasks = tasks();
    let held: Vec<&Task> = tasks
        .iter()
        .filter(|task| {
            task.claim.as_ref().is_some_and(|claim| {
                !claim.lease_expires_at.is_empty() && !claim.lease_expired(NOW)
            })
        })
        .collect();
    assert_eq!(held.len(), 1, "no fixture for a claim held elsewhere");
    assert_eq!(held[0].id, "TSK09");
    let claim = held[0].claim.as_ref().expect("the claim");
    assert!(
        parse_rfc3339(&claim.lease_expires_at).expect("an expiry")
            > parse_rfc3339(NOW).expect("now"),
        "the lease expires before the fixtures' clock"
    );
}

/// The shapes the screens are read against: zero, two and five review
/// rounds, open and resolved findings, open and resolved questions, children,
/// and done tasks that carry their history.
#[test]
fn the_fixtures_hold_every_shape_the_screens_read() {
    let tasks = tasks();

    // Review rounds: zero, two, and five. The budget counts external rounds,
    // so five external rounds are fifty per cent of the ceiling; five rounds
    // in all is a task that was reviewed, fixed and reviewed again. No fixture
    // has a self round alone: code-review fixtures carry their findings on an
    // external round, as ADR 0017 requires.
    assert!(
        tasks.iter().any(|task| task.review.rounds.is_empty()),
        "no fixture with zero review rounds"
    );
    assert!(
        tasks.iter().any(|task| task.review.rounds.len() == 2),
        "no fixture with two review rounds"
    );
    assert!(
        tasks.iter().any(|task| task.review.rounds.len() == 5),
        "no fixture with five review rounds"
    );
    assert!(
        tasks.iter().any(|task| task.external_rounds() == 5),
        "no fixture whose budget is spent"
    );

    // Findings: open, and resolved with a resolution and a comment.
    assert!(
        tasks.iter().any(|task| task.open_findings() > 0),
        "no fixture with an open finding"
    );
    let resolved: Vec<&coding_crew_core::Finding> = tasks
        .iter()
        .flat_map(|task| &task.review.findings)
        .filter(|finding| finding.status != coding_crew_core::FindingStatus::Open)
        .collect();
    assert!(
        resolved.iter().any(|finding| !finding.resolved.is_empty()
            && !finding.resolution.is_empty()
            && !finding.comments.is_empty()),
        "no resolved finding with a resolution and a comment"
    );

    // Questions: open and resolved.
    assert!(
        tasks
            .iter()
            .flat_map(|task| &task.questions)
            .any(|question| question.status == coding_crew_core::QuestionStatus::Open),
        "no open question"
    );
    assert!(
        tasks
            .iter()
            .flat_map(|task| &task.questions)
            .any(|question| question.status == coding_crew_core::QuestionStatus::Resolved),
        "no resolved question"
    );

    // Children: a parent and its subtasks, both directions of the link.
    let parent = tasks
        .iter()
        .find(|task| task.children.len() >= 2)
        .expect("a parent with subtasks");
    for child in &parent.children {
        let child = tasks
            .iter()
            .find(|task| &task.id == child)
            .unwrap_or_else(|| panic!("{child} names no task"));
        assert_eq!(child.parent.as_deref(), Some(parent.id.as_str()));
    }

    // Done tasks carry their history: rounds, findings, resolutions and
    // notes, so a finished task reads like a finished task.
    for task in tasks.iter().filter(|task| task.status == Status::Done) {
        assert!(
            !task.review.rounds.is_empty(),
            "{} is done with no round",
            task.id
        );
        assert!(!task.notes.is_empty(), "{} is done with no note", task.id);
        for finding in &task.review.findings {
            assert!(
                !finding.resolved.is_empty() && !finding.resolution.is_empty(),
                "{} carries an unresolved finding {}",
                task.id,
                finding.id
            );
        }
    }
    assert!(
        tasks
            .iter()
            .filter(|task| task.status == Status::Done)
            .any(|task| !task.review.findings.is_empty()),
        "no done task carries a finding and its resolution"
    );
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
        .find(|i| i.status == coding_crew_core::ItemStatus::Promoted);
    let promoted = promoted.expect("a promoted item");
    let task_id = promoted.promoted_to.clone().expect("the task it became");
    let task = store.load(&task_id).expect("the promoted task");
    assert_eq!(task.promoted_from.as_deref(), Some(promoted.id.as_str()));
    assert!(task.is_small_task());

    let dismissed = items
        .iter()
        .find(|i| i.status == coding_crew_core::ItemStatus::Dismissed);
    let dismissed = dismissed.expect("a dismissed item");
    assert!(!dismissed.dismissed_reason.is_empty());
}

/// Every field the model defines is populated by some fixture.
///
/// This is the pin the fixtures exist to keep: a field nobody fills is a
/// branch of the reader the screens can never show, and a new fixture that
/// leaves one behind fails here.
#[test]
fn every_field_the_model_defines_is_populated_by_a_fixture() {
    use coding_crew_core::task::{Effort, Priority};
    use coding_crew_core::{
        CriterionStatus, FindingStatus, Focus, ItemKind, ItemStatus, QuestionStatus, Severity,
        StepStatus, TaskKind, Verdict,
    };

    let tasks = tasks();
    let items = mock_store().list_items().expect("listing the mock items");

    macro_rules! populated {
        ($field:expr, $found:expr) => {
            assert!($found, "no fixture populates {}", $field);
        };
    }

    // Task: identity.
    populated!(
        "schema_version",
        tasks.iter().all(|t| t.schema_version == 1)
    );
    populated!("id", tasks.iter().all(|t| !t.id.is_empty()));
    populated!("title", tasks.iter().all(|t| !t.title.is_empty()));
    populated!("slug", tasks.iter().all(|t| !t.slug.is_empty()));
    populated!("parent", tasks.iter().any(|t| t.parent.is_some()));
    populated!("children", tasks.iter().any(|t| !t.children.is_empty()));
    populated!("kind task", tasks.iter().any(|t| t.kind == TaskKind::Task));
    populated!("kind bug", tasks.iter().any(|t| t.kind == TaskKind::Bug));
    populated!(
        "kind feature",
        tasks.iter().any(|t| t.kind == TaskKind::Feature)
    );
    populated!(
        "promoted_from",
        tasks.iter().any(|t| t.promoted_from.is_some())
    );
    populated!("created", tasks.iter().all(|t| !t.created.is_empty()));
    populated!("updated", tasks.iter().all(|t| !t.updated.is_empty()));

    // Task: status.
    populated!(
        "status_since",
        tasks.iter().all(|t| !t.status_since.is_empty())
    );
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
        populated!(
            format!("status {status:?}"),
            tasks.iter().any(|t| t.status == status)
        );
    }
    for overlay in ["blocked", "deferred", "cancelled"] {
        let found = tasks.iter().any(|task| {
            let overlay = match overlay {
                "blocked" => task.overlays.blocked.as_ref(),
                "deferred" => task.overlays.deferred.as_ref(),
                _ => task.overlays.cancelled.as_ref(),
            };
            overlay.is_some_and(|overlay| {
                !overlay.reason.is_empty() && !overlay.by.is_empty() && !overlay.at.is_empty()
            })
        });
        populated!(format!("overlay {overlay}"), found);
    }
    populated!(
        "claim",
        tasks.iter().any(|task| {
            task.claim.as_ref().is_some_and(|claim| {
                !claim.by.is_empty()
                    && claim.session.as_ref().is_some_and(|s| !s.is_empty())
                    && !claim.at.is_empty()
                    && !claim.lease_expires_at.is_empty()
            })
        })
    );
    populated!(
        "planned_by",
        tasks
            .iter()
            .any(|t| t.planned_by.as_ref().is_some_and(|s| !s.is_empty()))
    );

    // Task: meta, intent and scope.
    for priority in [
        Priority::Urgent,
        Priority::High,
        Priority::Normal,
        Priority::Low,
    ] {
        populated!(
            format!("priority {priority:?}"),
            tasks.iter().any(|t| t.meta.priority == priority)
        );
    }
    for effort in [Effort::Small, Effort::Medium, Effort::Large] {
        populated!(
            format!("effort {effort:?}"),
            tasks.iter().any(|t| t.meta.effort == effort)
        );
    }
    populated!("meta.tags", tasks.iter().any(|t| !t.meta.tags.is_empty()));
    populated!(
        "intent.outcome",
        tasks.iter().any(|t| !t.intent.outcome.is_empty())
    );
    populated!(
        "intent.problem",
        tasks.iter().any(|t| !t.intent.problem.is_empty())
    );
    populated!(
        "scope.in_scope",
        tasks.iter().any(|t| !t.scope.in_scope.is_empty())
    );
    populated!(
        "scope.out_of_scope",
        tasks.iter().any(|t| !t.scope.out_of_scope.is_empty())
    );

    // Acceptance criteria.
    populated!(
        "criterion",
        tasks.iter().any(|t| !t.acceptance_criteria.is_empty())
    );
    populated!(
        "criterion.id",
        tasks
            .iter()
            .flat_map(|t| &t.acceptance_criteria)
            .all(|c| !c.id.is_empty())
    );
    populated!(
        "criterion.description",
        tasks
            .iter()
            .flat_map(|t| &t.acceptance_criteria)
            .all(|c| !c.description.is_empty())
    );
    populated!(
        "criterion.verification",
        tasks
            .iter()
            .flat_map(|t| &t.acceptance_criteria)
            .any(|c| !c.verification.is_empty())
    );
    for status in [
        CriterionStatus::Pending,
        CriterionStatus::Passed,
        CriterionStatus::Failed,
        CriterionStatus::Unproven,
        CriterionStatus::Deferred,
    ] {
        populated!(
            format!("criterion status {status:?}"),
            tasks
                .iter()
                .flat_map(|t| &t.acceptance_criteria)
                .any(|c| c.status == status)
        );
    }
    populated!(
        "criterion.deferral",
        tasks
            .iter()
            .flat_map(|t| &t.acceptance_criteria)
            .filter_map(|c| c.deferral.as_ref())
            .any(|d| !d.reason.is_empty() && !d.follow_up.is_empty())
    );
    populated!(
        "criterion.failure_reason",
        tasks
            .iter()
            .flat_map(|t| &t.acceptance_criteria)
            .any(|c| !c.failure_reason.is_empty())
    );

    // Evidence.
    let evidence: Vec<&coding_crew_core::Evidence> = tasks
        .iter()
        .flat_map(|t| &t.acceptance_criteria)
        .flat_map(|c| &c.evidence)
        .collect();
    populated!("evidence", !evidence.is_empty());
    populated!(
        "evidence.command",
        evidence.iter().all(|e| e.manual || !e.command.is_empty())
    );
    populated!(
        "evidence.manual",
        evidence.iter().any(|e| e.manual && e.command.is_empty())
    );
    populated!(
        "evidence.exit_code 0",
        evidence.iter().any(|e| e.exit_code == Some(0))
    );
    populated!(
        "evidence.exit_code 1",
        evidence.iter().any(|e| e.exit_code == Some(1))
    );
    populated!(
        "evidence.summary",
        evidence.iter().all(|e| !e.summary.is_empty())
    );
    populated!("evidence.at", evidence.iter().all(|e| !e.at.is_empty()));
    populated!("evidence.by", evidence.iter().all(|e| !e.by.is_empty()));
    populated!(
        "evidence.session",
        evidence
            .iter()
            .all(|e| e.session.as_ref().is_some_and(|s| !s.is_empty()))
    );

    // Steps.
    let steps: Vec<&coding_crew_core::Step> = tasks.iter().flat_map(|t| &t.steps).collect();
    populated!("step", !steps.is_empty());
    populated!("step.id", steps.iter().all(|s| !s.id.is_empty()));
    populated!("step.order", steps.iter().all(|s| s.order > 0));
    populated!("step.action", steps.iter().all(|s| !s.action.is_empty()));
    populated!(
        "step.done_when",
        steps.iter().all(|s| !s.done_when.is_empty())
    );
    populated!("step.files", steps.iter().any(|s| !s.files.is_empty()));
    populated!("step.covers", steps.iter().any(|s| !s.covers.is_empty()));
    populated!(
        "step.depends_on",
        steps.iter().any(|s| !s.depends_on.is_empty())
    );
    populated!(
        "step.deviation",
        steps.iter().any(|s| !s.deviation.is_empty())
    );
    for status in [
        StepStatus::Pending,
        StepStatus::InProgress,
        StepStatus::Done,
        StepStatus::Skipped,
        StepStatus::Failed,
    ] {
        populated!(
            format!("step status {status:?}"),
            steps.iter().any(|s| s.status == status)
        );
    }

    // Review.
    populated!(
        "review.max_rounds",
        tasks.iter().all(|t| t.review.max_rounds > 0)
    );
    populated!(
        "review.rounds",
        tasks.iter().any(|t| !t.review.rounds.is_empty())
    );
    populated!(
        "review verdict ok",
        tasks.iter().any(|t| t.review.verdict == Some(Verdict::Ok))
    );
    populated!(
        "review verdict changes-needed",
        tasks
            .iter()
            .any(|t| t.review.verdict == Some(Verdict::ChangesNeeded))
    );
    populated!(
        "review.findings",
        tasks.iter().any(|t| !t.review.findings.is_empty())
    );
    let rounds: Vec<&coding_crew_core::Round> =
        tasks.iter().flat_map(|t| &t.review.rounds).collect();
    populated!("round.n", rounds.iter().any(|r| r.n == 0));
    populated!("round.n external", rounds.iter().any(|r| r.n > 0));
    populated!(
        "round.kind self",
        rounds.iter().any(|r| r.kind == RoundKind::SelfReview)
    );
    populated!(
        "round.kind external",
        rounds.iter().any(|r| r.kind == RoundKind::External)
    );
    for focus in [
        Focus::SpecCompliance,
        Focus::EdgeCases,
        Focus::Security,
        Focus::Empirical,
        Focus::Design,
    ] {
        populated!(
            format!("round focus {focus:?}"),
            rounds.iter().any(|r| r.focus == focus)
        );
    }
    populated!(
        "round.reviewer",
        rounds.iter().all(|r| !r.reviewer.is_empty())
    );
    populated!(
        "round.verdict ok",
        rounds.iter().any(|r| r.verdict == Verdict::Ok)
    );
    populated!(
        "round.verdict changes-needed",
        rounds.iter().any(|r| r.verdict == Verdict::ChangesNeeded)
    );
    populated!("round.at", rounds.iter().all(|r| !r.at.is_empty()));
    populated!("round.notes", rounds.iter().all(|r| !r.notes.is_empty()));
    populated!(
        "round.findings",
        rounds.iter().any(|r| !r.findings.is_empty())
    );
    populated!(
        "round with no findings",
        rounds.iter().any(|r| r.findings.is_empty())
    );

    // Findings and their comments.
    let findings: Vec<&coding_crew_core::Finding> =
        tasks.iter().flat_map(|t| &t.review.findings).collect();
    populated!("finding", !findings.is_empty());
    populated!("finding.id", findings.iter().all(|f| !f.id.is_empty()));
    populated!(
        "finding.severity",
        findings.iter().all(|f| {
            matches!(
                f.severity,
                Severity::Low | Severity::Medium | Severity::High
            )
        })
    );
    for severity in [Severity::Low, Severity::Medium, Severity::High] {
        populated!(
            format!("finding severity {severity:?}"),
            findings.iter().any(|f| f.severity == severity)
        );
    }
    for status in [
        FindingStatus::Open,
        FindingStatus::Fixed,
        FindingStatus::Dismissed,
    ] {
        populated!(
            format!("finding status {status:?}"),
            findings.iter().any(|f| f.status == status)
        );
    }
    populated!(
        "finding.description",
        findings.iter().all(|f| !f.description.is_empty())
    );
    populated!(
        "finding.created",
        findings.iter().all(|f| !f.created.is_empty())
    );
    populated!(
        "finding.resolved",
        findings
            .iter()
            .filter(|f| f.status != FindingStatus::Open)
            .all(|f| !f.resolved.is_empty())
    );
    populated!(
        "finding.resolution",
        findings
            .iter()
            .filter(|f| f.status != FindingStatus::Open)
            .all(|f| !f.resolution.is_empty())
    );
    populated!(
        "finding.comments",
        findings.iter().any(|f| !f.comments.is_empty())
    );
    let comments: Vec<&coding_crew_core::Comment> =
        findings.iter().flat_map(|f| &f.comments).collect();
    populated!("comment.id", comments.iter().all(|c| !c.id.is_empty()));
    populated!(
        "comment.author",
        comments.iter().all(|c| !c.author.is_empty())
    );
    populated!("comment.body", comments.iter().all(|c| !c.body.is_empty()));
    populated!("comment.at", comments.iter().all(|c| !c.at.is_empty()));

    // Questions.
    let questions: Vec<&coding_crew_core::Question> =
        tasks.iter().flat_map(|t| &t.questions).collect();
    populated!("question", !questions.is_empty());
    populated!("question.id", questions.iter().all(|q| !q.id.is_empty()));
    populated!(
        "question.question",
        questions.iter().all(|q| !q.question.is_empty())
    );
    populated!(
        "question.raised_by",
        questions.iter().all(|q| !q.raised_by.is_empty())
    );
    populated!(
        "question.raised_at",
        questions.iter().all(|q| !q.raised_at.is_empty())
    );
    populated!(
        "question status open",
        questions.iter().any(|q| q.status == QuestionStatus::Open)
    );
    populated!(
        "question status resolved",
        questions
            .iter()
            .any(|q| q.status == QuestionStatus::Resolved)
    );
    populated!(
        "question.answer",
        questions
            .iter()
            .filter(|q| q.status == QuestionStatus::Resolved)
            .all(|q| !q.answer.is_empty())
    );
    populated!(
        "question.resolved_by",
        questions
            .iter()
            .filter(|q| q.status == QuestionStatus::Resolved)
            .all(|q| !q.resolved_by.is_empty())
    );
    populated!(
        "question.resolved_at",
        questions
            .iter()
            .filter(|q| q.status == QuestionStatus::Resolved)
            .all(|q| !q.resolved_at.is_empty())
    );

    // Notes.
    let notes: Vec<&coding_crew_core::Note> = tasks.iter().flat_map(|t| &t.notes).collect();
    populated!("note", !notes.is_empty());
    populated!("note.id", notes.iter().all(|n| !n.id.is_empty()));
    populated!("note.author", notes.iter().all(|n| !n.author.is_empty()));
    populated!("note.body", notes.iter().all(|n| !n.body.is_empty()));
    populated!("note.at", notes.iter().all(|n| !n.at.is_empty()));
    populated!("depends_on", tasks.iter().any(|t| !t.depends_on.is_empty()));

    // Smaller tasks.
    populated!("item", !items.is_empty());
    populated!(
        "item.schema_version",
        items.iter().all(|i| i.schema_version == 1)
    );
    populated!("item.id", items.iter().all(|i| !i.id.is_empty()));
    populated!("item.title", items.iter().all(|i| !i.title.is_empty()));
    populated!("item.slug", items.iter().all(|i| !i.slug.is_empty()));
    for kind in [ItemKind::Idea, ItemKind::Feature, ItemKind::Bug] {
        populated!(
            format!("item kind {kind:?}"),
            items.iter().any(|i| i.kind == kind)
        );
    }
    for status in [
        ItemStatus::Open,
        ItemStatus::Dismissed,
        ItemStatus::Promoted,
    ] {
        populated!(
            format!("item status {status:?}"),
            items.iter().any(|i| i.status == status)
        );
    }
    populated!("item.body", items.iter().all(|i| !i.body.is_empty()));
    populated!("item.scope", items.iter().all(|i| !i.scope.is_empty()));
    populated!("item.tags", items.iter().all(|i| !i.tags.is_empty()));
    populated!("item.created", items.iter().all(|i| !i.created.is_empty()));
    populated!("item.updated", items.iter().all(|i| !i.updated.is_empty()));
    populated!(
        "item.promoted_to",
        items.iter().any(|i| i.promoted_to.is_some())
    );
    populated!(
        "item.dismissed_reason",
        items.iter().any(|i| !i.dismissed_reason.is_empty())
    );
}

/// A line longer than an 80-column pane wraps in it, so a link that long
/// exercises the wrap.
const WRAPPING_LINK: usize = 80;

/// The document constructs a note body carries, by the names the test pins.
fn markdown_constructs(body: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    if body.contains("> [!NOTE]") {
        found.insert("> [!NOTE]".to_string());
    }
    if body.contains("> [!WARNING]") {
        found.insert("> [!WARNING]".to_string());
    }
    let lines: Vec<&str> = body.lines().collect();
    let mut in_fence = false;
    // The indent of each open list item, outermost first.
    let mut depths: Vec<usize> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if in_fence {
            in_fence = !trimmed.starts_with("```");
            continue;
        }
        if let Some(language) = trimmed.strip_prefix("```") {
            in_fence = true;
            if !language.trim().is_empty() {
                found.insert(format!("```{}", language.trim()));
            }
            continue;
        }
        if trimmed.starts_with("- ") {
            while depths.last().is_some_and(|&depth| depth >= indent) {
                depths.pop();
            }
            depths.push(indent);
            if depths.len() >= 3 {
                found.insert("list nested three deep".to_string());
            }
        } else if indent == 0 && !trimmed.is_empty() {
            depths.clear();
        }
        if trimmed.starts_with('|') {
            let next = lines.get(index + 1).map_or("", |next| next.trim_start());
            if next.starts_with('|') && next.contains("---") {
                found.insert("table".to_string());
            }
        } else if trimmed.matches('`').count() >= 2 {
            found.insert("inline code".to_string());
        }
        if let Some(start) = line.find("](http") {
            let rest = &line[start + 2..];
            if rest.find(')').is_some_and(|end| end > WRAPPING_LINK) {
                found.insert("long link".to_string());
            }
        }
    }
    found
}

#[test]
fn the_fixtures_keep_the_constructs_the_document_is_pinned_to_show() {
    let mut found = HashSet::new();
    for task in tasks() {
        for note in &task.notes {
            found.extend(markdown_constructs(&note.body));
        }
    }
    let required = [
        "> [!NOTE]",
        "> [!WARNING]",
        "list nested three deep",
        "table",
        "inline code",
        "long link",
        "```rust",
        "```python",
        "```toml",
        "```bash",
        "```mermaid",
    ];
    let missing: Vec<&str> = required
        .into_iter()
        .filter(|construct| !found.contains(*construct))
        .collect();
    assert!(missing.is_empty(), "no note carries {missing:?}");
}

/// The fixtures' clock, as the generator states it (`FIXTURE_NOW`).
const FIXTURE_CLOCK: &str = "2026-11-19T08:00Z";

#[test]
fn the_fixtures_hold_one_manual_override() {
    let manual = tasks()
        .iter()
        .flat_map(|t| &t.acceptance_criteria)
        .flat_map(|c| &c.evidence)
        .filter(|e| e.manual)
        .count();
    assert_eq!(manual, 1, "the fixtures hold {manual} manual overrides");
}

#[test]
fn the_fixtures_hold_an_approved_task_that_has_waited_long() {
    // Approved at least a fortnight before the fixtures' clock, and still
    // waiting: its status is approved, so the wait runs from its approval.
    let now = parse_rfc3339(FIXTURE_CLOCK).expect("the fixtures' clock");
    let task = tasks()
        .into_iter()
        .find(|t| t.id == "TSK29")
        .expect("the claimable fixture");
    assert_eq!(task.status, Status::Approved);
    let waited = now - parse_rfc3339(&task.status_since).expect("its approval");
    assert!(
        waited >= 14 * 24 * 60 * 60,
        "{} has waited {waited} seconds",
        task.id
    );
}

#[test]
fn the_reopened_fixture_closes_twice_on_two_clean_rounds() {
    // TSK25 closed at its first external round, went back to work, and closed
    // again at its second. The journal tells the reopen between them.
    let task = tasks()
        .into_iter()
        .find(|t| t.id == "TSK25")
        .expect("the reopened fixture");
    assert_eq!(task.status, Status::Done);
    let external: Vec<Verdict> = task
        .review
        .rounds
        .iter()
        .filter(|r| r.kind == RoundKind::External)
        .map(|r| r.verdict)
        .collect();
    assert_eq!(
        external,
        [Verdict::Ok, Verdict::Ok],
        "TSK25's external rounds"
    );
}
