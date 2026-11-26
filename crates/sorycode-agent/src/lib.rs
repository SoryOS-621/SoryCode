use async_trait::async_trait;
use serde_json::{Value, json};
use sorycode_core::ids::*;
use sorycode_core::model::{Message, Role};
use sorycode_core::tool::ToolContext;
use sorycode_events::{EngineEvent, EventEnvelope, EventSink};
use sorycode_providers::{
    ChatRequest, ChatResponse, Provider, ProviderError, ProviderResult, StreamEvent,
    ToolDefinition, Usage,
};
use sorycode_security::{
    ActionRequest, ApprovalInfo, ExecDecision, ExecPolicy, Permission, PolicyDecision,
    PolicyEngine, RiskLevel,
};
use sorycode_tools::ToolRegistry;
use std::sync::Arc;

/// Trigger context compaction when the working message list exceeds this size.
const MAX_CONTEXT_MESSAGES: usize = 40;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub system_prompt: String,
    pub model: String,
    pub max_turns: usize,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    /// When true, writes/exec/network require approval instead of being allowed.
    pub strict_policy: bool,
}
impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are SoryCode, a helpful coding agent.".to_string(),
            model: "gpt-4o-mini".to_string(),
            max_turns: 20,
            temperature: Some(0.2),
            max_tokens: Some(4096),
            strict_policy: false,
        }
    }
}

#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    async fn approve(&self, info: ApprovalInfo) -> bool;
}

pub struct AutoApprove;
#[async_trait]
impl ApprovalHandler for AutoApprove {
    async fn approve(&self, _info: ApprovalInfo) -> bool {
        true
    }
}

pub struct AgentRuntime {
    pub provider: Arc<dyn Provider>,
    pub tools: ToolRegistry,
    pub policy: Arc<dyn PolicyEngine>,
    pub approvals: Arc<dyn ApprovalHandler>,
    pub event_sink: Option<Arc<dyn EventSink>>,
    pub config: AgentConfig,
    /// Cache of approved (tool, args) pairs — approved once, auto-allowed
    /// for identical calls within the same agent run (Codex-style).
    pub approval_cache: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    /// When set, only these tool names are exposed to the model and executable
    /// (per-role restriction). `None` = all tools.
    pub tool_filter: Option<std::collections::HashSet<String>>,
}

impl AgentRuntime {
    pub fn new(provider: Arc<dyn Provider>, config: AgentConfig) -> Self {
        let policy: Arc<dyn PolicyEngine> = if config.strict_policy {
            Arc::new(sorycode_security::DefaultPolicy {
                allow_write: false,
                allow_exec: false,
                allow_network: false,
            })
        } else {
            Arc::new(sorycode_security::DefaultPolicy::default())
        };
        Self {
            provider,
            tools: ToolRegistry::default_registry(),
            policy,
            approvals: Arc::new(AutoApprove),
            event_sink: None,
            config,
            approval_cache: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            tool_filter: None,
        }
    }
    pub async fn emit(&self, event: EngineEvent) {
        if let Some(sink) = &self.event_sink {
            sink.emit(EventEnvelope::new(event)).await;
        }
    }

