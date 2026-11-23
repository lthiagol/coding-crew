//! The result envelope: one JSON object on stdout per call (ADR 0014).

use serde::Serialize;
use serde_json::Value;

use crate::error::Failure;

/// What a verb that applied returns.
#[derive(Debug, Default)]
pub struct Done {
    pub data: Value,
    /// Paths written, relative to `coding-crew/`, in write order.
    pub wrote: Vec<String>,
    /// Journal kinds appended.
    pub journal: Vec<String>,
}

impl Done {
    pub fn data(data: Value) -> Self {
        Self {
            data,
            ..Self::default()
        }
    }
}

#[derive(Serialize)]
struct Applied<'a> {
    ok: bool,
    command: &'a str,
    data: &'a Value,
    wrote: &'a [String],
    journal: &'a [String],
}

#[derive(Serialize)]
struct Refused<'a> {
    ok: bool,
    command: &'a str,
    error: &'a Failure,
}

/// Print the envelope and return the exit code.
pub fn emit(command: &str, result: &Result<Done, Failure>) -> i32 {
    let (json, code) = match result {
        Ok(done) => (
            serde_json::to_string(&Applied {
                ok: true,
                command,
                data: &done.data,
                wrote: &done.wrote,
                journal: &done.journal,
            }),
            0,
        ),
        Err(failure) => (
            serde_json::to_string(&Refused {
                ok: false,
                command,
                error: failure,
            }),
            failure.code.exit(),
        ),
    };
    match json {
        Ok(json) => {
            println!("{json}");
            code
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"ok": false, "command": command,
                    "error": {"code": "internal", "message": error.to_string()}})
            );
            1
        }
    }
}
