//! Running a tool hugo does not own: the harness the catalogue reads and herdr.
//!
//! A tool is watched, not waited on. One that answers is read; one still
//! running at the bound is killed and reported, so the window never waits on
//! it.

use std::ffi::OsStr;
use std::io::{self, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// How often a run looks at the tool it started.
const POLL: Duration = Duration::from_millis(10);

/// How long past the bound the output of a tool that answered in time is
/// still read. The wait ends at the bound, and a tool that exits just before
/// it leaves its output to be drained: it is not a timeout.
const DRAIN_GRACE: Duration = Duration::from_millis(500);

/// What a tool printed, and how it ended.
pub struct Answer {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Run `tool` with `args`, and read its answer by `bound`.
///
/// The output is held to the bound and a short grace past it: a tool that left
/// a child holding its pipes must not hold the read longer. A tool still
/// running at the bound is killed, and the run answers with a timeout error.
pub fn run<I, S>(tool: &str, args: I, bound: Duration) -> io::Result<Answer>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = Command::new(tool)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // Both pipes are drained beside the wait: a tool that prints more than a
    // pipe holds would block before it exits, and the bound would then kill a
    // run that was only slow to be drained.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + bound;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(timed_out(bound));
            }
            Ok(None) => thread::sleep(POLL),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
    };
    let until = drained_by(deadline);
    Ok(Answer {
        status,
        stdout: received(&stdout, until, bound)?,
        stderr: received(&stderr, until, bound)?,
    })
}

/// When the output of a run is given up on: the bound, and the grace past it.
fn drained_by(deadline: Instant) -> Instant {
    deadline + DRAIN_GRACE
}

fn timed_out(bound: Duration) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!("timed out after {bound:?}"),
    )
}

/// Read a pipe to its end on its own thread, so the wait on the tool is not
/// the wait on its output. The bytes come back over a channel, so the run can
/// give up on them at the bound.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        let _ = sender.send(bytes);
    });
    receiver
}

/// The bytes a drain read, when it finishes by `until`. A drain that ended
/// without sending (its thread failed) reads as no bytes. A drain still
/// reading at `until` is abandoned, not joined: its thread ends with the pipe.
fn received(drain: &Receiver<Vec<u8>>, until: Instant, bound: Duration) -> io::Result<Vec<u8>> {
    match drain.recv_timeout(until.saturating_duration_since(Instant::now())) {
        Ok(bytes) => Ok(bytes),
        Err(RecvTimeoutError::Timeout) => Err(timed_out(bound)),
        Err(RecvTimeoutError::Disconnected) => Ok(Vec::new()),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_run_that_never_answers_is_killed_at_the_bound() {
        let started = Instant::now();
        let error = match run("sleep", ["30"], Duration::from_millis(100)) {
            Ok(_) => panic!("a run that never answers must time out"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(10), "{error}");
    }

    #[test]
    fn output_that_lands_just_past_the_bound_is_still_read() {
        // A tool that answered at the bound has its pipes drained after it:
        // a drain that finishes a moment late is read, not reported as a
        // timeout.
        let (sender, receiver) = mpsc::channel();
        let late = DRAIN_GRACE / 2;
        thread::spawn(move || {
            thread::sleep(late);
            let _ = sender.send(b"late\n".to_vec());
        });
        let deadline = Instant::now();
        let bytes = received(&receiver, drained_by(deadline), Duration::from_secs(1))
            .expect("the late output");
        assert_eq!(bytes, b"late\n");
    }

    #[test]
    fn a_run_that_answers_is_read_with_its_status() {
        let answer = run(
            "sh",
            ["-c", "echo ok; echo oops >&2; exit 3"],
            Duration::from_secs(10),
        )
        .expect("an answer");
        assert_eq!(answer.stdout, b"ok\n");
        assert_eq!(answer.stderr, b"oops\n");
        assert_eq!(answer.status.code(), Some(3));
    }
}