    /// Collect a streaming chat response, forwarding text deltas as
    /// `MessageDelta` events. The provider reports accumulated tool calls
    /// on the terminal (`done`) event.
    async fn collect_stream(
        &self,
        req: ChatRequest,
        session_id: SessionId,
        task_id: TaskId,
    ) -> ProviderResult<(ChatResponse, bool)> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamEvent>(64);
        let provider = self.provider.clone();
        let handle = tokio::spawn(async move { provider.chat_stream(req, tx).await });
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        while let Some(ev) = rx.recv().await {
            if !ev.delta.is_empty() {
                content.push_str(&ev.delta);
                self.emit(EngineEvent::MessageDelta {
                    session_id,
                    task_id: Some(task_id),
                    delta: ev.delta,
                })
                .await;
            }
            if !ev.tool_calls.is_empty() {
                tool_calls = ev.tool_calls.clone();
            }
            if ev.done {
                break;
            }
        }
        handle
            .await
            .map_err(|e| ProviderError::Api(format!("stream join: {}", e)))??;
        Ok((
            ChatResponse {
                content,
                tool_calls,
                usage: Usage::default(),
                finish_reason: None,
            },
            true,
        ))
    }

    /// Compact the message history when it grows too large: summarize the
    /// middle portion via the provider and keep system + summary + tail.
    /// Falls back to hard truncation if summarization fails.
    async fn compact_context(&self, messages: &mut Vec<Message>) -> ProviderResult<()> {
        const KEEP_TAIL: usize = 24;
        if messages.len() <= KEEP_TAIL + 8 {
            return Ok(());
        }
        let system = messages
            .first()
            .cloned()
            .unwrap_or_else(|| Message::system(""));
        let split = messages.len() - KEEP_TAIL;
        let middle: Vec<Message> = messages[1..split].to_vec();
        let tail: Vec<Message> = messages[split..].to_vec();

        let mut transcript = String::new();
        for m in &middle {
            let content: String = m.content.chars().take(400).collect();
            transcript.push_str(&format!("{:?}: {}\n", m.role, content));
            if transcript.len() > 24_000 {
                transcript.push_str("[...truncated...]\n");
                break;
            }
        }
        let req = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message::system(
                    "You compress conversation history for a coding agent. \
                     Write a dense summary preserving: user goals, decisions, \
                     files modified, tool results that matter, and current state. \
                     Max ~300 words.",
                ),
                Message::user(format!("Summarize:\n\n{}", transcript)),
            ],
            tools: vec![],
            temperature: Some(0.0),
            max_tokens: Some(800),
        };
        match self.provider.chat(req).await {
            Ok(resp) if !resp.content.trim().is_empty() => {
                messages.clear();
                messages.push(system);
                messages.push(Message::user(format!(
                    "[Earlier conversation summary]\n{}",
                    resp.content.trim()
                )));
                messages.extend(tail);
                Ok(())
            }
            Ok(_) => self.truncate_middle(messages, system, middle, tail),
            Err(e) => {
                tracing::warn!("summary call failed, truncating instead: {}", e);
                self.truncate_middle(messages, system, middle, tail)
            }
        }
    }

    /// Fallback compaction: keep each middle message but truncate content.
    fn truncate_middle(
        &self,
        messages: &mut Vec<Message>,
        system: Message,
        middle: Vec<Message>,
        tail: Vec<Message>,
    ) -> ProviderResult<()> {
        messages.clear();
        messages.push(system);
        for mut m in middle {
            if m.content.chars().count() > 300 {
                let truncated: String = m.content.chars().take(300).collect();
                m.content = format!("{}... [truncated]", truncated);
            }
            messages.push(m);
        }
        messages.extend(tail);
        Ok(())
    }

    /// Call the provider with retries on transient failures (network, 429, 5xx).
    /// Streaming is attempted first when supported, with fallback to `chat()`.
    async fn call_provider_with_retry(
        &self,
        req: ChatRequest,
        session_id: SessionId,
        task_id: TaskId,
    ) -> ProviderResult<(ChatResponse, bool)> {
        const MAX_ATTEMPTS: usize = 3;
        let mut last_err: Option<ProviderError> = None;
        for attempt in 0..MAX_ATTEMPTS {
            let result: ProviderResult<(ChatResponse, bool)> = if self.provider.supports_streaming()
            {
                match self.collect_stream(req.clone(), session_id, task_id).await {
                    Ok(pair) => Ok(pair),
                    Err(e) => {
                        tracing::warn!("provider stream failed, falling back to chat(): {}", e);
                        self.provider.chat(req.clone()).await.map(|r| (r, false))
                    }
                }
            } else {
                self.provider.chat(req.clone()).await.map(|r| (r, false))
            };
            match result {
                Ok(v) => return Ok(v),
                Err(e) if attempt + 1 < MAX_ATTEMPTS && is_retryable(&e) => {
                    tracing::warn!(
                        "provider attempt {}/{} failed ({}), retrying",
                        attempt + 1,
                        MAX_ATTEMPTS,
                        e
                    );
                    last_err = Some(e);
                    tokio::time::sleep(std::time::Duration::from_millis(
                        500 * (attempt as u64 + 1),
                    ))
                    .await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| ProviderError::Api("retries exhausted".into())))
    }

    pub async fn run_task(
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
        self.emit(EngineEvent::TaskStarted {
            project_id,
            session_id,
            task_id,
        })
        .await;
        let exec_policy = ExecPolicy::load(std::path::Path::new(&working_dir));
        let mut messages: Vec<Message> = Vec::new();
        let mut sys = self.config.system_prompt.clone();
        sys.push_str("\n\n");
        sys.push_str(&sorycode_context::world_state(std::path::Path::new(
            &working_dir,
        )));
        if let Some(ctx) = context_system {
            sys.push_str("\n\n");
            sys.push_str(&ctx);
        }
        messages.push(Message::system(sys));
        messages.extend(history);
        messages.push(Message::user(user_message));

        // Only Immediate tools are exposed to the model. Deferred tools
        // (e.g. git_status, git_diff, MCP tools) are discovered via
        // `tool_search`; the runtime still executes them once called.
        let tool_defs: Vec<ToolDefinition> = self
            .tools
            .list()
            .iter()
            .filter(|name| {
                let exposure_ok = self
                    .tools
                    .get(name)
                    .map(|t| t.exposure() == sorycode_tools::ToolExposure::Immediate)
                    .unwrap_or(false);
                let filter_ok = self
                    .tool_filter
                    .as_ref()
                    .map(|f| f.contains(**name))
                    .unwrap_or(true);
                exposure_ok && filter_ok
            })
            .map(|name| {
                let desc = self.tools.get(name).map(|t| t.description()).unwrap_or("");
                ToolDefinition {
                    name: name.to_string(),
                    description: desc.to_string(),
                    parameters: default_schema_for(name),
                }
            })
            .collect();

        for turn in 0..self.config.max_turns {
            if cancel
                .as_ref()
                .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                .unwrap_or(false)
            {
                self.emit(EngineEvent::TaskCancelled {
                    session_id,
                    task_id,
                })
                .await;
                return Err("cancelled".into());
            }
            self.emit(EngineEvent::TaskProgress {
                session_id,
                task_id,
                message: format!("turn {}", turn + 1),
            })
            .await;
            if messages.len() > MAX_CONTEXT_MESSAGES {
                if let Err(e) = self.compact_context(&mut messages).await {
                    tracing::warn!("context compaction failed: {}", e);
                }
            }
            let req = ChatRequest {
                model: self.config.model.clone(),
                messages: messages.clone(),
                tools: tool_defs.clone(),
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens,
            };
            let attempt = self
                .call_provider_with_retry(req, session_id, task_id)
                .await;
            let (resp, streamed) = match attempt {
                Ok(v) => v,
                Err(e) => {
                    let err = format!("provider error: {}", e);
                    self.emit(EngineEvent::TaskFailed {
                        session_id,
                        task_id,
                        error: err.clone(),
                    })
                    .await;
                    return Err(err);
                }
            };
            if !streamed && !resp.content.is_empty() {
                self.emit(EngineEvent::MessageDelta {
                    session_id,
                    task_id: Some(task_id),
                    delta: resp.content.clone(),
                })
                .await;
            }
            let mut assistant_msg = Message::assistant(resp.content.clone());
            assistant_msg.tool_calls = resp.tool_calls.clone();
            messages.push(assistant_msg);

            if resp.tool_calls.is_empty() {
                self.emit(EngineEvent::TaskCompleted {
                    session_id,
                    task_id,
                    result: resp.content.clone(),
                })
                .await;
                return Ok(resp.content);
            }

            for tc in &resp.tool_calls {
                if cancel
                    .as_ref()
                    .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
                    .unwrap_or(false)
                {
                    self.emit(EngineEvent::TaskCancelled {
                        session_id,
                        task_id,
                    })
                    .await;
                    return Err("cancelled".into());
                }
                let tool_call_id = ToolCallId::new();
                self.emit(EngineEvent::ToolStarted {
                    session_id,
                    task_id,
                    tool_call_id,
                    tool_name: tc.name.clone(),
                })
                .await;
                if tc.name.trim().is_empty() || !tc.arguments.is_object() {
                    let msg = format!(
                        "invalid tool call: name='{}' arguments must be a JSON object",
                        tc.name
                    );
                    self.emit(EngineEvent::ToolCompleted {
                        session_id,
                        task_id,
                        tool_call_id: ToolCallId::new(),
                        is_error: true,
                        summary: msg.clone(),
                    })
                    .await;
                    messages.push(Message {
                        role: Role::Tool,
                        content: msg,
                        tool_call_id: Some(tc.id.clone()),
                        tool_calls: vec![],
                    });
                    continue;
                }
                if self
                    .tool_filter
                    .as_ref()
                    .map(|f| !f.contains(&tc.name))
                    .unwrap_or(false)
                {
                    let msg = format!("tool '{}' not available for this agent role", tc.name);
                    messages.push(Message {
                        role: Role::Tool,
                        content: msg.clone(),
                        tool_call_id: Some(tc.id.clone()),
                        tool_calls: vec![],
                    });
                    self.emit(EngineEvent::ToolCompleted {
                        session_id,
                        task_id,
                        tool_call_id: ToolCallId::new(),
                        is_error: true,
                        summary: msg,
                    })
                    .await;
                    continue;
                }
                let tool = match self.tools.get(&tc.name) {
                    Some(t) => t,
                    None => {
                        let err = format!("unknown tool: {}", tc.name);
                        self.emit(EngineEvent::ToolCompleted {
                            session_id,
                            task_id,
                            tool_call_id,
                            is_error: true,
                            summary: err.clone(),
                        })
                        .await;
                        messages.push(Message {
                            role: Role::Tool,
                            content: err.clone(),
                            tool_call_id: Some(tc.id.clone()),
                            tool_calls: vec![],
                        });
                        continue;
                    }
                };
                let (perm, risk) = permission_for_tool(&tc.name);
                let action_req = ActionRequest {
                    tool_name: tc.name.clone(),
                    permission: perm,
                    risk,
                    description: format_tool_call(&tc.name, &tc.arguments),
                };
                let ctx = ToolContext {
                    project_id,
                    session_id,
                    task_id,
                    tool_call_id,
                    working_dir: working_dir.clone(),
                };
                // Exec policy (Codex-style): shell commands are checked against
                // allow/deny/prompt rules before the generic permission policy.
                // Deny always wins (even in auto-approve mode); Allow bypasses
                // approval; Prompt falls through to the normal policy.
                let decision = if matches!(tc.name.as_str(), "terminal" | "run_tests") {
                    match tc.arguments.get("command").and_then(|v| v.as_str()) {
                        Some(cmd) => match exec_policy.evaluate(cmd) {
                            ExecDecision::Deny(reason) => PolicyDecision::Deny(reason),
                            ExecDecision::Allow => PolicyDecision::Allow,
                            ExecDecision::Prompt => self.policy.check(&ctx, &action_req).await,
                        },
                        None => self.policy.check(&ctx, &action_req).await,
                    }
                } else {
                    self.policy.check(&ctx, &action_req).await
                };
                match decision {
                    PolicyDecision::Allow => {}
                    PolicyDecision::Deny(reason) => {
                        let msg = format!("denied: {}", reason);
                        self.emit(EngineEvent::ToolCompleted {
                            session_id,
                            task_id,
                            tool_call_id,
                            is_error: true,
                            summary: msg.clone(),
                        })
                        .await;
                        messages.push(Message {
                            role: Role::Tool,
                            content: msg,
                            tool_call_id: Some(tc.id.clone()),
                            tool_calls: vec![],
                        });
                        continue;
                    }
                    PolicyDecision::RequireApproval(info) => {
                        let cache_key = format!(
                            "{}:{}",
                            tc.name,
                            serde_json::to_string(&tc.arguments).unwrap_or_default()
                        );
                        let cached = self
                            .approval_cache
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .contains(&cache_key);
                        if cached {
                            tracing::debug!("approval cache hit for {}", tc.name);
                        } else {
                            self.emit(EngineEvent::ApprovalRequired {
                                session_id,
                                task_id,
                                approval_id: ApprovalId::new(),
                                action: info.action.clone(),
                                reason: info.reason.clone(),
                            })
                            .await;
                            let approved = self.approvals.approve(info).await;
                            if !approved {
                                let msg = "approval rejected".to_string();
                                self.emit(EngineEvent::ToolCompleted {
                                    session_id,
                                    task_id,
                                    tool_call_id,
                                    is_error: true,
                                    summary: msg.clone(),
                                })
                                .await;
                                messages.push(Message {
                                    role: Role::Tool,
                                    content: msg,
                                    tool_call_id: Some(tc.id.clone()),
                                    tool_calls: vec![],
                                });
                                continue;
                            }
                            self.approval_cache
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .insert(cache_key);
                        }
                    }
                }
                let result = tool.execute(tc.arguments.clone(), ctx).await;
                let (content, is_error) = match result {
                    Ok(out) => {
                        self.emit(EngineEvent::ToolOutput {
                            session_id,
                            task_id,
                            tool_call_id,
                            chunk: out.content.clone(),
                        })
                        .await;
                        (out.content, out.is_error)
                    }
                    Err(e) => (format!("tool error: {}", e), true),
                };
                self.emit(EngineEvent::ToolCompleted {
                    session_id,
                    task_id,
                    tool_call_id,
                    is_error,
                    summary: content.chars().take(200).collect(),
                })
                .await;
                messages.push(Message {
                    role: Role::Tool,
                    content,
                    tool_call_id: Some(tc.id.clone()),
                    tool_calls: vec![],
                });
            }
        }
        let err = "max turns exceeded".to_string();
        self.emit(EngineEvent::TaskFailed {
            session_id,
            task_id,
            error: err.clone(),
        })
        .await;
        Err(err)
    }
}

