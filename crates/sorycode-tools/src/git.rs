use super::{Tool, ToolExposure};
use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};

pub struct GitStatusTool;
#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &'static str {
        "git_status"
    }
    fn description(&self) -> &'static str {
        "Show git status."
    }
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Deferred
    }
    async fn execute(&self, _input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let repo = git2::Repository::open(&context.working_dir)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let statuses = repo
            .statuses(None)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let mut out = Vec::new();
        for s in statuses.iter() {
            out.push(format!("{:?} {}", s.status(), s.path().unwrap_or("?")));
        }
        if out.is_empty() {
            Ok(ToolOutput::ok("clean"))
        } else {
            Ok(ToolOutput::ok(out.join("\n")))
        }
    }
}

pub struct GitDiffTool;
#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &'static str {
        "git_diff"
    }
    fn description(&self) -> &'static str {
        "Show git diff."
    }
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Deferred
    }
    async fn execute(&self, _input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let out = tokio::process::Command::new("git")
            .args(["diff"])
            .current_dir(&context.working_dir)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolOutput::ok(
            String::from_utf8_lossy(&out.stdout).to_string(),
        ))
    }
}
