use serde::{Deserialize, Serialize};

use crate::actor::Actor;
use crate::lifecycle::{Overlays, Status};

/// Schema version written into every task file.
pub const SCHEMA_VERSION: u32 = 1;

/// A unit of work. See `docs/tasks/task-model.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    /// What kind of work this is. `bug` and `feature` are small tasks.
    #[serde(default)]
    pub kind: TaskKind,
    /// The smaller task this task became.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted_from: Option<String>,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
    /// A counter the writer increments on every write. Optimistic checks
    /// compare against it (ADR 0016).
    #[serde(default)]
    pub revision: u32,
    #[serde(default)]
    pub status: Status,
    /// When the task entered the status it is in.
    #[serde(default)]
    pub status_since: String,
    #[serde(default)]
    pub overlays: Overlays,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
    /// Session of the creator that planned the task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planned_by: Option<String>,
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    pub intent: Intent,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub review: Review,
    #[serde(default)]
    pub questions: Vec<Question>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

impl Task {
    /// A new task in `draft`.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let title = title.into();
        // One reading of the clock: the three stamps are the same instant.
        let now = crate::now_rfc3339();
        Self {
            schema_version: SCHEMA_VERSION,
            slug: slugify(&title),
            id: id.into(),
            title,
            parent: None,
            children: Vec::new(),
            kind: TaskKind::Task,
            promoted_from: None,
            created: now.clone(),
            updated: now.clone(),
            revision: 0,
            status_since: now,
            status: Status::Draft,
            overlays: Overlays::default(),
            claim: None,
            planned_by: None,
            meta: Meta::default(),
            intent: Intent::default(),
            scope: Scope::default(),
            acceptance_criteria: Vec::new(),
            steps: Vec::new(),
            review: Review::default(),
            questions: Vec::new(),
            notes: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    pub fn planned_by(mut self, actor: &Actor) -> Self {
        self.planned_by = actor.session.clone();
        self
    }

    pub fn criterion(&self, id: &str) -> Option<&AcceptanceCriterion> {
        self.acceptance_criteria.iter().find(|c| c.id == id)
    }

    pub fn criterion_mut(&mut self, id: &str) -> Option<&mut AcceptanceCriterion> {
        self.acceptance_criteria.iter_mut().find(|c| c.id == id)
    }

    pub fn step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    pub fn step_mut(&mut self, id: &str) -> Option<&mut Step> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    pub fn finding_mut(&mut self, id: &str) -> Option<&mut Finding> {
        self.review.findings.iter_mut().find(|f| f.id == id)
    }

    pub fn open_questions(&self) -> usize {
        self.questions
            .iter()
            .filter(|q| q.status == QuestionStatus::Open)
            .count()
    }

    pub fn open_findings(&self) -> usize {
        self.review
            .findings
            .iter()
            .filter(|f| f.status == FindingStatus::Open)
            .count()
    }

    pub fn self_rounds(&self) -> usize {
        self.review
            .rounds
            .iter()
            .filter(|r| r.kind == RoundKind::SelfReview)
            .count()
    }

    pub fn external_rounds(&self) -> usize {
        self.review
            .rounds
            .iter()
            .filter(|r| r.kind == RoundKind::External)
            .count()
    }

    /// True when every acceptance criterion is covered by at least one step.
    pub fn criteria_covered(&self) -> bool {
        self.acceptance_criteria
            .iter()
            .all(|c| self.steps.iter().any(|s| s.covers.contains(&c.id)))
    }

    /// True when every step carries a `done_when`.
    pub fn steps_have_done_when(&self) -> bool {
        self.steps.iter().all(|s| !s.done_when.trim().is_empty())
    }

    /// A small task is one the step rules do not gate.
    ///
    /// The exemption lives here so every gate applies it the same way.
    pub fn is_small_task(&self) -> bool {
        self.kind.is_small_task()
    }
}

/// What kind of work a task is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TaskKind {
    #[default]
    Task,
    Bug,
    Feature,
}

impl TaskKind {
    /// `bug` and `feature` are small tasks: the step rules do not apply.
    pub fn is_small_task(&self) -> bool {
        matches!(self, TaskKind::Bug | TaskKind::Feature)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TaskKind::Task => "task",
            TaskKind::Bug => "bug",
            TaskKind::Feature => "feature",
        }
    }
}

/// Lowercase, hyphenated form of a title.
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    pub at: String,
    #[serde(default)]
    pub lease_expires_at: String,
}

