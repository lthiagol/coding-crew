//! `project init|show|check`, and how a call finds its project.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use coding_crew_core::store::{DATA_DIR, WORK_DIR};
use coding_crew_core::task::SCHEMA_VERSION;
use coding_crew_core::{Activity, Item, Runtime, Store, Task};
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::identity::{ENV_PROJECT, Resolved, resolve};

/// A located project: its root, its `coding-crew/` folder and its store.
pub struct Project {
    pub root: PathBuf,
    pub data: PathBuf,
    pub store: Store,
}

/// The project root from `--project` or `TALITA_PROJECT`, if given.
pub fn given_root(flag: Option<&Path>) -> Option<PathBuf> {
    resolve(flag.and_then(Path::to_str), ENV_PROJECT).map(|part| PathBuf::from(part.value))
}

/// Find the project: the given root, which must hold `coding-crew/`, or the
/// nearest parent of the current directory that does (ADR 0010).
pub fn locate(flag: Option<&Path>) -> Result<Project, Failure> {
    let data = match given_root(flag) {
        Some(root) => {
            let data = root.join(DATA_DIR);
            if !data.is_dir() {
                return Err(Failure::new(
                    Code::ProjectNotFound,
                    format!("{} holds no {DATA_DIR} folder", root.display()),
                )
                .remedy(format!("talita project init --path {}", root.display())));
            }
            data
        }
        None => {
            let cwd = std::env::current_dir().map_err(Failure::internal)?;
            let store = Store::discover(&cwd).map_err(Failure::from)?;
            store
                .root()
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| Failure::internal("the work folder has no parent"))?
        }
    };
    let root = data.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok(Project {
        store: Store::new(data.join(WORK_DIR)),
        root,
        data,
    })
}

/// `project init`: creates `<dir>/coding-crew/work/`. The directory is
/// `--path`, else `--project`/`TALITA_PROJECT`, else the current directory.
pub fn init(
    path: Option<&Path>,
    project_flag: Option<&Path>,
    identity: &Resolved,
) -> Result<Done, Failure> {
    identity.writer()?;
    let root = match path
        .map(Path::to_path_buf)
        .or_else(|| given_root(project_flag))
    {
        Some(root) => root,
        None => std::env::current_dir().map_err(Failure::internal)?,
    };
    let data = root.join(DATA_DIR);
    if data.exists() {
        return Err(Failure::new(
            Code::ProjectExists,
            format!("{} already exists", data.display()),
        ));
    }
    let work = data.join(WORK_DIR);
    fs::create_dir_all(&work)
        .map_err(|e| Failure::internal(format!("creating {}: {e}", work.display())))?;
    let root = fs::canonicalize(&root).unwrap_or(root);
    Ok(Done {
        data: json!({ "project": root, "data": root.join(DATA_DIR) }),
        wrote: vec![format!("{WORK_DIR}/")],
        journal: Vec::new(),
    })
}

/// `project show`: paths, counts, journal and runtime presence, schema.
pub fn show(project: &Project) -> Result<Done, Failure> {
    let tasks = project.store.list()?;
    let items = project.store.list_items()?;
    let journal = project.store.activity_path();
    let runtime = project.store.runtime_path();
    Ok(Done::data(json!({
        "project": project.root,
        "data": project.data,
        "work": project.store.root(),
        "journal": { "path": journal, "exists": journal.is_file() },
        "runtime": { "path": runtime, "exists": runtime.is_file() },
        "schema_version": SCHEMA_VERSION,
        "tasks": counts(tasks.iter().map(|t| (word(&t.status), word(&t.kind)))),
        "items": counts(items.iter().map(|i| (word(&i.status), word(&i.kind)))),
    })))
}

