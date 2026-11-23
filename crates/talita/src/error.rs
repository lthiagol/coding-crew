//! The error codes and their exit codes (ADR 0014). The table is pinned to
//! `docs/talita/commands.md` by a test.

use coding_crew_core::StoreError;
use serde::Serialize;

/// A stable, dotted error code. Codes are added, never renamed.
// Later units raise the codes T5 does not.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Internal,
    UsageInvalid,
    InputInvalid,
    ProjectNotFound,
    TaskNotFound,
    PartNotFound,
    RunAbsent,
    CrewUnknown,
    StatusWrong,
    GateRefused,
    SectionFrozen,
    TaskCancelled,
    ReviewVerdictMismatch,
    ReviewBudgetSpent,
    FindingClosed,
    QuestionResolved,
    ClaimHeld,
    ClaimNotClaimable,
    CriterionCovered,
    CriterionNotFailed,
    CriterionDeferred,
    DependsCycle,
    OverlayAbsent,
    ItemNotOpen,
    ItemIsIdea,
    EvidenceTimeout,
    ProjectExists,
    RunActive,
    CrewExists,
    IdentityMissing,
    RoleWrong,
    ActorNotAgent,
    SectionNotOwned,
    ClaimNotHolder,
    ConflictLock,
    ConflictRevision,
    StoreUnreadable,
}

impl Code {
    #[cfg(test)]
    pub const ALL: [Code; 37] = [
        Code::Internal,
        Code::UsageInvalid,
        Code::InputInvalid,
        Code::ProjectNotFound,
        Code::TaskNotFound,
        Code::PartNotFound,
        Code::RunAbsent,
        Code::CrewUnknown,
        Code::StatusWrong,
        Code::GateRefused,
        Code::SectionFrozen,
        Code::TaskCancelled,
        Code::ReviewVerdictMismatch,
        Code::ReviewBudgetSpent,
        Code::FindingClosed,
        Code::QuestionResolved,
        Code::ClaimHeld,
        Code::ClaimNotClaimable,
        Code::CriterionCovered,
        Code::CriterionNotFailed,
        Code::CriterionDeferred,
        Code::DependsCycle,
        Code::OverlayAbsent,
        Code::ItemNotOpen,
        Code::ItemIsIdea,
        Code::EvidenceTimeout,
        Code::ProjectExists,
        Code::RunActive,
        Code::CrewExists,
        Code::IdentityMissing,
        Code::RoleWrong,
        Code::ActorNotAgent,
        Code::SectionNotOwned,
        Code::ClaimNotHolder,
        Code::ConflictLock,
        Code::ConflictRevision,
        Code::StoreUnreadable,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Code::Internal => "internal",
            Code::UsageInvalid => "usage.invalid",
            Code::InputInvalid => "input.invalid",
            Code::ProjectNotFound => "project.not_found",
            Code::TaskNotFound => "task.not_found",
            Code::PartNotFound => "part.not_found",
            Code::RunAbsent => "run.absent",
            Code::CrewUnknown => "crew.unknown",
            Code::StatusWrong => "status.wrong",
            Code::GateRefused => "gate.refused",
            Code::SectionFrozen => "section.frozen",
            Code::TaskCancelled => "task.cancelled",
            Code::ReviewVerdictMismatch => "review.verdict_mismatch",
            Code::ReviewBudgetSpent => "review.budget_spent",
            Code::FindingClosed => "finding.closed",
            Code::QuestionResolved => "question.resolved",
            Code::ClaimHeld => "claim.held",
            Code::ClaimNotClaimable => "claim.not_claimable",
            Code::CriterionCovered => "criterion.covered",
            Code::CriterionNotFailed => "criterion.not_failed",
            Code::CriterionDeferred => "criterion.deferred",
            Code::DependsCycle => "depends.cycle",
            Code::OverlayAbsent => "overlay.absent",
            Code::ItemNotOpen => "item.not_open",
            Code::ItemIsIdea => "item.is_idea",
            Code::EvidenceTimeout => "evidence.timeout",
            Code::ProjectExists => "project.exists",
            Code::RunActive => "run.active",
            Code::CrewExists => "crew.exists",
            Code::IdentityMissing => "identity.missing",
            Code::RoleWrong => "role.wrong",
            Code::ActorNotAgent => "actor.not_agent",
            Code::SectionNotOwned => "section.not_owned",
            Code::ClaimNotHolder => "claim.not_holder",
            Code::ConflictLock => "conflict.lock",
            Code::ConflictRevision => "conflict.revision",
            Code::StoreUnreadable => "store.unreadable",
        }
    }

    /// The process exit code of this code's class.
    pub fn exit(self) -> i32 {
        match self {
            Code::Internal => 1,
            Code::UsageInvalid | Code::InputInvalid => 2,
            Code::ProjectNotFound
            | Code::TaskNotFound
            | Code::PartNotFound
            | Code::RunAbsent
            | Code::CrewUnknown => 3,
            Code::StatusWrong
            | Code::GateRefused
            | Code::SectionFrozen
            | Code::TaskCancelled
            | Code::ReviewVerdictMismatch
            | Code::ReviewBudgetSpent
            | Code::FindingClosed
            | Code::QuestionResolved
            | Code::ClaimHeld
            | Code::ClaimNotClaimable
            | Code::CriterionCovered
            | Code::CriterionNotFailed
            | Code::CriterionDeferred
            | Code::DependsCycle
            | Code::OverlayAbsent
            | Code::ItemNotOpen
            | Code::ItemIsIdea
            | Code::EvidenceTimeout
            | Code::ProjectExists
            | Code::RunActive
            | Code::CrewExists => 4,
            Code::IdentityMissing
            | Code::RoleWrong
            | Code::ActorNotAgent
            | Code::SectionNotOwned
            | Code::ClaimNotHolder => 5,
            Code::ConflictLock | Code::ConflictRevision => 6,
            Code::StoreUnreadable => 7,
        }
    }
}