/// Whether a provider error is worth retrying (transient failures only).
fn is_retryable(e: &ProviderError) -> bool {
    match e {
        ProviderError::Http(_) | ProviderError::Timeout => true,
        ProviderError::Api(msg) => {
            let m = msg.to_lowercase();
            // Quota-exhaustion errors are NOT transient: retrying a daily
            // limit just burns attempts and spams the provider.
            if m.contains("per-day")
                || m.contains("per_day")
                || m.contains("daily")
                || m.contains("quota")
                || m.contains("insufficient")
            {
                return false;
            }
            m.contains("429")
                || m.contains("500")
                || m.contains("502")
                || m.contains("503")
                || m.contains("504")
                || m.contains("rate")
                || m.contains("overload")
                || m.contains("timeout")
        }
        _ => false,
    }
}

/// Human-readable tool call for approval prompts:
/// `terminal(command="cargo test", timeout_ms=30000)` instead of raw JSON debug.
fn format_tool_call(name: &str, args: &serde_json::Value) -> String {
    let Some(obj) = args.as_object() else {
        return name.to_string();
    };
    let parts: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
            let val = match v {
                serde_json::Value::String(s) => format!("\"{}\"", s),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                other => {
                    let s = other.to_string();
                    format!("\"{}\"", s.chars().take(60).collect::<String>())
                }
            };
            format!("{}={}", k, val)
        })
        .collect();
    format!("{}({})", name, parts.join(", "))
}

