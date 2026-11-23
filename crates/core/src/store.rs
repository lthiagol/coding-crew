use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::iter::Peekable;
use std::ops::Deref;
use std::path::{Component, Path, PathBuf};
use std::str::Chars;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use thiserror::Error;

use crate::activity::Activity;
use crate::item::Item;
use crate::runtime::Runtime;
use crate::task::{Task, slugify};

/// Name of the data folder inside a managed project.
pub const DATA_DIR: &str = "coding-crew";

/// Name of the work folder inside the data folder.
pub const WORK_DIR: &str = "work";

/// Name of the project's activity journal, beside the work folder.
pub const ACTIVITY_FILE: &str = "activity.json";

/// Name of the flow's runtime record, beside the journal.
pub const RUNTIME_FILE: &str = "runtime.json";

/// Name of the project's write lock, beside the work folder (ADR 0016).
pub const LOCK_FILE: &str = ".talita.lock";

/// How long a writer waits for the project lock before it gives up.
pub const LOCK_WAIT: Duration = Duration::from_secs(10);

/// How often a waiting writer retries the lock.
const LOCK_POLL: Duration = Duration::from_millis(10);

/// The journal of a project, from its work folder.
///
/// The journal sits beside `work/`, not inside it: the work folder holds one
/// file per work unit, and a journal in there would be read as a task.
pub fn activity_path(work: &Path) -> PathBuf {
    work.parent().unwrap_or(work).join(ACTIVITY_FILE)
}

/// The runtime record of a project, from its work folder.
///
/// The record sits beside the journal, and for the same reason outside
/// `work/`: the work folder holds one file per work unit.
pub fn runtime_path(work: &Path) -> PathBuf {
    work.parent().unwrap_or(work).join(RUNTIME_FILE)
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to read: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse task file: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("task {0} not found")]
    NotFound(String),
    #[error("no coding-crew folder found")]
    NoDataFolder,
    #[error("{} has no work folder", .0.display())]
    NoWorkFolder(PathBuf),
    #[error("{} is drive-relative; use an absolute path or one without a drive prefix", .0.display())]
    InvalidStart(PathBuf),
    #[error("{0}")]
    Item(String),
    #[error("another writer holds the project's lock; try again")]
    Locked,
    #[error("the journal holds {0} event(s) this reader cannot read; it is not rewritten")]
    JournalUnreadable(usize),
    #[error("task revision {expected} is stale; the file holds {found}. Re-read the task")]
    Revision { expected: u32, found: u32 },
}

