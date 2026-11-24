//! Starting the orchestrator: hugo opens a herdr tab, starts the agent the
//! configuration names, and prompts it to run `hugo --skill orchestrator` over
//! the queue. The loop is the orchestrator's; hugo only starts it.
//!
//! herdr is a command, so it is run from a thread the window does not wait
//! on. The launch is a trait: the tests stand a fake in for herdr, and no test
//! runs it.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

use coding_crew_core::actor::{Role, mint_session};
use coding_crew_core::bounded::{self, Answer};

use crate::skill::shell_word;

/// The label of the orchestrator's tab, and the name of its agent in herdr.
pub const ORCHESTRATOR: &str = "orchestrator";

/// The environment talita reads the orchestrator's identity from (ADR 0015).
const ENV_ACTOR: &str = "TALITA_ACTOR";
const ENV_ROLE: &str = "TALITA_ROLE";
const ENV_SESSION: &str = "TALITA_SESSION";

/// The code herdr answers with when no agent has the name asked for.
const AGENT_NOT_FOUND: &str = "agent_not_found";

/// The code herdr answers with when a prompt is sent to an agent already
/// blocked: the prompt is rejected before any input is sent.
const AGENT_BLOCKED: &str = "agent_blocked";

/// How long a herdr call may take before hugo gives up on it. herdr answers
/// from its own socket, so a call that runs past this is stuck, not slow. The
/// one call that waits on its agent has its own bound.
const HERDR_BOUND: Duration = Duration::from_secs(10);

/// How long herdr waits for a started agent to be ready for input.
const AGENT_START_TIMEOUT_MS: u64 = 30_000;

/// How long `herdr agent start` may take: its readiness wait, plus room to
/// report it.
const AGENT_START_BOUND: Duration = Duration::from_secs(35);

/// How long the prompt may take to show the orchestrator working. herdr waits
/// five seconds of a prompt sent from idle for that activity; the rest is
/// the submission.
const PROMPT_TIMEOUT_MS: u64 = 15_000;

/// How long a prompt call may take: the timeout it names, plus room to report.
const PROMPT_BOUND: Duration = Duration::from_secs(20);

/// What a start needs: the agent kind, the invocation that starts the agent
/// in its harness, the project's root, and the queue's task ids in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    /// The agent kind herdr starts the harness as.
    pub kind: String,
    /// The arguments that start the agent, as `herdr agent start` takes them
    /// after `--`.
    pub args: Vec<String>,
    /// The environment the agent's tab is opened with: a harness that reads
    /// its model from it needs it set before the agent starts.
    pub env: Vec<(String, String)>,
    /// The project's root: the folder the tab opens in.
    pub project: PathBuf,
    /// The queue's task ids, in the order the record holds them.
    pub tasks: Vec<String>,
}

/// A started orchestrator: the tab it runs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launched {
    pub tab: String,
}

/// Whether a prompt reached the orchestrator, given herdr's answer to it and
/// the orchestrator's state after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prompted {
    /// The orchestrator took it: it is working on it, or blocked on a dialog
    /// it raised after taking it.
    Taken,
    /// It is still idle, so the prompt did not reach it: it goes once more.
    Again,
    /// Neither: the orchestrator's state is not known.
    Refused,
}

/// The step after a prompt. An accepted prompt is taken, since herdr returns
/// once the agent is working. A prompt rejected as `agent_blocked` was not
/// delivered: the agent was blocked before submission, so it is refused. Any
/// other failed prompt is taken when the agent is working, blocked or done (a
/// timed-out prompt may have reached a harness that then raised a dialog);
/// left idle, it is sent once more; in any other state, or idle after the
/// resend, it is not taken and the error stands. `code` is herdr's error code
/// for the failed prompt, when it gave one.
fn prompted(sent: bool, code: Option<&str>, state: Option<&str>, resent: bool) -> Prompted {
    match (sent, code, state) {
        (true, _, _) => Prompted::Taken,
        (false, Some(AGENT_BLOCKED), _) => Prompted::Refused,
        (false, _, Some("working" | "blocked" | "done")) => Prompted::Taken,
        (false, _, Some("idle")) if !resent => Prompted::Again,
        _ => Prompted::Refused,
    }
}

