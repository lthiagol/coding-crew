//! `run start|stop|show|refresh`, `crew join|leave|assign|idle|stall` and
//! `queue add|drop|show`: the runtime record's written fields, and the
//! derived fields every talita write recomputes (ADR 0018).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use coding_crew_core::claim::claim_failures;
use coding_crew_core::runtime::EarlierRound;
use coding_crew_core::store::RUNTIME_FILE;
use coding_crew_core::{
    Actor, ActorKind, AgentState, Budget, CrewMember, Finding, FindingStatus, FlowRole,
    ProjectLock, Role, RoundKind, Runtime, RuntimeError, RuntimeRound, Severities, Severity, Stamp,
    Status, Store, Task,
};
use serde_json::json;

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::project::Project;
use crate::write::{Call, required_text};

// ---- the write path

/// The crew-row side effect a task verb leaves in the record (ADR 0018).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffect {
    /// The verb leaves no row certain.
    None,
    /// `claim take`: the actor's row holds the task, `since` now, executing.
    Take,
    /// `task finish`, `task re-review`, an external `round file`: the actor's
    /// row is idle and keeps its hold.
    Idle,
    /// `claim release`, `task cancel`: every row holding the task released.
    Release,
    /// `task pass-review`, `task escalate`: every row holding the task
    /// released and idle.
    ReleaseIdle,
}

/// Recompute the record after a task write and write it, in the same locked
/// call. `Ok(false)` when the project has no record: nothing is written. A
/// record talita cannot read refuses the write (ADR 0018).
pub fn after_task_write(
    lock: &ProjectLock<'_>,
    effect: SideEffect,
    actor: &Actor,
    task: &str,
    now: &str,
) -> Result<bool, Failure> {
    let Some(mut record) = read(lock)? else {
        return Ok(false);
    };
    apply(&mut record, effect, actor, task, now);
    write(lock, record, now)?;
    Ok(true)
}

/// Recompute the record and write it, with no crew-row side effect. The item
/// verbs' write path.
pub fn refreshed(lock: &ProjectLock<'_>) -> Result<bool, Failure> {
    let Some(record) = read(lock)? else {
        return Ok(false);
    };
    write(lock, record, &coding_crew_core::now_rfc3339())?;
    Ok(true)
}

/// The record with the last write stamped and its derived fields recomputed
/// from the task files, saved. Every write of the record goes through here, so
/// the stamp and the write use one instant.
fn write(lock: &ProjectLock<'_>, mut record: Runtime, now: &str) -> Result<Runtime, Failure> {
    record.updated_at = Some(now.to_string());
    let record = derived(record, &lock.list()?, now);
    lock.write_runtime(&record)?;
    Ok(record)
}

/// Apply a task verb's side effect to the crew rows (ADR 0018). A row is the
/// one whose `actor` is the acting actor; a verb by an actor with no row
/// changes nothing.
fn apply(record: &mut Runtime, effect: SideEffect, actor: &Actor, task: &str, now: &str) {
    match effect {
        SideEffect::None => {}
        SideEffect::Take => {
            if let Some(member) = record
                .crew
                .iter_mut()
                .find(|member| member.actor.as_deref() == Some(actor.id.as_str()))
            {
                member.holds = Some(task.to_string());
                member.since = Some(now.to_string());
                member.state = AgentState::Executing;
            }
        }
        SideEffect::Idle => {
            if let Some(member) = record
                .crew
                .iter_mut()
                .find(|member| member.actor.as_deref() == Some(actor.id.as_str()))
            {
                member.state = AgentState::Idle;
            }
        }
        SideEffect::Release | SideEffect::ReleaseIdle => {
            for member in record
                .crew
                .iter_mut()
                .filter(|member| member.holds.as_deref() == Some(task))
            {
                member.holds = None;
                member.since = None;
                if effect == SideEffect::ReleaseIdle {
                    member.state = AgentState::Idle;
                }
            }
        }
    }
}

