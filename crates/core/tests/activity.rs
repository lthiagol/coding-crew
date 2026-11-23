//! The committed journal tells the chronology the fixtures already tell.

use std::collections::HashMap;
use std::path::PathBuf;

use coding_crew_core::{Activity, EventKind, Store, parse_rfc3339};

fn project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("mock-data/coding-crew")
}

fn journal() -> Activity {
    let text = std::fs::read_to_string(project().join("activity.json")).expect("the journal");
    serde_json::from_str(&text).expect("a journal")
}

fn store() -> Store {
    Store::new(project().join("work"))
}

#[test]
fn the_journal_is_beside_the_work_folder_and_is_not_a_task() {
    let path = project().join("activity.json");
    assert!(path.is_file(), "{} is not a file", path.display());
    assert!(
        !project().join("work/activity.json").exists(),
        "the journal is inside the work folder"
    );
    let work = store().list().expect("the tasks");
    assert!(
        work.iter().all(|task| task.id != "activity"),
        "the journal was read as a task"
    );
}

#[test]
fn the_journal_holds_the_last_few_events_and_more() {
    // The Overview shows the last few: the source has more than it shows,
    // so the left-out count is real.
    let events = journal().events;
    assert!(events.len() > 5, "only {} events", events.len());
}

#[test]
fn every_event_names_a_task_that_existed_before_it() {
    let tasks: HashMap<String, coding_crew_core::Task> = store()
        .list()
        .expect("the tasks")
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect();
    for event in journal().events {
        let task = tasks
            .get(&event.task)
            .unwrap_or_else(|| panic!("{} names no task", event.task));
        let at = parse_rfc3339(&event.at)
            .unwrap_or_else(|| panic!("{} has an unreadable at", event.task));
        let created = parse_rfc3339(&task.created)
            .unwrap_or_else(|| panic!("{} has an unreadable created", event.task));
        assert!(
            created <= at,
            "{} happens before {} was created",
            event.at,
            event.task
        );
    }
}

#[test]
fn every_task_has_a_created_event_and_its_events_keep_the_history_order() {
    let tasks = store().list().expect("the tasks");
    let events = journal().events;
    for task in tasks {
        let history: Vec<&coding_crew_core::ActivityEvent> = events
            .iter()
            .filter(|event| event.task == task.id)
            .collect();
        assert!(!history.is_empty(), "{} has no activity", task.id);
        assert_eq!(
            history[0].kind,
            EventKind::Created,
            "{} does not start with its creation",
            task.id
        );
        assert_eq!(history[0].at, task.created, "{} created at", task.id);
        let mut previous = None;
        for event in history {
            let at = parse_rfc3339(&event.at)
                .unwrap_or_else(|| panic!("{} has an unreadable at", task.id));
            if let Some(previous) = previous {
                assert!(
                    previous <= at,
                    "{}: {} happens before the event before it",
                    task.id,
                    event.at
                );
            }
            previous = Some(at);
        }
    }
}

#[test]
fn the_journal_is_in_the_order_the_file_holds() {
    let events = journal().events;
    let mut previous = None;
    for event in &events {
        let at = parse_rfc3339(&event.at)
            .unwrap_or_else(|| panic!("{} has an unreadable at", event.task));
        if let Some(previous) = previous {
            assert!(
                previous <= at,
                "the journal is out of order at {}",
                event.at
            );
        }
        previous = Some(at);
    }
}

#[test]
fn a_claim_comes_after_the_completion_of_what_the_task_waits_on() {
    let tasks: HashMap<String, coding_crew_core::Task> = store()
        .list()
        .expect("the tasks")
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect();
    let events = journal().events;
    for event in events
        .iter()
        .filter(|event| event.kind == EventKind::Claimed)
    {
        let task = &tasks[&event.task];
        for dependency in &task.depends_on {
            let completed = events
                .iter()
                .find(|done| done.task == *dependency && done.kind == EventKind::Completed)
                .unwrap_or_else(|| panic!("{dependency} has no completion"));
            assert!(
                completed.at < event.at,
                "{} was claimed before {dependency} completed",
                task.id
            );
        }
    }
}

#[test]
fn the_journal_holds_the_moves_a_history_can_tell() {
    // Every move the history can hold is in the committed journal.
    let kinds: Vec<EventKind> = journal().events.iter().map(|event| event.kind).collect();
    for kind in [
        EventKind::Started,
        EventKind::Released,
        EventKind::Blocked,
        EventKind::Unblocked,
        EventKind::Deferred,
        EventKind::Undeferred,
        EventKind::Reopened,
        EventKind::Resumed,
    ] {
        assert!(kinds.contains(&kind), "the journal holds no {kind:?}");
    }
}
