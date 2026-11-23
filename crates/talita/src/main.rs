//! talita — Task Administration, Lifecycle & Interaction Terminal Application,
//! the CLI tool of coding-crew. Agent-facing: every call prints one JSON
//! envelope on stdout and exits with the code of its class (ADR 0014).

mod cli;
mod envelope;
mod error;
mod identity;
mod project;
mod session;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let code = cli::run(args);
    std::process::exit(code);
}