/// The work files of one project.
///
/// Layout:
/// ```text
/// <project>/coding-crew/work/TSK01-<slug>.json
/// ```
///
/// One folder holds tasks (`TSK*`) and smaller tasks (`ITM*`).
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Store rooted at a work folder.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Store of a project, by its root.
    pub fn for_project(project: &Path) -> Self {
        Self::new(project.join(DATA_DIR).join(WORK_DIR))
    }

    /// Find the store of the nearest project containing `start`, walking up.
    ///
    /// The nearest `coding-crew/` is the project, and it owns the work
    /// folder: the store is rooted at its `work/`, whether or not that folder
    /// exists yet. The walk stops at the nearest project, so an ancestor
    /// project's tasks are never this project's.
    ///
    /// `start` may be relative; the root returned is absolute.
    pub fn discover(start: &Path) -> Result<Self, StoreError> {
        let data = nearest_project(start, None)?;
        Ok(Self::new(data.join(WORK_DIR)))
    }

    /// Find the work folder of the nearest project containing `start`,
    /// walking up.
    ///
    /// The rule is the same as [`Store::discover`], but the nearest project
    /// must have a `work/` folder. When it does not, the error names its
    /// `coding-crew/` folder and the walk does not continue: an ancestor's
    /// work is not this project's.
    ///
    /// `start` may be relative; the root returned is absolute.
    pub fn discover_work(start: &Path) -> Result<Self, StoreError> {
        let data = nearest_project(start, None)?;
        if data.join(WORK_DIR).is_dir() {
            Ok(Self::new(data.join(WORK_DIR)))
        } else {
            Err(StoreError::NoWorkFolder(data))
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path of the project's journal: beside the work folder.
    pub fn activity_path(&self) -> PathBuf {
        activity_path(&self.root)
    }

    /// Path of the flow's runtime record: beside the journal.
    pub fn runtime_path(&self) -> PathBuf {
        runtime_path(&self.root)
    }

    /// Path of the project's write lock: beside the work folder.
    pub fn lock_path(&self) -> PathBuf {
        self.root.parent().unwrap_or(&self.root).join(LOCK_FILE)
    }

    /// Path of a task file.
    pub fn task_path(&self, task: &Task) -> PathBuf {
        self.root.join(format!("{}-{}.json", task.id, task.slug))
    }

    /// All tasks, ordered by id.
    pub fn list(&self) -> Result<Vec<Task>, StoreError> {
        let dir = self.root.clone();
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut tasks: Vec<Task> = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if !file_starts_with(&path, "TSK") {
                continue;
            }
            match load_task_path(&path) {
                Ok(task) => tasks.push(task),
                // A file that does not parse is skipped, not fatal.
                Err(_) => continue,
            }
        }
        let mut newest: BTreeMap<String, Task> = BTreeMap::new();
        for task in tasks {
            if newest
                .get(&task.id)
                .is_none_or(|held| is_newer(&task, held))
            {
                newest.insert(task.id.clone(), task);
            }
        }
        let mut tasks: Vec<Task> = newest.into_values().collect();
        tasks.sort_by(|a, b| by_number(&a.id, &b.id));
        Ok(tasks)
    }

    /// One task by id. Where two files hold the id, the newer record is
    /// returned (see [`Store::list`]).
    pub fn load(&self, id: &str) -> Result<Task, StoreError> {
        let dir = self.root.clone();
        if !dir.is_dir() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        let mut newest: Option<Task> = None;
        let mut failure: Option<StoreError> = None;
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if file_id(&path) == Some(id) && file_starts_with(&path, "TSK") {
                match load_task_path(&path) {
                    Ok(task) if task.id == id => {
                        if newest.as_ref().is_none_or(|held| is_newer(&task, held)) {
                            newest = Some(task);
                        }
                    }
                    Ok(_) => {}
                    Err(error) => failure = failure.or(Some(error)),
                }
            }
        }
        match (newest, failure) {
            (Some(task), _) => Ok(task),
            (None, Some(error)) => Err(error),
            (None, None) => Err(StoreError::NotFound(id.to_string())),
        }
    }

    /// Ids that more than one work file holds, ordered by number.
    ///
    /// One id never has two files (TM § 11). A crash between a rename's
    /// write and its cleanup leaves two; readers keep the newer record, and
    /// A check reports these ids.
    pub fn duplicate_ids(&self) -> Result<Vec<String>, StoreError> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }
        let mut files: BTreeMap<String, usize> = BTreeMap::new();
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(id) = file_id(&path) {
                *files.entry(id.to_string()).or_insert(0) += 1;
            }
        }
        let mut ids: Vec<String> = files
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(id, _)| id)
            .collect();
        ids.sort_by(|a, b| by_number(a, b));
        Ok(ids)
    }

    /// Take the project's write lock, waiting up to [`LOCK_WAIT`].
    ///
    /// Every write takes it for its whole read–check–write (ADR 0016). Reads
    /// take none. The lock is released when the guard drops.
    pub fn lock(&self) -> Result<ProjectLock<'_>, StoreError> {
        fs::create_dir_all(&self.root)?;
        let file = acquire(&self.lock_path(), LOCK_WAIT)?;
        Ok(ProjectLock {
            store: self,
            _file: file,
        })
    }

    /// The revision the file under `id` holds; 0 when there is none.
    fn revision_on_disk(&self, id: &str) -> Result<u32, StoreError> {
        match self.load(id) {
            Ok(task) => Ok(task.revision),
            Err(StoreError::NotFound(_)) => Ok(0),
            Err(error) => Err(error),
        }
    }

    /// Remove files of the same id left under an older name.
    fn remove_other_copies(&self, id: &str, keep: &Path) -> Result<(), StoreError> {
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path == keep {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if file_id(&path) == Some(id) && name.ends_with(".json") {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    /// Create a task in `draft`, under the lock: the id scan and the write
    /// are one step, so two creators cannot take the same id.
    pub fn create(&self, title: &str) -> Result<Task, StoreError> {
        self.lock()?.create(title)
    }

    /// All smaller tasks, ordered by id.
    pub fn list_items(&self) -> Result<Vec<Item>, StoreError> {
        let dir = self.root.clone();
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut items: Vec<Item> = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if !file_starts_with(&path, "ITM") {
                continue;
            }
            match load_item_path(&path) {
                Ok(item) => items.push(item),
                Err(_) => continue,
            }
        }
        items.sort_by(|a, b| by_number(&a.id, &b.id));
        Ok(items)
    }

    /// One smaller task by id.
    pub fn load_item(&self, id: &str) -> Result<Item, StoreError> {
        let dir = self.root.clone();
        if !dir.is_dir() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if file_id(&path) == Some(id) && file_starts_with(&path, "ITM") {
                let item = load_item_path(&path)?;
                if item.id == id {
                    return Ok(item);
                }
            }
        }
        Err(StoreError::NotFound(id.to_string()))
    }

    /// Create a smaller task. The kind starts as an idea.
    pub fn create_item(&self, title: &str) -> Result<Item, StoreError> {
        self.lock()?.create_item(title)
    }

    /// Next free smaller-task id, scanning what exists.
    pub fn next_item_id(&self) -> Result<String, StoreError> {
        Ok(format!("ITM{:02}", self.highest_number("ITM")? + 1))
    }

    /// Promote a smaller task into a task, under the lock.
    pub fn promote_item(&self, id: &str) -> Result<Task, StoreError> {
        self.lock()?.promote_item(id)
    }

    /// Next free task id, scanning what exists.
    pub fn next_id(&self) -> Result<String, StoreError> {
        Ok(format!("TSK{:02}", self.highest_number("TSK")? + 1))
    }

    /// Highest plain number among the work files named `<prefix><digits>`.
    ///
    /// Reads file names, not parsed records: a file that does not parse still
    /// holds its id, so a new task must not take it. Child ids (`TSK03.1`) are
    /// not plain numbers and are skipped.
    fn highest_number(&self, prefix: &str) -> Result<u32, StoreError> {
        let mut highest: u32 = 0;
        if !self.root.is_dir() {
            return Ok(highest);
        }
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(id) = file_id(&path)
                && let Some(number) = id.strip_prefix(prefix)
                && let Ok(number) = number.parse::<u32>()
            {
                highest = highest.max(number);
            }
        }
        Ok(highest)
    }
}

