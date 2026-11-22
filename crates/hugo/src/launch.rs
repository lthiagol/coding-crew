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

use crate::bounded::{self, Answer};

/// The label of the orchestrator's tab, and the name of its agent in herdr.
pub const ORCHESTRATOR: &str = "orchestrator";

/// The code herdr answers with when no agent has the name asked for.
const AGENT_NOT_FOUND: &str = "agent_not_found";

/// How long a herdr call may take before hugo gives up on it. herdr answers
/// from its own socket, so a call that runs past this is stuck, not slow. The
/// one call that waits on its agent has its own bound.
const HERDR_BOUND: Duration = Duration::from_secs(10);

/// How long herdr waits for a started agent to be ready for input.
const AGENT_START_TIMEOUT_MS: u64 = 30_000;

/// How long `herdr agent start` may take: its readiness wait, plus room to
/// report it.
const AGENT_START_BOUND: Duration = Duration::from_secs(35);

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
        if not_found(&answer.stdout) {
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
        // error says how to send the prompt by hand.
        herdr_within(&agent_start_args(req, &pane), AGENT_START_BOUND)
            .with_context(|| format!("tab {tab_id} is open"))?;
        herdr(&agent_prompt_args(req)).with_context(|| {
            format!("tab {tab_id} is open; prompt it with `herdr agent prompt {ORCHESTRATOR}`")
        })?;
        Ok(Launched { tab: tab_id })
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
fn agent_prompt_args(req: &LaunchRequest) -> Vec<String> {
    let mut args = words(&["agent", "prompt", ORCHESTRATOR]);
    args.push(prompt(req));
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

/// Whether herdr's answer is its "no such agent" error. herdr writes its
/// errors as JSON on stdout, with a code.
fn not_found(stdout: &[u8]) -> bool {
    serde_json::from_slice::<Value>(stdout)
        .is_ok_and(|answer| answer["error"]["code"] == AGENT_NOT_FOUND)
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
        assert!(not_found(missing));
        let other = br#"{"error":{"code":"agent_blocked","message":"busy"},"id":"cli:agent:get"}"#;
        assert!(!not_found(other));
        assert!(!not_found(b"herdr: socket refused"));
        assert!(!not_found(b""));
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
