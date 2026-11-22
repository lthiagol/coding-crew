//! The task → markdown projection.
//!
//! The record stays JSON; this is its human-facing view, as
//! [ADR 0009](../../../docs/decisions/0009-markdown-projection.md) decides.
//!
//! Free text — intent, scope, criterion descriptions, step prose, notes,
//! findings, questions and comments — is markdown in its own right. Values
//! that must not be interpreted — ids, statuses, kinds, timestamps, actors and
//! sessions, file paths and verification commands — are escaped or rendered as
//! code.
//!
//! The projection is pure: the second slice supplies the project context for
//! reverse dependencies and children, and nothing here reads a clock, a
//! terminal or the filesystem.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry;

use crate::lifecycle::{Overlay, Overlays};
use crate::task::{
    Claim, Finding, FindingStatus, Focus, Priority, Question, QuestionStatus, Round, RoundKind,
    Severity, StepStatus, Task, Verdict,
};

/// What an empty section or field says.
const EMPTY: &str = "_none_";

/// Project a task to one markdown document.
///
/// The sections are fixed and ordered: Header, Intent, Scope, Acceptance
/// criteria, Steps, Review, Questions, Notes, Dependencies, Children. Every
/// section is present, even when empty.
///
/// `tasks` is the project context. It is read only to derive what waits on
/// `task` and the progress of its children; neither input is mutated.
///
/// When `tasks` holds more than one record for an id, one canonical record per
/// id is chosen: the one with the smallest serialized form. The choice is by
/// content and independent of slice order, and the derived lists read only
/// those canonical records.
pub fn task_to_markdown(task: &Task, tasks: &[Task]) -> String {
    let canonical = canonical_records(tasks);
    let mut out = String::new();

    header(&mut out, task);
    intent(&mut out, task);
    scope(&mut out, task);
    criteria(&mut out, task);
    steps(&mut out, task);
    review(&mut out, task);
    questions(&mut out, task);
    notes(&mut out, task);
    dependencies(&mut out, task, &canonical);
    children(&mut out, task, &canonical);

    out
}

fn header(out: &mut String, task: &Task) {
    out.push_str("# Header\n\n");
    out.push_str("| field | value |\n|---|---|\n");
    row(out, "id", &code(&task.id));
    row(out, "title", &escape_literal(&task.title));
    row(out, "status", &code(task.status.as_str()));
    row(out, "kind", &code(task.kind.as_str()));
    row(out, "overlays", &overlays_text(&task.overlays));
    row(
        out,
        "claim",
        &task
            .claim
            .as_ref()
            .map(claim_text)
            .unwrap_or_else(|| EMPTY.to_string()),
    );
    row(out, "priority", &code(priority_str(task.meta.priority)));
    row(out, "effort", &code(task.meta.effort.as_str()));
    row(out, "tags", &codes(&task.meta.tags));
}

fn intent(out: &mut String, task: &Task) {
    out.push_str("\n# Intent\n\n");
    out.push_str("**Outcome**\n\n");
    prose_block(out, &task.intent.outcome);
    out.push_str("\n**Problem**\n\n");
    prose_block(out, &task.intent.problem);
}

fn scope(out: &mut String, task: &Task) {
    out.push_str("\n# Scope\n\n");
    scope_list(out, "In scope", &task.scope.in_scope);
    out.push('\n');
    scope_list(out, "Out of scope", &task.scope.out_of_scope);
}