/// The project's write lock, held for one read–check–write.
///
/// Every write goes through this guard, so the read it checks against is the
/// one it writes over (ADR 0016). [`Store::create`], [`Store::create_item`]
/// and [`Store::promote_item`] take it themselves; a caller that changes a
/// record it loaded takes the guard with [`Store::lock`] and writes through
/// it. Reads go through `Deref` to the store and take no lock. The `flock` is
/// released when the guard drops.
pub struct ProjectLock<'a> {
    store: &'a Store,
    _file: fs::File,
}

impl Deref for ProjectLock<'_> {
    type Target = Store;

    fn deref(&self) -> &Store {
        self.store
    }
}

impl ProjectLock<'_> {
    /// Write a task file, atomically, and move its revision on.
    ///
    /// `task.revision` must be the one the file holds (a new task holds 0),
    /// or the write is refused with [`StoreError::Revision`]. On success the
    /// file holds the revision plus one, and so does `task`. A file the same
    /// task had under another name is removed, so one task is one file.
    pub fn save(&self, task: &mut Task) -> Result<(), StoreError> {
        let found = self.revision_on_disk(&task.id)?;
        if task.revision != found {
            return Err(StoreError::Revision {
                expected: task.revision,
                found,
            });
        }
        fs::create_dir_all(&self.root)?;
        let mut next = task.clone();
        next.revision = found + 1;
        let path = self.task_path(&next);
        write_atomic(&path, &json_text(&next)?)?;
        task.revision = next.revision;
        self.remove_other_copies(&task.id, &path)
    }

    /// Create a task in `draft`: the id scan and the write, under this lock.
    pub fn create(&self, title: &str) -> Result<Task, StoreError> {
        let mut task = Task::new(self.next_id()?, title);
        self.save(&mut task)?;
        Ok(task)
    }

    /// Write a smaller task file, atomically. A file the same item had under
    /// another name is removed, so one item is one file.
    pub fn save_item(&self, item: &Item) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{}-{}.json", item.id, item.slug));
        write_atomic(&path, &json_text(item)?)?;
        self.remove_other_copies(&item.id, &path)
    }

    /// Create a smaller task. The kind starts as an idea.
    pub fn create_item(&self, title: &str) -> Result<Item, StoreError> {
        let item = Item::new(self.next_item_id()?, title);
        self.save_item(&item)?;
        Ok(item)
    }

    /// Promote a smaller task into a task.
    ///
    /// The item is written first, so a crash cannot produce two tasks from
    /// one item.
    pub fn promote_item(&self, id: &str) -> Result<Task, StoreError> {
        let mut item = self.load_item(id)?;
        let task_id = self.next_id()?;
        let mut task = item
            .promote(task_id)
            .map_err(|error| StoreError::Item(error.to_string()))?;
        self.save_item(&item)?;
        self.save(&mut task)?;
        Ok(task)
    }

    /// Write the journal, atomically.
    ///
    /// A journal read with events this reader cannot read would lose them on
    /// rewrite, so the write is refused: the file stays as it is.
    pub fn write_activity(&self, activity: &Activity) -> Result<(), StoreError> {
        if activity.unreadable > 0 {
            return Err(StoreError::JournalUnreadable(activity.unreadable));
        }
        fs::create_dir_all(&self.root)?;
        write_atomic(&self.activity_path(), &json_text(activity)?)
    }

    /// Write the runtime record, atomically.
    pub fn write_runtime(&self, runtime: &Runtime) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        write_atomic(&self.runtime_path(), &json_text(runtime)?)
    }
}

