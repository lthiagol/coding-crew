//! Shared domain model for `coding-crew`.
//!
//! The model is specified in `docs/tasks/task-model.md`; this crate is its
//! code. Both frontends — talita (CLI) and raul (TUI) — build on it.

pub mod actor;
pub mod clock;
pub mod item;
pub mod lifecycle;
pub mod markdown;
pub mod store;
pub mod task;

pub use actor::{Actor, ActorKind, Role};
pub use clock::{now_rfc3339, parse_rfc3339};
pub use item::{Item, ItemError, ItemKind, ItemStatus};
pub use lifecycle::{Event, Overlay, Overlays, Reason, Status, TransitionContext, TransitionError};
pub use markdown::{escape_literal, task_to_markdown};
pub use store::{Store, StoreError};
pub use task::{
    AcceptanceCriterion, Claim, Comment, CriterionStatus, Deferral, Evidence, Finding,
    FindingStatus, Focus, Intent, Meta, Note, Question, QuestionStatus, Review, Round, RoundKind,
    Scope, Severity, Step, StepStatus, Task, TaskKind, Verdict,
};