/// What a start found: an orchestrator started, or one already named in herdr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    Started(Launched),
    AlreadyRunning,
}

/// Where the orchestrator is started. herdr is the one implementation; tests
/// stand a fake in for it.
pub trait Launcher: Send + Sync {
    /// Whether herdr already names an orchestrator agent. An error is herdr
    /// failing to say, not an answer: it is not read as "not named".
    fn orchestrator_named(&self) -> Result<bool>;

    /// Open a tab, start the orchestrator in it, and prompt it.
    fn start_orchestrator(&self, req: &LaunchRequest) -> Result<Launched>;
}

/// The orchestrator's talita identity, for its tab's environment. hugo mints
/// the session at `s` (Q7), so the agent never handles it: the actor is the
/// agent's name in herdr, and the role is the orchestrator's. Each call mints
/// a fresh session.
pub fn orchestrator_identity() -> Vec<(String, String)> {
    vec![
        (ENV_ACTOR.to_string(), ORCHESTRATOR.to_string()),
        (
            ENV_ROLE.to_string(),
            Role::Orchestrator.as_str().to_string(),
        ),
        (
            ENV_SESSION.to_string(),
            mint_session(Role::Orchestrator.as_str(), ORCHESTRATOR),
        ),
    ]
}

/// The launcher hugo uses when it runs inside herdr. `None` outside herdr: the
/// `s` key says why and starts nothing.
pub fn from_env() -> Option<Arc<dyn Launcher>> {
    Herdr::from_env().map(|herdr| Arc::new(herdr) as Arc<dyn Launcher>)
}

/// Start the orchestrator unless herdr already names one. This is what the
/// window's thread runs: the check and the start are both herdr's, so both
/// run off the window's thread.
pub fn start(launcher: &dyn Launcher, req: &LaunchRequest) -> Result<Start> {
    if launcher.orchestrator_named()? {
        return Ok(Start::AlreadyRunning);
    }
    launcher.start_orchestrator(req).map(Start::Started)
}

/// herdr, the session hugo runs in.
pub struct Herdr {
    /// The workspace the tab opens in: `HERDR_WORKSPACE_ID`.
    workspace: String,
}

impl Herdr {
    /// herdr, when hugo runs in a pane herdr manages: `HERDR_ENV` is `1` there.
    pub fn from_env() -> Option<Self> {
        let inside = std::env::var("HERDR_ENV").is_ok_and(|value| value == "1");
        inside.then(|| Self {
            workspace: std::env::var("HERDR_WORKSPACE_ID").unwrap_or_default(),
        })
    }
}

impl Launcher for Herdr {
    fn orchestrator_named(&self) -> Result<bool> {
        let args = words(&["agent", "get", ORCHESTRATOR]);
        let answer = herdr_answer(&args)?;
        if answer.status.success() {
            return Ok(true);
        }
        if not_found(&answer.stdout, &answer.stderr) {
            return Ok(false);
        }
        Err(refused(&args, &answer))
    }

    fn start_orchestrator(&self, req: &LaunchRequest) -> Result<Launched> {
        if self.workspace.is_empty() {
            bail!("HERDR_WORKSPACE_ID is not set");
        }
        let tab = herdr_json(&tab_create_args(&self.workspace, req))?;
        let tab_id = text_at(&tab, &["result", "tab", "tab_id"])?;
        let pane = text_at(&tab, &["result", "root_pane", "pane_id"])?;
        // The tab is open from here on: a failure after it names the tab, so
        // the human can see what was left behind. An agent that started but
        // was not prompted is named in herdr, so `s` will not prompt it: the
        // error carries the prompt to send by hand.
        let after = |error: anyhow::Error| {
            after_tab(
                &tab_id,
                req,
                self.orchestrator_named().unwrap_or(true),
                error,
            )
        };
        herdr_within(&agent_start_args(req, &pane), AGENT_START_BOUND).map_err(after)?;
        self.prompt_orchestrator(req).map_err(after)?;
        Ok(Launched { tab: tab_id })
    }
}

