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
use crate::review::RoundFlags;
use crate::write::{Call, Ran};
use crate::{claim, evidence, item, journal, lifecycle, review, runtime, session, task};

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

    /// The model that runs the call; optional, recorded on its journal events [env: TALITA_MODEL].
    #[arg(long, global = true, value_name = "MODEL")]
    model: Option<String>,

    /// The harness that runs the call; optional, recorded on its journal events [env: TALITA_HARNESS].
    #[arg(long, global = true, value_name = "HARNESS")]
    harness: Option<String>,

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
    /// A task: create it, read it, change its content, and move its lifecycle.
    Task {
        #[command(subcommand)]
        verb: TaskVerb,
    },
    /// A runner's hold on a task, with a lease.
    Claim {
        #[command(subcommand)]
        verb: ClaimVerb,
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
    /// What a criterion's verification shows: the run, and the override.
    Evidence {
        #[command(subcommand)]
        verb: EvidenceVerb,
    },
    /// A review round: the self pre-screen or the reviewer's external round.
    Round {
        #[command(subcommand)]
        verb: RoundVerb,
    },
    /// A defect a round raised.
    Finding {
        #[command(subcommand)]
        verb: FindingVerb,
    },
    /// Discussion under a finding.
    Comment {
        #[command(subcommand)]
        verb: CommentVerb,
    },
    /// A smaller task: an idea, a bug or a small feature (ITM*).
    Item {
        #[command(subcommand)]
        verb: ItemVerb,
    },
    /// The flow's run: start it, stop it, read it, repair it.
    Run {
        #[command(subcommand)]
        verb: RunVerb,
    },
    /// The agents of the flow.
    Crew {
        #[command(subcommand)]
        verb: CrewVerb,
    },
    /// The run's tasks, in the order they are taken.
    Queue {
        #[command(subcommand)]
        verb: QueueVerb,
    },
    /// The project's journal: what happened, oldest first.
    Journal {
        #[command(subcommand)]
        verb: JournalVerb,
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
        /// blocked, deferred or cancelled; may be given more than once.
        #[arg(long = "overlay")]
        overlays: Vec<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        claimable: bool,
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// human or agent.
        #[arg(long)]
        lane: Option<String>,
        /// Only the tasks waiting on a human, with no overlay.
        #[arg(long = "needs-you")]
        needs_you: bool,
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
    /// draft → groomed, when the outcome, a criterion and the out-of-scope are written.
    Groom { id: String },
    /// groomed → approved; sets planned_by and freezes intent, scope and criteria.
    Approve {
        id: String,
        #[arg(long, value_name = "HUMAN")]
        instructed_by: Option<String>,
    },
    /// approved → executing, for the claim's holder.
    Start { id: String },
    /// executing → code-review, for the claim's holder.
    Finish { id: String },
    /// code-review → done.
    PassReview { id: String },
    /// code-review → remediating.
    RequestChanges { id: String },
    /// remediating → code-review.
    ReReview { id: String },
    /// code-review → needs-human, when the review budget is spent.
    Escalate { id: String },
    /// needs-human → executing, on a human's instruction.
    Resume {
        id: String,
        #[arg(long, value_name = "HUMAN")]
        instructed_by: Option<String>,
        #[arg(long)]
        reason: String,
        /// Raise review.max_rounds by N.
        #[arg(long, value_name = "N")]
        grant_rounds: Option<u32>,
    },
    /// done → executing, on a human's instruction.
    Reopen {
        id: String,
        #[arg(long, value_name = "HUMAN")]
        instructed_by: Option<String>,
        #[arg(long)]
        reason: String,
    },
    /// Hold the task; the reason is required.
    Block {
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Clear the blocked overlay.
    Unblock { id: String },
    /// Park the task; the reason is required.
    Defer {
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Clear the deferred overlay.
    Undefer { id: String },
    /// Cancel the task for good; the reason is required.
    Cancel {
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ClaimVerb {
    /// Take the task, or take over an expired claim.
    Take {
        id: String,
        /// 30s, 5m, 2h or 1d. Default 2h.
        #[arg(long)]
        lease: Option<String>,
    },
    /// Extend the holder's lease.
    Renew {
        id: String,
        /// 30s, 5m, 2h or 1d. Default 2h.
        #[arg(long)]
        lease: Option<String>,
    },
    /// Give the claim up; the reason is required.
    Release {
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// The claim and whether its lease is live.
    Show { id: String },
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
    /// Park it on a follow-up task. The working runner.
    Defer {
        id: String,
        criterion: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long, value_name = "TSK")]
        follow_up: Option<String>,
    },
    /// Clear the deferral; the status is derived again.
    Undefer { id: String, criterion: String },
    /// Why a failed criterion failed. Refused unless it is failed.
    Explain {
        id: String,
        criterion: String,
        #[arg(long)]
        failure_reason: Option<String>,
    },
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
    /// Mark it in progress. The working runner.
    Begin { id: String, step: String },
    /// Mark it done; the deviation is optional.
    Done {
        id: String,
        step: String,
        #[arg(long)]
        deviation: Option<String>,
    },
    /// Mark it skipped; the deviation says why.
    Skip {
        id: String,
        step: String,
        #[arg(long)]
        deviation: Option<String>,
    },
    /// Mark it failed; the deviation says why.
    Fail {
        id: String,
        step: String,
        #[arg(long)]
        deviation: Option<String>,
    },
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
enum EvidenceVerb {
    /// Run the criterion's verification and record its exit code.
    Run {
        id: String,
        criterion: String,
        /// 30s, 5m, 2h or 1d. Default 10m.
        #[arg(long)]
        timeout: Option<String>,
        #[arg(long)]
        summary: Option<String>,
    },
    /// Record a manual pass, marked as the bypass (ADR 0019).
    Override {
        id: String,
        criterion: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum RoundVerb {
    /// File the round whole: the self pre-screen or the external review.
    File {
        id: String,
        /// self or external.
        #[arg(long, required_unless_present = "input", conflicts_with = "input")]
        kind: Option<String>,
        /// spec-compliance, edge-cases, security, empirical or design.
        #[arg(long, required_unless_present = "input", conflicts_with = "input")]
        focus: Option<String>,
        /// ok or changes-needed.
        #[arg(long, required_unless_present = "input", conflicts_with = "input")]
        verdict: Option<String>,
        #[arg(long, conflicts_with = "input")]
        notes: Option<String>,
        /// A JSON file holding the whole round; - reads stdin.
        #[arg(long, value_name = "PATH")]
        input: Option<String>,
    },
    /// The rounds, or the one numbered N, each with its findings.
    Show { id: String, n: Option<u32> },
}

#[derive(Debug, Subcommand)]
enum FindingVerb {
    /// A new FIND-nn in the latest external round. The reviewer that filed it.
    Open {
        id: String,
        /// low, medium or high.
        #[arg(long)]
        severity: String,
        #[arg(long)]
        description: String,
    },
    /// Mark it fixed. The remediating runner.
    Resolve {
        id: String,
        finding: String,
        #[arg(long)]
        resolution: String,
    },
    /// Mark it dismissed, keeping the reason.
    Dismiss {
        id: String,
        finding: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Debug, Subcommand)]
enum CommentVerb {
    /// A new CMT-nn under a finding, open or closed. Any agent role; a human.
    Add {
        id: String,
        finding: String,
        #[arg(long)]
        body: String,
    },
}

#[derive(Debug, Subcommand)]
enum ItemVerb {
    /// A new ITMnn, open.
    Create {
        /// idea, feature or bug.
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long = "scope", num_args = 1.., value_name = "TEXT")]
        scope: Vec<String>,
        #[arg(long = "tag", num_args = 1.., value_name = "TAG")]
        tags: Vec<String>,
    },
    /// The item as stored, and whether it may be promoted.
    Show { id: String },
    /// Item summaries, in id order.
    List {
        #[arg(long = "status")]
        statuses: Vec<String>,
        #[arg(long)]
        kind: Option<String>,
    },
    /// idea → feature or bug. The id does not change.
    Shape {
        id: String,
        #[arg(long)]
        kind: String,
        #[arg(long = "scope", num_args = 1.., value_name = "TEXT")]
        scope: Vec<String>,
    },
    /// Dismiss it, keeping the reason.
    Dismiss {
        id: String,
        #[arg(long)]
        reason: String,
    },
    /// Turn it into a task in draft.
    Promote { id: String },
}

#[derive(Debug, Subcommand)]
enum JournalVerb {
    /// The events oldest first, and the count this reader cannot read.
    Show {
        /// Keep one task's events.
        #[arg(long, value_name = "TSK")]
        task: Option<String>,
        /// Keep these kinds; may be given more than once.
        #[arg(long = "kind", value_name = "K")]
        kinds: Vec<String>,
        /// Keep the events at or after this RFC 3339 instant.
        #[arg(long, value_name = "TS")]
        since: Option<String>,
        /// Keep the last N events, still oldest first.
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
enum RunVerb {
    /// A new record: the run begins.
    Start {
        #[arg(long = "task", value_name = "TSK")]
        tasks: Vec<String>,
    },
    /// Stop the run; the reason is required.
    Stop {
        #[arg(long)]
        why: String,
    },
    /// The record, and whether its last write has gone stale.
    Show,
    /// Recompute the derived fields; the repair after a crash.
    Refresh,
}

#[derive(Debug, Subcommand)]
enum CrewVerb {
    /// Add an agent to the crew.
    Join {
        #[arg(id = "crew-actor", value_name = "ACTOR")]
        actor: String,
        /// builder, reviewer or orchestrator.
        #[arg(long)]
        role: String,
        #[arg(long)]
        model: String,
    },
    /// Remove an agent from the crew.
    Leave {
        #[arg(id = "crew-actor", value_name = "ACTOR")]
        actor: String,
    },
    /// Give an agent a task: its hold, its since, and its state.
    Assign {
        #[arg(id = "crew-actor", value_name = "ACTOR")]
        actor: String,
        #[arg(value_name = "TSK")]
        id: String,
    },
    /// The agent is idle; it keeps its hold.
    Idle {
        #[arg(id = "crew-actor", value_name = "ACTOR")]
        actor: String,
    },
    /// The agent cannot go on.
    Stall {
        #[arg(id = "crew-actor", value_name = "ACTOR")]
        actor: String,
    },
}

#[derive(Debug, Subcommand)]
enum QueueVerb {
    /// Append tasks, or insert them before one.
    Add {
        #[arg(required = true, num_args = 1.., value_name = "TSK")]
        ids: Vec<String>,
        #[arg(long, value_name = "TSK")]
        before: Option<String>,
    },
    /// Remove tasks; an id the queue does not hold changes nothing.
    Drop {
        #[arg(required = true, num_args = 1.., value_name = "TSK")]
        ids: Vec<String>,
    },
    /// The queue, with its derived status and claimable flag.
    Show,
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
const VALUE_FLAGS: [&str; 7] = [
    "--project",
    "--as",
    "--role",
    "--session",
    "--model",
    "--harness",
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
    let (command, result) = dispatch(&talita, first_role(&args));
    envelope::emit(command, &result)
}

fn dispatch(talita: &Talita, caller_role: Option<String>) -> (&'static str, Result<Done, Failure>) {
    let flags = Flags {
        actor: talita.actor.clone(),
        role: talita.role.clone(),
        session: talita.session.clone(),
        model: talita.model.clone(),
        harness: talita.harness.clone(),
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
                identity.and_then(|_| project::init(path.as_deref(), project_flag)),
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
        Noun::Claim { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                ClaimVerb::Take { id, lease } => {
                    ("claim take", run(&|c| claim::take(c, id, lease.as_deref())))
                }
                ClaimVerb::Renew { id, lease } => (
                    "claim renew",
                    run(&|c| claim::renew(c, id, lease.as_deref())),
                ),
                ClaimVerb::Release { id, reason } => (
                    "claim release",
                    run(&|c| claim::release(c, id, reason.as_deref())),
                ),
                ClaimVerb::Show { id } => (
                    "claim show",
                    project::locate(project_flag).and_then(|p| claim::show(&p, id)),
                ),
            }
        }
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
                CriterionVerb::Defer {
                    id,
                    criterion,
                    reason,
                    follow_up,
                } => (
                    "criterion defer",
                    run(&|c| {
                        parts::criterion_defer(
                            c,
                            id,
                            criterion,
                            reason.as_deref(),
                            follow_up.as_deref(),
                        )
                    }),
                ),
                CriterionVerb::Undefer { id, criterion } => (
                    "criterion undefer",
                    run(&|c| parts::criterion_undefer(c, id, criterion)),
                ),
                CriterionVerb::Explain {
                    id,
                    criterion,
                    failure_reason,
                } => (
                    "criterion explain",
                    run(&|c| parts::criterion_explain(c, id, criterion, failure_reason.as_deref())),
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
                StepVerb::Begin { id, step } => {
                    ("step begin", run(&|c| parts::step_begin(c, id, step)))
                }
                StepVerb::Done {
                    id,
                    step,
                    deviation,
                } => (
                    "step done",
                    run(&|c| parts::step_done(c, id, step, deviation.as_deref())),
                ),
                StepVerb::Skip {
                    id,
                    step,
                    deviation,
                } => (
                    "step skip",
                    run(&|c| parts::step_skip(c, id, step, deviation.as_deref())),
                ),
                StepVerb::Fail {
                    id,
                    step,
                    deviation,
                } => (
                    "step fail",
                    run(&|c| parts::step_fail(c, id, step, deviation.as_deref())),
                ),
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
        Noun::Evidence { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                EvidenceVerb::Run {
                    id,
                    criterion,
                    timeout,
                    summary,
                } => (
                    "evidence run",
                    run(&|c| {
                        evidence::run(c, id, criterion, timeout.as_deref(), summary.as_deref())
                    }),
                ),
                EvidenceVerb::Override {
                    id,
                    criterion,
                    reason,
                } => (
                    "evidence override",
                    run(&|c| evidence::manual(c, id, criterion, reason.as_deref())),
                ),
            }
        }
        Noun::Round { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                RoundVerb::File {
                    id,
                    kind,
                    focus,
                    verdict,
                    notes,
                    input,
                } => (
                    "round file",
                    run(&|c| {
                        review::round_file(
                            c,
                            id,
                            RoundFlags {
                                kind: kind.as_deref(),
                                focus: focus.as_deref(),
                                verdict: verdict.as_deref(),
                                notes: notes.as_deref(),
                                input: input.as_deref(),
                            },
                        )
                    }),
                ),
                RoundVerb::Show { id, n } => (
                    "round show",
                    project::locate(project_flag).and_then(|p| review::round_show(&p, id, *n)),
                ),
            }
        }
        Noun::Finding { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                FindingVerb::Open {
                    id,
                    severity,
                    description,
                } => (
                    "finding open",
                    run(&|c| review::finding_open(c, id, severity, description)),
                ),
                FindingVerb::Resolve {
                    id,
                    finding,
                    resolution,
                } => (
                    "finding resolve",
                    run(&|c| review::finding_resolve(c, id, finding, resolution)),
                ),
                FindingVerb::Dismiss {
                    id,
                    finding,
                    reason,
                } => (
                    "finding dismiss",
                    run(&|c| review::finding_dismiss(c, id, finding, reason)),
                ),
            }
        }
        Noun::Comment { verb } => {
            let run = writer(project_flag, identity, talita.expect_revision);
            match verb {
                CommentVerb::Add { id, finding, body } => (
                    "comment add",
                    run(&|c| review::comment_add(c, id, finding, body)),
                ),
            }
        }
        Noun::Item { verb } => item_verb(verb, project_flag, identity),
        Noun::Run { verb } => {
            let run = writer(project_flag, identity, None);
            match verb {
                RunVerb::Start { tasks } => ("run start", run(&|c| runtime::start(c, tasks))),
                RunVerb::Stop { why } => ("run stop", run(&|c| runtime::stop(c, why))),
                RunVerb::Show => (
                    "run show",
                    project::locate(project_flag).and_then(|p| runtime::show(&p)),
                ),
                RunVerb::Refresh => ("run refresh", run(&|c| runtime::refresh(c))),
            }
        }
        Noun::Crew { verb } => match verb {
            CrewVerb::Join { actor, role, model } => {
                // The subcommand's `--role` names the crew's role. clap gives
                // both flags one id, so the crew's value wins the global; the
                // caller's own flag is the first one said.
                let caller = Resolved::from_flags(&Flags {
                    role: caller_role,
                    ..flags.clone()
                });
                let run = writer(project_flag, caller, None);
                ("crew join", run(&|c| runtime::join(c, actor, role, model)))
            }
            CrewVerb::Leave { actor } => {
                let run = writer(project_flag, identity, None);
                ("crew leave", run(&|c| runtime::leave(c, actor)))
            }
            CrewVerb::Assign { actor, id } => {
                let run = writer(project_flag, identity, None);
                ("crew assign", run(&|c| runtime::assign(c, actor, id)))
            }
            CrewVerb::Idle { actor } => {
                let run = writer(project_flag, identity, None);
                ("crew idle", run(&|c| runtime::idle(c, actor)))
            }
            CrewVerb::Stall { actor } => {
                let run = writer(project_flag, identity, None);
                ("crew stall", run(&|c| runtime::stall(c, actor)))
            }
        },
        Noun::Queue { verb } => {
            let run = writer(project_flag, identity, None);
            match verb {
                QueueVerb::Add { ids, before } => (
                    "queue add",
                    run(&|c| runtime::queue_add(c, ids, before.as_deref())),
                ),
                QueueVerb::Drop { ids } => ("queue drop", run(&|c| runtime::queue_drop(c, ids))),
                QueueVerb::Show => (
                    "queue show",
                    project::locate(project_flag).and_then(|p| runtime::queue_show(&p)),
                ),
            }
        }
        Noun::Journal { verb } => match verb {
            JournalVerb::Show {
                task,
                kinds,
                since,
                limit,
            } => (
                "journal show",
                project::locate(project_flag).and_then(|project| {
                    journal::show(
                        &project,
                        &journal::Filter {
                            task: task.as_deref(),
                            kinds,
                            since: since.as_deref(),
                            limit: *limit,
                        },
                    )
                }),
            ),
        },
    }
}

type Verb<'a> = &'a dyn Fn(&Call) -> Result<Done, Failure>;

/// A write's runner: the writer's identity, then the project, then the verb.
fn writer<'a>(
    project_flag: Option<&'a std::path::Path>,
    identity: Result<Resolved, Failure>,
    expect_revision: Option<u32>,
) -> impl Fn(Verb) -> Result<Done, Failure> + 'a {
    let identity = identity.and_then(|id| id.writer().map(|actor| (actor, id.model, id.harness)));
    move |verb| {
        let (actor, model, harness) = identity.clone()?;
        let project = project::locate(project_flag)?;
        verb(&Call {
            project: &project,
            actor,
            ran: Ran {
                model: model.map(|part| part.value),
                harness: harness.map(|part| part.value),
            },
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
            run(&|c| task::create(c, title, kind.as_deref(), parent.as_deref())),
        ),
        TaskVerb::Show { id } => ("task show", read(&|p| task::show(p, id))),
        TaskVerb::List {
            statuses,
            overlays,
            kind,
            claimable,
            tags,
            lane,
            needs_you,
        } => (
            "task list",
            read(&|p| {
                task::list(
                    p,
                    &task::ListFilter {
                        statuses: statuses.clone(),
                        overlays: overlays.clone(),
                        kind: kind.clone(),
                        claimable: *claimable,
                        tags: tags.clone(),
                        lane: lane.clone(),
                        needs_you: *needs_you,
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
        TaskVerb::Groom { id } => ("task groom", run(&|c| lifecycle::groom(c, id))),
        TaskVerb::Approve { id, instructed_by } => (
            "task approve",
            run(&|c| lifecycle::approve(c, id, instructed_by.as_deref())),
        ),
        TaskVerb::Start { id } => ("task start", run(&|c| lifecycle::start(c, id))),
        TaskVerb::Finish { id } => ("task finish", run(&|c| lifecycle::finish(c, id))),
        TaskVerb::PassReview { id } => {
            ("task pass-review", run(&|c| lifecycle::pass_review(c, id)))
        }
        TaskVerb::RequestChanges { id } => (
            "task request-changes",
            run(&|c| lifecycle::request_changes(c, id)),
        ),
        TaskVerb::ReReview { id } => ("task re-review", run(&|c| lifecycle::re_review(c, id))),
        TaskVerb::Escalate { id } => ("task escalate", run(&|c| lifecycle::escalate(c, id))),
        TaskVerb::Resume {
            id,
            instructed_by,
            reason,
            grant_rounds,
        } => (
            "task resume",
            run(&|c| lifecycle::resume(c, id, instructed_by.as_deref(), reason, *grant_rounds)),
        ),
        TaskVerb::Reopen {
            id,
            instructed_by,
            reason,
        } => (
            "task reopen",
            run(&|c| lifecycle::reopen(c, id, instructed_by.as_deref(), reason)),
        ),
        TaskVerb::Block { id, reason } => (
            "task block",
            run(&|c| lifecycle::block(c, id, reason.as_deref())),
        ),
        TaskVerb::Unblock { id } => ("task unblock", run(&|c| lifecycle::unblock(c, id))),
        TaskVerb::Defer { id, reason } => (
            "task defer",
            run(&|c| lifecycle::defer(c, id, reason.as_deref())),
        ),
        TaskVerb::Undefer { id } => ("task undefer", run(&|c| lifecycle::undefer(c, id))),
        TaskVerb::Cancel { id, reason } => (
            "task cancel",
            run(&|c| lifecycle::cancel(c, id, reason.as_deref())),
        ),
    }
}

fn item_verb(
    verb: &ItemVerb,
    project_flag: Option<&std::path::Path>,
    identity: Result<Resolved, Failure>,
) -> (&'static str, Result<Done, Failure>) {
    let read = |f: &dyn Fn(&Project) -> Result<Done, Failure>| {
        project::locate(project_flag).and_then(|p| f(&p))
    };
    let run = writer(project_flag, identity, None);
    match verb {
        ItemVerb::Create {
            kind,
            title,
            body,
            scope,
            tags,
        } => (
            "item create",
            run(&|c| item::create(c, kind, title, body.as_deref(), scope, tags)),
        ),
        ItemVerb::Show { id } => ("item show", read(&|p| item::show(p, id))),
        ItemVerb::List { statuses, kind } => (
            "item list",
            read(&|p| item::list(p, statuses, kind.as_deref())),
        ),
        ItemVerb::Shape { id, kind, scope } => {
            ("item shape", run(&|c| item::shape(c, id, kind, scope)))
        }
        ItemVerb::Dismiss { id, reason } => {
            ("item dismiss", run(&|c| item::dismiss(c, id, reason)))
        }
        ItemVerb::Promote { id } => ("item promote", run(&|c| item::promote(c, id))),
    }
}

/// The caller's role on a `crew join` call: its `--role` before the crew
/// row's. A `crew join` always carries the row's flag after the verb, so the
/// first of two is the caller's; one is the row's alone, and the env decides.
fn first_role(args: &[OsString]) -> Option<String> {
    let mut roles = Vec::new();
    let mut args = args.iter().skip(1);
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        if arg == "--role" {
            if let Some(value) = args.next() {
                roles.push(value.to_string_lossy().into_owned());
            }
        } else if let Some(value) = arg.strip_prefix("--role=") {
            roles.push(value.to_string());
        } else if VALUE_FLAGS.contains(&arg.as_ref()) {
            args.next();
        }
    }
    (roles.len() > 1).then(|| roles[0].clone())
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