fn criteria(out: &mut String, task: &Task) {
    out.push_str("\n# Acceptance criteria\n\n");
    if task.acceptance_criteria.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
        return;
    }

    out.push_str(
        "| id | description | verification | status | evidence | deferral | failure reason |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|\n");
    for criterion in &task.acceptance_criteria {
        // The evidence is quoted below the table: what a command printed is
        // a quotation, and a table cell cannot hold a block.
        let evidence = if criterion.evidence.is_empty() {
            EMPTY.to_string()
        } else {
            "_quoted below_".to_string()
        };
        let deferral = criterion
            .deferral
            .as_ref()
            .map(deferral_text)
            .unwrap_or_else(|| EMPTY.to_string());
        let verification = if is_multiline(&criterion.verification) {
            "_see below_".to_string()
        } else {
            cell(&code(&criterion.verification))
        };

        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            cell(&code(&criterion.id)),
            cell(&criterion.description),
            verification,
            cell(&code(criterion.effective_status().as_str())),
            cell(&evidence),
            cell(&deferral),
            cell(&criterion.failure_reason),
        ));
    }

    for criterion in &task.acceptance_criteria {
        if is_multiline(&criterion.verification) {
            out.push('\n');
            out.push_str(&format!("**Verification for {}**\n\n", code(&criterion.id)));
            out.push_str("---\n\n");
            fenced_block(out, &criterion.verification);
        }

        if !criterion.evidence.is_empty() {
            out.push('\n');
            out.push_str(&format!("**Evidence for {}**\n\n", code(&criterion.id)));
            for (index, evidence) in criterion.evidence.iter().enumerate() {
                if index > 0 {
                    out.push('\n');
                }
                blockquote(out, &evidence_text(evidence));
            }
        }
    }
}

fn steps(out: &mut String, task: &Task) {
    out.push_str("\n# Steps\n\n");
    if task.steps.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
        return;
    }

    for (index, step) in task.steps.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!("## {}\n\n", code(&step.id)));
        fixed_bullet(out, "order", &code(&step.order.to_string()));
        prose_bullet(out, "action", &step.action);
        prose_bullet(out, "done when", &step.done_when);
        fixed_bullet(out, "files", &codes(&step.files));
        fixed_bullet(out, "status", &code(step_status_str(step.status)));
        fixed_bullet(out, "covers", &codes(&step.covers));
        fixed_bullet(out, "depends on", &codes(&step.depends_on));
        prose_bullet(out, "deviation", &step.deviation);
    }
}

fn review(out: &mut String, task: &Task) {
    out.push_str("\n# Review\n\n");
    let verdict = task
        .review
        .verdict
        .map(verdict_str)
        .map(code)
        .unwrap_or_else(|| EMPTY.to_string());
    out.push_str(&format!("- **verdict:** {verdict}\n"));
    out.push_str(&format!(
        "- **budget:** {} external rounds\n",
        code(&format!(
            "{}/{}",
            task.external_rounds(),
            task.review.max_rounds
        ))
    ));

    out.push_str("\n**Rounds**\n\n");
    let mut shown = HashSet::new();
    if task.review.rounds.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
    } else {
        for (index, round) in task.review.rounds.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            round_block(out, round, &task.review.findings, &mut shown);
        }
    }

    // A finding no round names is still a finding: it keeps its own block,
    // after the rounds, at the rounds' own heading level.
    let mut listed = HashSet::new();
    let mut orphans: Vec<&Finding> = Vec::new();
    for finding in &task.review.findings {
        if shown.contains(finding.id.as_str()) || !listed.insert(finding.id.as_str()) {
            continue;
        }
        orphans.push(finding);
    }
    if !orphans.is_empty() {
        out.push_str("\n**Findings not in a round**\n\n");
        for (index, finding) in orphans.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            finding_block(out, finding, 2);
        }
    }
}

fn questions(out: &mut String, task: &Task) {
    out.push_str("\n# Questions\n\n");
    let open: Vec<&Question> = task
        .questions
        .iter()
        .filter(|q| q.status == QuestionStatus::Open)
        .collect();
    let resolved: Vec<&Question> = task
        .questions
        .iter()
        .filter(|q| q.status == QuestionStatus::Resolved)
        .collect();

    out.push_str("**Open**\n\n");
    if open.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
    } else {
        for (index, question) in open.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            question_block(out, question);
        }
    }

    out.push_str("\n**Resolved**\n\n");
    if resolved.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
    } else {
        for (index, question) in resolved.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            question_block(out, question);
        }
    }
}

fn notes(out: &mut String, task: &Task) {
    out.push_str("\n# Notes\n\n");
    if task.notes.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
        return;
    }

    for (index, note) in task.notes.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!("## {}\n\n", code(&note.id)));
        fixed_bullet(out, "author", &code(&note.author));
        fixed_bullet(out, "at", &code(&note.at));
        out.push('\n');
        prose_block(out, &note.body);
    }
}