fn permission_for_tool(name: &str) -> (Permission, RiskLevel) {
    match name {
        "read_file" | "list_files" | "search" | "git_status" | "git_diff" | "update_plan"
        | "tool_search" => (Permission::ReadFile, RiskLevel::Low),
        "write_file" | "edit_file" | "apply_patch" => (Permission::WriteFile, RiskLevel::Medium),
        "web_search" | "github_search" => (Permission::NetworkAccess, RiskLevel::Medium),
        "terminal" | "run_tests" => (Permission::ExecuteCommand, RiskLevel::Medium),
        _ => (Permission::ExecuteCommand, RiskLevel::High),
    }
}

fn default_schema_for(name: &str) -> Value {
    let obj =
        |props: Value, req: Vec<&str>| json!({"type":"object","properties":props,"required":req});
    match name {
        "read_file" => obj(
            json!({"path":{"type":"string"},"offset":{"type":"integer"},"limit":{"type":"integer"}}),
            vec!["path"],
        ),
        "write_file" => obj(
            json!({"path":{"type":"string"},"content":{"type":"string"}}),
            vec!["path", "content"],
        ),
        "edit_file" => obj(
            json!({"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"},"replace_all":{"type":"boolean"}}),
            vec!["path", "old_string"],
        ),
        "list_files" => obj(
            json!({"path":{"type":"string"},"max_depth":{"type":"integer"},"limit":{"type":"integer"}}),
            vec![],
        ),
        "search" => obj(
            json!({"pattern":{"type":"string"},"path":{"type":"string"},"glob":{"type":"string"},"limit":{"type":"integer"}}),
            vec!["pattern"],
        ),
        "terminal" => obj(
            json!({"command":{"type":"string"},"timeout_ms":{"type":"integer"},"cwd":{"type":"string"},"session_id":{"type":"string"},"close":{"type":"boolean"}}),
            vec![],
        ),
        "run_tests" => obj(
            json!({"command":{"type":"string"},"timeout_ms":{"type":"integer"}}),
            vec![],
        ),
        "update_plan" => obj(
            json!({"plan":{"type":"array","items":{"type":"object","properties":{"step":{"type":"string"},"status":{"type":"string","enum":["pending","in_progress","completed"]}},"required":["step","status"]}}}),
            vec!["plan"],
        ),
        "web_search" => obj(
            json!({"query":{"type":"string"},"limit":{"type":"integer"}}),
            vec!["query"],
        ),
        "github_search" => obj(
            json!({"query":{"type":"string"},"kind":{"type":"string","enum":["repos","issues"]},"limit":{"type":"integer"}}),
            vec!["query"],
        ),
        "git_status" => obj(json!({}), vec![]),
        "git_diff" => obj(json!({}), vec![]),
        "tool_search" => obj(
            json!({"query":{"type":"string"},"limit":{"type":"integer"}}),
            vec!["query"],
        ),
        "apply_patch" => obj(json!({"patch":{"type":"string"}}), vec!["patch"]),
        _ => obj(json!({}), vec![]),
    }
}
