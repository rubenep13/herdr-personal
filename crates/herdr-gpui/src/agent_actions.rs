//! Starting an agent in a space, as the tree offers it. The graphical
//! connection does not admit `agent.start`, so, as fan-out and the
//! orchestrator do, a worker runs the official `herdr` CLI on the space's
//! host, locally or over SSH: `tab create` gives the space a fresh shell, and
//! `agent start` launches the agent in that pane. Nothing here drives the
//! agent's process or types its command into a terminal.
//!
//! A start is bounded three ways: the CLI's own readiness timeout, the
//! output it may print, and an absolute deadline that stops the script even
//! while it keeps making progress. Only a rejection Herdr makes before
//! launching fails a start. Once `agent start` was sent, a timeout, a
//! cancellation, a lost connection, or any other answer leaves the outcome
//! unknown, for the snapshot to settle; it is never retried. The one wait is
//! for the new shell to finish starting, on the very terminal the tab made.

use crate::teleport::{AgentKind, Envelope, Host, PaneInfoResult, TabCreated};
#[cfg(test)]
use herdr_client::ConnectTarget;
use herdr_client::ScriptLimits;
use serde::Deserialize;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

/// How long `agent start` waits for the agent to be ready, in milliseconds.
pub(crate) const READY_TIMEOUT_MS: &str = "30000";
/// The most a step may print before it is stopped.
pub(crate) const MAX_OUTPUT: u64 = 1024 * 1024;
/// The whole start, readiness included, ends by then.
pub(crate) const DEADLINE: Duration = Duration::from_secs(45);
/// How long a new tab's shell may stay busy starting up.
const SHELL_WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(100);

/// Rejections `agent start` makes before launching anything, so a start
/// that meets one certainly did not run (`app/agents.rs` in Herdr).
const BEFORE_LAUNCH: [&str; 8] = [
    "agent_name_taken",
    "agent_pane_busy",
    "agent_pane_not_found",
    "agent_pane_unavailable",
    "invalid_agent_argument",
    "invalid_agent_name",
    "invalid_agent_timeout",
    "unsupported_agent_kind",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    CheckPane,
    CreateTab,
    StartAgent,
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CheckPane => "Checking the new pane",
            Self::CreateTab => "Opening a tab in the space",
            Self::StartAgent => "Starting the agent",
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Starting the agent was cancelled.")]
    Cancelled,
    #[error("The agent was not ready within {} seconds.", .0.as_secs())]
    Deadline(Duration),
    #[error("{step}: Herdr's answer could not be read.")]
    Decode {
        step: Step,
        #[source]
        source: serde_json::Error,
    },
    /// Herdr refused, with the code and message it gave.
    #[error("Herdr refused: {message} ({code}).")]
    Rejected { code: String, message: String },
    #[error("{step} failed.")]
    Script {
        step: Step,
        #[source]
        source: herdr_client::Error,
    },
}

/// What to start, and where: a space by its ID on the host the worker runs
/// against, which the caller resolved from the exact endpoint and session.
pub(crate) struct StartAgent {
    pub(crate) workspace_id: String,
    pub(crate) kind: AgentKind,
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) enum StartOutcome {
    /// Herdr detected the agent, ready, in `pane_id`.
    Ready { pane_id: String },
    /// Nothing was launched.
    Failed(Error),
    /// The launch was sent to `pane_id`; whether the agent runs is for the
    /// snapshot to tell.
    Unknown { pane_id: String, error: Error },
}

/// A Herdr agent name no other start uses: the kind, then random hex, within
/// Herdr's 32 lowercase letters, digits, and dashes.
pub(crate) fn agent_name(kind: AgentKind) -> String {
    const MAX: usize = 32;
    let hex = uuid::Uuid::new_v4().simple().to_string();
    let room = MAX.saturating_sub(kind.name().len() + 1).min(12);
    format!("{}-{}", kind.name(), &hex[..room])
}

/// Runs `request` on `host`. Blocking: call it only on a worker. Setting
/// `cancelled` stops the script in flight.
pub(crate) fn start_agent(
    host: &Host,
    request: &StartAgent,
    cancelled: &AtomicBool,
    deadline: Duration,
) -> StartOutcome {
    let stop = AtomicBool::new(false);
    let expired = AtomicBool::new(false);
    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        let watchdog = scope.spawn(|| {
            let end = Instant::now() + deadline;
            while !done.load(Ordering::Acquire) {
                if cancelled.load(Ordering::Acquire) {
                    stop.store(true, Ordering::Release);
                    return;
                }
                let now = Instant::now();
                if now >= end {
                    expired.store(true, Ordering::Release);
                    stop.store(true, Ordering::Release);
                    return;
                }
                thread::park_timeout((end - now).min(POLL));
            }
        });
        let steps = Steps {
            host,
            stop: &stop,
            expired: &expired,
            deadline,
        };
        let outcome = steps.run(request);
        done.store(true, Ordering::Release);
        watchdog.thread().unpark();
        outcome
    })
}