fn dependencies<'a>(out: &mut String, task: &Task, canonical: &HashMap<&'a str, Canonical<'a>>) {
    out.push_str("\n# Dependencies\n\n");
    out.push_str("**Waits on**\n\n");
    if task.depends_on.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
    } else {
        let mut seen = std::collections::HashSet::new();
        for id in &task.depends_on {
            if !seen.insert(id.as_str()) {
                continue;
            }
            out.push_str(&dependency_line(id, find(canonical, id)));
        }
    }

    out.push_str("\n**Waits on this**\n\n");
    let mut waiting: Vec<&Task> = canonical
        .values()
        .map(|record| record.task)
        .filter(|other| other.id != task.id && other.depends_on.iter().any(|id| id == &task.id))
        .collect();
    waiting.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    if waiting.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
    } else {
        for other in waiting {
            out.push_str(&dependency_line(&other.id, Some(other)));
        }
    }
}

fn children<'a>(out: &mut String, task: &Task, canonical: &HashMap<&'a str, Canonical<'a>>) {
    out.push_str("\n# Children\n\n");
    let mut ids: Vec<&str> = task.children.iter().map(|id| id.as_str()).collect();
    for record in canonical.values() {
        let other = record.task;
        if other.parent.as_deref() == Some(task.id.as_str()) {
            ids.push(other.id.as_str());
        }
    }
    ids.sort_unstable();
    ids.dedup();

    if ids.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
        return;
    }

    for id in &ids {
        match find(canonical, id) {
            Some(child) => {
                let criteria_total = child.acceptance_criteria.len();
                let criteria_passed = child
                    .acceptance_criteria
                    .iter()
                    .filter(|criterion| {
                        criterion.effective_status() == crate::CriterionStatus::Passed
                    })
                    .count();
                let steps_total = child.steps.len();
                let steps_done = child
                    .steps
                    .iter()
                    .filter(|step| step.status == StepStatus::Done)
                    .count();
                let title = escape_literal(&child.title);
                // A subtask is a thing to be done or done: its box says
                // which, the way a task list's box does.
                let checkbox = if child.status == crate::Status::Done {
                    "[x]"
                } else {
                    "[ ]"
                };
                let mut line = format!(
                    "- {checkbox} {} — {}",
                    code(&child.id),
                    code(child.status.as_str())
                );
                if !title.is_empty() {
                    line.push_str(&format!(" — {title}"));
                }
                line.push_str(&format!(
                    " — criteria {} passed, steps {} done\n",
                    code(&format!("{criteria_passed}/{criteria_total}")),
                    code(&format!("{steps_done}/{steps_total}")),
                ));
                out.push_str(&line);
            }
            // The record is not there, so there is no state a box could
            // carry: an unknown child is a bullet, and the line says so.
            None => out.push_str(&format!("- {} — {}\n", code(id), code("unknown"))),
        }
    }
}

fn row(out: &mut String, field: &str, value: &str) {
    out.push_str(&format!("| {field} | {} |\n", cell(value)));
}

fn scope_list(out: &mut String, title: &str, items: &[String]) {
    out.push_str(&format!("**{title}**\n\n"));
    if items.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
        return;
    }
    for item in items {
        let item = item.trim();
        if item.is_empty() {
            out.push_str(&format!("- {EMPTY}\n"));
        } else {
            out.push_str(&format!("- {}\n", item.replace('\n', "\n  ")));
        }
    }
}

fn fixed_bullet(out: &mut String, label: &str, value: &str) {
    out.push_str(&format!("- **{label}:** {}\n", cell(value)));
}

fn prose_bullet(out: &mut String, label: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        out.push_str(&format!("- **{label}:** {EMPTY}\n"));
    } else {
        out.push_str(&format!("- **{label}:** {}\n", value.replace('\n', "\n  ")));
    }
}

fn prose_block(out: &mut String, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        out.push_str(EMPTY);
    } else {
        out.push_str(value);
    }
    out.push('\n');
}

/// Quote a value: every line carries the marker, so a value that holds a
/// newline cannot leave the quotation.
fn blockquote(out: &mut String, value: &str) {
    for line in value.lines() {
        out.push_str("> ");
        out.push_str(line);
        out.push('\n');
    }
}

