use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use coding_crew_core::store::{DATA_DIR, WORK_DIR};
use coding_crew_core::{Store, StoreError};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("coding-crew-store-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch folder");
    dir
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
