//! The noun–verb grammar (ADR 0012) and the dispatch to each noun's module.
//! A later noun adds a `Noun` variant and its file.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand};

use crate::envelope::{self, Done};
use crate::error::{Code, Failure};
use crate::identity::{Flags, Resolved};
use crate::{project, session};

/// Task Administration, Lifecycle & Interaction Terminal Application — the CLI
/// tool of coding-crew. Every call prints one JSON object on stdout.
#[derive(Debug, Parser)]
#[command(name = "talita", version, about)]
struct Talita {
    /// Project root [env: TALITA_PROJECT]. Default: the nearest parent holding a coding-crew folder.
    #[arg(long, global = true, value_name = "PATH")]
    project: Option<PathBuf>,

    /// Actor id, e.g. builder-1 [env: TALITA_ACTOR].
    #[arg(long = "as", global = true, value_name = "ACTOR")]
    actor: Option<String>,

    /// creator, runner (or builder), reviewer, orchestrator, or human [env: TALITA_ROLE].
    #[arg(long, global = true, value_name = "ROLE")]
    role: Option<String>,

    /// The working session; required for every agent write [env: TALITA_SESSION].
    #[arg(long, global = true, value_name = "ID")]
    session: Option<String>,

    #[command(subcommand)]
    noun: Noun,
}

#[derive(Debug, Subcommand)]
enum Noun {
    /// The working session: mint one, or show what a call resolves to.
    Session {
        #[command(subcommand)]
        verb: SessionVerb,
    },
    /// The managed project's coding-crew folder.
    Project {
        #[command(subcommand)]
        verb: ProjectVerb,
    },
}

#[derive(Debug, Subcommand)]
enum SessionVerb {
    /// Mint a session id for --as and --role. Writes nothing.
    New,
    /// The identity this call resolves to, and where each part came from.
    Show,
}

#[derive(Debug, Subcommand)]
enum ProjectVerb {
    /// Create <dir>/coding-crew/work/.
    Init {
        /// Directory to initialise. Default: --project, else the current directory.
        #[arg(long, value_name = "DIR")]
        path: Option<PathBuf>,
    },
    /// Paths, counts by status and kind, journal and runtime presence.
    Show,
    /// Report unreadable files, duplicate ids and broken links. Writes nothing.
    Check,
}

/// Global flags that take a value, for naming the command of a usage error.
const VALUE_FLAGS: [&str; 4] = ["--project", "--as", "--role", "--session"];

/// Parse, run, print the envelope; returns the exit code.
pub fn run(args: Vec<OsString>) -> i32 {
    let talita = match Talita::try_parse_from(&args) {
        Ok(talita) => talita,
        Err(error) => {
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return 0;
            }
            let failure = Failure::new(Code::UsageInvalid, error.render().to_string().trim_end())
                .remedy("talita --help");
            return envelope::emit(&called(&args), &Err(failure));
        }
    };
    let (command, result) = dispatch(&talita);
    envelope::emit(command, &result)
}

fn dispatch(talita: &Talita) -> (&'static str, Result<Done, Failure>) {
    let flags = Flags {
        actor: talita.actor.clone(),
        role: talita.role.clone(),
        session: talita.session.clone(),
    };
    let identity = Resolved::from_flags(&flags);
    let project_flag = talita.project.as_deref();
    match &talita.noun {
        Noun::Session { verb } => match verb {
            SessionVerb::New => ("session new", identity.and_then(|id| session::new(&id))),
            SessionVerb::Show => ("session show", identity.and_then(|id| session::show(&id))),
        },
        Noun::Project { verb } => match verb {
            ProjectVerb::Init { path } => (
                "project init",
                identity.and_then(|id| project::init(path.as_deref(), project_flag, &id)),
            ),
            ProjectVerb::Show => (
                "project show",
                project::locate(project_flag).and_then(|p| project::show(&p)),
            ),
            ProjectVerb::Check => (
                "project check",
                project::locate(project_flag).and_then(|p| project::check(&p)),
            ),
        },
    }
}

/// The noun and verb as called, best effort, for a call that did not parse.
fn called(args: &[OsString]) -> String {
    let mut words = Vec::new();
    let mut skip = false;
    for arg in args.iter().skip(1) {
        let arg = arg.to_string_lossy();
        if skip {
            skip = false;
        } else if arg.starts_with('-') {
            skip = VALUE_FLAGS.contains(&arg.as_ref());
        } else {
            words.push(arg.into_owned());
            if words.len() == 2 {
                break;
            }
        }
    }
    words.join(" ")
}