impl Serialize for Code {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One failing gate, by its task-model name.
#[derive(Debug, Clone, Serialize)]
pub struct GateDetail {
    pub gate: String,
    pub detail: String,
}

/// Why a call was refused: the envelope's `error`.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub code: Code,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<GateDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}

impl Failure {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            gates: Vec::new(),
            remedy: None,
        }
    }

    pub fn remedy(mut self, remedy: impl Into<String>) -> Self {
        self.remedy = Some(remedy.into());
        self
    }

    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::new(Code::Internal, error.to_string())
    }
}

impl From<StoreError> for Failure {
    fn from(error: StoreError) -> Self {
        let code = match &error {
            StoreError::Io(_) | StoreError::Item(_) => Code::Internal,
            StoreError::Parse(_) | StoreError::JournalUnreadable(_) => Code::StoreUnreadable,
            StoreError::NotFound(_) => Code::TaskNotFound,
            StoreError::NoDataFolder | StoreError::NoWorkFolder(_) => Code::ProjectNotFound,
            StoreError::InvalidStart(_) => Code::InputInvalid,
            StoreError::Locked => Code::ConflictLock,
            StoreError::Revision { .. } => Code::ConflictRevision,
        };
        let failure = Failure::new(code, error.to_string());
        match code {
            Code::ProjectNotFound => failure.remedy("talita project init"),
            _ => failure,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Code;
    use std::collections::BTreeMap;

    fn commands_md() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/talita/commands.md");
        std::fs::read_to_string(path).expect("docs/talita/commands.md")
    }

    /// The rows of the `| Exit | Class | Codes |` table: code → exit.
    fn table(doc: &str) -> BTreeMap<String, i32> {
        let mut codes = BTreeMap::new();
        let mut inside = false;
        for line in doc.lines() {
            if line.starts_with("| Exit | Class | Codes |") {
                inside = true;
                continue;
            }
            if !inside || line.starts_with("|--") || line.starts_with("|---") {
                continue;
            }
            if !line.starts_with('|') {
                break;
            }
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let exit: i32 = cells[1].parse().expect("an exit code");
            for code in cells[3].split('`').skip(1).step_by(2) {
                assert!(
                    codes.insert(code.to_string(), exit).is_none(),
                    "{code} is listed twice"
                );
            }
        }
        codes
    }

    #[test]
    fn the_code_table_matches_commands_md() {
        let doc = commands_md();
        let documented = table(&doc);
        assert!(!documented.is_empty(), "commands.md has no code table");
        let built: BTreeMap<String, i32> = Code::ALL
            .iter()
            .map(|code| (code.as_str().to_string(), code.exit()))
            .collect();
        assert_eq!(built, documented);
    }

    /// Every "`code` (n)" mention in commands.md agrees with the table.
    #[test]
    fn every_mentioned_exit_code_matches() {
        let doc = commands_md();
        let built: BTreeMap<&str, i32> = Code::ALL.iter().map(|c| (c.as_str(), c.exit())).collect();
        let mut checked = 0;
        for piece in doc.split('`').collect::<Vec<_>>().windows(2) {
            let (code, after) = (piece[0], piece[1]);
            let Some(exit) = after
                .strip_prefix(" (")
                .and_then(|rest| rest.split([')', ',']).next())
                .and_then(|n| n.parse::<i32>().ok())
            else {
                continue;
            };
            if let Some(want) = built.get(code) {
                assert_eq!(*want, exit, "commands.md gives `{code}` exit {exit}");
                checked += 1;
            }
        }
        assert!(checked > 10, "only {checked} mentions found");
    }

    #[test]
    fn codes_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in Code::ALL {
            assert!(seen.insert(code.as_str()), "{} twice", code.as_str());
        }
    }
}