impl Herdr {
    /// Prompt the orchestrator to run the skill, and confirm it took the prompt.
    /// herdr answers from a TUI that may not be ready yet: a prompt it did not
    /// take leaves the agent idle, and is sent once more. The error of a prompt
    /// still not taken stands, and `after_tab` names the prompt to send by hand.
    fn prompt_orchestrator(&self, req: &LaunchRequest) -> Result<()> {
        let args = agent_prompt_args(req);
        let mut resent = false;
        loop {
            let answer = herdr_answer_within(&args, PROMPT_BOUND);
            let accepted = matches!(&answer, Ok(answer) if answer.status.success());
            let code = match &answer {
                Ok(answer) if !accepted => error_code(&answer.stdout, &answer.stderr),
                _ => None,
            };
            let state = if accepted { None } else { self.agent_state() };
            match prompted(accepted, code.as_deref(), state.as_deref(), resent) {
                Prompted::Taken => return Ok(()),
                Prompted::Again => resent = true,
                Prompted::Refused => {
                    return Err(match answer {
                        Ok(answer) => refused(&args, &answer),
                        Err(error) => error,
                    });
                }
            }
        }
    }

    /// The orchestrator's state in herdr, when herdr says it.
    fn agent_state(&self) -> Option<String> {
        let answer = herdr_json(&words(&["agent", "get", ORCHESTRATOR])).ok()?;
        state_of(&answer)
    }
}

/// The state herdr's answer to `agent get` gives the agent.
fn state_of(answer: &Value) -> Option<String> {
    text_at(answer, &["result", "agent", "agent_status"]).ok()
}

/// The error for a start that failed once its tab was open. When herdr may
/// name the orchestrator, the error carries the prompt, since `s` will refuse
/// to start a second one. `named` is true when the check failed too: it is
/// not known that nothing is named.
fn after_tab(tab: &str, req: &LaunchRequest, named: bool, error: anyhow::Error) -> anyhow::Error {
    if named {
        error.context(format!(
            "tab {tab} is open; if the orchestrator is named in herdr, send it the prompt by hand: herdr agent prompt {ORCHESTRATOR} {}",
            shell_word(&prompt(req))
        ))
    } else {
        error.context(format!("tab {tab} is open"))
    }
}

/// The prompt the orchestrator is started with: the skill to run, the project
/// it runs in, and the tasks in order.
pub fn prompt(req: &LaunchRequest) -> String {
    format!(
        "Run `hugo --skill orchestrator` and follow it. Project: {}. Tasks, in order: {}.",
        req.project.display(),
        req.tasks.join(", ")
    )
}

/// The command that opens the orchestrator's tab in the workspace, with the
/// environment the agent starts in.
fn tab_create_args(workspace: &str, req: &LaunchRequest) -> Vec<String> {
    let mut args = words(&["tab", "create", "--workspace", workspace, "--cwd"]);
    args.push(req.project.display().to_string());
    args.extend(words(&["--label", ORCHESTRATOR]));
    for (key, value) in &req.env {
        args.extend(words(&["--env"]));
        args.push(format!("{key}={value}"));
    }
    args
}

/// The command that starts the orchestrator's agent in the tab's pane. It
/// names herdr's readiness timeout, so the call's bound is known to exceed it.
fn agent_start_args(req: &LaunchRequest, pane: &str) -> Vec<String> {
    let mut args = words(&[
        "agent",
        "start",
        ORCHESTRATOR,
        "--kind",
        &req.kind,
        "--pane",
        pane,
        "--timeout",
        &AGENT_START_TIMEOUT_MS.to_string(),
        "--",
    ]);
    args.extend(req.args.iter().cloned());
    args
}

/// The command that prompts the orchestrator to run the skill over the queue.
/// `--wait` with `--until working` or `--until blocked` returns once the agent
/// is seen working or blocked (left idle, it took no prompt), and fails with
/// `agent_prompt_stalled` when neither is seen within herdr's five seconds. The
/// default wait would hold the call for the whole run.
fn agent_prompt_args(req: &LaunchRequest) -> Vec<String> {
    let mut args = words(&["agent", "prompt", ORCHESTRATOR]);
    args.push(prompt(req));
    args.extend(words(&[
        "--wait",
        "--until",
        "working",
        "--until",
        "blocked",
        "--timeout",
        &PROMPT_TIMEOUT_MS.to_string(),
    ]));
    args
}

