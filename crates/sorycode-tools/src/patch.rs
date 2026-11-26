use super::Tool;
use super::filesystem::resolve_confined;
use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};
use std::sync::atomic::{AtomicU64, Ordering};

static PATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct ApplyPatchTool;
#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &'static str {
        "apply_patch"
    }
    fn description(&self) -> &'static str {
        "Apply a unified diff patch via `git apply` inside the working dir. Param: patch (required)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let patch = input
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("patch required".into()))?;
        let _ = resolve_confined(&context.working_dir, ".")?;
        let id = PATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp =
            std::env::temp_dir().join(format!("sorycode_patch_{}_{}.diff", std::process::id(), id));
        tokio::fs::write(&tmp, patch)
            .await
            .map_err(|e| ToolError::Io(e.to_string()))?;
        let result = tokio::process::Command::new("git")
            .args(["apply", "--whitespace=nowarn"])
            .arg(tmp.to_str().unwrap_or(""))
            .current_dir(&context.working_dir)
            .output()
            .await;
        let _ = tokio::fs::remove_file(&tmp).await;
        let out = result.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        if out.status.success() {
            Ok(ToolOutput::ok("patch applied"))
        } else {
            let mut msg = String::from_utf8_lossy(&out.stderr).to_string();
            let stdout = String::from_utf8_lossy(&out.stdout);
            if !stdout.is_empty() {
                msg.push_str("\n");
                msg.push_str(&stdout);
            }
            Ok(ToolOutput::err(msg))
        }
    }
}
