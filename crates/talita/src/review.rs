//! `round file|show`, `finding open|resolve|dismiss` and `comment add`: the
//! review record (ADR 0017). A round is filed whole; the rules are core's.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use coding_crew_core::lifecycle::{
    GateFailure, check_independence, check_remediation, check_round_owner, latest_external,
};
use coding_crew_core::{
    Actor, Comment, Finding, FindingStatus, Focus, Round, RoundKind, Section, Severity, Status,
    Task, Verdict,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::envelope::Done;
use crate::error::{Code, Failure, GateDetail};
use crate::parts;
use crate::project::Project;
use crate::task::word;
use crate::write::{Call, next_part_id, own, part_not_found, required_text, text};

/// The flags of `round file`. `input` carries the whole round instead.
#[derive(Debug, Default)]
pub struct RoundFlags<'a> {
    pub kind: Option<&'a str>,
    pub focus: Option<&'a str>,
    pub verdict: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub input: Option<&'a str>,
}

/// The round as the verb holds it, flags or input read alike.
struct RoundSpec {
    kind: RoundKind,
    focus: Focus,
    verdict: Verdict,
    notes: String,
    findings: Vec<FindingSpec>,
}

struct FindingSpec {
    reference: String,
    severity: Severity,
    description: String,
}

// ---- round

/// `round file`: the self pre-screen or the reviewer's external round, filed
/// whole, with its findings and its verdict (ADR 0017).
pub fn round_file(call: &Call, id: &str, flags: RoundFlags<'_>) -> Result<Done, Failure> {
    let spec = match flags.input {
        Some(input) => read_round(input)?,
        None => {
            let kind = parse_kind(required_flag("--kind", flags.kind)?)?;
            let focus = parse_focus(required_flag("--focus", flags.focus)?)?;
            let verdict = parse_verdict(required_flag("--verdict", flags.verdict)?)?;
            let notes = flags
                .notes
                .map(|value| text("--notes", value))
                .transpose()?
                .unwrap_or_default();
            RoundSpec {
                kind,
                focus,
                verdict,
                notes,
                findings: Vec::new(),
            }
        }
    };
    if spec.kind == RoundKind::SelfReview && !spec.findings.is_empty() {
        return Err(Failure::new(
            Code::InputInvalid,
            "a self round carries no findings: the runner fixes what it finds before finish",
        )
        .remedy(format!(
            "talita round file {id} --kind self --focus <f> --verdict ok|changes-needed"
        )));
    }
    call.task(id, |task, actor| {
        check_filing(task, actor, &spec, id)?;
        Ok(file(task, actor, spec))
    })
}