/// The record with its derived fields recomputed from the task files (ADR
/// 0018). The written fields are untouched.
fn derived(mut record: Runtime, tasks: &[Task], now: &str) -> Runtime {
    let by_id: BTreeMap<&str, &Task> = tasks.iter().map(|task| (task.id.as_str(), task)).collect();
    let done = |dependency: &str| {
        by_id
            .get(dependency)
            .is_some_and(|task| task.status == Status::Done)
    };
    for item in &mut record.queue {
        match by_id.get(item.id.as_str()) {
            Some(task) => {
                item.status = task.status;
                item.claimable = claim_failures(task, Some(&done), now).is_empty();
            }
            None => item.claimable = false,
        }
    }

    let in_flight = record
        .crew
        .iter()
        .filter_map(|member| member.holds.as_deref())
        .find_map(|id| by_id.get(id).copied());
    let Some(task) = in_flight else {
        record.round = None;
        record.budget = Budget::default();
        record.findings = 0;
        record.severities = None;
        return record;
    };

    let external = task.status == Status::Remediating
        || record.crew.iter().any(|member| {
            member.role == FlowRole::Reviewer
                && member.state == AgentState::Reviewing
                && member.holds.as_deref() == Some(task.id.as_str())
        });
    let filed = task.external_rounds() as u32;
    let number = if external { filed + 1 } else { 0 };
    let earlier: Vec<EarlierRound> = task
        .review
        .rounds
        .iter()
        .filter(|round| round.kind == RoundKind::External)
        .map(|round| EarlierRound {
            number: round.n,
            raised: round.findings.len() as u32,
        })
        .collect();
    record.round = Some(RuntimeRound {
        task: task.id.clone(),
        stage: if external {
            RoundKind::External
        } else {
            RoundKind::SelfReview
        },
        number,
        earlier,
    });
    record.budget = Budget {
        spent: if external { number } else { filed },
        total: task.review.max_rounds,
    };
    let open: Vec<&Finding> = task
        .review
        .findings
        .iter()
        .filter(|finding| finding.status == FindingStatus::Open)
        .collect();
    record.findings = open.len() as u32;
    record.severities = Some(Severities {
        high: open.iter().filter(|f| f.severity == Severity::High).count() as u32,
        medium: open
            .iter()
            .filter(|f| f.severity == Severity::Medium)
            .count() as u32,
        low: open.iter().filter(|f| f.severity == Severity::Low).count() as u32,
    });
    record
}

// ---- run

/// `run start`: a new record for a new run.
pub fn start(call: &Call, ids: &[String]) -> Result<Done, Failure> {
    orchestrator(&call.actor, "run start")?;
    let lock = call.project.store.lock()?;
    if let Some(record) = read(&lock)?
        && record.stamp.runs
    {
        return Err(Failure::new(
            Code::RunActive,
            "the project holds a run that is still active",
        )
        .remedy("talita run stop --why <t>"));
    }
    let mut queue = Vec::new();
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.as_str()) {
            continue;
        }
        let task = lock.load(id)?;
        queue.push(coding_crew_core::QueueItem {
            id: task.id,
            status: task.status,
            claimable: false,
        });
    }
    let now = coding_crew_core::now_rfc3339();
    let record = Runtime {
        started_at: now.clone(),
        updated_at: None,
        crew: Vec::new(),
        queue,
        round: None,
        budget: Budget::default(),
        findings: 0,
        severities: None,
        stamp: Stamp {
            runs: true,
            why: None,
        },
    };
    Ok(done_record(write(&lock, record, &now)?))
}

/// `run stop`: the stamp stops the run; every row is idle and keeps its hold.
pub fn stop(call: &Call, why: &str) -> Result<Done, Failure> {
    orchestrator(&call.actor, "run stop")?;
    let why = required_text("--why", why)?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    record.stamp = Stamp {
        runs: false,
        why: Some(why),
    };
    for member in &mut record.crew {
        member.state = AgentState::Idle;
    }
    let now = coding_crew_core::now_rfc3339();
    Ok(done_record(write(&lock, record, &now)?))
}

/// `run show`: the record, and whether its last write has gone stale.
pub fn show(project: &Project) -> Result<Done, Failure> {
    let record = read_path(&project.store.runtime_path())?.ok_or_else(run_absent)?;
    let stale = stale(&record, &project.store);
    Ok(Done::data(json!({ "runtime": record, "stale": stale })))
}

/// `run refresh`: recompute every derived field and write the record. The
/// repair after a crash.
pub fn refresh(call: &Call) -> Result<Done, Failure> {
    orchestrator(&call.actor, "run refresh")?;
    let lock = call.project.store.lock()?;
    let record = require(&lock)?;
    let now = coding_crew_core::now_rfc3339();
    Ok(done_record(write(&lock, record, &now)?))
}

/// The record is stale when its last write is older than the longest lease
/// the crew holds (ADR 0018). A record that names no last write, or holds no
/// lease, is never stale. A last write that does not parse is stale: a corrupt
/// record cannot hold a run alive, as a malformed lease reads expired.
fn stale(record: &Runtime, store: &Store) -> bool {
    let Some(lease) = longest_lease(record, store) else {
        return false;
    };
    let Some(written) = record.updated_at.as_deref() else {
        return false;
    };
    let Some(updated) = coding_crew_core::parse_rfc3339(written) else {
        return true;
    };
    let Some(now) = coding_crew_core::parse_rfc3339(&coding_crew_core::now_rfc3339()) else {
        return false;
    };
    u64::try_from(now - updated).is_ok_and(|age| Duration::from_secs(age) > lease)
}

