//! `evidence run|override`: what a criterion's verification shows (ADR 0019).
//! The run executes the frozen verification; the override is the marked
//! bypass. The project lock is held only around the append, never while the
//! command runs.

use std::io;
use std::time::Duration;

use coding_crew_core::bounded::{self, Answer};
use coding_crew_core::{CriterionStatus, Evidence, Task};
use serde_json::json;

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::parts::{work_runner, working_runner};
use crate::task::word;
use crate::write::{Call, duration, part_not_found, refuse_cancelled, required_text, text};

/// The time a run gets when the call names none (ADR 0019).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The most a summary may say, in characters.
const SUMMARY_CHARS: usize = 200;

/// The lines of the command's output a run returns.
const OUTPUT_LINES: usize = 40;

/// `evidence run`: run the criterion's verification with `sh -c`, from the
/// project root, and record what it returned. On timeout the entry is still
/// written, with no exit code, and the call refuses (`evidence.timeout`).
pub fn run(
    call: &Call,
    id: &str,
    cid: &str,
    timeout: Option<&str>,
    summary: Option<&str>,
) -> Result<Done, Failure> {
    let bound = timeout
        .map(|value| duration("--timeout", value))
        .transpose()?
        .unwrap_or(DEFAULT_TIMEOUT);
    let summary = summary.map(|value| text("--summary", value)).transpose()?;

    // The lock is not held while the command runs (ADR 0019): read the
    // verification, run it, then take the lock for the append.
    let task = call.project.store.load(id)?;
    refuse_cancelled(&task)?;
    let command = plan(call, &task, id, cid)?;
    match bounded::run_in(
        Some(call.project.root.as_path()),
        "sh",
        ["-c", command.as_str()],
        bound,
    ) {
        Ok(answer) => {
            let exit_code = answer.status.code();
            let summary = cut(&summary.unwrap_or_else(|| last_spoken(&answer)));
            let silent = exit_code == Some(0) && summary.trim().is_empty();
            let mut done = record(call, id, cid, &command, exit_code, summary, tail(&answer))?;
            if silent {
                done.data["warnings"] = json!([{
                    "code": "evidence.summary_empty",
                    "message": "exit 0 with no summary is a failure: pass --summary <what it showed>",
                }]);
            }
            Ok(done)
        }
        Err(error) if error.kind() == io::ErrorKind::TimedOut => {
            let summary = cut(&summary.unwrap_or_default());
            record(call, id, cid, &command, None, summary, Vec::new())?;
            Err(
                Failure::new(Code::EvidenceTimeout, format!("{id} {cid}: {error}")).remedy(
                    format!("talita evidence override {id} {cid} --reason <why>"),
                ),
            )
        }
        Err(error) => Err(Failure::internal(format!("running {command}: {error}"))),
    }
}

/// `evidence override`: the marked bypass. The reason is the summary; the
/// criterion reads `passed`, and raul draws the entry as a manual override.
pub fn manual(call: &Call, id: &str, cid: &str, reason: Option<&str>) -> Result<Done, Failure> {
    let reason = reason
        .map(|value| required_text("--reason", value))
        .transpose()?
        .ok_or_else(|| Failure::new(Code::InputInvalid, "--reason is required"))?;
    call.task(id, |task, actor| {
        work_runner(actor, "evidence override")?;
        working_runner(task, actor, "evidence")?;
        let Some(criterion) = task.criterion(cid) else {
            return Err(part_not_found(task, cid));
        };
        if criterion.deferral.is_some() {
            return Err(deferred(id, cid));
        }
        // The bypass is for a criterion no command can prove. A criterion whose
        // latest command run passed is proven, whatever bypass came after it.
        let proven = criterion
            .evidence
            .iter()
            .rev()
            .find(|e| !e.manual)
            .is_some_and(|e| e.outcome() == CriterionStatus::Passed);
        if proven && criterion.effective_status() == CriterionStatus::Passed {
            return Err(Failure::new(
                Code::StatusWrong,
                format!("{cid} is already proven by its command; there is nothing to override"),
            )
            .remedy(format!("talita task show {id}")));
        }
        let criterion = task.criterion_mut(cid).expect("checked above");
        criterion.push_evidence(Evidence {
            command: String::new(),
            exit_code: Some(0),
            summary: reason,
            at: coding_crew_core::now_rfc3339(),
            by: actor.id.clone(),
            session: actor.session.clone(),
            manual: true,
        });
        let criterion = task.criterion(cid).expect("just written");
        Ok(json!({
            "criterion": criterion,
            "exit_code": 0,
            "status": word(&criterion.effective_status()),
        }))
    })
}

