use sorycode_agent::{AgentConfig, AgentRuntime, ApprovalHandler};
use sorycode_core::ids::*;
use sorycode_core::model::Message;
use sorycode_events::EventSink;
use sorycode_providers::Provider;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    Main,
    Planner,
    Researcher,
    Coder,
    Tester,
    Reviewer,
    Fixer,
}

pub struct Orchestrator {
    pub provider: Arc<dyn Provider>,
    pub base_config: AgentConfig,
    pub event_sink: Option<Arc<dyn EventSink>>,
    pub approvals: Option<Arc<dyn ApprovalHandler>>,
}

impl Orchestrator {
    pub fn new(provider: Arc<dyn Provider>, base_config: AgentConfig) -> Self {
        Self {
            provider,
            base_config,
            event_sink: None,
            approvals: None,
        }
    }
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }
    pub fn with_approvals(mut self, approvals: Arc<dyn ApprovalHandler>) -> Self {
        self.approvals = Some(approvals);
        self
    }
    pub fn agent_for(&self, role: AgentRole) -> AgentRuntime {
        let mut cfg = self.base_config.clone();
        cfg.system_prompt = system_prompt_for(role);
        let mut agent = AgentRuntime::new(self.provider.clone(), cfg);
        agent.event_sink = self.event_sink.clone();
        if let Some(a) = &self.approvals {
            agent.approvals = a.clone();
        }
        agent.tool_filter = tools_for_role(role);
        agent
    }
    pub async fn run_main_task(
        &self,
        project_id: Option<ProjectId>,
        session_id: SessionId,
        task_id: TaskId,
        user_message: String,
        working_dir: String,
        context_system: Option<String>,
        history: Vec<Message>,
        cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<String, String> {
        let agent = self.agent_for(AgentRole::Main);
        agent
            .run_task(
                project_id,
                session_id,
                task_id,
                user_message,
                working_dir,
                context_system,
                history,
                cancel,
            )
            .await
    }
}

fn system_prompt_for(role: AgentRole) -> String {
    match role {
        AgentRole::Main => include_str!("../prompts/main.md").to_string(),
        AgentRole::Planner => include_str!("../prompts/planner.md").to_string(),
        AgentRole::Researcher => include_str!("../prompts/researcher.md").to_string(),
        AgentRole::Coder => include_str!("../prompts/coder.md").to_string(),
        AgentRole::Tester => include_str!("../prompts/tester.md").to_string(),
        AgentRole::Reviewer => include_str!("../prompts/reviewer.md").to_string(),
        AgentRole::Fixer => include_str!("../prompts/fixer.md").to_string(),
    }
}

/// Tools each role is allowed to call (enforced by the agent loop, not just
/// prompt guidance). `tool_search` is always allowed so deferred tools stay
/// discoverable.
fn tools_for_role(role: AgentRole) -> Option<std::collections::HashSet<String>> {
    let names: &[&str] = match role {
        AgentRole::Main => return None, // all tools
        AgentRole::Planner => &[
            "list_files",
            "search",
            "read_file",
            "update_plan",
            "tool_search",
            "web_search",
        ],
        AgentRole::Researcher => &[
            "list_files",
            "search",
            "read_file",
            "git_status",
            "git_diff",
            "tool_search",
            "web_search",
        ],
        AgentRole::Coder => &[
            "list_files",
            "search",
            "read_file",
            "edit_file",
            "write_file",
            "apply_patch",
            "terminal",
            "update_plan",
            "tool_search",
        ],
        AgentRole::Tester => &[
            "list_files",
            "search",
            "read_file",
            "edit_file",
            "terminal",
            "run_tests",
            "update_plan",
            "tool_search",
        ],
        AgentRole::Reviewer => &[
            "list_files",
            "search",
            "read_file",
            "git_status",
            "git_diff",
            "tool_search",
            "terminal",
        ],
        AgentRole::Fixer => &[
            "list_files",
            "search",
            "read_file",
            "edit_file",
            "terminal",
            "update_plan",
            "tool_search",
        ],
    };
    Some(names.iter().map(|s| s.to_string()).collect())
}
