//! Identity resolution (ADR 0015): actor, role, session, model and harness,
//! each from its flag or else its env variable. Flags beat env; nothing
//! defaults.

use coding_crew_core::{Actor, Role};
use serde::Serialize;

use crate::error::{Code, Failure};

pub const ENV_PROJECT: &str = "TALITA_PROJECT";
pub const ENV_ACTOR: &str = "TALITA_ACTOR";
pub const ENV_ROLE: &str = "TALITA_ROLE";
pub const ENV_SESSION: &str = "TALITA_SESSION";
pub const ENV_MODEL: &str = "TALITA_MODEL";
pub const ENV_HARNESS: &str = "TALITA_HARNESS";

/// Where a value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Flag,
    Env,
}

/// One resolved part and its source.
#[derive(Debug, Clone, Serialize)]
pub struct Part {
    pub value: String,
    pub source: Source,
}

/// A flag value, else the env variable. An empty value counts as unset.
pub fn resolve(flag: Option<&str>, env: &str) -> Option<Part> {
    if let Some(value) = flag.filter(|v| !v.is_empty()) {
        return Some(Part {
            value: value.to_string(),
            source: Source::Flag,
        });
    }
    std::env::var(env)
        .ok()
        .filter(|v| !v.is_empty())
        .map(|value| Part {
            value,
            source: Source::Env,
        })
}

/// What `--role` names: an agent role, or `human` (a kind, not a role).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleWord {
    Agent(Role),
    Human,
}

impl RoleWord {
    /// `builder` is read as `runner` (ADR 0012).
    pub fn parse(word: &str) -> Result<Self, Failure> {
        Ok(match word {
            "creator" => Self::Agent(Role::Creator),
            "runner" | "builder" => Self::Agent(Role::Runner),
            "reviewer" => Self::Agent(Role::Reviewer),
            "orchestrator" => Self::Agent(Role::Orchestrator),
            "human" => Self::Human,
            other => {
                return Err(Failure::new(
                    Code::InputInvalid,
                    format!(
                        "role `{other}` is not one of creator, runner, builder, reviewer, orchestrator, human"
                    ),
                ));
            }
        })
    }

    /// The task files' word.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agent(role) => role.as_str(),
            Self::Human => "human",
        }
    }
}

/// The identity flags as given on the command line.
#[derive(Debug, Clone, Default)]
pub struct Flags {
    pub actor: Option<String>,
    pub role: Option<String>,
    pub session: Option<String>,
    pub model: Option<String>,
    pub harness: Option<String>,
}

/// What a call resolves to, part by part. Any part may be missing.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub actor: Option<Part>,
    pub role: Option<(Part, RoleWord)>,
    pub session: Option<Part>,
    pub model: Option<Part>,
    pub harness: Option<Part>,
}

impl Resolved {
    pub fn from_flags(flags: &Flags) -> Result<Self, Failure> {
        let role = resolve(flags.role.as_deref(), ENV_ROLE)
            .map(|part| RoleWord::parse(&part.value).map(|word| (part, word)))
            .transpose()?;
        Ok(Self {
            actor: resolve(flags.actor.as_deref(), ENV_ACTOR),
            role,
            session: resolve(flags.session.as_deref(), ENV_SESSION),
            model: resolve(flags.model.as_deref(), ENV_MODEL),
            harness: resolve(flags.harness.as_deref(), ENV_HARNESS),
        })
    }

    /// The actor of a write. Refuses `identity.missing` for a missing actor
    /// or role, and for an agent without a session. A human's session is not
    /// recorded. `project init` is the one write that needs no identity: it
    /// writes no task and no journal.
    pub fn writer(&self) -> Result<Actor, Failure> {
        let mut missing = Vec::new();
        if self.actor.is_none() {
            missing.push("actor (--as or TALITA_ACTOR)");
        }
        if self.role.is_none() {
            missing.push("role (--role or TALITA_ROLE)");
        }
        let agent = !matches!(self.role, Some((_, RoleWord::Human)));
        if agent && self.session.is_none() {
            missing.push("session (--session or TALITA_SESSION)");
        }
        if !missing.is_empty() {
            return Err(Failure::new(
                Code::IdentityMissing,
                format!("a write needs its {}", missing.join(", ")),
            )
            .remedy("talita session new --as <actor> --role <role>"));
        }
        let (Some(actor), Some((_, word))) = (&self.actor, &self.role) else {
            unreachable!("checked above");
        };
        Ok(match (word, &self.session) {
            (RoleWord::Agent(role), Some(session)) => {
                Actor::agent(&actor.value, *role).in_session(&session.value)
            }
            _ => Actor::human(&actor.value),
        })
    }
}