impl Claim {
    /// Whether the lease has expired at `now`.
    ///
    /// The stamps are RFC 3339 instants, compared as instants, not as text:
    /// the zone is read, so equivalent readings in different offsets agree.
    /// An empty lease never expires. A nonempty lease that is not a stamp is
    /// treated as expired, so it cannot hold a task forever. A `now` that is
    /// not a stamp establishes nothing, so the lease stays live.
    pub fn lease_expired(&self, now: &str) -> bool {
        if self.lease_expires_at.is_empty() {
            return false;
        }
        let Some(lease) = crate::parse_rfc3339(&self.lease_expires_at) else {
            return true;
        };
        match crate::parse_rfc3339(now) {
            Some(now) => now >= lease,
            None => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub effort: Effort,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            priority: Priority::Normal,
            effort: Effort::Medium,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Urgent,
    High,
    #[default]
    Normal,
    Low,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Urgent => "urgent",
            Priority::High => "high",
            Priority::Normal => "normal",
            Priority::Low => "low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Effort {
    #[serde(rename = "S")]
    Small,
    #[serde(rename = "M")]
    #[default]
    Medium,
    #[serde(rename = "L")]
    Large,
}

impl Effort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Small => "S",
            Effort::Medium => "M",
            Effort::Large => "L",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub problem: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default)]
    pub in_scope: Vec<String>,
    #[serde(default)]
    pub out_of_scope: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub verification: String,
    #[serde(default)]
    pub status: CriterionStatus,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferral: Option<Deferral>,
    /// Why the criterion failed, when it did.
    #[serde(default)]
    pub failure_reason: String,
}

impl AcceptanceCriterion {
    /// Status from the latest evidence, or the stored status when there is
    /// none. A manual override is recorded as evidence. A deferral wins over
    /// both: a criterion parked on purpose is deferred.
    pub fn effective_status(&self) -> CriterionStatus {
        if self.deferral.is_some() {
            return CriterionStatus::Deferred;
        }
        match self.evidence.last() {
            Some(evidence) => evidence.outcome(),
            None => self.status,
        }
    }

    /// Writes the stored status from the derived one (ADR 0019). A criterion
    /// with neither evidence nor a deferral is `pending`. A passing criterion
    /// carries no failure reason: one left from an earlier failure clears.
    pub fn mirror_status(&mut self) {
        self.status = match (&self.deferral, self.evidence.last()) {
            (Some(_), _) => CriterionStatus::Deferred,
            (None, Some(evidence)) => evidence.outcome(),
            (None, None) => CriterionStatus::Pending,
        };
        if self.status == CriterionStatus::Passed {
            self.failure_reason.clear();
        }
    }

    /// Appends evidence and mirrors the status it leaves.
    pub fn push_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
        self.mirror_status();
    }

    /// Sets or clears the deferral and mirrors the status it leaves.
    pub fn set_deferral(&mut self, deferral: Option<Deferral>) {
        self.deferral = deferral;
        self.mirror_status();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CriterionStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Unproven,
    Deferred,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(default)]
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Written by `evidence override`: the outcome is a judgement, not a
    /// command's exit code. raul marks it.
    #[serde(default)]
    pub manual: bool,
}

