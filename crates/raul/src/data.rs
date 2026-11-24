//! Reading the work folder. raul reads; it writes nothing.
//!
//! Every view is derived from the files on disk, so the folder is read again
//! on every redraw.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use coding_crew_core::lifecycle;
use coding_crew_core::store::{ACTIVITY_FILE, activity_path, by_number};
use coding_crew_core::{Activity, ActivityError, Item, ItemKind, Status, Task};

// The human lane's vocabulary is the model's, beside the statuses it names.
pub use coding_crew_core::Reason;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod support;

/// A file that could not be read. The rest of the folder still renders.
#[derive(Debug, Clone)]
pub struct ReadError {
    pub file: String,
    pub error: String,
}

/// Why there is no work folder to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// No `coding-crew` folder was found at or above the start.
    NoFolder,
    /// The nearest project has a `coding-crew` folder, but no work folder.
    NoWork,
}

impl Missing {
    /// The sentence the screens print.
    pub fn headline(self) -> &'static str {
        match self {
            Missing::NoFolder => "no coding-crew folder found",
            Missing::NoWork => "this project has no work folder",
        }
    }
}

/// The work folder, as it is on disk.
#[derive(Debug, Default)]
pub struct Work {
    pub root: PathBuf,
    pub tasks: Vec<Task>,
    pub items: Vec<Item>,
    pub errors: Vec<ReadError>,
    /// The project's journal, when it has one.
    pub activity: Option<Activity>,
    /// Why the folder is not there, when it is not.
    pub missing: Option<Missing>,
}

impl Work {
    /// Read the folder. A file that does not parse is reported, not fatal.
    #[cfg(test)]
    pub fn load(root: &Path) -> Self {
        Self::load_missing(root, Missing::NoFolder)
    }