/// Who may file the round, in what status, against what budget, and the
/// verdict rules against the whole round (commands.md § `round file`).
fn check_filing(task: &Task, actor: &Actor, spec: &RoundSpec, id: &str) -> Result<(), Failure> {
    match spec.kind {
        RoundKind::SelfReview => {
            if !matches!(task.status, Status::Executing | Status::Remediating) {
                return Err(status_wrong(
                    id,
                    "a self round is filed in executing or remediating",
                    task.status,
                ));
            }
            owned(Section::Review, task, actor, "round file")?;
            parts::working_runner(task, actor, "rounds")?;
        }
        RoundKind::External => {
            if task.status != Status::CodeReview {
                return Err(status_wrong(
                    id,
                    "an external round is filed in code-review",
                    task.status,
                ));
            }
            owned(Section::Review, task, actor, "round file")?;
            let failures = independence_failures(task, actor);
            if !failures.is_empty() {
                return Err(gate_refusal("round file", id, failures));
            }
            let filed = task.external_rounds() as u32;
            if filed >= task.review.max_rounds {
                return Err(Failure::new(
                    Code::ReviewBudgetSpent,
                    format!(
                        "the budget of {} external rounds is spent ({filed} filed); {id} takes no more",
                        task.review.max_rounds
                    ),
                )
                .remedy(format!("talita task show {id}")));
            }
            match spec.verdict {
                Verdict::Ok if task.open_findings() > 0 || !spec.findings.is_empty() => {
                    return Err(Failure::new(
                        Code::ReviewVerdictMismatch,
                        format!(
                            "an ok round is filed while {} finding{} would be open",
                            task.open_findings() + spec.findings.len(),
                            if task.open_findings() + spec.findings.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        ),
                    )
                    .remedy(format!("talita round file {id} --kind external --focus <f> --verdict changes-needed")));
                }
                Verdict::ChangesNeeded if spec.findings.is_empty() => {
                    return Err(Failure::new(
                        Code::ReviewVerdictMismatch,
                        "a changes-needed round names no finding",
                    )
                    .remedy(format!(
                        "talita round file {id} --input <path>: list the findings with the round"
                    )));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// The one write: the findings, the round and `review.verdict`.
fn file(task: &mut Task, actor: &Actor, spec: RoundSpec) -> Value {
    let RoundSpec {
        kind,
        focus,
        verdict,
        notes,
        findings,
    } = spec;
    let now = coding_crew_core::now_rfc3339();
    let repeated = latest_external(task).is_some_and(|round| round.focus == focus);
    let n = match kind {
        RoundKind::SelfReview => 0,
        RoundKind::External => task.external_rounds() as u32 + 1,
    };
    let mut refs = BTreeMap::new();
    let mut ids = Vec::new();
    for finding in findings {
        let fid = next_part_id("FIND", task.review.findings.iter().map(|f| f.id.as_str()));
        task.review.findings.push(Finding {
            id: fid.clone(),
            severity: finding.severity,
            status: FindingStatus::Open,
            description: finding.description,
            created: now.clone(),
            ..Finding::default()
        });
        refs.insert(finding.reference, fid.clone());
        ids.push(fid);
    }
    task.review.rounds.push(Round {
        n,
        kind,
        focus,
        reviewer: actor.id.clone(),
        session: actor.session.clone(),
        verdict,
        at: now,
        notes,
        findings: ids,
    });
    task.review.verdict = Some(verdict);
    let mut warnings: Vec<Value> = Vec::new();
    if repeated {
        warnings.push(json!({
            "code": "review.focus_repeated",
            "message": format!("{} repeats the previous external round's focus", word(&focus)),
        }));
    }
    json!({
        "round": task.review.rounds.last(),
        "refs": refs,
        "warnings": warnings,
    })
}

/// `round show`: the rounds (or those numbered `n`), each with its findings.
pub fn round_show(project: &Project, id: &str, n: Option<u32>) -> Result<Done, Failure> {
    let task = project.store.load(id)?;
    let rounds: Vec<&Round> = task
        .review
        .rounds
        .iter()
        .filter(|round| n.is_none_or(|n| round.n == n))
        .collect();
    if let Some(n) = n
        && rounds.is_empty()
    {
        return Err(part_not_found(&task, &format!("round {n}")));
    }
    let views: Vec<Value> = rounds
        .iter()
        .map(|round| {
            let findings: Vec<&Finding> = round
                .findings
                .iter()
                .filter_map(|id| {
                    task.review
                        .findings
                        .iter()
                        .find(|finding| &finding.id == id)
                })
                .collect();
            json!({ "round": round, "findings": findings })
        })
        .collect();
    Ok(Done::data(json!({ "rounds": views })))
}

// ---- finding

/// `finding open`: a finding added to the reviewer's latest round, which must
/// be an external `changes-needed` round filed by this session.
pub fn finding_open(
    call: &Call,
    id: &str,
    severity: &str,
    description: &str,
) -> Result<Done, Failure> {
    let severity = parse_severity(severity)?;
    let description = required_text("--description", description)?;
    call.task(id, |task, actor| {
        if task.status != Status::CodeReview {
            return Err(status_wrong(
                id,
                "a finding is opened in code-review",
                task.status,
            ));
        }
        owned(Section::Review, task, actor, "finding open")?;
        let failures = round_owner_failures(task, actor);
        if !failures.is_empty() {
            return Err(gate_refusal("finding open", id, failures));
        }
        let Some(round) = latest_external(task) else {
            unreachable!("the round-owner check names the missing round");
        };
        let (n, verdict) = (round.n, round.verdict);
        if verdict != Verdict::ChangesNeeded {
            return Err(Failure::new(
                Code::ReviewVerdictMismatch,
                format!(
                    "the latest external round is {}, not changes-needed; a finding needs a changes-needed round",
                    word(&verdict)
                ),
            )
            .remedy(format!(
                "talita round file {id} --kind external --focus <f> --verdict changes-needed --input <path>"
            )));
        }
        let fid = next_part_id("FIND", task.review.findings.iter().map(|f| f.id.as_str()));
        task.review.findings.push(Finding {
            id: fid.clone(),
            severity,
            status: FindingStatus::Open,
            description,
            created: coding_crew_core::now_rfc3339(),
            ..Finding::default()
        });
        if let Some(latest) = task
            .review
            .rounds
            .iter_mut()
            .find(|round| round.kind == RoundKind::External && round.n == n)
        {
            latest.findings.push(fid.clone());
        }
        Ok(json!({ "finding": task.review.findings.iter().find(|f| f.id == fid) }))
    })
}

/// `finding resolve`: the runner's outcome on a finding, written once.
pub fn finding_resolve(
    call: &Call,
    id: &str,
    finding: &str,
    resolution: &str,
) -> Result<Done, Failure> {
    let resolution = required_text("--resolution", resolution)?;
    close(
        call,
        id,
        "finding resolve",
        finding,
        resolution,
        FindingStatus::Fixed,
    )
}

/// `finding dismiss`: the runner's reason a finding is not fixed, written once.
pub fn finding_dismiss(
    call: &Call,
    id: &str,
    finding: &str,
    reason: &str,
) -> Result<Done, Failure> {
    let reason = required_text("--reason", reason)?;
    close(
        call,
        id,
        "finding dismiss",
        finding,
        reason,
        FindingStatus::Dismissed,
    )
}

fn close(
    call: &Call,
    id: &str,
    verb: &str,
    finding_id: &str,
    resolution: String,
    outcome: FindingStatus,
) -> Result<Done, Failure> {
    call.task(id, |task, actor| {
        if task.status != Status::Remediating {
            return Err(status_wrong(
                id,
                "a finding is closed in remediating",
                task.status,
            ));
        }
        owned(Section::FindingOutcome, task, actor, verb)?;
        if task.review.findings.iter().all(|f| f.id != finding_id) {
            return Err(part_not_found(task, finding_id));
        }
        let failures = remediation_failures(task, actor);
        if !failures.is_empty() {
            return Err(gate_refusal(verb, id, failures));
        }
        let Some(finding) = task.finding_mut(finding_id) else {
            unreachable!("the finding was checked");
        };
        if finding.status != FindingStatus::Open {
            let state = word(&finding.status);
            return Err(Failure::new(
                Code::FindingClosed,
                format!("{finding_id} is already {state}"),
            )
            .remedy(format!("talita task show {id}")));
        }
        finding.status = outcome;
        finding.resolved = coding_crew_core::now_rfc3339();
        finding.resolution = resolution;
        Ok(json!({ "finding": task.review.findings.iter().find(|f| f.id == finding_id) }))
    })
}

// ---- comment

/// `comment add`: discussion under a finding, open or closed. Any agent role;
/// a human too (ADR 0017, `Section::Notes`).
pub fn comment_add(call: &Call, id: &str, finding: &str, body: &str) -> Result<Done, Failure> {
    let body = required_text("--body", body)?;
    call.task(id, |task, actor| {
        owned(Section::Notes, task, actor, "comment add")?;
        if task.review.findings.iter().all(|f| f.id != finding) {
            return Err(part_not_found(task, finding));
        }
        let cid = next_part_id(
            "CMT",
            task.review
                .findings
                .iter()
                .flat_map(|f| f.comments.iter().map(|c| c.id.as_str())),
        );
        let comment = Comment {
            id: cid.clone(),
            author: actor.id.clone(),
            body,
            at: coding_crew_core::now_rfc3339(),
        };
        let target = task.finding_mut(finding).expect("the finding was checked");
        target.comments.push(comment);
        Ok(json!({ "comment": task.review.findings.iter().find(|f| f.id == finding).and_then(|f| f.comments.iter().find(|c| c.id == cid)) }))
    })
}

// ---- the rules talita calls in core

fn independence_failures(task: &Task, actor: &Actor) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    check_independence(&mut failures, task, actor);
    failures
}

fn round_owner_failures(task: &Task, actor: &Actor) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    check_round_owner(&mut failures, task, actor);
    failures
}

fn remediation_failures(task: &Task, actor: &Actor) -> Vec<GateFailure> {
    let mut failures = Vec::new();
    check_remediation(&mut failures, task, actor);
    failures
}

// ---- refusals

/// Core's ownership table, as the noun's codes: the verb names a role, so a
/// wrong role is `role.wrong`; a human is `actor.not_agent`.
fn owned(section: Section, task: &Task, actor: &Actor, verb: &str) -> Result<(), Failure> {
    match own(section, actor, task) {
        Ok(()) => Ok(()),
        Err(mut failure) if failure.code == Code::SectionNotOwned => {
            failure.code = Code::RoleWrong;
            failure.message = format!("{verb}: {}", failure.message);
            Err(failure)
        }
        Err(failure) => Err(failure),
    }
}

fn status_wrong(id: &str, when: &str, status: Status) -> Failure {
    Failure::new(
        Code::StatusWrong,
        format!("{when}; {id} is {}", status.as_str()),
    )
    .remedy(format!("talita task show {id}"))
}

/// A refused check, with every failing gate core reported.
fn gate_refusal(verb: &str, id: &str, failures: Vec<GateFailure>) -> Failure {
    let count = failures.len();
    let gates: Vec<GateDetail> = failures
        .into_iter()
        .map(|failure| GateDetail {
            gate: failure.gate.to_string(),
            detail: failure.message,
        })
        .collect();
    let remedy = gates
        .first()
        .map(|gate| match gate.gate.as_str() {
            "review.independent" | "review.round_owner" => {
                "talita session new --as <actor> --role reviewer".to_string()
            }
            "remediation.session_distinct" => {
                "talita session new --as <actor> --role runner".to_string()
            }
            _ => format!("talita task show {id}"),
        })
        .unwrap_or_else(|| format!("talita task show {id}"));
    let mut failure = Failure::new(
        Code::GateRefused,
        format!(
            "{verb} is refused by {count} gate{}",
            if count == 1 { "" } else { "s" }
        ),
    )
    .remedy(remedy);
    failure.gates = gates;
    failure
}

// ---- input

fn required_flag<'a>(flag: &str, value: Option<&'a str>) -> Result<&'a str, Failure> {
    value.ok_or_else(|| {
        Failure::new(
            Code::UsageInvalid,
            format!("round file needs {flag} or --input"),
        )
    })
}

fn parse_kind(value: &str) -> Result<RoundKind, Failure> {
    Ok(match value {
        "self" => RoundKind::SelfReview,
        "external" => RoundKind::External,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("kind `{other}` is not one of self, external"),
            ));
        }
    })
}