/// One round, then the findings it raised.
///
/// `shown` carries the findings the earlier rounds already painted: a finding
/// belongs to the first round that claims it, and a later round that names it
/// again says nothing.
fn round_block(out: &mut String, round: &Round, findings: &[Finding], shown: &mut HashSet<String>) {
    out.push_str(&format!(
        "## Round {} — {}\n\n",
        code(&round.n.to_string()),
        code(round_kind_str(round.kind)),
    ));
    fixed_bullet(out, "focus", &code(focus_str(round.focus)));
    fixed_bullet(out, "reviewer", &code(&round.reviewer));
    fixed_bullet(out, "verdict", &code(verdict_str(round.verdict)));
    fixed_bullet(out, "at", &code(&round.at));
    prose_bullet(out, "notes", &round.notes);

    if round.findings.is_empty() {
        fixed_bullet(out, "findings", EMPTY);
    }

    let mut named = HashSet::new();
    let mut missing = Vec::new();
    let mut raised = Vec::new();
    for id in &round.findings {
        if !named.insert(id.as_str()) {
            continue;
        }
        match findings.iter().find(|finding| finding.id == *id) {
            Some(finding) => {
                if shown.insert(finding.id.clone()) {
                    raised.push(finding);
                }
            }
            None => missing.push(id.as_str()),
        }
    }

    for id in missing {
        fixed_bullet(out, "missing finding", &code(id));
    }
    for finding in raised {
        out.push('\n');
        finding_block(out, finding, 3);
    }
}

fn finding_block(out: &mut String, finding: &Finding, level: usize) {
    out.push_str(&"#".repeat(level));
    out.push(' ');
    out.push_str(&code(&finding.id));
    out.push_str("\n\n");
    fixed_bullet(out, "severity", &code(severity_str(finding.severity)));
    fixed_bullet(out, "status", &code(finding_status_str(finding.status)));
    fixed_bullet(out, "created", &code(&finding.created));
    fixed_bullet(out, "resolved", &code(&finding.resolved));
    prose_bullet(out, "description", &finding.description);
    prose_bullet(out, "resolution", &finding.resolution);

    out.push_str("\n**Comments**\n\n");
    if finding.comments.is_empty() {
        out.push_str(EMPTY);
        out.push('\n');
        return;
    }
    for comment in &finding.comments {
        out.push_str(&format!(
            "- {} — {} at {}\n",
            code(&comment.id),
            code(&comment.author),
            code(&comment.at),
        ));
        let body = comment.body.trim();
        if body.is_empty() {
            out.push_str(&format!("  {EMPTY}\n"));
        } else {
            out.push_str(&format!("  {}\n", body.replace('\n', "\n  ")));
        }
    }
}

fn question_block(out: &mut String, question: &Question) {
    out.push_str(&format!("## {}\n\n", code(&question.id)));
    prose_bullet(out, "question", &question.question);
    fixed_bullet(out, "raised by", &code(&question.raised_by));
    fixed_bullet(out, "raised at", &code(&question.raised_at));
    if question.status == QuestionStatus::Resolved {
        prose_bullet(out, "answer", &question.answer);
        fixed_bullet(out, "resolved by", &code(&question.resolved_by));
        fixed_bullet(out, "resolved at", &code(&question.resolved_at));
    }
}

fn dependency_line(id: &str, task: Option<&Task>) -> String {
    // A dependency is a reference to another record, so its id is a link.
    let reference = link(id);
    match task {
        Some(task) => {
            let title = escape_literal(&task.title);
            if title.is_empty() {
                format!("- {reference} — {}\n", code(task.status.as_str()))
            } else {
                format!("- {reference} — {} — {title}\n", code(task.status.as_str()))
            }
        }
        None => format!("- {reference} — {}\n", code("unknown")),
    }
}

/// One canonical record per id, with the serialized form that chose it.
struct Canonical<'a> {
    task: &'a Task,
    key: String,
}

/// Resolve the context to one canonical record per id.
///
/// Each record is serialized once. The record with the smallest serialized
/// form wins, so the choice is by content, not by slice order. Identical
/// records share a key, so the choice stays deterministic either way. The map
/// borrows the records.
fn canonical_records<'a>(tasks: &'a [Task]) -> HashMap<&'a str, Canonical<'a>> {
    let mut records: HashMap<&'a str, Canonical<'a>> = HashMap::with_capacity(tasks.len());
    for task in tasks {
        let key = serde_json::to_string(task).expect("a task serializes: it has no map or float");
        match records.entry(task.id.as_str()) {
            Entry::Vacant(slot) => {
                slot.insert(Canonical { task, key });
            }
            Entry::Occupied(mut slot) => {
                if key < slot.get().key {
                    slot.insert(Canonical { task, key });
                }
            }
        }
    }
    records
}