/// What a run must find before the command starts: the revision the caller
/// expected, the working runner, the criterion, no deferral, and a
/// verification to run. Returns the command exactly as stored (ADR 0019).
fn plan(call: &Call, task: &Task, id: &str, cid: &str) -> Result<String, Failure> {
    expect_revision(call, task, id)?;
    work_runner(&call.actor, "evidence run")?;
    working_runner(task, &call.actor, "evidence")?;
    let Some(criterion) = task.criterion(cid) else {
        return Err(part_not_found(task, cid));
    };
    if criterion.deferral.is_some() {
        return Err(deferred(id, cid));
    }
    if criterion.verification.trim().is_empty() {
        return Err(Failure::new(
            Code::InputInvalid,
            format!("{id} {cid} has no verification to run"),
        )
        .remedy(format!(
            "talita criterion revise {id} {cid} --verification <cmd>"
        )));
    }
    Ok(criterion.verification.clone())
}

/// A stale `--expect-revision` refuses before the command runs: the caller
/// pays for a file read, not a run that is then thrown away.
fn expect_revision(call: &Call, task: &Task, id: &str) -> Result<(), Failure> {
    if let Some(expected) = call.expect_revision
        && expected != task.revision
    {
        return Err(Failure::new(
            Code::ConflictRevision,
            format!(
                "task revision {expected} is stale; the file holds {}",
                task.revision
            ),
        )
        .remedy(format!("talita task show {id}")));
    }
    Ok(())
}

/// Append the entry under the lock, after the run, and check again that the
/// caller is still the working runner and the criterion still takes evidence.
fn record(
    call: &Call,
    id: &str,
    cid: &str,
    command: &str,
    exit_code: Option<i32>,
    summary: String,
    output: Vec<String>,
) -> Result<Done, Failure> {
    call.task(id, |task, actor| {
        work_runner(actor, "evidence run")?;
        working_runner(task, actor, "evidence")?;
        let Some(criterion) = task.criterion(cid) else {
            return Err(part_not_found(task, cid));
        };
        if criterion.deferral.is_some() {
            return Err(deferred(id, cid));
        }
        let criterion = task.criterion_mut(cid).expect("checked above");
        criterion.push_evidence(Evidence {
            command: command.to_string(),
            exit_code,
            summary,
            at: coding_crew_core::now_rfc3339(),
            by: actor.id.clone(),
            session: actor.session.clone(),
            manual: false,
        });
        let criterion = task.criterion(cid).expect("just written");
        Ok(json!({
            "criterion": criterion,
            "exit_code": exit_code,
            "status": word(&criterion.effective_status()),
            "output": output,
        }))
    })
}

fn deferred(id: &str, cid: &str) -> Failure {
    Failure::new(
        Code::CriterionDeferred,
        format!("{cid} is deferred: the follow-up carries it"),
    )
    .remedy(format!("talita criterion undefer {id} {cid}"))
}

/// The summary a run gives itself: the last non-empty line of stdout, or of
/// stderr when stdout has none (ADR 0019).
fn last_spoken(answer: &Answer) -> String {
    last_line(&answer.stdout)
        .or_else(|| last_line(&answer.stderr))
        .unwrap_or_default()
}

fn last_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// A summary is at most 200 characters, cut where a character ends.
fn cut(text: &str) -> String {
    text.chars().take(SUMMARY_CHARS).collect()
}

/// The last lines of a run's output: stdout's, then stderr's.
fn tail(answer: &Answer) -> Vec<String> {
    let mut lines: Vec<String> = String::from_utf8_lossy(&answer.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    lines.extend(
        String::from_utf8_lossy(&answer.stderr)
            .lines()
            .map(str::to_string),
    );
    let keep = lines.len().saturating_sub(OUTPUT_LINES);
    lines.split_off(keep)
}