    /// Read the folder, saying why it is absent when it is.
    pub fn load_missing(root: &Path, reason: Missing) -> Self {
        let mut work = Work {
            root: root.to_path_buf(),
            ..Work::default()
        };
        if !root.is_dir() {
            work.missing = Some(reason);
            return work;
        }
        let Ok(entries) = fs::read_dir(root) else {
            work.missing = Some(reason);
            return work;
        };
        // read_dir order is the filesystem's, not the folder's: the same work
        // folder must render the same bytes wherever it is copied. The file
        // name is unique per record, so ordering the entries by it first lets
        // the stable id sort below keep that order inside a duplicate id.
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        for path in paths {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    work.errors.push(ReadError {
                        file: name.to_string(),
                        error: error.to_string(),
                    });
                    continue;
                }
            };
            if name.starts_with("TSK") {
                match serde_json::from_str::<Task>(&contents) {
                    Ok(task) => work.tasks.push(task),
                    Err(error) => work.errors.push(ReadError {
                        file: name.to_string(),
                        error: error.to_string(),
                    }),
                }
            } else if name.starts_with("ITM") {
                match serde_json::from_str::<Item>(&contents) {
                    Ok(item) => work.items.push(item),
                    Err(error) => work.errors.push(ReadError {
                        file: name.to_string(),
                        error: error.to_string(),
                    }),
                }
            }
        }
        work.tasks.sort_by(|a, b| by_number(&a.id, &b.id));
        work.items.sort_by(|a, b| by_number(&a.id, &b.id));
        // The journal is optional: a project without one has no activity,
        // not a broken one. A journal that is there but does not parse is a
        // file that could not be read, like a task that does not parse.
        let path = activity_path(root);
        match Activity::read(&path) {
            Ok(activity) => work.activity = Some(activity),
            Err(ActivityError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => work.errors.push(ReadError {
                file: ACTIVITY_FILE.to_string(),
                error: error.to_string(),
            }),
        }
        // The message breaks the file-name tie, so the error order is total.
        work.errors
            .sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.error.cmp(&b.error)));
        work
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty() && self.items.is_empty()
    }

    /// Whether the journal was there and could not be read.
    ///
    /// A journal that is absent is not a broken one, and the two states do
    /// not say the same thing.
    pub fn activity_unreadable(&self) -> bool {
        self.errors.iter().any(|error| error.file == ACTIVITY_FILE)
    }

    /// One task by id.
    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// One smaller task by id.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }

    /// The title of a record by id — a task or a smaller task.
    ///
    /// The detail pane opens from the Backlog too, and the backlog holds
    /// both. The title is looked up again on every redraw, so a changed
    /// file shows up on the next one.
    pub fn record_title(&self, id: &str) -> Option<String> {
        self.task(id)
            .map(|task| task.title.clone())
            .or_else(|| self.item(id).map(|item| item.title.clone()))
    }

    /// Tasks per status, in the order of the lifecycle.
    pub fn status_counts(&self) -> Vec<(Status, usize)> {
        STATUSES
            .iter()
            .map(|status| {
                let count = self
                    .tasks
                    .iter()
                    .filter(|task| task.status == *status)
                    .count();
                (*status, count)
            })
            .collect()
    }

    /// Smaller tasks per kind.
    pub fn kind_counts(&self) -> Vec<(ItemKind, usize)> {
        KINDS
            .iter()
            .map(|kind| {
                let count = self.items.iter().filter(|item| item.kind == *kind).count();
                (*kind, count)
            })
            .collect()
    }

    /// The tasks whose next action is the human's, with the reason each
    /// waits. The rule is core's: `lifecycle::needs_you`.
    pub fn needs_you(&self) -> Vec<(&Task, Reason)> {
        self.tasks
            .iter()
            .filter_map(|task| lifecycle::needs_you(task).map(|reason| (task, reason)))
            .collect()
    }

    /// How many tasks wait on a human, per reason.
    pub fn needs_you_counts(&self) -> (usize, usize, usize) {
        let mut counts = (0, 0, 0);
        for (_, reason) in self.needs_you() {
            match reason {
                Reason::Approve => counts.0 += 1,
                Reason::Review => counts.1 += 1,
                Reason::Escalated => counts.2 += 1,
            }
        }
        counts
    }

    /// The tasks an agent could claim at `now`.
    ///
    /// `now` is the frame's instant when a screen draws: the claim test and
    /// every count and row it feeds answer from the same clock.
    pub fn claimable_at(&self, now: &str) -> Vec<&Task> {
        let mut claimable: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|task| task.status == Status::Approved)
            .filter(|task| !has_overlay(task))
            .filter(|task| !live_claim(task, now))
            .filter(|task| self.dependencies_done(task))
            .collect();
        // Dependency order first: the shortest dependency chain comes first.
        let mut depths: HashMap<String, u32> = HashMap::new();
        claimable.sort_by_key(|task| (self.depth(task, &mut depths), task.id.clone()));
        claimable
    }

    /// The first claimable task at `now` — what happens next.
    pub fn next_at(&self, now: &str) -> Option<&Task> {
        self.claimable_at(now).into_iter().next()
    }

    /// Whether every dependency of the task is done.
    fn dependencies_done(&self, task: &Task) -> bool {
        task.depends_on.iter().all(|dependency| {
            self.task(dependency)
                .is_some_and(|dependency| dependency.status == Status::Done)
        })
    }

    /// How deep a task sits in the dependency graph.
    fn depth(&self, task: &Task, depths: &mut HashMap<String, u32>) -> u32 {
        let mut visiting = Vec::new();
        self.depth_of(task, depths, &mut visiting)
    }

    fn depth_of(
        &self,
        task: &Task,
        depths: &mut HashMap<String, u32>,
        visiting: &mut Vec<String>,
    ) -> u32 {
        if let Some(depth) = depths.get(&task.id) {
            return *depth;
        }
        // A dependency cycle would loop forever; the graph is acyclic by
        // design, but the reader does not have to trust that.
        if visiting.contains(&task.id) {
            return 0;
        }
        visiting.push(task.id.clone());
        let deepest = task
            .depends_on
            .iter()
            .filter_map(|dependency| self.task(dependency))
            .map(|dependency| self.depth_of(dependency, depths, visiting))
            .max()
            .unwrap_or(0);
        visiting.pop();
        let depth = deepest + 1;
        depths.insert(task.id.clone(), depth);
        depth
    }
}

/// The statuses of the lifecycle, in order.
pub const STATUSES: [Status; 8] = [
    Status::Draft,
    Status::Groomed,
    Status::Approved,
    Status::Executing,
    Status::CodeReview,
    Status::Remediating,
    Status::NeedsHuman,
    Status::Done,
];

/// The kinds of a smaller task.
pub const KINDS: [ItemKind; 3] = [ItemKind::Idea, ItemKind::Feature, ItemKind::Bug];

/// Whether the task carries any overlay.
pub fn has_overlay(task: &Task) -> bool {
    task.overlays.any()
}

