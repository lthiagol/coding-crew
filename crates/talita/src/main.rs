//! talita — Task Administration, Lifecycle & Interaction Terminal Application,
//! the CLI tool of coding-crew. Agent-facing: every call prints one JSON
//! envelope on stdout and exits with the code of its class (ADR 0014).

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
mod session;
mod task;
mod write;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let code = cli::run(args);
    std::process::exit(code);
}
