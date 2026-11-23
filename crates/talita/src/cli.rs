//! The noun–verb grammar (ADR 0012) and the dispatch to each noun's module.
//! A later noun adds a `Noun` variant and its file.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand};

use crate::envelope::{self, Done};
use crate::error::{Code, Failure};
use crate::identity::{Flags, Resolved};
use crate::parts::{self, StepFields};
use crate::project::{self, Project};
use crate::write::Call;
use crate::{session, task};

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

    /// Refuse with conflict.revision unless the task's revision is N.
    #[arg(long, global = true, value_name = "N")]
    expect_revision: Option<u32>,

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
    /// A task: create it, read it, and write its title, intent and meta.
    Task {
        #[command(subcommand)]
        verb: TaskVerb,
    },
    /// A task's scope: what is in it and what is not.
    Scope {
        #[command(subcommand)]
        verb: ScopeVerb,
    },
    /// A task's acceptance criteria.
    Criterion {
        #[command(subcommand)]
        verb: CriterionVerb,
    },
    /// A task's steps.
    Step {
        #[command(subcommand)]
        verb: StepVerb,
    },
    /// A task's open decisions.
    Question {
        #[command(subcommand)]
        verb: QuestionVerb,
    },
    /// What the work learned.
    Note {
        #[command(subcommand)]
        verb: NoteVerb,
    },
}