fn parse_focus(value: &str) -> Result<Focus, Failure> {
    Ok(match value {
        "spec-compliance" => Focus::SpecCompliance,
        "edge-cases" => Focus::EdgeCases,
        "security" => Focus::Security,
        "empirical" => Focus::Empirical,
        "design" => Focus::Design,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!(
                    "focus `{other}` is not one of spec-compliance, edge-cases, security, empirical, design"
                ),
            ));
        }
    })
}

fn parse_verdict(value: &str) -> Result<Verdict, Failure> {
    Ok(match value {
        "ok" => Verdict::Ok,
        "changes-needed" => Verdict::ChangesNeeded,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("verdict `{other}` is not one of ok, changes-needed"),
            ));
        }
    })
}

fn parse_severity(value: &str) -> Result<Severity, Failure> {
    Ok(match value {
        "low" => Severity::Low,
        "medium" => Severity::Medium,
        "high" => Severity::High,
        other => {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("severity `{other}` is not one of low, medium, high"),
            ));
        }
    })
}

/// The closed input schema of `--input` (commands.md § `round file`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputRound {
    kind: String,
    focus: String,
    verdict: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    findings: Vec<InputFinding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFinding {
    #[serde(rename = "ref")]
    reference: String,
    severity: String,
    description: String,
}