/// Run herdr with `args`, bounded, and return what it answered. A call that
/// cannot run, or runs past the bound, is an error that names the command.
fn herdr_answer(args: &[String]) -> Result<Answer> {
    herdr_answer_within(args, HERDR_BOUND)
}

/// Run herdr with `args`, held to `bound`, and return what it answered.
fn herdr_answer_within(args: &[String], bound: Duration) -> Result<Answer> {
    bounded::run("herdr", args, bound).with_context(|| format!("herdr {}", command(args)))
}

/// Run herdr with `args`, and return its stdout. A refusal names the command
/// and herdr's own words for why.
fn herdr(args: &[String]) -> Result<Vec<u8>> {
    herdr_within(args, HERDR_BOUND)
}

/// Run herdr with `args`, held to `bound`, and return its stdout.
fn herdr_within(args: &[String], bound: Duration) -> Result<Vec<u8>> {
    let answer = herdr_answer_within(args, bound)?;
    if answer.status.success() {
        return Ok(answer.stdout);
    }
    Err(refused(args, &answer))
}

/// Run herdr and read its answer as JSON.
fn herdr_json(args: &[String]) -> Result<Value> {
    let stdout = herdr(args)?;
    serde_json::from_slice(&stdout)
        .with_context(|| format!("herdr {}: the answer is not JSON", command(args)))
}

/// The error for a herdr call that ran and failed: its command, and herdr's
/// own words for why.
fn refused(args: &[String], answer: &Answer) -> anyhow::Error {
    let stderr = String::from_utf8_lossy(&answer.stderr);
    let stdout = String::from_utf8_lossy(&answer.stdout);
    let why = [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|why| !why.is_empty())
        .unwrap_or("failed");
    anyhow!("herdr {}: {why}", command(args))
}

/// Whether a failed herdr answer is its "no such agent" error. herdr writes
/// its errors as JSON with a code, on stdout or on stderr by version: both
/// are read.
fn not_found(stdout: &[u8], stderr: &[u8]) -> bool {
    error_code(stdout, stderr).is_some_and(|code| code == AGENT_NOT_FOUND)
}

/// The code of herdr's JSON error in a failed answer, read from stderr or
/// stdout by version, whichever holds it.
fn error_code(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    [stderr, stdout].into_iter().find_map(|stream| {
        serde_json::from_slice::<Value>(stream)
            .ok()?
            .get("error")?
            .get("code")?
            .as_str()
            .map(str::to_string)
    })
}

/// The first two words of a herdr command, for an error to name.
fn command(args: &[String]) -> String {
    args.iter().take(2).cloned().collect::<Vec<_>>().join(" ")
}

/// The text at a path in herdr's answer. An answer without it names the path.
fn text_at(value: &Value, path: &[&str]) -> Result<String> {
    path.iter()
        .try_fold(value, |node, key| node.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("herdr answered without {}", path.join(".")))
}

fn words(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// A launcher that records its requests and answers as told: the tests' stand
/// in for herdr.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct FakeLauncher {
    /// Whether herdr names an orchestrator already.
    pub named: bool,
    /// The error the check answers with, when herdr cannot say.
    pub check_error: Option<String>,
    /// The error a start answers with, when it fails.
    pub error: Option<String>,
    /// The requests the starts received, in order.
    pub requests: std::sync::Mutex<Vec<LaunchRequest>>,
}

#[cfg(test)]
impl Launcher for FakeLauncher {
    fn orchestrator_named(&self) -> Result<bool> {
        match &self.check_error {
            Some(error) => bail!("{error}"),
            None => Ok(self.named),
        }
    }

