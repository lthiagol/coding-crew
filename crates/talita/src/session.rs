//! `session new|show` (ADR 0015). Neither writes anything.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
    let session = format!("ses-{}-{actor}-{}", role.as_str(), random_base32());
    let env = format!(
        "{ENV_ACTOR}={actor} {ENV_ROLE}={} {ENV_SESSION}={session}",
        role.as_str()
    );
    Ok(Done::data(json!({ "session": session, "env": env })))
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