fn read_round(input: &str) -> Result<RoundSpec, Failure> {
    let read = if input == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer).map(|_| buffer)
    } else {
        std::fs::read_to_string(input)
    };
    let text =
        read.map_err(|error| Failure::new(Code::InputInvalid, format!("--input: {error}")))?;
    let round: InputRound = serde_json::from_str(&text)
        .map_err(|error| Failure::new(Code::InputInvalid, format!("--input: {error}")))?;
    let kind = parse_kind(&round.kind)?;
    let focus = parse_focus(&round.focus)?;
    let verdict = parse_verdict(&round.verdict)?;
    let mut seen = BTreeSet::new();
    let mut findings = Vec::new();
    for finding in round.findings {
        let reference = non_empty("finding ref", &finding.reference)?;
        if !seen.insert(reference.clone()) {
            return Err(Failure::new(
                Code::InputInvalid,
                format!("finding ref `{reference}` is repeated"),
            ));
        }
        findings.push(FindingSpec {
            reference,
            severity: parse_severity(&finding.severity)?,
            description: non_empty("finding description", &finding.description)?,
        });
    }
    Ok(RoundSpec {
        kind,
        focus,
        verdict,
        notes: round.notes.unwrap_or_default(),
        findings,
    })
}

/// A value of the JSON input, stored verbatim: `@` is not a file here.
fn non_empty(label: &str, value: &str) -> Result<String, Failure> {
    if value.trim().is_empty() {
        return Err(Failure::new(
            Code::InputInvalid,
            format!("{label} is empty"),
        ));
    }
    Ok(value.to_string())
}