/// Take the exclusive `flock` on the lock file, polling until `wait` runs out.
///
/// The lock belongs to the open file, so a second writer in this process waits
/// on the first as another process would. Dropping the file frees it, and a
/// crashed writer frees it too: no stale lock file to clean.
pub(crate) fn acquire(path: &Path, wait: Duration) -> Result<fs::File, StoreError> {
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    let deadline = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                thread::sleep(LOCK_POLL);
            }
            Err(fs::TryLockError::WouldBlock) => return Err(StoreError::Locked),
            Err(fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

/// A value as the file holds it: pretty JSON with one trailing newline.
fn json_text<T: Serialize>(value: &T) -> Result<String, StoreError> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

/// Write `text` to `path` atomically: a `.json.tmp` beside it is synced and
/// renamed over the target, so a reader sees the old file or the new one.
fn write_atomic(path: &Path, text: &str) -> Result<(), StoreError> {
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Whether `a` is the newer record of one id: the higher revision, then the
/// later `updated`, then the slug. The order is total, so the pick never
/// depends on directory order.
fn is_newer(a: &Task, b: &Task) -> bool {
    (a.revision, &a.updated, &a.slug) > (b.revision, &b.updated, &b.slug)
}

/// Orders ids by their numbers, not their text: `TSK99` before `TSK100`,
/// and `TSK03` before its child `TSK03.1`.
pub fn by_number(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let (x, y) = (digits(&mut a), digits(&mut b));
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(&y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                a.next();
                b.next();
            }
        }
    }
}

/// The run of digits at the front of `chars`, without its leading zeros.
fn digits(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut run = String::new();
    while let Some(c) = chars.next_if(char::is_ascii_digit) {
        run.push(c);
    }
    run.trim_start_matches('0').to_string()
}

/// The nearest project's data folder at or above `start`.
///
/// The first `coding-crew/` folder is the project: the walk stops there
/// whether or not it holds `work/`. `inside` bounds the walk when the caller
/// owns the search; the public methods walk to the filesystem root.
fn nearest_project(start: &Path, inside: Option<&Path>) -> Result<PathBuf, StoreError> {
    let mut dir = absolute(start)?;
    let stop = inside.map(absolute).transpose()?;
    loop {
        let candidate = dir.join(DATA_DIR);
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if stop.as_deref() == Some(dir.as_path()) || !dir.pop() {
            return Err(StoreError::NoDataFolder);
        }
    }
}

/// The path as an absolute one, with `.` and `..` resolved lexically.
///
/// A relative start is taken from the current directory, so the store's
/// meaning does not change with the cwd. Without the lexical normalization,
/// the ancestor walk would pop `..` as if it were a folder name and inspect
/// paths outside the resolved start's chain. The path is never resolved
/// against the filesystem, so a start that does not exist still works.
///
/// A drive-relative start such as `C:foo` is rejected: joining it to the
/// current directory would not make it absolute, and its meaning would still
/// depend on that drive's current directory.
fn absolute(path: &Path) -> Result<PathBuf, StoreError> {
    if has_prefix_without_root(path) {
        return Err(StoreError::InvalidStart(path.to_path_buf()));
    }
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize(&joined))
}

