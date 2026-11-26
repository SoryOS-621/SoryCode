use super::Tool;
use super::filesystem::resolve_confined;
use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

const MAX_OUTPUT: usize = 30_000;
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_TIMEOUT_MS: u64 = 600_000;
const DONE_MARKER: &str = "__SORY_DONE_";

fn truncate(s: String) -> String {
    if s.len() > MAX_OUTPUT {
        format!(
            "{}\n[... truncated at {} bytes]",
            &s[..MAX_OUTPUT],
            MAX_OUTPUT
        )
    } else {
        s
    }
}

struct ShellSession {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Child,
}

/// Persistent shell sessions, Codex `unified_exec` style: commands run inside
/// a long-lived `sh` process identified by `session_id`, so cwd/env/aliases
/// survive between calls. Without `session_id`, a one-shot command is run.
pub struct TerminalTool {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<ShellSession>>>>>,
}

impl Default for TerminalTool {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl TerminalTool {
    async fn get_or_spawn(
        &self,
        id: &str,
        cwd: &std::path::Path,
    ) -> Result<Arc<Mutex<ShellSession>>, ToolError> {
        let mut map = self.sessions.lock().await;
        if let Some(s) = map.get(id) {
            return Ok(s.clone());
        }
        // `exec sh 2>&1` merges stderr into the stdout pipe.
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("exec sh 2>&1")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ToolError::ExecutionFailed("no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ToolError::ExecutionFailed("no stdout".into()))?;
        let session = Arc::new(Mutex::new(ShellSession {
            stdin,
            stdout: BufReader::new(stdout),
            child,
        }));
        map.insert(id.to_string(), session.clone());
        Ok(session)
    }

    async fn run_in_session(
        &self,
        session: &Arc<Mutex<ShellSession>>,
        command: &str,
        timeout_ms: u64,
    ) -> Result<(String, i32, bool), ToolError> {
        let session = session.clone();
        let cmd = command.to_string();
        tokio::time::timeout(Duration::from_millis(timeout_ms), async move {
            let mut sess = session.lock().await;
            // marker guarantees we can detect command completion
            let script = format!("{}\nprintf '\\n{}%s__\\n' \"$?\"\n", cmd, DONE_MARKER);
            sess.stdin
                .write_all(script.as_bytes())
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            sess.stdin
                .flush()
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let mut out = String::new();
            let code: Option<i32>;
            let mut line = String::new();
            loop {
                line.clear();
                let n = sess
                    .stdout
                    .read_line(&mut line)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                if n == 0 {
                    return Err(ToolError::ExecutionFailed("shell session closed".into()));
                }
                if let Some(pos) = line.find(DONE_MARKER) {
                    let rest = &line[pos + DONE_MARKER.len()..];
                    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    code = digits.parse::<i32>().ok();
                    let before = &line[..pos];
                    if !before.trim().is_empty() {
                        out.push_str(before);
                        out.push('\n');
                    }
                    break;
                }
                out.push_str(&line);
                if out.len() > MAX_OUTPUT * 2 {
                    return Err(ToolError::ExecutionFailed("output too large".into()));
                }
            }
            Ok((truncate(out), code.unwrap_or(-1), false))
        })
        .await
        .unwrap_or_else(|_| {
            // timeout: kill the whole session so the next call respawns cleanly
            Ok((String::new(), -1, true))
        })
    }
}

#[async_trait]
impl Tool for TerminalTool {
    fn name(&self) -> &'static str {
        "terminal"
    }
    fn description(&self) -> &'static str {
        "Run a shell command. Params: command (required unless close=true), session_id (persistent shell, cwd/env persist between calls), timeout_ms (default 60000, max 600000), cwd (subdir, first call only), close (true to kill the session)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let close = input
            .get("close")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let session_id = input.get("session_id").and_then(|v| v.as_str());

