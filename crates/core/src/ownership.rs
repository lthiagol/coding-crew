//! Section ownership: which role may write which part of a task (ADR 0013;
//! `docs/roles/roles.md`, "Ownership by section"). The table is code, so every
//! writer shares one answer.
//!
//! A section with no owner is written by no verb. Its value follows from a
//! transition or from the writer's own bookkeeping.

use thiserror::Error;

use crate::actor::{Actor, ActorKind, Role};
use crate::lifecycle::Status;

/// A part of a task that a verb writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Intent,
    Scope,
    Criteria,
    Meta,
    DependsOn,
    Questions,
    Steps,
    Claim,
    Evidence,
    Review,
    /// A finding's outcome: the runner closes what the reviewer opened.
    FindingOutcome,
    Notes,
    Overlays,
    /// `status`, `status_since` and `planned_by`: written by transitions only.
    Status,
}

impl Section {
    /// The section's name in the task file, or in the words the docs use.
    pub fn as_str(&self) -> &'static str {
        match self {
            Section::Intent => "intent",
            Section::Scope => "scope",
            Section::Criteria => "acceptance_criteria",
            Section::Meta => "meta",
            Section::DependsOn => "depends_on",
            Section::Questions => "questions",
            Section::Steps => "steps",
            Section::Claim => "claim",
            Section::Evidence => "evidence",
            Section::Review => "review",
            Section::FindingOutcome => "finding outcome",
            Section::Notes => "notes",
            Section::Overlays => "overlays",
            Section::Status => "status",
        }
    }
}

/// Who may write a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Creator,
    Runner,
    Reviewer,
    /// Any actor, humans included.
    Any,
}

impl Owner {
    /// The role that owns the section, or `None` for [`Owner::Any`].
    pub fn role(&self) -> Option<Role> {
        match self {
            Owner::Creator => Some(Role::Creator),
            Owner::Runner => Some(Role::Runner),
            Owner::Reviewer => Some(Role::Reviewer),
            Owner::Any => None,
        }
    }
}

/// The owner of `section` in `status`, or `None` when no verb writes it.
///
/// The steps change hands at approval: the creator writes them while the task
/// is being groomed, and the runner once it is approved. Frozen content
/// (intent, scope, criteria) is a phase rule, not an owner rule; it is checked
/// by the verbs that write it.
pub fn owner(section: Section, status: Status) -> Option<Owner> {
    match section {
        Section::Intent
        | Section::Scope
        | Section::Criteria
        | Section::Meta
        | Section::DependsOn
        | Section::Questions => Some(Owner::Creator),
        Section::Steps => Some(match status {
            Status::Draft | Status::Groomed => Owner::Creator,
            _ => Owner::Runner,
        }),
        Section::Claim | Section::Evidence => Some(Owner::Runner),
        Section::Review => Some(Owner::Reviewer),
        Section::FindingOutcome => Some(Owner::Runner),
        Section::Notes | Section::Overlays => Some(Owner::Any),
        Section::Status => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OwnershipError {
    #[error("{} is written by no verb: a transition writes it", .0.as_str())]
    NoOwner(Section),
    #[error("only agents write {}", .0.as_str())]
    NotAnAgent(Section),
    #[error("{} belongs to the {} role", .section.as_str(), .role.as_str())]
    NotOwned { section: Section, role: Role },
}

/// Whether `actor` may write `section` in `status`.
///
/// A section with no owner refuses every writer. A section with an owner
/// takes that role, as an agent; [`Owner::Any`] takes any actor.
pub fn authorize(section: Section, actor: &Actor, status: Status) -> Result<(), OwnershipError> {
    let Some(owner) = owner(section, status) else {
        return Err(OwnershipError::NoOwner(section));
    };
    let Some(role) = owner.role() else {
        return Ok(());
    };
    if actor.kind != ActorKind::Agent {
        return Err(OwnershipError::NotAnAgent(section));
    }
    if actor.role != Some(role) {
        return Err(OwnershipError::NotOwned { section, role });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_section_has_the_owner_the_roles_doc_names() {
        assert_eq!(
            owner(Section::Criteria, Status::Draft),
            Some(Owner::Creator)
        );
        assert_eq!(owner(Section::Claim, Status::Approved), Some(Owner::Runner));
        assert_eq!(
            owner(Section::Evidence, Status::Executing),
            Some(Owner::Runner)
        );
        assert_eq!(
            owner(Section::Review, Status::CodeReview),
            Some(Owner::Reviewer)
        );
        assert_eq!(
            owner(Section::FindingOutcome, Status::Remediating),
            Some(Owner::Runner)
        );
        assert_eq!(owner(Section::Notes, Status::Done), Some(Owner::Any));
        assert_eq!(owner(Section::Overlays, Status::Done), Some(Owner::Any));
    }

    #[test]
    fn steps_pass_from_the_creator_to_the_runner_at_approval() {
        assert_eq!(owner(Section::Steps, Status::Draft), Some(Owner::Creator));
        assert_eq!(owner(Section::Steps, Status::Groomed), Some(Owner::Creator));
        assert_eq!(owner(Section::Steps, Status::Approved), Some(Owner::Runner));
        assert_eq!(
            owner(Section::Steps, Status::Executing),
            Some(Owner::Runner)
        );
    }

    #[test]
    fn the_status_has_no_owner_so_no_verb_writes_it() {
        let runner = Actor::agent("runner-1", Role::Runner).in_session("s");
        assert_eq!(
            authorize(Section::Status, &runner, Status::Executing),
            Err(OwnershipError::NoOwner(Section::Status))
        );
    }

    #[test]
    fn an_owner_less_write_is_refused_for_every_actor() {
        let creator = Actor::agent("creator-1", Role::Creator).in_session("s");
        let human = Actor::human("luiz");
        for actor in [creator, human] {
            assert!(matches!(
                authorize(Section::Status, &actor, Status::Approved),
                Err(OwnershipError::NoOwner(_))
            ));
        }
    }

    #[test]
    fn a_role_writes_only_its_own_sections() {
        let runner = Actor::agent("runner-1", Role::Runner).in_session("s");
        assert!(authorize(Section::Evidence, &runner, Status::Executing).is_ok());
        assert_eq!(
            authorize(Section::Criteria, &runner, Status::Draft),
            Err(OwnershipError::NotOwned {
                section: Section::Criteria,
                role: Role::Creator,
            })
        );
    }

    #[test]
    fn a_human_writes_only_the_sections_with_no_role() {
        let human = Actor::human("luiz");
        assert!(authorize(Section::Notes, &human, Status::Done).is_ok());
        assert!(authorize(Section::Overlays, &human, Status::Done).is_ok());
        assert_eq!(
            authorize(Section::Evidence, &human, Status::Executing),
            Err(OwnershipError::NotAnAgent(Section::Evidence))
        );
    }

    #[test]
    fn an_orchestrator_writes_no_task_section() {
        let orchestrator = Actor::agent("orch-1", Role::Orchestrator).in_session("s");
        assert!(authorize(Section::Notes, &orchestrator, Status::Done).is_ok());
        assert!(matches!(
            authorize(Section::Review, &orchestrator, Status::CodeReview),
            Err(OwnershipError::NotOwned { .. })
        ));
    }
}