/// The longest lease the crew holds: a held task's claim's `lease_expires_at`
/// minus its `at`. A lease that has already expired still counts; a row with
/// no claim, or one that does not read, has no length.
fn longest_lease(record: &Runtime, store: &Store) -> Option<Duration> {
    record
        .crew
        .iter()
        .filter_map(|member| member.holds.as_deref())
        .filter_map(|id| store.load(id).ok())
        .filter_map(|task| task.claim)
        .filter_map(|claim| {
            let start = coding_crew_core::parse_rfc3339(&claim.at)?;
            let end = coding_crew_core::parse_rfc3339(&claim.lease_expires_at)?;
            u64::try_from(end - start).ok().map(Duration::from_secs)
        })
        .max()
}

// ---- crew

/// `crew join`: a new row, idle.
pub fn join(call: &Call, actor: &str, role: &str, model: &str) -> Result<Done, Failure> {
    orchestrator(&call.actor, "crew join")?;
    let role = parse_role(role)?;
    let model = required_text("--model", model)?;
    let actor = required_text("<actor>", actor)?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    if record
        .crew
        .iter()
        .any(|member| member.actor.as_deref() == Some(actor.as_str()))
    {
        return Err(
            Failure::new(Code::CrewExists, format!("{actor} is already in the crew"))
                .remedy(format!("talita crew leave {actor}")),
        );
    }
    record.crew.push(CrewMember {
        role,
        model,
        actor: Some(actor),
        holds: None,
        since: None,
        state: AgentState::Idle,
    });
    let now = coding_crew_core::now_rfc3339();
    Ok(done_crew(write(&lock, record, &now)?))
}

/// `crew leave`: removes the row.
pub fn leave(call: &Call, actor: &str) -> Result<Done, Failure> {
    orchestrator(&call.actor, "crew leave")?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    let at = record
        .crew
        .iter()
        .position(|member| member.actor.as_deref() == Some(actor))
        .ok_or_else(|| crew_unknown(actor))?;
    record.crew.remove(at);
    let now = coding_crew_core::now_rfc3339();
    Ok(done_crew(write(&lock, record, &now)?))
}

/// `crew assign`: the row holds the task, `since` now, and its role's state.
pub fn assign(call: &Call, actor: &str, id: &str) -> Result<Done, Failure> {
    orchestrator(&call.actor, "crew assign")?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    let task = lock.load(id)?;
    let now = coding_crew_core::now_rfc3339();
    let member = record
        .crew
        .iter_mut()
        .find(|member| member.actor.as_deref() == Some(actor))
        .ok_or_else(|| crew_unknown(actor))?;
    member.holds = Some(task.id);
    member.since = Some(now.clone());
    member.state = match member.role {
        FlowRole::Reviewer => AgentState::Reviewing,
        _ => AgentState::Executing,
    };
    Ok(done_crew(write(&lock, record, &now)?))
}

/// `crew idle`: idle, keeping the hold.
pub fn idle(call: &Call, actor: &str) -> Result<Done, Failure> {
    set_state(call, actor, "crew idle", AgentState::Idle)
}

/// `crew stall`: the agent cannot go on.
pub fn stall(call: &Call, actor: &str) -> Result<Done, Failure> {
    set_state(call, actor, "crew stall", AgentState::Blocked)
}

fn set_state(call: &Call, actor: &str, verb: &str, state: AgentState) -> Result<Done, Failure> {
    orchestrator(&call.actor, verb)?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    let member = record
        .crew
        .iter_mut()
        .find(|member| member.actor.as_deref() == Some(actor))
        .ok_or_else(|| crew_unknown(actor))?;
    member.state = state;
    let now = coding_crew_core::now_rfc3339();
    Ok(done_crew(write(&lock, record, &now)?))
}

// ---- queue

