use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use coding_crew_core::store::{DATA_DIR, WORK_DIR};
use coding_crew_core::{
    Activity, ActivityEvent, EventKind, Item, Runtime, Store, StoreError, Task,
};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("coding-crew-store-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch folder");
    // macOS's temp dir is behind a symlink (/var -> /private/var); discovery
    // returns canonical paths.
    dir.canonicalize().expect("a canonical scratch folder")
}

/// A work folder inside a scratch project. The store's lock sits beside
/// `work/`, so a store rooted at the bare scratch folder would lock a file in
/// the shared temp dir.
fn work_scratch(name: &str) -> PathBuf {
    let work = scratch(name).join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&work).expect("a work folder");
    work
}

/// A task write under the project lock, as every write is made (ADR 0016).
fn save(store: &Store, task: &mut Task) {
    store
        .lock()
        .expect("the project lock")
        .save(task)
        .expect("the task");
}

/// Changing the process cwd is global, so cwd-owning tests take this lock and
/// restore the cwd when the guard drops. No other test in this file reads the
/// cwd; every fixture here is built from an absolute path.
static CWD_LOCK: Mutex<()> = Mutex::new(());

struct Cwd {
    original: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl Cwd {
    fn enter(dir: &Path) -> Self {
        let lock = CWD_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let original = std::env::current_dir().expect("the current directory");
        std::env::set_current_dir(dir).expect("changing the current directory");
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for Cwd {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).expect("restoring the current directory");
    }
}

#[test]
fn a_nested_start_finds_the_nearest_project() {
    let project = scratch("nested");
    let work = project.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&work).expect("a work folder");
    let start = project.join("crates").join("raul");
    fs::create_dir_all(&start).expect("a nested folder");

    let store = Store::discover(&start).expect("the project's store");
    assert_eq!(store.root(), work);
    assert_eq!(
        Store::discover_work(&start)
            .expect("the work folder")
            .root(),
        work
    );
}

#[test]
fn the_nearest_project_wins_over_an_outer_one() {
    // Both projects hold work: the walk must stop at the inner one instead
    // of reaching the outer one, which also has a store.
    let outer = scratch("inner-outer");
    let outer_work = outer.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&outer_work).expect("the outer work folder");

    let inner = outer.join("inner");
    let inner_work = inner.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&inner_work).expect("the inner work folder");
    let start = inner.join("src");
    fs::create_dir_all(&start).expect("a nested folder");

    assert_eq!(
        Store::discover(&start).expect("a store").root(),
        inner_work,
        "the inner project's store is the one found"
    );
    assert_eq!(
        Store::discover_work(&start).expect("a store").root(),
        inner_work,
        "the inner project's work is the one found"
    );
}

#[test]
fn a_project_without_work_does_not_borrow_an_ancestor() {
    let ancestor = scratch("ancestor");
    let ancestor_work = ancestor.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&ancestor_work).expect("the ancestor's work folder");

    let project = ancestor.join("project");
    let data = project.join(DATA_DIR);
    fs::create_dir_all(&data).expect("a coding-crew folder without work");
    let start = project.join("src");
    fs::create_dir_all(&start).expect("a nested folder");

    // The nearest project owns the store even when work is not there yet.
    assert_eq!(
        Store::discover(&start).expect("a store").root(),
        data.join(WORK_DIR),
        "the nearest project is the one found, not the ancestor"
    );
    // The strict rule names the nearest project's folder and stops there.
    let error = Store::discover_work(&start).expect_err("the nearest project has no work folder");
    match error {
        StoreError::NoWorkFolder(path) => assert_eq!(
            path, data,
            "the error names the nearest project, not the ancestor"
        ),
        other => panic!("expected the no-work error, got {other:?}"),
    }
}

#[test]
fn a_relative_start_with_a_parent_dir_selects_the_outer_project() {
    // base holds the outer project. base/work is the cwd and holds a sibling
    // project. base/project/src is the resolved start, so its ancestor chain
    // meets base and never meets base/work.
    let base = scratch("cwd-relative");
    let outer_work = base.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&outer_work).expect("the outer project");
    let cwd = base.join("work");
    let sibling = cwd.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&sibling).expect("a sibling project in the cwd");
    let start = base.join("project").join("src");
    fs::create_dir_all(&start).expect("a start below the outer project");

    let guard = Cwd::enter(&cwd);
    // Resolves to base/project/src. Its ancestor chain never meets base/work,
    // so the cwd sibling is not an ancestor of the start. An unnormalized
    // walk pops into base/work and returns the sibling.
    let relative = Path::new("../project/src");

    assert_eq!(
        Store::discover(relative)
            .expect("the outer project's store")
            .root(),
        outer_work,
        "the resolved start selects the outer project, not the cwd sibling"
    );
    assert_eq!(
        Store::discover_work(relative)
            .expect("the outer project's work")
            .root(),
        outer_work,
        "the resolved start selects the outer project, not the cwd sibling"
    );

    drop(guard);
    let _ = fs::remove_dir_all(&base);
}

