//! Shared domain model for `coding-crew`.
//!
//! The model is specified in `docs/tasks/task-model.md`; this crate is its
//! code, and it also holds what both TUIs share — the screen's plumbing and
//! the words for a length of time — so there is one implementation of each.

pub mod activity;
pub mod actor;
pub mod bounded;
pub mod catalog;
pub mod claim;
pub mod clock;
pub mod config;
pub mod harness;
pub mod item;
pub mod lifecycle;
pub mod markdown;
pub mod ownership;
pub mod runtime;
pub mod screen;
pub mod store;
pub mod task;

pub use activity::{Activity, ActivityError, ActivityEvent, EventKind};
pub use actor::{Actor, ActorKind, FlowRole, Role};
pub use claim::{ClaimError, DEFAULT_LEASE};
pub use clock::{age_text, now_rfc3339, parse_rfc3339};
pub use config::{Agent, CONFIG_FILE, Config, ConfigError, Gate, OrderStep};
pub use harness::{Harness, Invocation, Models};
pub use item::{Item, ItemError, ItemKind, ItemStatus};
pub use lifecycle::{
    Event, GateFailure, Overlay, Overlays, Reason, Status, TransitionContext, TransitionError,
};
pub use markdown::{escape_literal, item_to_markdown, task_to_markdown};
pub use ownership::{Owner, OwnershipError, Section};
pub use runtime::{
    AgentState, Budget, Composition, CrewMember, QueueItem, Runtime, RuntimeError, RuntimeRound,
    Severities, Stamp,
};
pub use store::{Store, StoreError};
pub use task::{
    AcceptanceCriterion, Claim, Comment, CriterionStatus, Deferral, Evidence, Finding,
    FindingStatus, Focus, Intent, Meta, Note, Question, QuestionStatus, ReReview, Review, Round,
    RoundKind, Scope, Severity, Step, StepStatus, Task, TaskKind, Verdict,
};
