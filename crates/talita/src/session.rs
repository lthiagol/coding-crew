//! `session new|show` (ADR 0015). Neither writes anything.

use coding_crew_core::actor::mint_session;
use serde_json::json;

use crate::envelope::Done;
use crate::error::{Code, Failure};
use crate::identity::{ENV_ACTOR, ENV_ROLE, ENV_SESSION, Resolved, RoleWord};

/// `session show`: the identity this call resolves to, and each part's source.
pub fn show(identity: &Resolved) -> Result<Done, Failure> {
    let kind = match identity.role {
        Some((_, RoleWord::Human)) => Some("human"),
        Some(_) => Some("agent"),
        None => None,
    };
    Ok(Done::data(json!({
        "actor": identity.actor,
        "role": identity.role.as_ref().map(|(part, word)| json!({
            "value": word.as_str(),
            "source": part.source,
        })),
        "kind": kind,
        "session": identity.session,
        "model": identity.model,
        "harness": identity.harness,
    })))
}

/// `session new`: mints `ses-<role>-<actor>-<12 random base32>`.
pub fn new(identity: &Resolved) -> Result<Done, Failure> {
    let mut missing = Vec::new();
    if identity.actor.is_none() {
        missing.push("--as");
    }
    if identity.role.is_none() {
        missing.push("--role");
    }
    let (Some(actor), Some((_, role))) = (&identity.actor, &identity.role) else {
        return Err(Failure::new(
            Code::IdentityMissing,
            format!("session new needs {}", missing.join(" and ")),
        )
        .remedy("talita session new --as <actor> --role <role>"));
    };
    let actor = actor.value.as_str();
    if actor.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Failure::new(
            Code::InputInvalid,
            format!("actor `{actor}` holds whitespace"),
        ));
    }
    let session = mint_session(role.as_str(), actor);
    let env = format!(
        "{ENV_ACTOR}={actor} {ENV_ROLE}={} {ENV_SESSION}={session}",
        role.as_str()
    );
    Ok(Done::data(json!({ "session": session, "env": env })))
}
