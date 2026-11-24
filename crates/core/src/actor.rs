use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Who acts on a task. Humans carry no role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub kind: ActorKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

impl Actor {
    pub fn agent(id: impl Into<String>, role: Role) -> Self {
        Self {
            id: id.into(),
            kind: ActorKind::Agent,
            role: Some(role),
            session: None,
        }
    }

    pub fn human(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: ActorKind::Human,
            role: None,
            session: None,
        }
    }

    pub fn in_session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorKind {
    Human,
    Agent,
}

/// The actor roles of the task model: the words a task file holds. `runner` is
/// the role that claims and executes a task; the flow names it `builder`
/// (see [`FlowRole`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Creator,
    Runner,
    Reviewer,
    Orchestrator,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Creator => "creator",
            Role::Runner => "runner",
            Role::Reviewer => "reviewer",
            Role::Orchestrator => "orchestrator",
        }
    }
}

/// A role in the flow: the seat an agent fills in the crew, or the role an
/// order step names. The flow's code-writing role is the **builder**; the task
/// model calls the same role `runner` in task files, and `runner` is still read
/// here so an older configuration or runtime record loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlowRole {
    Creator,
    #[serde(alias = "runner")]
    Builder,
    Reviewer,
    Orchestrator,
}

impl FlowRole {
    /// The word the flow writes and the screens show.
    pub fn as_str(self) -> &'static str {
        match self {
            FlowRole::Creator => "creator",
            FlowRole::Builder => "builder",
            FlowRole::Reviewer => "reviewer",
            FlowRole::Orchestrator => "orchestrator",
        }
    }
}

/// A fresh working session, minted as `talita session new` mints it: writes
/// nothing, and the id is `ses-<role>-<actor>-<12 random base32>` (ADR 0015).
pub fn mint_session(role: &str, actor: &str) -> String {
    format!("ses-{role}-{actor}-{}", random_base32())
}

/// Twelve base32 characters (RFC 4648 alphabet, lower case): 60 random bits.
fn random_base32() -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    // `RandomState` is seeded from the OS's randomness.
    let mut hasher = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    hasher.write_u128(nanos);
    hasher.write_u32(std::process::id());
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    let mut bits = hasher.finish();
    (0..12)
        .map(|_| {
            let c = ALPHABET[(bits & 31) as usize] as char;
            bits >>= 5;
            c
        })
        .collect()
}