#[cfg(unix)]
#[test]
fn an_absolute_start_with_a_parent_dir_selects_the_outer_project() {
    let base = scratch("cwd-absolute");
    let outer_work = base.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&outer_work).expect("the outer project");
    let sibling = base.join("work").join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&sibling).expect("a sibling project");
    let start = base.join("project").join("src");
    fs::create_dir_all(&start).expect("a start below the outer project");

    // Keeping `..` used to let the lexical walk pop into base/work, which is
    // not an ancestor of the resolved base/project/src.
    let dotted = base.join("work").join("..").join("project").join("src");
    assert!(dotted.is_absolute(), "{}", dotted.display());

    assert_eq!(
        Store::discover(&dotted)
            .expect("the outer project's store")
            .root(),
        outer_work,
        "the resolved start selects the outer project, not the sibling"
    );
    assert_eq!(
        Store::discover_work(&dotted)
            .expect("the outer project's work")
            .root(),
        outer_work,
        "the resolved start selects the outer project, not the sibling"
    );

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn dots_and_a_missing_tail_still_find_the_project() {
    let project = scratch("dots");
    let work = project.join(DATA_DIR).join(WORK_DIR);
    fs::create_dir_all(&work).expect("a work folder");

    // A start that does not exist, named through `.` and `..`.
    let start = project.join("crates").join("..").join("missing").join(".");
    assert_eq!(
        Store::discover(&start).expect("the project's store").root(),
        work
    );
    assert_eq!(
        Store::discover_work(&start)
            .expect("the project's work folder")
            .root(),
        work
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn saving_a_task_keeps_tasks_whose_ids_only_start_with_its_id() {
    // TSK03.1 and TSK030 start with TSK03 but are other tasks.
    let store = Store::new(work_scratch("prefix-save"));
    save(&store, &mut Task::new("TSK03.1", "child"));
    save(&store, &mut Task::new("TSK030", "other"));
    save(&store, &mut Task::new("TSK03", "parent"));

    let ids: Vec<String> = store
        .list()
        .expect("the tasks")
        .into_iter()
        .map(|task| task.id)
        .collect();
    assert_eq!(ids, ["TSK03", "TSK03.1", "TSK030"]);
}

#[test]
fn load_returns_only_the_task_it_names() {
    let store = Store::new(work_scratch("prefix-load"));
    save(&store, &mut Task::new("TSK03.1", "child"));

    assert!(matches!(store.load("TSK03"), Err(StoreError::NotFound(_))));
    assert_eq!(store.load("TSK03.1").expect("the child").id, "TSK03.1");
}

#[test]
fn load_checks_the_id_inside_the_file() {
    // A file named for TSK03 whose record says TSK03.1 is not TSK03.
    let store = Store::new(work_scratch("load-content"));
    let mut child = Task::new("TSK03.1", "child");
    save(&store, &mut child);
    fs::rename(
        store.task_path(&child),
        store.root().join("TSK03-child.json"),
    )
    .expect("a renamed file");

    assert!(matches!(store.load("TSK03"), Err(StoreError::NotFound(_))));
}

#[test]
fn load_item_returns_only_the_item_it_names() {
    let store = Store::new(work_scratch("item-prefix"));
    let item = store.create_item("first").expect("an item");

    assert!(matches!(
        store.load_item("ITM0"),
        Err(StoreError::NotFound(_))
    ));
    assert_eq!(store.load_item(&item.id).expect("the item").id, item.id);
}

#[test]
fn a_file_that_does_not_parse_still_holds_its_id() {
    // A corrupt record keeps its id: the next id must not reuse it, and the
    // save must not delete it.
    let root = work_scratch("corrupt-id");
    fs::write(root.join("TSK05-broken.json"), "{ not json").expect("a corrupt task");
    fs::write(root.join("ITM05-broken.json"), "{ not json").expect("a corrupt item");
    let store = Store::new(&root);

    let task = store.create("next").expect("a task");
    assert_eq!(task.id, "TSK06");
    assert!(
        root.join("TSK05-broken.json").is_file(),
        "the corrupt task survives"
    );

    let item = store.create_item("next").expect("an item");
    assert_eq!(item.id, "ITM06");
    assert!(
        root.join("ITM05-broken.json").is_file(),
        "the corrupt item survives"
    );
}

#[test]
fn two_writers_creating_at_once_take_no_duplicate_id() {
    // Each create scans the folder for the next id, then writes. The scan and
    // the write are one locked step, so a second creator waits for the first
    // to finish. The first writer holds the lock after its scan. A create that
    // did not wait would scan the same folder, take the same id and write it
    // while the first writer still holds it. No timing is involved: the
    // second create is either blocked by the lock or it finishes at once.
    let project = scratch("two-creators");
    let store = Store::new(project.join(DATA_DIR).join(WORK_DIR));

    let first = store.lock().expect("the project lock");
    let scanned = first.next_id().expect("the next id");
    assert_eq!(scanned, "TSK01");

    let (finished, racing) = mpsc::channel();
    let racer = {
        let store = store.clone();
        std::thread::spawn(move || {
            let task = store.create("racer").expect("a task");
            let _ = finished.send(());
            task.id
        })
    };
    // The racer may not finish while the first writer holds the lock. A
    // racer that does finish here took its id from the scan above.
    assert!(
        racing.recv_timeout(Duration::from_secs(5)).is_err(),
        "a create finished while another writer held the project lock"
    );

    let mut claimed = Task::new(scanned, "first");
    first.save(&mut claimed).expect("the first task");
    drop(first);

    let racer_id = racer.join().expect("the racer");
    assert_eq!(racer_id, "TSK02", "the racer took the first writer's id");

    let ids: Vec<String> = store
        .list()
        .expect("the tasks")
        .into_iter()
        .map(|task| task.id)
        .collect();
    assert_eq!(ids, ["TSK01", "TSK02"], "two tasks share an id");

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn two_writers_appending_to_the_journal_lose_no_event() {
    // Each append reads the journal, adds an event and rewrites it. Without
    // the lock held across the read, two appends read the same journal and
    // one event is lost.
    let project = scratch("two-journal");
    let store = Store::new(project.join(DATA_DIR).join(WORK_DIR));
    let path = store.activity_path();

    std::thread::scope(|scope| {
        for writer in 0..2 {
            let store = store.clone();
            let path = path.clone();
            scope.spawn(move || {
                for n in 0..25 {
                    let lock = store.lock().expect("the project lock");
                    let mut journal = if path.is_file() {
                        Activity::read(&path).expect("the journal")
                    } else {
                        Activity::default()
                    };
                    journal.events.push(ActivityEvent {
                        at: "2026-11-23T10:00:00Z".to_string(),
                        kind: EventKind::Commented,
                        task: "TSK01".to_string(),
                        by: format!("writer-{writer}"),
                        note: Some(n.to_string()),
                    });
                    lock.write_activity(&journal).expect("the journal");
                }
            });
        }
    });

    let journal = Activity::read(&path).expect("the journal");
    assert_eq!(journal.events.len(), 50, "an event was lost");
    let _ = fs::remove_dir_all(&project);
}

#[test]
fn ids_sort_by_number_not_by_text() {
    // TSK100 is after TSK99, and TSK03.1 after TSK03; text order says otherwise.
    let store = Store::new(work_scratch("numeric-order"));
    for id in ["TSK100", "TSK99", "TSK03.1", "TSK03"] {
        save(&store, &mut Task::new(id, id));
    }
    for id in ["ITM100", "ITM99"] {
        store
            .lock()
            .expect("the project lock")
            .save_item(&item(id))
            .expect("an item");
    }

    let tasks: Vec<String> = store
        .list()
        .expect("the tasks")
        .into_iter()
        .map(|task| task.id)
        .collect();
    assert_eq!(tasks, ["TSK03", "TSK03.1", "TSK99", "TSK100"]);

    let items: Vec<String> = store
        .list_items()
        .expect("the items")
        .into_iter()
        .map(|item| item.id)
        .collect();
    assert_eq!(items, ["ITM99", "ITM100"]);
}

#[test]
fn saving_a_renamed_item_removes_its_old_file() {
    // An item renamed by its slug leaves no second file for the same id.
    let store = Store::new(work_scratch("item-rename"));
    let mut item = store.create_item("first").expect("an item");
    item.slug = "second".to_string();
    store
        .lock()
        .expect("the project lock")
        .save_item(&item)
        .expect("the renamed item");

    let files: Vec<String> = fs::read_dir(store.root())
        .expect("the work folder")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("a name")
        })
        .collect();
    assert_eq!(files, [format!("{}-second.json", item.id)]);
}

#[test]
fn the_journal_and_the_runtime_record_round_trip_through_their_writers() {
    // The committed fixtures, written through the writers, read back equal.
    let mock = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew");
    let journal = Activity::read(&mock.join("activity.json")).expect("the journal");
    let runtime = Runtime::read(&mock.join("runtime.json")).expect("the record");

    let project = scratch("round-trip");
    let store = Store::new(project.join(DATA_DIR).join(WORK_DIR));
    let lock = store.lock().expect("the project lock");
    lock.write_activity(&journal).expect("the journal");
    lock.write_runtime(&runtime).expect("the record");
    drop(lock);

    assert_eq!(
        Activity::read(&store.activity_path()).expect("the journal"),
        journal
    );
    assert_eq!(
        Runtime::read(&store.runtime_path()).expect("the record"),
        runtime
    );
    assert!(!store.activity_path().with_extension("json.tmp").exists());
    let _ = fs::remove_dir_all(&project);
}

#[test]
fn a_journal_with_events_it_cannot_read_is_not_rewritten() {
    // Rewriting would drop the unreadable events, so the write is refused and
    // the file stays as it was.
    let project = scratch("unreadable-journal");
    let store = Store::new(project.join(DATA_DIR).join(WORK_DIR));
    let path = store.activity_path();
    let text = r#"{"events": [{"at": "2026-11-23T10:00:00Z", "kind": "from_the_future", "task": "TSK01", "by": "runner"}]}"#;
    fs::create_dir_all(store.root()).expect("a work folder");
    fs::write(&path, text).expect("a journal");

    let journal = Activity::read(&path).expect("the journal");
    assert_eq!(journal.unreadable, 1);
    assert!(matches!(
        store
            .lock()
            .expect("the project lock")
            .write_activity(&journal),
        Err(StoreError::JournalUnreadable(1))
    ));
    assert_eq!(fs::read_to_string(&path).expect("the journal"), text);
    let _ = fs::remove_dir_all(&project);
}

#[test]
fn a_writer_holding_a_stale_revision_is_refused() {
    // Two writers read revision 1. The first write moves the file to 2; the
    // second, still holding 1, is refused and the file keeps the first's text.
    let store = Store::new(work_scratch("stale-revision"));
    let created = store.create("first").expect("a task");
    assert_eq!(created.revision, 1);

    let mut first = store.load(&created.id).expect("the task");
    let mut second = first.clone();
    first.title = "first, revised".to_string();
    second.title = "second, revised".to_string();

    save(&store, &mut first);
    assert_eq!(first.revision, 2, "the write moves the revision on");

    let error = store
        .lock()
        .expect("the project lock")
        .save(&mut second)
        .expect_err("the stale write");
    assert!(matches!(
        error,
        StoreError::Revision {
            expected: 1,
            found: 2
        }
    ));
    assert_eq!(second.revision, 1, "a refused write changes nothing");
    let stored = store.load(&created.id).expect("the task");
    assert_eq!(stored.title, "first, revised");
    assert_eq!(stored.revision, 2);
}

#[test]
fn two_writers_editing_one_task_lose_no_change() {
    // Each edit loads the task under the lock, adds a tag and saves. Loading
    // before the lock would let two edits read the same task, and one tag
    // would be lost.
    let project = scratch("two-editors");
    let store = Store::new(project.join(DATA_DIR).join(WORK_DIR));
    store.create("shared").expect("a task");

    std::thread::scope(|scope| {
        for writer in 0..2 {
            let store = store.clone();
            scope.spawn(move || {
                for n in 0..25 {
                    let lock = store.lock().expect("the project lock");
                    let mut task = lock.load("TSK01").expect("the task");
                    task.meta.tags.push(format!("writer-{writer}-{n}"));
                    lock.save(&mut task).expect("the task");
                }
            });
        }
    });

    let task = store.load("TSK01").expect("the task");
    assert_eq!(task.meta.tags.len(), 50, "an edit was lost");
    assert_eq!(task.revision, 51, "every write moved the revision on");
    let _ = fs::remove_dir_all(&project);
}

fn item(id: &str) -> Item {
    Item::new(id, id)
}