fn find<'a>(canonical: &HashMap<&'a str, Canonical<'a>>, id: &str) -> Option<&'a Task> {
    canonical.get(id).map(|record| record.task)
}

fn evidence_text(evidence: &crate::task::Evidence) -> String {
    let mut text = code(&evidence.command);
    text.push_str(" — exit ");
    text.push_str(&match evidence.exit_code {
        Some(exit_code) => code_owned(&exit_code.to_string()),
        None => EMPTY.to_string(),
    });
    if !evidence.summary.trim().is_empty() {
        text.push_str(" — ");
        // The summary is a fixed field of the record: what the command
        // printed. It is quoted, so it is a literal, not prose. Escape it
        // line by line, so a line that would open a block stays inside the
        // quotation, and keep the line breaks.
        text.push_str(&escape_lines(&evidence.summary));
    }
    if !evidence.by.is_empty() {
        text.push_str(" — by ");
        text.push_str(&code(&evidence.by));
    }
    if let Some(session) = &evidence.session
        && !session.is_empty()
    {
        text.push_str(" in ");
        text.push_str(&code(session));
    }
    if !evidence.at.is_empty() {
        text.push_str(" at ");
        text.push_str(&code(&evidence.at));
    }
    text
}

fn deferral_text(deferral: &crate::task::Deferral) -> String {
    let mut text = deferral.reason.trim().to_string();
    if text.is_empty() {
        text.push_str(EMPTY);
    }
    if !deferral.follow_up.is_empty() {
        text.push_str(" — follow up: ");
        text.push_str(&code(&deferral.follow_up));
    }
    text
}

fn overlays_text(overlays: &Overlays) -> String {
    let mut parts = Vec::new();
    if let Some(overlay) = &overlays.blocked {
        parts.push(overlay_text("blocked", overlay));
    }
    if let Some(overlay) = &overlays.deferred {
        parts.push(overlay_text("deferred", overlay));
    }
    if let Some(overlay) = &overlays.cancelled {
        parts.push(overlay_text("cancelled", overlay));
    }
    if parts.is_empty() {
        EMPTY.to_string()
    } else {
        parts.join("<br>")
    }
}

fn overlay_text(kind: &str, overlay: &Overlay) -> String {
    let reason = overlay.reason.trim();
    let mut text = format!(
        "{}: {}",
        code(kind),
        if reason.is_empty() { EMPTY } else { reason }
    );
    if !overlay.by.is_empty() {
        text.push_str(&format!(" — by {}", code(&overlay.by)));
    }
    if !overlay.at.is_empty() {
        text.push_str(&format!(" at {}", code(&overlay.at)));
    }
    text
}

fn claim_text(claim: &Claim) -> String {
    let mut text = format!("by {}", code(&claim.by));
    if let Some(session) = &claim.session
        && !session.is_empty()
    {
        text.push_str(&format!(" in {}", code(session)));
    }
    if claim.at.is_empty() {
        text.push_str(" at _none_");
    } else {
        text.push_str(&format!(" at {}", code(&claim.at)));
    }
    if claim.lease_expires_at.is_empty() {
        text.push_str(" — lease _none_");
    } else {
        text.push_str(&format!(" — lease {}", code(&claim.lease_expires_at)));
    }
    text
}

