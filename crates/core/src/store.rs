use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use crate::item::Item;
use crate::task::{Task, slugify};

/// Name of the data folder inside a managed project.
pub const DATA_DIR: &str = "coding-crew";

/// Name of the work folder inside the data folder.
pub const WORK_DIR: &str = "work";

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
        tasks.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(tasks)
    }

    /// One task by id.
    pub fn load(&self, id: &str) -> Result<Task, StoreError> {
        let dir = self.root.clone();
        if !dir.is_dir() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with(id)
                && file_starts_with(&path, "TSK")
            {
                return load_task_path(&path);
            }
        }
        Err(StoreError::NotFound(id.to_string()))
    }

    /// Write a task file, atomically. A file the same task had under another
    /// name is removed, so one task is one file.
    pub fn save(&self, task: &Task) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        let path = self.task_path(task);
        let json = serde_json::to_string_pretty(task)?;
        let tmp = path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        self.remove_other_copies(task, &path)?;
        Ok(())
    }

    /// Remove files of the same task left under an older name.
    fn remove_other_copies(&self, task: &Task, keep: &Path) -> Result<(), StoreError> {
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path == keep {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with(&task.id) && name.ends_with(".json") {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    /// Create a task in `draft`.
    pub fn create(&self, title: &str) -> Result<Task, StoreError> {
        let task = Task::new(self.next_id()?, title);
        self.save(&task)?;
        Ok(task)
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
        items.sort_by(|a, b| a.id.cmp(&b.id));
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
            if file_starts_with(&path, "ITM")
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with(id)
            {
                return load_item_path(&path);
            }
        }
        Err(StoreError::NotFound(id.to_string()))
    }

    /// Write a smaller task file, atomically.
    pub fn save_item(&self, item: &Item) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{}-{}.json", item.id, item.slug));
        let json = serde_json::to_string_pretty(item)?;
        let tmp = path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Create a smaller task. The kind starts as an idea.
    pub fn create_item(&self, title: &str) -> Result<Item, StoreError> {
        let item = Item::new(self.next_item_id()?, title);
        self.save_item(&item)?;
        Ok(item)
    }

    /// Next free smaller-task id, scanning what exists.
    pub fn next_item_id(&self) -> Result<String, StoreError> {
        let mut highest: u32 = 0;
        for item in self.list_items()? {
            if let Some(number) = item.id.strip_prefix("ITM")
                && let Ok(number) = number.parse::<u32>()
            {
                highest = highest.max(number);
            }
        }
        Ok(format!("ITM{:02}", highest + 1))
    }

    /// Promote a smaller task into a task.
    ///
    /// The item is written first, so a crash cannot produce two tasks from
    /// one item.
    pub fn promote_item(&self, id: &str) -> Result<Task, StoreError> {
        let mut item = self.load_item(id)?;
        let task_id = self.next_id()?;
        let task = item
            .promote(task_id)
            .map_err(|error| StoreError::Item(error.to_string()))?;
        self.save_item(&item)?;
        self.save(&task)?;
        Ok(task)
    }

    /// Next free task id, scanning what exists.
    pub fn next_id(&self) -> Result<String, StoreError> {
        let mut highest: u32 = 0;
        for task in self.list()? {
            if let Some(number) = task.id.strip_prefix("TSK")
                && let Ok(number) = number.parse::<u32>()
            {
                highest = highest.max(number);
            }
        }
        Ok(format!("TSK{:02}", highest + 1))
    }
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