/// Whether a path has a prefix (a Windows drive, say) but no root.
fn has_prefix_without_root(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::Prefix(_)))
        && !path.is_absolute()
}

/// Resolve `.` and `..` in a path without touching the filesystem.
///
/// A `..` at the filesystem root stays at the root; a leading `..` on a
/// relative path is kept, and a `..` that would rise above one already kept
/// is kept too.
fn normalize(path: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => resolved.push(prefix.as_os_str()),
            Component::RootDir => resolved.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let last = resolved.components().next_back();
                if matches!(last, Some(Component::Normal(_))) {
                    resolved.pop();
                } else if !path.is_absolute() {
                    resolved.push(Component::ParentDir.as_os_str());
                }
            }
            Component::Normal(part) => resolved.push(part),
        }
    }
    resolved
}

fn load_task_path(path: &Path) -> Result<Task, StoreError> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn load_item_path(path: &Path) -> Result<Item, StoreError> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

/// Whether a file's name starts with an id prefix.
fn file_starts_with(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.starts_with(prefix))
}

/// The id a work file is named for: its name up to the first `-`.
///
/// Ids have no `-` and the slug follows it, so `TSK03.1-x.json` names `TSK03.1`.
fn file_id(path: &Path) -> Option<&str> {
    path.file_name()?
        .to_str()?
        .split_once('-')
        .map(|(id, _)| id)
}