fn codes(values: &[String]) -> String {
    if values.is_empty() {
        return EMPTY.to_string();
    }
    values
        .iter()
        .map(|value| code(value))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A reference to another record, as a markdown link.
///
/// The label is the id in its code span; the destination is the id,
/// percent-encoded, so a value full of markdown cannot close the link or
/// open one of its own.
fn link(id: &str) -> String {
    format!("[{}]({})", code(id), link_destination(id))
}

fn link_destination(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Render a value as an inline code span.
///
/// The fence is longer than any run of backticks in the value, so a value
/// cannot close the span early. Newlines are flattened: a code span is one
/// line.
fn code(value: &str) -> String {
    code_owned(&oneline(value))
}

fn code_owned(value: &str) -> String {
    if value.is_empty() {
        return EMPTY.to_string();
    }
    let longest = longest_backtick_run(value);
    if longest == 0 {
        return format!("`{value}`");
    }
    let fence = "`".repeat(longest + 1);
    let pad = if value.starts_with('`') || value.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{value}{pad}{fence}")
}

/// A value of several lines is a procedure, not a command: a code span would
/// flatten it, so it is written as a fenced block.
fn is_multiline(value: &str) -> bool {
    let value = value.trim();
    value.contains('\n') || value.contains('\r')
}

fn fenced_block(out: &mut String, value: &str) {
    let value = value.trim();
    let fence = "`".repeat(longest_backtick_run(value).saturating_add(1).max(3));
    out.push_str(&fence);
    out.push('\n');
    out.push_str(value);
    out.push('\n');
    out.push_str(&fence);
    out.push('\n');
}

fn longest_backtick_run(value: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for ch in value.chars() {
        if ch == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// Escape a value so markdown cannot interpret it.
///
/// The rule the projection holds for every field that must not be
/// interpreted. The detail pane's header labels its record by the same
/// rule, so a title cannot open markdown wherever it is shown.
pub fn escape_literal(value: &str) -> String {
    escape_markdown(&oneline(value))
}

/// Escape every line of a multiline fixed value, keeping the line breaks.
///
/// The same rule `escape_literal` holds, applied per line: a value that is
/// quoted line by line cannot open a block on any of them, and the escape is
/// invisible in the render. Leading whitespace becomes character references,
/// so an indented line cannot open a code block and its indent still shows.
fn escape_lines(value: &str) -> String {
    value
        .trim()
        .lines()
        .map(|line| {
            let text = line.trim_start_matches([' ', '\t']);
            let mut out = String::new();
            for ch in line[..line.len() - text.len()].chars() {
                out.push_str(if ch == '\t' { "&#9;" } else { "&#32;" });
            }
            out.push_str(&escape_markdown(text));
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(
            ch,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '('
                | ')'
                | '#'
                | '+'
                | '-'
                | '.'
                | '!'
                | '<'
                | '>'
                | '~'
                | '='
                | '&'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// A table cell. Pipes are escaped and newlines are line breaks, so a value
/// cannot open a cell or a row. Markdown inside the cell is left alone.
fn cell(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return EMPTY.to_string();
    }
    let value = escape_cell_pipes(value);
    value.replace("\r\n", "<br>").replace(['\n', '\r'], "<br>")
}

/// Escape every pipe, whatever precedes it.
///
/// GFM splits a row at a pipe that follows a backslash, even inside a code
/// span, and then drops that backslash. So a pipe is always escaped: a value's
/// own backslash survives, because the one added is the one dropped.
fn escape_cell_pipes(value: &str) -> String {
    value.replace('|', "\\|")
}

fn oneline(value: &str) -> String {
    value
        .replace("\r\n", " ")
        .replace(['\n', '\r'], " ")
        .trim()
        .to_string()
}

fn priority_str(priority: Priority) -> &'static str {
    match priority {
        Priority::Urgent => "urgent",
        Priority::High => "high",
        Priority::Normal => "normal",
        Priority::Low => "low",
    }
}

fn step_status_str(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "pending",
        StepStatus::InProgress => "in-progress",
        StepStatus::Done => "done",
        StepStatus::Skipped => "skipped",
        StepStatus::Failed => "failed",
    }
}

fn round_kind_str(kind: RoundKind) -> &'static str {
    match kind {
        RoundKind::SelfReview => "self",
        RoundKind::External => "external",
    }
}

fn focus_str(focus: Focus) -> &'static str {
    match focus {
        Focus::SpecCompliance => "spec-compliance",
        Focus::EdgeCases => "edge-cases",
        Focus::Security => "security",
        Focus::Empirical => "empirical",
        Focus::Design => "design",
    }
}

fn verdict_str(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Ok => "ok",
        Verdict::ChangesNeeded => "changes-needed",
    }
}

fn severity_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
    }
}

fn finding_status_str(status: FindingStatus) -> &'static str {
    match status {
        FindingStatus::Open => "open",
        FindingStatus::Fixed => "fixed",
        FindingStatus::Dismissed => "dismissed",
    }
}