/// Whether a claim is live — held, and not expired.
pub fn live_claim(task: &Task, now: &str) -> bool {
    task.claim
        .as_ref()
        .is_some_and(|claim| !claim.lease_expired(now))
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_crew_core::{Claim, EventKind, Overlay};

    fn mock_work() -> Work {
        Work::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/work"))
    }

    fn work_of(tasks: Vec<Task>) -> Work {
        Work {
            root: PathBuf::new(),
            tasks,
            items: Vec::new(),
            errors: Vec::new(),
            activity: None,
            missing: None,
        }
    }

    /// The tests' clock: every fixture claim except the held-elsewhere one
    /// has expired by it, and that one is still live.
    const NOW: &str = "2026-11-19T08:00:00Z";

    fn approved(id: &str, depends_on: &[&str]) -> Task {
        let mut task = Task::new(id, format!("task {id}"));
        task.status = Status::Approved;
        task.depends_on = depends_on.iter().map(|id| id.to_string()).collect();
        task
    }

    fn count_of<T: PartialEq>(counts: &[(T, usize)], key: T) -> usize {
        counts
            .iter()
            .find(|(counted, _)| *counted == key)
            .map(|(_, count)| *count)
            .unwrap_or(0)
    }

    #[test]
    fn reads_the_mock_folder() {
        let work = mock_work();
        assert!(work.missing.is_none());
        assert_eq!(work.tasks.len(), 30);
        assert_eq!(work.items.len(), 5);
        assert!(work.errors.is_empty());
    }

    #[test]
    fn reads_the_mock_journal_beside_the_work_folder() {
        let work = mock_work();
        let activity = work.activity.as_ref().expect("the mock journal");
        assert!(!activity.is_empty(), "the mock journal has no events");
        // The journal sits beside work/, not inside it.
        let journal =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mock-data/coding-crew/activity.json");
        assert!(journal.is_file(), "{} is not a file", journal.display());
        assert!(
            !work.root.join("activity.json").exists(),
            "the journal is inside the work folder"
        );
        assert!(work.errors.is_empty(), "{:?}", work.errors);
    }

    #[test]
    fn the_mock_journal_reads_every_move_it_holds() {
        // The moves the history tells are events the journal reads back, and
        // none is unreadable: the Overview counts what the journal holds.
        let work = mock_work();
        let activity = work.activity.as_ref().expect("the mock journal");
        assert_eq!(
            activity.unreadable, 0,
            "the mock journal has unreadable events"
        );
        let kinds: Vec<EventKind> = activity.events.iter().map(|event| event.kind).collect();
        for kind in [
            EventKind::Started,
            EventKind::Released,
            EventKind::Blocked,
            EventKind::Unblocked,
            EventKind::Deferred,
            EventKind::Undeferred,
            EventKind::Reopened,
        ] {
            assert!(kinds.contains(&kind), "the mock journal holds no {kind:?}");
        }
    }

    #[test]
    fn tasks_and_items_load_in_numeric_id_order() {
        let dir = std::env::temp_dir().join(format!("raul-id-order-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a work folder");
        let task = |id: &str| {
            format!(
                r#"{{"schema_version":1,"id":"{id}","title":"t","slug":"t","created":"2026-11-02T09:00Z","updated":"2026-11-02T09:00Z","status":"approved","status_since":"2026-11-02T09:00Z"}}"#
            )
        };
        let item = |id: &str| {
            format!(
                r#"{{"schema_version":1,"id":"{id}","title":"t","slug":"t","kind":"idea","status":"open"}}"#
            )
        };
        fs::write(dir.join("TSK10-ten.json"), task("TSK10")).expect("a task");
        fs::write(dir.join("TSK2-two.json"), task("TSK2")).expect("a task");
        fs::write(dir.join("ITM10-ten.json"), item("ITM10")).expect("an item");
        fs::write(dir.join("ITM2-two.json"), item("ITM2")).expect("an item");
        let work = Work::load(&dir);
        let tasks: Vec<&str> = work.tasks.iter().map(|task| task.id.as_str()).collect();
        let items: Vec<&str> = work.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(tasks, ["TSK2", "TSK10"]);
        assert_eq!(items, ["ITM2", "ITM10"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_project_without_a_journal_has_no_activity_and_no_error() {
        let dir = std::env::temp_dir().join(format!("raul-no-journal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a work folder");
        let work = Work::load(&dir);
        assert!(work.activity.is_none());
        assert!(work.errors.is_empty(), "{:?}", work.errors);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_journal_that_does_not_parse_is_reported_not_fatal() {
        let dir = std::env::temp_dir().join(format!("raul-broken-journal-{}", std::process::id()));
        let work_dir = dir.join("work");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&work_dir).expect("a work folder");
        fs::write(dir.join("activity.json"), "{ not json").expect("a broken journal");
        let work = Work::load(&work_dir);
        assert!(work.activity.is_none());
        assert_eq!(work.errors.len(), 1);
        assert_eq!(work.errors[0].file, "activity.json");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_journal_is_read_again_on_every_load() {
        let dir = std::env::temp_dir().join(format!("raul-journal-reload-{}", std::process::id()));
        let work_dir = dir.join("work");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&work_dir).expect("a work folder");
        let journal = dir.join("activity.json");
        let write = |kind: &str| {
            fs::write(
                &journal,
                format!(
                    r#"{{"events":[{{"at":"2026-11-01T09:00Z","kind":"{kind}","task":"TSK01","by":"human"}}]}}"#
                ),
            )
            .expect("writing the journal");
        };
        write("created");
        let first = Work::load(&work_dir);
        assert_eq!(
            first.activity.as_ref().expect("a journal").events[0].kind,
            coding_crew_core::EventKind::Created
        );
        write("completed");
        let second = Work::load(&work_dir);
        assert_eq!(
            second.activity.as_ref().expect("a journal").events[0].kind,
            coding_crew_core::EventKind::Completed,
            "the journal was remembered between loads"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn counts_the_tasks_by_status() {
        let work = mock_work();
        let counts = work.status_counts();
        assert_eq!(count_of(&counts, Status::Draft), 6);
        assert_eq!(count_of(&counts, Status::Groomed), 3);
        assert_eq!(count_of(&counts, Status::Approved), 4);
        assert_eq!(count_of(&counts, Status::Executing), 3);
        assert_eq!(count_of(&counts, Status::CodeReview), 3);
        assert_eq!(count_of(&counts, Status::Remediating), 3);
        assert_eq!(count_of(&counts, Status::NeedsHuman), 3);
        assert_eq!(count_of(&counts, Status::Done), 5);
    }

    #[test]
    fn counts_the_smaller_tasks_by_kind() {
        let work = mock_work();
        let counts = work.kind_counts();
        assert_eq!(count_of(&counts, ItemKind::Idea), 1);
        assert_eq!(count_of(&counts, ItemKind::Feature), 2);
        assert_eq!(count_of(&counts, ItemKind::Bug), 2);
    }

    #[test]
    fn counts_what_waits_on_a_human() {
        let work = mock_work();
        // Two groomed tasks wait for approval, one of them blocked and so
        // out of the human lane; three wait for review, three are escalated.
        assert_eq!(work.needs_you_counts(), (2, 3, 3));
    }

    #[test]
    fn a_blocked_task_does_not_wait_on_a_human() {
        let mut task = Task::new("TSK01", "blocked");
        task.status = Status::Groomed;
        task.overlays.blocked = Some(Overlay {
            reason: "waiting".into(),
            by: "creator-01".into(),
            at: "2026-11-13T10:00Z".into(),
        });
        let work = work_of(vec![task]);
        assert_eq!(work.needs_you_counts(), (0, 0, 0));
    }

    #[test]
    fn a_claimable_task_is_approved_and_free() {
        let work = work_of(vec![approved("TSK01", &[])]);
        assert_eq!(work.claimable_at(NOW).len(), 1);
        assert_eq!(
            work.next_at(NOW).map(|task| task.id.as_str()),
            Some("TSK01")
        );
    }

    #[test]
    fn a_task_waits_for_its_dependencies() {
        let mut dependency = approved("TSK01", &[]);
        dependency.status = Status::Done;
        let work = work_of(vec![dependency, approved("TSK02", &["TSK01"])]);
        assert_eq!(work.claimable_at(NOW).len(), 1);
        assert_eq!(
            work.next_at(NOW).map(|task| task.id.as_str()),
            Some("TSK02")
        );
    }

    #[test]
    fn an_unfinished_dependency_blocks_the_claim() {
        let dependency = approved("TSK01", &[]);
        let work = work_of(vec![dependency, approved("TSK02", &["TSK01"])]);
        let claimable: Vec<&str> = work
            .claimable_at(NOW)
            .iter()
            .map(|task| task.id.as_str())
            .collect();
        assert_eq!(claimable, vec!["TSK01"]);
        assert_eq!(
            work.next_at(NOW).map(|task| task.id.as_str()),
            Some("TSK01")
        );
    }

    #[test]
    fn a_claim_holds_the_task() {
        let mut task = approved("TSK01", &[]);
        task.claim = Some(Claim {
            by: "runner-01".into(),
            session: Some("session-runner-01".into()),
            at: "2026-11-13T10:00Z".into(),
            lease_expires_at: "2099-01-01T00:00:00Z".into(),
        });
        let work = work_of(vec![task]);
        assert!(work.claimable_at(NOW).is_empty());
    }

    #[test]
    fn an_overlay_holds_the_task() {
        let mut task = approved("TSK01", &[]);
        task.overlays.deferred = Some(Overlay {
            reason: "later".into(),
            by: "creator-01".into(),
            at: "2026-11-13T10:00Z".into(),
        });
        let work = work_of(vec![task]);
        assert!(work.claimable_at(NOW).is_empty());
    }

    #[test]
    fn the_next_task_comes_first_in_dependency_order() {
        let mut first = approved("TSK01", &[]);
        first.status = Status::Done;
        let second = approved("TSK02", &["TSK01"]);
        let third = approved("TSK03", &["TSK01"]);
        let work = work_of(vec![first, second, third]);
        let next = work.next_at(NOW).expect("a claimable task");
        assert_eq!(next.id, "TSK02");
    }

    #[test]
    fn the_mock_data_has_a_claimable_task() {
        // One task was approved after the last cycle finished: its
        // dependencies are done, so an agent can take it right now.
        let work = mock_work();
        let claimable: Vec<&str> = work
            .claimable_at(NOW)
            .iter()
            .map(|task| task.id.as_str())
            .collect();
        assert_eq!(claimable, vec!["TSK29"]);
        assert_eq!(
            work.next_at(NOW).map(|task| task.id.as_str()),
            Some("TSK29")
        );
    }

    #[test]
    fn a_missing_folder_is_not_an_error() {
        let work = Work::load(Path::new("/nowhere/coding-crew/work"));
        assert_eq!(work.missing, Some(Missing::NoFolder));
        assert!(work.is_empty());
    }

    #[test]
    fn a_missing_folder_keeps_the_reason_it_is_missing() {
        let work = Work::load_missing(
            Path::new("/nowhere/project/coding-crew/work"),
            Missing::NoWork,
        );
        assert_eq!(work.missing, Some(Missing::NoWork));
        assert_eq!(
            work.missing.expect("a reason").headline(),
            "this project has no work folder"
        );
    }

    #[test]
    fn duplicate_ids_keep_the_file_name_order() {
        // The files are created in name order, and both ext4 and tmpfs hand
        // the same four names back in a different order. Only the loader's
        // pre-parse name sort can give the order asserted here.
        let dir = std::env::temp_dir().join(format!("raul-duplicate-load-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a work folder");
        for (name, contents) in support::DUPLICATE_ID_FILES {
            fs::write(dir.join(name), contents).expect("writing a duplicate-id file");
        }
        let work = Work::load(&dir);
        let tasks: Vec<(&str, &str)> = work
            .tasks
            .iter()
            .map(|task| (task.id.as_str(), task.title.as_str()))
            .collect();
        let items: Vec<(&str, &str)> = work
            .items
            .iter()
            .map(|item| (item.id.as_str(), item.title.as_str()))
            .collect();
        assert_eq!(tasks, vec![("TSK01", "Alpha task"), ("TSK01", "Beta task")]);
        assert_eq!(items, vec![("ITM01", "Alpha item"), ("ITM01", "Beta item")]);
        assert!(work.errors.is_empty(), "{:?}", work.errors);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_files_are_ordered_by_name() {
        // The files are created in a different order from their names, and
        // read_dir hands back whatever order the filesystem keeps.
        let work = Work::load(&support::malformed_work("unreadable-order"));
        let files: Vec<&str> = work
            .errors
            .iter()
            .map(|error| error.file.as_str())
            .collect();
        assert_eq!(files, support::NAME_ORDER);
    }

    #[test]
    fn the_shared_text_helpers_have_one_implementation() {
        // The helpers live in the theme crate. A private copy in raul or hugo
        // is a second rule for measuring, cutting and padding, and the two
        // drift. The names are joined at run time so this file does not match
        // its own scan.
        let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the crates folder");
        let names = [
            "text_width",
            "cut_to",
            "tail_to",
            "head_to",
            "first_grapheme",
            "pad",
        ]
        .map(|name| format!("fn {name}("));
        let mut copies: Vec<String> = Vec::new();
        for krate in ["raul", "hugo"] {
            let mut pending = vec![crates.join(krate)];
            while let Some(dir) = pending.pop() {
                for entry in fs::read_dir(&dir).expect("a crate's tree") {
                    let path = entry.expect("a source file").path();
                    if path.is_dir() {
                        pending.push(path);
                        continue;
                    }
                    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                        continue;
                    }
                    let text = fs::read_to_string(&path).expect("a crate source");
                    for name in &names {
                        if text.contains(name.as_str()) {
                            copies.push(format!("{}: {name}", path.display()));
                        }
                    }
                }
            }
        }
        assert!(
            copies.is_empty(),
            "a private copy of the shared helper is back: {copies:?}"
        );
    }
}
