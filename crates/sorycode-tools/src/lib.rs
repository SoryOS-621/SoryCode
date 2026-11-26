use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};

/// Whether a tool's spec is sent to the model on every request.
///
/// `Immediate` tools are always exposed in `ChatRequest.tools`.
/// `Deferred` tools stay hidden until discovered via `tool_search`;
/// they remain executable once the model knows their name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExposure {
    Immediate,
    Deferred,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// Defaults to `Immediate`; override to hide the tool behind `tool_search`.
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Immediate
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError>;
}

pub mod filesystem;
pub mod git;
pub mod github;
pub mod patch;
pub mod plan;
pub mod search;
pub mod terminal;
pub mod tool_search;
pub mod web;

pub use filesystem::*;
pub use git::*;
pub use github::*;
pub use patch::*;
pub use plan::*;
pub use search::*;
pub use terminal::*;
pub use tool_search::*;
pub use web::*;

/// Shared index of deferred tools `(name, description)`, populated by
/// [`ToolRegistry::register`] and consumed by [`ToolSearchTool`].
pub type DeferredToolIndex = std::sync::Arc<std::sync::RwLock<Vec<(String, String)>>>;

pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
    deferred_index: DeferredToolIndex,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Default::default(),
            deferred_index: Default::default(),
        }
    }
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        if tool.exposure() == ToolExposure::Deferred {
            if let Ok(mut idx) = self.deferred_index.write() {
                idx.push((tool.name().to_string(), tool.description().to_string()));
            }
        }
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    /// Handle to the deferred-tool index, for constructing a `ToolSearchTool`.
    pub fn deferred_index(&self) -> DeferredToolIndex {
        self.deferred_index.clone()
    }
    pub fn default_registry() -> Self {
        let mut r = Self::new();
        r.register(ReadFileTool);
        r.register(WriteFileTool);
        r.register(EditFileTool);
        r.register(ListFilesTool);
        r.register(UpdatePlanTool);
        r.register(SearchTool);
        r.register(GitStatusTool);
        r.register(GitDiffTool);
        r.register(ApplyPatchTool);
        r.register(RunTestsTool);
        r.register(TerminalTool::default());
        r.register(WebSearchTool);
        r.register(GitHubSearchTool);
        r.register(ToolSearchTool::new(r.deferred_index()));
        r
    }
}
impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