/// `project check`: parses every file and reports what is wrong. Writes
/// nothing; a report with findings is still `ok`.
pub fn check(project: &Project) -> Result<Done, Failure> {
    let work = project.store.root();
    let mut unreadable = Vec::new();
    let mut tasks: Vec<Task> = Vec::new();
    let mut files = 0usize;
    if work.is_dir() {
        let mut paths: Vec<PathBuf> = fs::read_dir(work)
            .map_err(Failure::internal)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        paths.sort();
        for path in paths {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) => {
                    unreadable.push(json!({ "file": rel(&name), "error": e.to_string() }));
                    continue;
                }
            };
            files += 1;
            let parsed = if name.starts_with("TSK") {
                serde_json::from_str::<Task>(&text).map(|t| tasks.push(t))
            } else if name.starts_with("ITM") {
                serde_json::from_str::<Item>(&text).map(|_| ())
            } else {
                continue;
            };
            if let Err(e) = parsed {
                unreadable.push(json!({ "file": rel(&name), "error": e.to_string() }));
            }
        }
    }
    let journal = project.store.activity_path();
    if journal.is_file() {
        files += 1;
        if let Err(e) = Activity::read(&journal) {
            unreadable.push(
                json!({ "file": coding_crew_core::store::ACTIVITY_FILE, "error": e.to_string() }),
            );
        }
    }
    let runtime = project.store.runtime_path();
    if runtime.is_file() {
        files += 1;
        if let Err(e) = Runtime::read(&runtime) {
            unreadable.push(
                json!({ "file": coding_crew_core::store::RUNTIME_FILE, "error": e.to_string() }),
            );
        }
    }

    let duplicate_ids = project.store.duplicate_ids()?;
    let by_id: BTreeMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    let mut family = Vec::new();
    let mut dangling = Vec::new();
    let mut criteria = Vec::new();
    for task in &tasks {
        if let Some(parent) = &task.parent {
            match by_id.get(parent.as_str()) {
                None => family.push(
                    json!({ "task": task.id, "detail": format!("parent {parent} does not exist") }),
                ),
                Some(p) if !p.children.contains(&task.id) => family.push(json!({
                    "task": task.id,
                    "detail": format!("parent {parent} does not list it in children"),
                })),
                Some(_) => {}
            }
        }
        for child in &task.children {
            match by_id.get(child.as_str()) {
                None => family.push(
                    json!({ "task": task.id, "detail": format!("child {child} does not exist") }),
                ),
                Some(c) if c.parent.as_deref() != Some(task.id.as_str()) => family.push(json!({
                    "task": task.id,
                    "detail": format!("child {child} names another parent"),
                })),
                Some(_) => {}
            }
        }
        for on in &task.depends_on {
            if !by_id.contains_key(on.as_str()) {
                dangling.push(json!({ "task": task.id, "on": on }));
            }
        }
        for criterion in &task.acceptance_criteria {
            let derived = criterion.effective_status();
            if criterion.status != derived {
                criteria.push(json!({
                    "task": task.id,
                    "criterion": criterion.id,
                    "stored": word(&criterion.status),
                    "derived": word(&derived),
                }));
            }
        }
    }
    family.sort_by_key(|v| v["task"].as_str().unwrap_or_default().to_string());
    family.dedup();

    let clean = unreadable.is_empty()
        && duplicate_ids.is_empty()
        && family.is_empty()
        && dangling.is_empty()
        && criteria.is_empty();
    Ok(Done::data(json!({
        "clean": clean,
        "files": files,
        "unreadable": unreadable,
        "duplicate_ids": duplicate_ids,
        "family_mismatches": family,
        "dangling_depends_on": dangling,
        "criterion_status_mismatches": criteria,
    })))
}

/// A work file's path relative to `coding-crew/`.
fn rel(name: &str) -> String {
    format!("{WORK_DIR}/{name}")
}

/// The record's word for an enum value.
fn word<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(s)) => s,
        _ => String::new(),
    }
}

/// Totals by status and by kind.
fn counts(pairs: impl Iterator<Item = (String, String)>) -> Value {
    let mut total = 0usize;
    let mut status: BTreeMap<String, usize> = BTreeMap::new();
    let mut kind: BTreeMap<String, usize> = BTreeMap::new();
    for (s, k) in pairs {
        total += 1;
        *status.entry(s).or_default() += 1;
        *kind.entry(k).or_default() += 1;
    }
    json!({ "total": total, "by_status": status, "by_kind": kind })
}
