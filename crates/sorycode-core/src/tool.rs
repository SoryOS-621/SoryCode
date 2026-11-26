use serde::{Deserialize, Serialize};
use crate::ids::{ProjectId, SessionId, TaskId, ToolCallId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub project_id: Option<ProjectId>,
    pub session_id: SessionId,
    pub task_id: TaskId,
    pub tool_call_id: ToolCallId,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
    pub metadata: serde_json::Value,
}

impl ToolOutput {
    pub fn ok(content: impl Into<String>) -> Self { Self { content: content.into(), is_error: false, metadata: serde_json::Value::Null } }
    pub fn err(content: impl Into<String>) -> Self { Self { content: content.into(), is_error: true, metadata: serde_json::Value::Null } }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid input: {0}")] InvalidInput(String),
    #[error("permission denied: {0}")] PermissionDenied(String),
    #[error("not found: {0}")] NotFound(String),
    #[error("io error: {0}")] Io(String),
    #[error("execution failed: {0}")] ExecutionFailed(String),
    #[error("cancelled")] Cancelled,
    #[error("timeout")] Timeout,
}
