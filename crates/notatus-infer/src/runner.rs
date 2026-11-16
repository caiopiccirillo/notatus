//! Running an external model runner process.
//!
//! The runner is started once and kept alive, so a model loads one time and
//! then serves request after request: one JSON line in on stdin, one JSON
//! line out on stdout. Its stderr is kept for error messages.

use crate::{
    ExternalModelSpec, InferenceProtocolError, InferenceRequest, InferenceResponse,
    decode_response, encode_request,
};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use thiserror::Error;

/// How long a request may take. Generous, because the first one usually
/// includes loading the model.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// How many of the runner's last stderr lines are kept for error messages.
const STDERR_LINES: usize = 20;

/// A running model runner process.
pub struct ModelRunner {
    child: Child,
    stdin: ChildStdin,
    responses: Receiver<std::io::Result<String>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    timeout: Duration,
}

impl ModelRunner {
    /// Starts the runner described by `spec`.
    pub fn spawn(spec: &ExternalModelSpec) -> Result<Self, RunnerError> {
        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .envs(&spec.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| RunnerError::Spawn {
                program: spec.program.display().to_string(),
                source,
            })?;

        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let (sender, responses) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let failed = line.is_err();
                if sender.send(line).is_err() || failed {
                    return;
                }
            }
        });

        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_tail = tail.clone();
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut tail = stderr_tail
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                if tail.len() == STDERR_LINES {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
        });

        Ok(Self {
            child,
            stdin,
            responses,
            stderr: tail,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Sets how long [`infer`](Self::infer) waits for each response.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sends `request` and waits for its response. Blank lines the runner
    /// prints are ignored.
    pub fn infer(&mut self, request: &InferenceRequest) -> Result<InferenceResponse, RunnerError> {
        let line = encode_request(request)?;
        if let Err(source) = self
            .stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.flush())
        {
            return Err(self.exited().unwrap_or(RunnerError::Io(source)));
        }

        loop {
            let line = match self.responses.recv_timeout(self.timeout) {
                Ok(Ok(line)) => line,
                Ok(Err(source)) => return Err(RunnerError::Io(source)),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(RunnerError::Timeout {
                        seconds: self.timeout.as_secs(),
                    });
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(self.exited().unwrap_or(RunnerError::Closed {
                        stderr: self.stderr_tail(),
                    }));
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            let response = decode_response(&line).map_err(|source| RunnerError::Response {
                line: shorten(&line),
                source,
            })?;
            if response.request_id != request.request_id {
                return Err(RunnerError::MismatchedResponse {
                    expected: request.request_id.as_str().to_string(),
                    found: response.request_id.as_str().to_string(),
                });
            }
            return Ok(response);
        }
    }

    /// The runner's last lines of stderr, oldest first.
    pub fn stderr_tail(&self) -> Vec<String> {
        self.stderr
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    /// The error to report when the runner has exited, if it has.
    fn exited(&mut self) -> Option<RunnerError> {
        // Give stderr a moment to be read before reporting it.
        let status = (0..20).find_map(|_| match self.child.try_wait() {
            Ok(Some(status)) => Some(status),
            _ => {
                thread::sleep(Duration::from_millis(10));
                None
            }
        })?;
        thread::sleep(Duration::from_millis(20));
        Some(RunnerError::Exited {
            status,
            stderr: self.stderr_tail(),
        })
    }
}

impl Drop for ModelRunner {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn shorten(line: &str) -> String {
    const MAX: usize = 200;
    match line.char_indices().nth(MAX) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line.to_string(),
    }
}

fn stderr_suffix(stderr: &[String]) -> String {
    if stderr.is_empty() {
        String::new()
    } else {
        format!("; it printed:\n{}", stderr.join("\n"))
    }
}

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("could not start model runner {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("model runner I/O failed: {0}")]
    Io(#[source] std::io::Error),
    #[error(transparent)]
    Protocol(#[from] InferenceProtocolError),
    #[error("model runner sent a line that is not a response ({source}): {line}")]
    Response {
        line: String,
        #[source]
        source: InferenceProtocolError,
    },
    #[error("model runner answered request {found} while {expected} was pending")]
    MismatchedResponse { expected: String, found: String },
    #[error("model runner did not answer within {seconds} seconds")]
    Timeout { seconds: u64 },
    #[error("model runner exited with {status}{}", stderr_suffix(stderr))]
    Exited {
        status: ExitStatus,
        stderr: Vec<String>,
    },
    #[error("model runner closed its output{}", stderr_suffix(stderr))]
    Closed { stderr: Vec<String> },
}