/// The file name a task would get.
pub fn file_name(task: &Task) -> String {
    format!("{}-{}.json", task.id, slugify(&task.title))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("coding-crew-walk-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    #[test]
    fn the_walk_does_not_leave_its_own_boundary() {
        // An unrelated coding-crew folder above the boundary must not
        // change the result: the tests own this walk.
        let root = scratch("boundary");
        let start = root.join("src");
        fs::create_dir_all(&start).expect("a nested folder");

        let error = nearest_project(&start, Some(&root)).expect_err("there is no project here");
        assert!(
            matches!(error, StoreError::NoDataFolder),
            "expected the no-data error, got {error:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_project_at_the_boundary_is_found() {
        let root = scratch("boundary-project");
        let work = root.join(DATA_DIR).join(WORK_DIR);
        fs::create_dir_all(&work).expect("a work folder");

        let data = nearest_project(&root, Some(&root)).expect("the project");
        assert_eq!(data, root.join(DATA_DIR));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_journal_sits_beside_the_work_folder() {
        // The journal is a sibling of `work/`, not a file inside it: the
        // work folder reads every json file in it as a work unit.
        let project = Path::new("/project/coding-crew");
        let work = project.join(WORK_DIR);
        assert_eq!(
            activity_path(&work),
            project.join(ACTIVITY_FILE),
            "the journal is not beside work/"
        );
        let store = Store::new(&work);
        assert_eq!(store.activity_path(), project.join(ACTIVITY_FILE));
    }

    #[test]
    fn the_runtime_record_sits_beside_the_journal() {
        // The record is outside `work/` for the journal's reason: the work
        // folder holds one file per work unit.
        let project = Path::new("/project/coding-crew");
        let work = project.join(WORK_DIR);
        assert_eq!(
            runtime_path(&work),
            project.join(RUNTIME_FILE),
            "the record is not beside the journal"
        );
        assert_eq!(runtime_path(&work).parent(), activity_path(&work).parent());
        let store = Store::new(&work);
        assert_eq!(store.runtime_path(), project.join(RUNTIME_FILE));
    }

    #[test]
    fn a_held_lock_refuses_a_second_writer_once_its_wait_runs_out() {
        // Two open files conflict even in one process, as two writers would.
        let root = scratch("held-lock");
        fs::create_dir_all(&root).expect("a folder");
        let path = root.join(LOCK_FILE);

        let held = acquire(&path, Duration::ZERO).expect("the first writer");
        assert!(matches!(
            acquire(&path, Duration::from_millis(50)),
            Err(StoreError::Locked)
        ));
        drop(held);
        acquire(&path, Duration::ZERO).expect("the lock is free again");
        let _ = fs::remove_dir_all(&root);
    }

    /// A work folder inside a scratch project, so the lock sits beside it.
    fn work_folder(name: &str) -> PathBuf {
        let work = scratch(name).join(DATA_DIR).join(WORK_DIR);
        fs::create_dir_all(&work).expect("a work folder");
        work
    }

    #[test]
    fn a_crash_mid_rename_leaves_one_task_for_readers() {
        // A rename writes the new file, then removes the old. A crash between
        // the two leaves both: readers keep the newer record, and the next
        // save removes the stale copy.
        let work = work_folder("crash-rename");
        let store = Store::new(&work);
        let task = store.create("first").expect("a task");
        assert_eq!(task.revision, 1);

        let mut renamed = task.clone();
        renamed.slug = "second".to_string();
        renamed.revision = 2;
        write_atomic(
            &work.join("TSK01-second.json"),
            &json_text(&renamed).unwrap(),
        )
        .unwrap();

        assert_eq!(store.duplicate_ids().expect("the ids"), ["TSK01"]);
        let listed = store.list().expect("the tasks");
        assert_eq!(listed.len(), 1, "readers see the id twice");
        assert_eq!(listed[0].slug, "second");
        assert_eq!(store.load("TSK01").expect("the task").slug, "second");

        let mut newest = store.load("TSK01").expect("the task");
        store
            .lock()
            .expect("the lock")
            .save(&mut newest)
            .expect("a save");
        assert!(store.duplicate_ids().expect("the ids").is_empty());
        assert!(!work.join("TSK01-first.json").exists());
        let _ = fs::remove_dir_all(work.parent().and_then(Path::parent).unwrap());
    }

    #[test]
    fn ids_compare_by_their_numbers() {
        assert_eq!(by_number("TSK9", "TSK10"), Ordering::Less);
        assert_eq!(by_number("TSK100", "TSK99"), Ordering::Greater);
        assert_eq!(by_number("TSK03", "TSK03.1"), Ordering::Less);
        assert_eq!(by_number("TSK03.2", "TSK03.10"), Ordering::Less);
        assert_eq!(by_number("TSK007", "TSK7"), Ordering::Equal);
    }

    #[test]
    fn a_relative_start_becomes_absolute() {
        let cwd = std::env::current_dir().expect("the cwd");
        let resolved = absolute(Path::new("src")).expect("the current directory");
        assert!(resolved.is_absolute(), "{}", resolved.display());
        assert_eq!(resolved, cwd.join("src"));
    }

    #[test]
    fn dots_are_resolved_before_the_walk() {
        assert_eq!(
            absolute(Path::new("./a/./b/../src")).expect("the cwd"),
            std::env::current_dir()
                .expect("the cwd")
                .join("a")
                .join("src")
        );
    }

    #[cfg(unix)]
    #[test]
    fn dots_cannot_leave_the_root() {
        assert_eq!(normalize(Path::new("/a/../../b")), Path::new("/b"));
        assert_eq!(normalize(Path::new("/..")), Path::new("/"));
    }

    #[test]
    fn leading_parents_stay_on_a_relative_path() {
        assert_eq!(normalize(Path::new("../..")), Path::new("../.."));
        assert_eq!(normalize(Path::new("../../a/../b")), Path::new("../../b"));
        assert_eq!(normalize(Path::new("a/../../b")), Path::new("../b"));
    }

    #[cfg(unix)]
    #[test]
    fn a_drive_like_name_is_an_ordinary_relative_path() {
        // Unix has no drive prefix, so this is a plain name and must survive.
        let resolved = absolute(Path::new("C:foo")).expect("a plain name");
        assert_eq!(
            resolved,
            std::env::current_dir().expect("the cwd").join("C:foo")
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_prefix_without_a_root_is_detected() {
        assert!(has_prefix_without_root(Path::new("C:foo")));
        assert!(has_prefix_without_root(Path::new("C:")));
        assert!(!has_prefix_without_root(Path::new(r"C:\foo")));
        assert!(!has_prefix_without_root(Path::new(r"\foo")));
        assert!(!has_prefix_without_root(Path::new("foo")));
    }

    #[cfg(windows)]
    #[test]
    fn a_drive_relative_start_is_rejected() {
        let error = absolute(Path::new("C:foo")).expect_err("a drive-relative start");
        match error {
            StoreError::InvalidStart(path) => assert_eq!(path, Path::new("C:foo")),
            other => panic!("expected the invalid-start error, got {other:?}"),
        }

        // Ordinary relative and absolute paths still resolve.
        assert!(
            absolute(Path::new("src"))
                .expect("a relative start")
                .is_absolute()
        );
        assert!(
            absolute(Path::new(r"C:\src"))
                .expect("an absolute start")
                .is_absolute()
        );
    }
}