        if close {
            if let Some(id) = session_id {
                if let Some(sess) = self.sessions.lock().await.remove(id) {
                    let mut s = sess.lock().await;
                    let _ = s.child.kill().await;
                }
                return Ok(ToolOutput::ok(format!("session {} closed", id)));
            }
            return Ok(ToolOutput::err("close=true requires session_id"));
        }

        let command = input.get("command").and_then(|v| v.as_str());
        let Some(command) = command else {
            return Ok(ToolOutput::err("command required (or close=true)"));
        };
        let timeout_ms = input
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(1_000, MAX_TIMEOUT_MS);

        if let Some(id) = session_id {
            let cwd = match input.get("cwd").and_then(|v| v.as_str()) {
                Some(c) => resolve_confined(&context.working_dir, c)?,
                None => std::path::PathBuf::from(&context.working_dir),
            };
            let session = self.get_or_spawn(id, &cwd).await?;
            let (content, code, timed_out) =
                self.run_in_session(&session, command, timeout_ms).await?;
            if timed_out {
                // kill the session: state is unknown
                self.sessions.lock().await.remove(id);
                let sess = session.clone();
                tokio::spawn(async move {
                    let mut s = sess.lock().await;
                    let _ = s.child.kill().await;
                });
                return Ok(ToolOutput::err(format!(
                    "command timed out after {}ms (session {} killed)",
                    timeout_ms, id
                )));
            }
            return Ok(ToolOutput {
                content,
                is_error: code != 0,
                metadata: serde_json::json!({ "exit_code": code, "session_id": id }),
            });
        }

        // one-shot fallback (no persistent session)
        let cwd = std::path::PathBuf::from(&context.working_dir);
        let (code, content, timed_out) = run_shell(command, &cwd, timeout_ms).await?;
        if timed_out {
            return Ok(ToolOutput::err(format!(
                "command timed out after {}ms",
                timeout_ms
            )));
        }
        Ok(ToolOutput {
            content,
            is_error: code != 0,
            metadata: serde_json::json!({ "exit_code": code }),
        })
    }
}

async fn run_shell(
    command: &str,
    cwd: &std::path::Path,
    timeout_ms: u64,
) -> Result<(i32, String, bool), ToolError> {
    let mut child = tokio::process::Command::new("sh")
        .args(["-c", command])
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    // kill_on_drop ensures the child dies if the wait future is dropped on timeout
    let wait = child.wait_with_output();
    tokio::pin!(wait);
    let status = match tokio::time::timeout(Duration::from_millis(timeout_ms), &mut wait).await {
        Ok(res) => res.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
        Err(_) => return Ok((-1, String::new(), true)),
    };
    let mut content = String::from_utf8_lossy(&status.stdout).to_string();
    let stderr = String::from_utf8_lossy(&status.stderr);
    if !stderr.is_empty() {
        content.push_str("\n[stderr]\n");
        content.push_str(&stderr);
    }
    content.push_str(&format!("\n[exit {}]", status.status.code().unwrap_or(-1)));
    Ok((status.status.code().unwrap_or(-1), truncate(content), false))
}

pub struct RunTestsTool;
#[async_trait]
impl Tool for RunTestsTool {
    fn name(&self) -> &'static str {
        "run_tests"
    }
    fn description(&self) -> &'static str {
        "Run tests in the project. Params: command (default \"cargo test\"), timeout_ms (default 300000)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let cmd = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("cargo test");
        let timeout_ms = input
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(300_000)
            .clamp(1_000, MAX_TIMEOUT_MS);
        let cwd = std::path::PathBuf::from(&context.working_dir);
        let (_code, content, timed_out) = run_shell(cmd, &cwd, timeout_ms).await?;
        if timed_out {
            return Ok(ToolOutput::err(format!(
                "tests timed out after {}ms",
                timeout_ms
            )));
        }
        let is_error = _code != 0;
        Ok(ToolOutput {
            content,
            is_error,
            metadata: serde_json::json!({ "exit_code": _code }),
        })
    }
}