    fn start_orchestrator(&self, req: &LaunchRequest) -> Result<Launched> {
        self.requests
            .lock()
            .expect("the requests")
            .push(req.clone());
        match &self.error {
            Some(error) => bail!("{error}"),
            None => Ok(Launched {
                tab: "w1:t3".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> LaunchRequest {
        LaunchRequest {
            kind: "claude".to_string(),
            args: vec![
                "--model".to_string(),
                "claude-opus-5-5".to_string(),
                "--effort".to_string(),
                "medium".to_string(),
            ],
            env: Vec::new(),
            project: PathBuf::from("/work/project"),
            tasks: vec!["TSK10".to_string(), "TSK29".to_string()],
        }
    }

    #[test]
    fn the_tab_opens_in_the_workspace_at_the_project_and_is_labelled() {
        assert_eq!(
            tab_create_args("w1", &request()),
            [
                "tab",
                "create",
                "--workspace",
                "w1",
                "--cwd",
                "/work/project",
                "--label",
                "orchestrator"
            ]
        );
    }

    #[test]
    fn the_tab_carries_the_environment_its_agent_starts_with() {
        let mut req = request();
        req.env = vec![("OPENCODE_CONFIG_CONTENT".to_string(), "{}".to_string())];
        assert_eq!(
            tab_create_args("w1", &req).last().map(String::as_str),
            Some("OPENCODE_CONFIG_CONTENT={}")
        );
        assert!(
            tab_create_args("w1", &req)
                .windows(2)
                .any(|pair| pair == ["--env", "OPENCODE_CONFIG_CONTENT={}"]),
        );
    }

    #[test]
    fn the_agent_starts_on_the_harness_and_its_model_in_the_tabs_pane() {
        assert_eq!(
            agent_start_args(&request(), "w1:p9"),
            [
                "agent",
                "start",
                "orchestrator",
                "--kind",
                "claude",
                "--pane",
                "w1:p9",
                "--timeout",
                "30000",
                "--",
                "--model",
                "claude-opus-5-5",
                "--effort",
                "medium"
            ]
        );
    }

    #[test]
    fn the_agent_start_bound_outlasts_herdrs_readiness_timeout() {
        // herdr gives up on readiness at its timeout; hugo must not give up on
        // the call first, or a slow harness is killed before it is prompted.
        let args = agent_start_args(&request(), "w1:p9");
        let flag = args
            .iter()
            .position(|arg| arg == "--timeout")
            .expect("a readiness timeout");
        let timeout = Duration::from_millis(args[flag + 1].parse().expect("milliseconds"));
        assert!(AGENT_START_BOUND > timeout, "{timeout:?}");
    }

    #[test]
    fn the_orchestrator_is_minted_a_session_and_named_in_its_tab() {
        // The env hugo builds: the actor and role talita reads, and a session
        // minted as `talita session new` mints it.
        let env = orchestrator_identity();
        let value = |key: &str| {
            env.iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("{key} is not in {env:?}"))
        };
        assert_eq!(value("TALITA_ACTOR"), "orchestrator");
        assert_eq!(value("TALITA_ROLE"), "orchestrator");
        let session = value("TALITA_SESSION");
        let suffix = session
            .strip_prefix("ses-orchestrator-orchestrator-")
            .unwrap_or_else(|| panic!("{session}"));
        assert_eq!(suffix.len(), 12, "{session}");
        assert!(
            suffix.chars().all(|c| matches!(c, 'a'..='z' | '2'..='7')),
            "{session}"
        );
        assert_eq!(env.len(), 3, "{env:?}");
    }

    #[test]
    fn each_start_mints_a_fresh_orchestrator_session() {
        let session = |env: Vec<(String, String)>| {
            env.into_iter()
                .find(|(key, _)| key == "TALITA_SESSION")
                .map(|(_, value)| value)
        };
        assert_ne!(
            session(orchestrator_identity()),
            session(orchestrator_identity())
        );
    }

    #[test]
    fn the_prompt_waits_until_the_orchestrator_is_seen_working_or_blocked() {
        let args = agent_prompt_args(&request());
        assert_eq!(
            args[4..],
            [
                "--wait",
                "--until",
                "working",
                "--until",
                "blocked",
                "--timeout",
                "15000"
            ]
        );
        // The call's bound is the timeout the prompt names, plus room to report.
        assert!(
            PROMPT_BOUND > Duration::from_millis(PROMPT_TIMEOUT_MS),
            "{PROMPT_BOUND:?}"
        );
    }

    #[test]
    fn a_prompt_accepted_is_taken_without_asking_the_state() {
        assert_eq!(prompted(true, None, None, false), Prompted::Taken);
        assert_eq!(prompted(true, None, None, true), Prompted::Taken);
    }

    #[test]
    fn a_prompt_left_idle_is_sent_once_more_and_then_it_is_refused() {
        assert_eq!(prompted(false, None, Some("idle"), false), Prompted::Again);
        assert_eq!(prompted(false, None, Some("idle"), true), Prompted::Refused);
    }

    #[test]
    fn a_prompt_that_failed_is_taken_when_the_orchestrator_is_working_blocked_or_done() {
        assert_eq!(
            prompted(false, None, Some("working"), false),
            Prompted::Taken
        );
        assert_eq!(prompted(false, None, Some("done"), true), Prompted::Taken);
        // A prompt taken into a permission dialog: the orchestrator is running,
        // so a resend would queue the prompt twice.
        assert_eq!(
            prompted(false, None, Some("blocked"), false),
            Prompted::Taken
        );
        assert_eq!(
            prompted(false, None, Some("blocked"), true),
            Prompted::Taken
        );
    }

    #[test]
    fn a_prompt_rejected_as_blocked_before_input_is_refused_even_when_blocked() {
        // herdr rejects a prompt to an agent already blocked before any input
        // is sent: the prompt never reached the orchestrator, so it is not taken.
        assert_eq!(
            prompted(false, Some("agent_blocked"), Some("blocked"), false),
            Prompted::Refused
        );
        assert_eq!(
            prompted(false, Some("agent_blocked"), Some("blocked"), true),
            Prompted::Refused
        );
    }

    #[test]
    fn a_timed_out_prompt_is_taken_when_the_orchestrator_is_blocked() {
        // A timeout does not say the prompt was rejected: it may have been
        // taken before the dialog came up.
        assert_eq!(
            prompted(false, Some("timeout"), Some("blocked"), false),
            Prompted::Taken
        );
        assert_eq!(
            prompted(false, None, Some("blocked"), false),
            Prompted::Taken
        );
    }

    #[test]
    fn a_prompt_is_refused_when_the_orchestrator_state_is_unknown() {
        // The error carries the prompt for the human.
        assert_eq!(
            prompted(false, None, Some("unknown"), false),
            Prompted::Refused
        );
        assert_eq!(prompted(false, None, None, false), Prompted::Refused);
    }

    #[test]
    fn the_error_code_is_read_from_herdrs_json_error_on_either_stream() {
        let blocked =
            br#"{"error":{"code":"agent_blocked","message":"busy"},"id":"cli:agent:prompt"}"#;
        assert_eq!(error_code(blocked, b"").as_deref(), Some("agent_blocked"));
        assert_eq!(error_code(b"", blocked).as_deref(), Some("agent_blocked"));
        assert_eq!(error_code(b"herdr: timed out", b""), None);
        assert_eq!(error_code(br#"{"result":{}}"#, b""), None);
    }

    #[test]
    fn the_state_is_read_from_herdrs_agent_answer() {
        let answer: Value = serde_json::from_str(
            r#"{"result":{"agent":{"name":"orchestrator","agent_status":"idle"}}}"#,
        )
        .expect("an answer");
        assert_eq!(state_of(&answer).as_deref(), Some("idle"));
        let missing: Value = serde_json::from_str(r#"{"result":{"agent":{}}}"#).expect("an answer");
        assert_eq!(state_of(&missing), None);
    }

    #[test]
    fn the_prompt_names_the_skill_the_project_and_the_tasks_in_order() {
        assert_eq!(
            agent_prompt_args(&request())[..3],
            ["agent", "prompt", "orchestrator"]
        );
        let text = prompt(&request());
        assert!(text.contains("`hugo --skill orchestrator`"), "{text}");
        assert!(text.contains("Project: /work/project."), "{text}");
        assert!(text.contains("Tasks, in order: TSK10, TSK29."), "{text}");
    }

    #[test]
    fn the_tab_and_its_pane_are_read_from_herdrs_answer() {
        let answer: Value = serde_json::from_str(
            r#"{"result":{"tab":{"tab_id":"w1:t3"},"root_pane":{"pane_id":"w1:p9"}}}"#,
        )
        .expect("an answer");
        assert_eq!(
            text_at(&answer, &["result", "tab", "tab_id"]).expect("the tab"),
            "w1:t3"
        );
        assert_eq!(
            text_at(&answer, &["result", "root_pane", "pane_id"]).expect("the pane"),
            "w1:p9"
        );
        let error = text_at(&answer, &["result", "workspace", "workspace_id"])
            .expect_err("no workspace in the answer");
        assert!(
            error.to_string().contains("result.workspace.workspace_id"),
            "{error}"
        );
    }

    #[test]
    fn a_start_hands_the_request_to_the_launcher() {
        let fake = FakeLauncher::default();
        let started = start(&fake, &request()).expect("the start");
        assert_eq!(
            started,
            Start::Started(Launched {
                tab: "w1:t3".to_string()
            })
        );
        assert_eq!(*fake.requests.lock().expect("the requests"), [request()]);
    }

    #[test]
    fn an_orchestrator_herdr_already_names_is_not_started_again() {
        let fake = FakeLauncher {
            named: true,
            ..Default::default()
        };
        assert_eq!(
            start(&fake, &request()).expect("the check"),
            Start::AlreadyRunning
        );
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    #[test]
    fn a_start_that_fails_is_returned_with_its_error() {
        let fake = FakeLauncher {
            error: Some("herdr agent start: no pane".to_string()),
            ..Default::default()
        };
        let error = start(&fake, &request()).expect_err("the start fails");
        assert!(error.to_string().contains("no pane"), "{error}");
    }

    #[test]
    fn a_check_herdr_cannot_answer_starts_nothing() {
        // An error from the check is not "not named": a tab opened on a
        // failed check could be a second orchestrator.
        let fake = FakeLauncher {
            check_error: Some("herdr agent get: timed out after 10s".to_string()),
            ..Default::default()
        };
        let error = start(&fake, &request()).expect_err("the check fails");
        assert!(error.to_string().contains("timed out"), "{error}");
        assert!(fake.requests.lock().expect("the requests").is_empty());
    }

    #[test]
    fn only_herdrs_agent_not_found_answer_means_not_named() {
        let missing = br#"{"error":{"code":"agent_not_found","message":"agent target orchestrator not found"},"id":"cli:agent:get"}"#;
        assert!(not_found(missing, b""));
        let other = br#"{"error":{"code":"agent_blocked","message":"busy"},"id":"cli:agent:get"}"#;
        assert!(!not_found(other, b""));
        assert!(!not_found(b"herdr: socket refused", b""));
        assert!(!not_found(b"", b""));
    }

    #[test]
    fn herdrs_agent_not_found_on_stderr_means_not_named() {
        // This machine's herdr writes its errors as JSON on stderr, exit 1,
        // with stdout empty: that is "not named", not a failed check.
        let missing = br#"{"error":{"code":"agent_not_found","message":"agent target orchestrator not found"},"id":"cli:agent:get"}"#;
        assert!(not_found(b"", missing));
    }

    #[test]
    fn a_failed_start_names_the_prompt_when_herdr_may_name_the_orchestrator() {
        // An agent that started but was not prompted is named, and `s` will
        // refuse: the error must carry the prompt to send it by hand.
        let error = after_tab(
            "w1:t3",
            &request(),
            true,
            anyhow!("herdr agent start: timed out"),
        );
        let text = format!("{error:#}");
        assert!(text.starts_with("tab w1:t3 is open; "), "{text}");
        assert!(text.contains(&shell_word(&prompt(&request()))), "{text}");
        assert!(text.contains("herdr agent start: timed out"), "{text}");
    }

    #[test]
    fn a_failed_start_says_only_that_the_tab_is_open_when_nothing_is_named() {
        let error = after_tab(
            "w1:t3",
            &request(),
            false,
            anyhow!("herdr agent start: no pane"),
        );
        let text = format!("{error:#}");
        assert!(text.starts_with("tab w1:t3 is open: "), "{text}");
        assert!(!text.contains("prompt"), "{text}");
    }

    #[test]
    fn a_start_without_a_workspace_runs_nothing() {
        // The check is before any herdr command: with no workspace there is
        // no tab to open, and herdr is not run.
        let herdr = Herdr {
            workspace: String::new(),
        };
        let error = herdr
            .start_orchestrator(&request())
            .expect_err("no workspace");
        assert!(error.to_string().contains("HERDR_WORKSPACE_ID"), "{error}");
    }
}