#[derive(Debug, Subcommand)]
enum TaskVerb {
    /// A new task in draft; with --parent, the child TSKnn.m.
    Create {
        #[arg(long)]
        title: String,
        /// task, bug or feature.
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, value_name = "TSK")]
        parent: Option<String>,
    },
    /// The task as stored, and what core derives from it.
    Show { id: String },
    /// Task summaries, in id order.
    List {
        #[arg(long = "status")]
        statuses: Vec<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        claimable: bool,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// Title, slug and file name. Draft or groomed.
    Retitle {
        id: String,
        #[arg(long)]
        title: String,
    },
    /// The intent. Draft or groomed.
    Describe {
        id: String,
        #[arg(long)]
        outcome: Option<String>,
        #[arg(long)]
        problem: Option<String>,
    },
    /// urgent, high, normal or low. Before approval.
    Prioritize {
        id: String,
        #[arg(long)]
        priority: String,
    },
    /// S, M or L. Before approval.
    Estimate {
        id: String,
        #[arg(long)]
        effort: String,
    },
    /// Add or remove tags. Before approval.
    Tag {
        id: String,
        #[arg(long)]
        add: Vec<String>,
        #[arg(long)]
        remove: Vec<String>,
    },
    /// Tasks that must be done first.
    Depend {
        id: String,
        #[arg(long, num_args = 1.., value_name = "TSK")]
        on: Vec<String>,
    },
    /// Drop dependencies.
    Undepend {
        id: String,
        #[arg(long, num_args = 1.., value_name = "TSK")]
        on: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ScopeVerb {
    /// Append to scope.in_scope.
    Include {
        id: String,
        #[arg(long)]
        item: String,
    },
    /// Append to scope.out_of_scope.
    Exclude {
        id: String,
        #[arg(long)]
        item: String,
    },
    /// Remove entry n (1-based) of in_scope (--in) or out_of_scope (--out).
    Drop {
        id: String,
        #[arg(long = "in", value_name = "N")]
        inside: Option<usize>,
        #[arg(long = "out", value_name = "N")]
        outside: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
enum CriterionVerb {
    /// A new ACCEPT-nn, pending.
    Add {
        id: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        verification: String,
    },
    /// Its description or verification.
    Revise {
        id: String,
        criterion: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        verification: Option<String>,
    },
    /// Remove it; refused while a step covers it.
    Remove { id: String, criterion: String },
}

#[derive(Debug, clap::Args)]
struct StepArgs {
    #[arg(long)]
    action: Option<String>,
    #[arg(long)]
    done_when: Option<String>,
    #[arg(long, num_args = 1.., value_name = "ACCEPT")]
    covers: Option<Vec<String>>,
    #[arg(long, num_args = 1.., value_name = "PATH")]
    files: Option<Vec<String>>,
    #[arg(long, num_args = 1.., value_name = "STP")]
    after: Option<Vec<String>>,
    /// Required after approval: the step is a deviation.
    #[arg(long)]
    deviation: Option<String>,
}

impl StepArgs {
    fn fields(&self) -> StepFields {
        StepFields {
            action: self.action.clone(),
            done_when: self.done_when.clone(),
            covers: self.covers.clone(),
            files: self.files.clone(),
            after: self.after.clone(),
            deviation: self.deviation.clone(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum StepVerb {
    /// A new STP-nn, last in order.
    Add {
        id: String,
        #[command(flatten)]
        fields: StepArgs,
    },
    /// The given fields.
    Revise {
        id: String,
        step: String,
        #[command(flatten)]
        fields: StepArgs,
    },
    /// Remove it. Draft or groomed; after approval, skip it.
    Remove { id: String, step: String },
}

#[derive(Debug, Subcommand)]
enum QuestionVerb {
    /// A new open QST-nn. Any role.
    Ask {
        id: String,
        #[arg(long)]
        question: String,
    },
    /// Resolve it. The creator.
    Answer {
        id: String,
        question: String,
        #[arg(long)]
        answer: String,
        #[arg(long, value_name = "HUMAN")]
        instructed_by: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum NoteVerb {
    /// A new NOTE-nn. Any role.
    Add {
        id: String,
        #[arg(long)]
        body: String,
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
const VALUE_FLAGS: [&str; 5] = [
    "--project",
    "--as",
    "--role",
    "--session",
    "--expect-revision",
];

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
        Noun::Task { verb } => task_verb(verb, project_flag, identity, talita.expect_revision),
        Noun::Scope { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                ScopeVerb::Include { id, item } => {
                    ("scope include", run(&|c| parts::include(c, id, item)))
                }
                ScopeVerb::Exclude { id, item } => {
                    ("scope exclude", run(&|c| parts::exclude(c, id, item)))
                }
                ScopeVerb::Drop {
                    id,
                    inside,
                    outside,
                } => (
                    "scope drop",
                    run(&|c| parts::drop(c, id, *inside, *outside)),
                ),
            }
        }
        Noun::Criterion { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                CriterionVerb::Add {
                    id,
                    description,
                    verification,
                } => (
                    "criterion add",
                    run(&|c| parts::criterion_add(c, id, description, verification)),
                ),
                CriterionVerb::Revise {
                    id,
                    criterion,
                    description,
                    verification,
                } => (
                    "criterion revise",
                    run(&|c| {
                        parts::criterion_revise(
                            c,
                            id,
                            criterion,
                            description.as_deref(),
                            verification.as_deref(),
                        )
                    }),
                ),
                CriterionVerb::Remove { id, criterion } => (
                    "criterion remove",
                    run(&|c| parts::criterion_remove(c, id, criterion)),
                ),
            }
        }
        Noun::Step { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                StepVerb::Add { id, fields } => (
                    "step add",
                    run(&|c| parts::step_add(c, id, fields.fields())),
                ),
                StepVerb::Revise { id, step, fields } => (
                    "step revise",
                    run(&|c| parts::step_revise(c, id, step, fields.fields())),
                ),
                StepVerb::Remove { id, step } => {
                    ("step remove", run(&|c| parts::step_remove(c, id, step)))
                }
            }
        }
        Noun::Question { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                QuestionVerb::Ask { id, question } => {
                    ("question ask", run(&|c| parts::ask(c, id, question)))
                }
                QuestionVerb::Answer {
                    id,
                    question,
                    answer,
                    instructed_by,
                } => (
                    "question answer",
                    run(&|c| parts::answer(c, id, question, answer, instructed_by.as_deref())),
                ),
            }
        }
        Noun::Note { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                NoteVerb::Add { id, body } => ("note add", run(&|c| parts::note(c, id, body))),
            }
        }
    }
}

type Verb<'a> = &'a dyn Fn(&Call) -> Result<Done, Failure>;

/// A write's runner: the writer's identity, then the project, then the verb.
fn writer<'a>(
    project_flag: Option<&'a std::path::Path>,
    identity: Result<Resolved, Failure>,
    expect_revision: Option<u32>,
) -> impl Fn(Verb) -> Result<Done, Failure> + 'a {
    let actor = identity.and_then(|id| id.writer());
    move |verb| {
        let actor = actor.clone()?;
        let project = project::locate(project_flag)?;
        verb(&Call {
            project: &project,
            actor,
            expect_revision,
        })
    }
}

fn task_verb(
    verb: &TaskVerb,
    project_flag: Option<&std::path::Path>,
    identity: Result<Resolved, Failure>,
    expect_revision: Option<u32>,
) -> (&'static str, Result<Done, Failure>) {
    let read = |f: &dyn Fn(&Project) -> Result<Done, Failure>| {
        project::locate(project_flag).and_then(|p| f(&p))
    };
    let run = writer(project_flag, identity, expect_revision);
    match verb {
        TaskVerb::Create {
            title,
            kind,
            parent,
        } => (
            "task create",
            run(&|c| {
                task::create(
                    c.project,
                    &c.actor,
                    title,
                    kind.as_deref(),
                    parent.as_deref(),
                )
            }),
        ),
        TaskVerb::Show { id } => ("task show", read(&|p| task::show(p, id))),
        TaskVerb::List {
            statuses,
            kind,
            claimable,
            tags,
        } => (
            "task list",
            read(&|p| {
                task::list(
                    p,
                    &task::ListFilter {
                        statuses: statuses.clone(),
                        kind: kind.clone(),
                        claimable: *claimable,
                        tags: tags.clone(),
                    },
                )
            }),
        ),
        TaskVerb::Retitle { id, title } => ("task retitle", run(&|c| task::retitle(c, id, title))),
        TaskVerb::Describe {
            id,
            outcome,
            problem,
        } => (
            "task describe",
            run(&|c| task::describe(c, id, outcome.as_deref(), problem.as_deref())),
        ),
        TaskVerb::Prioritize { id, priority } => (
            "task prioritize",
            run(&|c| task::prioritize(c, id, priority)),
        ),
        TaskVerb::Estimate { id, effort } => {
            ("task estimate", run(&|c| task::estimate(c, id, effort)))
        }
        TaskVerb::Tag { id, add, remove } => ("task tag", run(&|c| task::tag(c, id, add, remove))),
        TaskVerb::Depend { id, on } => ("task depend", run(&|c| task::depend(c, id, on))),
        TaskVerb::Undepend { id, on } => ("task undepend", run(&|c| task::undepend(c, id, on))),
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
