//! talita — Task Administration, Lifecycle & Interaction Terminal Application,
//! the CLI tool of coding-crew. Agent-facing: every call prints one JSON
//! envelope on stdout and exits with the code of its class (ADR 0014).
//!
//! The binary is a thin caller of [`run`]. The library exists so other crates'
//! tests can drive talita's verbs against a scratch project.

mod claim;
mod cli;
mod envelope;
mod error;
mod evidence;
mod identity;
mod item;
mod journal;
mod lifecycle;
mod parts;
mod project;
mod review;
mod runtime;
mod session;
mod task;
mod write;

use std::ffi::OsString;

/// One talita call: `args` is the command line, program name first. Prints
/// the call's envelope on stdout and returns its exit code.
pub fn run(args: Vec<OsString>) -> i32 {
    cli::run(args)
}