struct Steps<'a> {
    host: &'a Host,
    stop: &'a AtomicBool,
    expired: &'a AtomicBool,
    deadline: Duration,
}

impl Steps<'_> {
    fn run(&self, request: &StartAgent) -> StartOutcome {
        let created = match self.call::<TabCreated>(
            Step::CreateTab,
            &[
                "tab",
                "create",
                "--workspace",
                &request.workspace_id,
                "--no-focus",
            ],
        ) {
            Ok(created) => created,
            Err(error) => return StartOutcome::Failed(error),
        };
        let pane = created.root_pane.pane_id;
        let terminal = created.root_pane.terminal_id;
        let start = [
            "agent",
            "start",
            &request.name,
            "--kind",
            request.kind.name(),
            "--pane",
            &pane,
            "--timeout",
            READY_TIMEOUT_MS,
        ];
        let begun = Instant::now();
        loop {
            let error = match self.capture(Step::StartAgent, &start) {
                Ok(_) => return StartOutcome::Ready { pane_id: pane },
                Err(error) => error,
            };
            let Error::Rejected { code, .. } = &error else {
                return StartOutcome::Unknown {
                    pane_id: pane,
                    error,
                };
            };
            if !BEFORE_LAUNCH.contains(&code.as_str()) {
                return StartOutcome::Unknown {
                    pane_id: pane,
                    error,
                };
            }
            if code != "agent_pane_busy" || !self.still_starting(&pane, terminal.as_deref(), begun)
            {
                return StartOutcome::Failed(error);
            }
        }
    }

    /// Whether a busy new shell may be waited for once more: within the wait,
    /// and still the terminal the tab was created with, since a public pane
    /// ID can name a replacement.
    fn still_starting(&self, pane: &str, terminal: Option<&str>, begun: Instant) -> bool {
        let Some(terminal) = terminal.filter(|id| !id.is_empty()) else {
            return false;
        };
        let pause = Instant::now() + POLL;
        while Instant::now() < pause {
            if self.stop.load(Ordering::Acquire) {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if begun.elapsed() >= SHELL_WAIT {
            return false;
        }
        self.call::<PaneInfoResult>(Step::CheckPane, &["pane", "get", pane])
            .is_ok_and(|current| {
                current.pane.pane_id == pane
                    && current.pane.terminal_id.as_deref() == Some(terminal)
            })
    }

    fn call<T: for<'de> Deserialize<'de>>(&self, step: Step, args: &[&str]) -> Result<T, Error> {
        let output = self.capture(step, args)?;
        serde_json::from_slice::<Envelope<T>>(&output)
            .map(|envelope| envelope.result)
            .map_err(|source| Error::Decode { step, source })
    }

    fn capture(&self, step: Step, args: &[&str]) -> Result<Vec<u8>, Error> {
        let limits = ScriptLimits {
            output: MAX_OUTPUT,
            idle: self.deadline,
        };
        self.host
            .capture_limited(&Host::herdr_line(args), limits, self.stop)
            .map_err(|source| {
                if self.expired.load(Ordering::Acquire) {
                    Error::Deadline(self.deadline)
                } else if self.stop.load(Ordering::Acquire) {
                    Error::Cancelled
                } else {
                    rejection(&source).unwrap_or(Error::Script { step, source })
                }
            })
    }
}

#[derive(Deserialize)]
struct Rejection {
    error: RejectionBody,
}

#[derive(Deserialize)]
struct RejectionBody {
    code: String,
    #[serde(default)]
    message: String,
}

/// The rejection Herdr's CLI printed, when it exited with one: status 1 and
/// a JSON error on stderr. Anything else, such as `ssh`'s 255, says nothing
/// about what ran.
fn rejection(error: &herdr_client::Error) -> Option<Error> {
    let herdr_client::Error::ScriptExit { status, stderr } = error else {
        return None;
    };
    if status.code() != Some(1) {
        return None;
    }
    let rejection: Rejection = serde_json::from_str(stderr.trim()).ok()?;
    Some(Error::Rejected {
        code: rejection.error.code,
        message: rejection.error.message,
    })
}

mod ui;

pub(crate) use ui::AgentStart;

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used)]
mod tests;

// Host scripts need /bin/sh, as for Teleport's own live test.
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod live_tests;