/// `queue add`: append the ids, or insert them before one. An id the queue
/// already holds is not added twice.
pub fn queue_add(call: &Call, ids: &[String], before: Option<&str>) -> Result<Done, Failure> {
    orchestrator(&call.actor, "queue add")?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    let mut tasks = BTreeMap::new();
    for id in ids {
        tasks.insert(id.as_str(), lock.load(id)?);
    }
    let at = match before {
        Some(before) => Some(
            record
                .queue
                .iter()
                .position(|item| item.id == before)
                .ok_or_else(|| {
                    Failure::new(
                        Code::TaskNotFound,
                        format!("the queue does not hold {before}"),
                    )
                    .remedy("talita queue show")
                })?,
        ),
        None => None,
    };
    let mut fresh = Vec::new();
    for id in ids {
        if record.queue.iter().any(|item| &item.id == id)
            || fresh
                .iter()
                .any(|item: &coding_crew_core::QueueItem| &item.id == id)
        {
            continue;
        }
        let task = &tasks[id.as_str()];
        fresh.push(coding_crew_core::QueueItem {
            id: id.clone(),
            status: task.status,
            claimable: false,
        });
    }
    match at {
        Some(at) => {
            for (offset, item) in fresh.into_iter().enumerate() {
                record.queue.insert(at + offset, item);
            }
        }
        None => record.queue.extend(fresh),
    }
    let now = coding_crew_core::now_rfc3339();
    Ok(done_queue(write(&lock, record, &now)?))
}

/// `queue drop`: removes the ids the queue holds; the rest changes nothing.
pub fn queue_drop(call: &Call, ids: &[String]) -> Result<Done, Failure> {
    orchestrator(&call.actor, "queue drop")?;
    let lock = call.project.store.lock()?;
    let mut record = require(&lock)?;
    record.queue.retain(|item| !ids.contains(&item.id));
    let now = coding_crew_core::now_rfc3339();
    Ok(done_queue(write(&lock, record, &now)?))
}

/// `queue show`: the queue as the last write left it. A project with no
/// record has an empty queue, not a broken one.
pub fn queue_show(project: &Project) -> Result<Done, Failure> {
    let queue = read_path(&project.store.runtime_path())?
        .map(|record| record.queue)
        .unwrap_or_default();
    Ok(Done::data(json!({ "queue": queue })))
}

// ---- the record

fn read(lock: &ProjectLock<'_>) -> Result<Option<Runtime>, Failure> {
    read_path(&lock.runtime_path())
}

/// A record that does not parse refuses the write; a project without one has
/// no runtime, not a broken one (ADR 0018).
fn read_path(path: &Path) -> Result<Option<Runtime>, Failure> {
    match Runtime::read(path) {
        Ok(record) => Ok(Some(record)),
        Err(RuntimeError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Failure::new(Code::StoreUnreadable, error.to_string())),
    }
}

fn require(lock: &ProjectLock<'_>) -> Result<Runtime, Failure> {
    read(lock)?.ok_or_else(run_absent)
}

fn done_record(record: Runtime) -> Done {
    Done {
        data: json!({ "runtime": record }),
        wrote: vec![RUNTIME_FILE.to_string()],
        journal: Vec::new(),
    }
}

fn done_crew(record: Runtime) -> Done {
    Done {
        data: json!({ "crew": record.crew }),
        wrote: vec![RUNTIME_FILE.to_string()],
        journal: Vec::new(),
    }
}

fn done_queue(record: Runtime) -> Done {
    Done {
        data: json!({ "queue": record.queue }),
        wrote: vec![RUNTIME_FILE.to_string()],
        journal: Vec::new(),
    }
}

/// The flow verbs are the orchestrator's; a human is refused as an agent is
/// not the actor (ADR 0018).
fn orchestrator(actor: &Actor, verb: &str) -> Result<(), Failure> {
    if actor.kind != ActorKind::Agent {
        return Err(Failure::new(
            Code::ActorNotAgent,
            format!("{verb} is the orchestrator's act"),
        )
        .remedy("act as --role orchestrator"));
    }
    if actor.role != Some(Role::Orchestrator) {
        return Err(Failure::new(
            Code::RoleWrong,
            format!("{verb} needs the orchestrator role"),
        )
        .remedy("act as --role orchestrator"));
    }
    Ok(())
}

fn parse_role(value: &str) -> Result<FlowRole, Failure> {
    Ok(match value {
        "builder" => FlowRole::Builder,
        "reviewer" => FlowRole::Reviewer,
        "orchestrator" => FlowRole::Orchestrator,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("role `{other}` is not one of builder, reviewer, orchestrator"),
            ));
        }
    })
}

fn run_absent() -> Failure {
    Failure::new(Code::RunAbsent, "the project has no runtime record")
        .remedy("talita run start --task <TSK>")
}

fn crew_unknown(actor: &str) -> Failure {
    Failure::new(Code::CrewUnknown, format!("{actor} is not in the crew")).remedy(format!(
        "talita crew join {actor} --role <role> --model <model>"
    ))
}