impl Evidence {
    pub fn outcome(&self) -> CriterionStatus {
        match self.exit_code {
            Some(0) if !self.summary.trim().is_empty() => CriterionStatus::Passed,
            Some(_) => CriterionStatus::Failed,
            None => CriterionStatus::Unproven,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deferral {
    pub reason: String,
    #[serde(default)]
    pub follow_up: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    #[serde(default)]
    pub order: u32,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub done_when: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub status: StepStatus,
    #[serde(default)]
    pub covers: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Note when the runner took a different path than the step described.
    #[serde(default)]
    pub deviation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
    #[default]
    Pending,
    InProgress,
    Done,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    #[serde(default = "default_max_rounds")]
    pub max_rounds: u32,
    #[serde(default)]
    pub rounds: Vec<Round>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    #[serde(default)]
    pub findings: Vec<Finding>,
    /// Each hand-back to review, oldest first. Append-only.
    #[serde(default)]
    pub re_reviews: Vec<ReReview>,
}

fn default_max_rounds() -> u32 {
    5
}

impl Default for Review {
    fn default() -> Self {
        Self {
            max_rounds: default_max_rounds(),
            rounds: Vec::new(),
            verdict: None,
            findings: Vec::new(),
            re_reviews: Vec::new(),
        }
    }
}

/// A dated record of one hand-back: the task went back to `code-review` after
/// its runner answered the findings of an external round.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReReview {
    /// When the task went back to `code-review`, RFC 3339.
    pub at: String,
    /// The external round whose findings were worked.
    pub after_round: u32,
    /// The remediating actor.
    pub by: String,
    /// The remediating session. The `remediation.session_distinct` gate checks
    /// it is not the execution session (`claim.session`).
    #[serde(default)]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Round {
    pub n: u32,
    pub kind: RoundKind,
    pub focus: Focus,
    #[serde(default)]
    pub reviewer: String,
    /// The session that filed the round. `reviewer` stays the actor id.
    #[serde(default)]
    pub session: Option<String>,
    pub verdict: Verdict,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RoundKind {
    #[default]
    #[serde(rename = "self")]
    SelfReview,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Focus {
    #[default]
    SpecCompliance,
    EdgeCases,
    Security,
    Empirical,
    Design,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    #[default]
    Ok,
    ChangesNeeded,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default)]
    pub status: FindingStatus,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub resolved: String,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    #[default]
    Open,
    Fixed,
    Dismissed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub status: QuestionStatus,
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub raised_by: String,
    #[serde(default)]
    pub raised_at: String,
    #[serde(default)]
    pub resolved_by: String,
    #[serde(default)]
    pub resolved_at: String,
    /// The human who decided it, when a human did. Questions are not
    /// journaled, so the name lives on the question (ADR 0015).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructed_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum QuestionStatus {
    #[default]
    Open,
    Resolved,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(lease: &str) -> Claim {
        Claim {
            by: "runner-1".to_string(),
            session: None,
            at: "2026-11-07T12:00:00Z".to_string(),
            lease_expires_at: lease.to_string(),
        }
    }

    #[test]
    fn an_empty_lease_never_expires() {
        assert!(!claim("").lease_expired("2999-01-01T00:00:00Z"));
    }

    #[test]
    fn a_lease_expires_on_the_instant_it_names() {
        let claim = claim("2026-11-07T13:00:00Z");
        assert!(!claim.lease_expired("2026-11-07T12:59:59Z"));
        assert!(claim.lease_expired("2026-11-07T13:00:00Z"));
        assert!(claim.lease_expired("2026-11-07T13:00:01Z"));
    }

    #[test]
    fn a_lease_without_seconds_expires_on_the_minute_it_says() {
        let claim = claim("2026-11-07T13:00Z");
        assert!(!claim.lease_expired("2026-11-07T12:59:59Z"));
        assert!(claim.lease_expired("2026-11-07T13:00:00Z"));
        assert!(claim.lease_expired("2026-11-07T13:00:30Z"));
    }

    #[test]
    fn offsets_name_the_same_instant() {
        // 15:00+02:00 and 08:00-05:00 are both 13:00Z.
        let ahead = claim("2026-11-07T15:00:00+02:00");
        let behind = claim("2026-11-07T08:00:00-05:00");
        assert!(!ahead.lease_expired("2026-11-07T12:59:59Z"));
        assert!(ahead.lease_expired("2026-11-07T13:00:00Z"));
        assert!(!behind.lease_expired("2026-11-07T12:59:59Z"));
        assert!(behind.lease_expired("2026-11-07T13:00:00Z"));
    }

    #[test]
    fn a_malformed_lease_is_expired() {
        assert!(claim("not a stamp").lease_expired("2026-11-07T13:00:00Z"));
    }

    #[test]
    fn a_malformed_now_leaves_the_lease_live() {
        assert!(!claim("2026-11-07T13:00:00Z").lease_expired("not a stamp"));
        assert!(!claim("2026-11-07T13:00:00Z").lease_expired(""));
    }

    #[test]
    fn a_task_written_before_the_new_fields_reads_with_defaults() {
        let text = r#"{
          "id": "TSK01",
          "title": "an older task",
          "review": {
            "rounds": [{"n": 1, "kind": "external", "focus": "security", "verdict": "ok"}],
            "findings": []
          },
          "acceptance_criteria": [{
            "id": "ACCEPT-01",
            "evidence": [{"command": "cargo test", "exit_code": 0, "summary": "3 passed"}]
          }]
        }"#;
        let task: Task = serde_json::from_str(text).expect("an older task");
        assert_eq!(task.revision, 0);
        assert!(task.review.re_reviews.is_empty());
        assert_eq!(task.review.rounds[0].session, None);
        assert!(!task.acceptance_criteria[0].evidence[0].manual);
    }
}
