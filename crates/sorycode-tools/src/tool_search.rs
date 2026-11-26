use super::{DeferredToolIndex, Tool, ToolExposure};
use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};

/// Lets the model discover deferred tools by name/description substring.
/// Results are returned as a tool message; the model can then call any
/// matching deferred tool directly (the agent executes any registered tool).
pub struct ToolSearchTool {
    index: DeferredToolIndex,
}

impl ToolSearchTool {
    pub fn new(index: DeferredToolIndex) -> Self {
        Self { index }
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &'static str {
        "tool_search"
    }
    fn description(&self) -> &'static str {
        "Search for additional tools that are not listed in this prompt. Params: query (required, substring matched case-insensitively against tool name and description), limit (default 10)."
    }
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Immediate
    }
    async fn execute(&self, input: Value, _context: ToolContext) -> Result<ToolOutput, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("query required".into()))?;
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(10)
            .max(1);
        let q = query.to_lowercase();
        let idx = self
            .index
            .read()
            .map_err(|_| ToolError::ExecutionFailed("tool index poisoned".into()))?;
        let mut out: Vec<String> = Vec::new();
        for (name, desc) in idx.iter() {
            if name.to_lowercase().contains(&q) || desc.to_lowercase().contains(&q) {
                out.push(format!("{}: {}", name, desc));
                if out.len() >= limit {
                    break;
                }
            }
        }
        if out.is_empty() {
            Ok(ToolOutput::ok(format!(
                "no deferred tools matching '{}'",
                query
            )))
        } else {
            Ok(ToolOutput::ok(out.join("\n")))
        }
    }
}
