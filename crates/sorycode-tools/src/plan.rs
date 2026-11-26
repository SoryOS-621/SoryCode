use super::Tool;
use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};

/// `update_plan` — Codex-style todo list. The model keeps a structured plan
/// visible to the user; each call replaces the full plan. Statuses:
/// pending | in_progress | completed.
pub struct UpdatePlanTool;

#[async_trait]
impl Tool for UpdatePlanTool {
    fn name(&self) -> &'static str {
        "update_plan"
    }
    fn description(&self) -> &'static str {
        "Update the visible task plan. Param: plan = array of {step: string, status: \"pending\"|\"in_progress\"|\"completed\"}. Call again to revise the whole plan."
    }
    async fn execute(&self, input: Value, _context: ToolContext) -> Result<ToolOutput, ToolError> {
        let steps = input
            .get("plan")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::InvalidInput("plan array required".into()))?;
        if steps.is_empty() {
            return Ok(ToolOutput::err("plan must not be empty"));
        }
        if steps.len() > 30 {
            return Ok(ToolOutput::err("plan too long (max 30 steps)"));
        }
        let mut out = String::from("[plan]\n");
        let mut in_progress = 0;
        for (i, s) in steps.iter().enumerate() {
            let step = s.get("step").and_then(|v| v.as_str()).unwrap_or("");
            let status = s
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            if step.trim().is_empty() {
                return Ok(ToolOutput::err(format!("step {} has empty text", i + 1)));
            }
            let (icon, st) = match status {
                "completed" => ("[x]", "completed"),
                "in_progress" => {
                    in_progress += 1;
                    ("[>]", "in_progress")
                }
                "pending" => ("[ ]", "pending"),
                other => return Ok(ToolOutput::err(format!("invalid status '{}'", other))),
            };
            out.push_str(&format!("{} {} ({})\n", icon, step.trim(), st));
        }
        if in_progress > 1 {
            return Ok(ToolOutput::err("only one step may be in_progress"));
        }
        Ok(ToolOutput::ok(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> ToolContext {
        ToolContext {
            project_id: None,
            session_id: sorycode_core::ids::SessionId::new(),
            task_id: sorycode_core::ids::TaskId::new(),
            tool_call_id: sorycode_core::ids::ToolCallId::new(),
            working_dir: ".".into(),
        }
    }

    #[tokio::test]
    async fn valid_plan_renders() {
        let out = UpdatePlanTool
            .execute(
                json!({"plan": [
                    {"step": "read code", "status": "completed"},
                    {"step": "write fix", "status": "in_progress"},
                    {"step": "test", "status": "pending"}
                ]}),
                ctx(),
            )
            .await
            .unwrap();
        assert!(out.content.contains("[plan]"));
        assert!(out.content.contains("write fix"));
        assert!(!out.is_error);
    }

    #[tokio::test]
    async fn rejects_two_in_progress() {
        let out = UpdatePlanTool
            .execute(
                json!({"plan": [
                    {"step": "a", "status": "in_progress"},
                    {"step": "b", "status": "in_progress"}
                ]}),
                ctx(),
            )
            .await
            .unwrap();
        assert!(out.is_error);
    }

    #[tokio::test]
    async fn rejects_bad_status() {
        let out = UpdatePlanTool
            .execute(json!({"plan": [{"step": "a", "status": "bogus"}]}), ctx())
            .await
            .unwrap();
        assert!(out.is_error);
    }
}
