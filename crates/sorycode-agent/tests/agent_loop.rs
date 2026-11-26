//! Integration tests for `AgentRuntime::run_task` using a mock provider.
//! No network access, no real API keys.

use async_trait::async_trait;
use serde_json::json;
use sorycode_agent::{AgentConfig, AgentRuntime, ApprovalHandler};
use sorycode_core::ids::{SessionId, TaskId};
use sorycode_core::model::{Message, Role, ToolCallRequest};
use sorycode_events::{EngineEvent, EventEnvelope, EventSink};
use sorycode_providers::{
    ChatRequest, ChatResponse, Provider, ProviderError, ProviderResult, Usage,
};
use sorycode_security::ApprovalInfo;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Mock provider
// ---------------------------------------------------------------------------

/// A scripted provider: each `chat()` call pops the next queued response.
/// Every request is recorded so tests can inspect the message history the
/// agent sent (e.g. to verify tool results were fed back to the model).
struct MockProvider {
    responses: Mutex<VecDeque<ProviderResult<ChatResponse>>>,
    requests: Mutex<Vec<ChatRequest>>,
    /// If set, the flag is raised inside `chat()` before returning — used to
    /// simulate cancellation arriving mid-run.
    set_flag_on_call: Option<Arc<AtomicBool>>,
}

impl MockProvider {
    fn new(responses: Vec<ProviderResult<ChatResponse>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
            set_flag_on_call: None,
        }
    }

    fn with_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.set_flag_on_call = Some(flag);
        self
    }

    fn call_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    /// All messages sent in the most recent provider request.
    fn last_request_messages(&self) -> Vec<Message> {
        self.requests
            .lock()
            .unwrap()
            .last()
            .map(|r| r.messages.clone())
            .unwrap_or_default()
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn chat(&self, req: ChatRequest) -> ProviderResult<ChatResponse> {
        self.requests.lock().unwrap().push(req);
        if let Some(flag) = &self.set_flag_on_call {
            flag.store(true, Ordering::Relaxed);
        }
        match self.responses.lock().unwrap().pop_front() {
            Some(resp) => resp,
            None => Err(ProviderError::Api(
                "mock: no more scripted responses".into(),
            )),
        }
    }
}

fn ok_response(content: &str) -> ProviderResult<ChatResponse> {
    Ok(ChatResponse {
        content: content.to_string(),
        tool_calls: vec![],
        usage: Usage::default(),
        finish_reason: Some("stop".into()),
    })
}

fn tool_call_response(name: &str, arguments: serde_json::Value) -> ProviderResult<ChatResponse> {
    Ok(ChatResponse {
        content: String::new(),
        tool_calls: vec![ToolCallRequest {
            id: format!("call-{}", name),
            name: name.to_string(),
            arguments,
        }],
        usage: Usage::default(),
        finish_reason: Some("tool_calls".into()),
    })
}

// ---------------------------------------------------------------------------
// Test doubles: approval handler + event sink
// ---------------------------------------------------------------------------

struct RejectAll;

#[async_trait]
impl ApprovalHandler for RejectAll {
    async fn approve(&self, _info: ApprovalInfo) -> bool {
        false
    }
}

#[derive(Clone, Default)]
struct RecordingSink {
    events: Arc<Mutex<Vec<EngineEvent>>>,
}

#[async_trait]
impl EventSink for RecordingSink {
    async fn emit(&self, envelope: EventEnvelope) {
        self.events.lock().unwrap().push(envelope.event);
    }
}

impl RecordingSink {
    fn has(&self, pred: impl Fn(&EngineEvent) -> bool) -> bool {
        self.events.lock().unwrap().iter().any(pred)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_workdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sorycode-agent-test-{}-{}",
        std::process::id(),
        tag
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp workdir");
    dir
}

fn runtime_with(
    provider: Arc<MockProvider>,
    config: AgentConfig,
    sink: Option<RecordingSink>,
) -> AgentRuntime {
    let mut agent = AgentRuntime::new(provider, config);
    if let Some(s) = sink {
        agent.event_sink = Some(Arc::new(s));
    }
    agent
}

async fn run(agent: &AgentRuntime, working_dir: &PathBuf) -> Result<String, String> {
    agent
        .run_task(
            None,
            SessionId::new(),
            TaskId::new(),
            "do the thing".to_string(),
            working_dir.to_string_lossy().to_string(),
            None,
            vec![],
            None,
        )
        .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// 1. Provider returns plain content, no tool calls → Ok(content) and a
///    TaskCompleted event is emitted.
#[tokio::test]
async fn simple_completion_returns_content_and_emits_task_completed() {
    let provider = Arc::new(MockProvider::new(vec![ok_response("all done")]));
    let sink = RecordingSink::default();
    let agent = runtime_with(provider.clone(), AgentConfig::default(), Some(sink.clone()));
    let dir = temp_workdir("simple");

    let result = run(&agent, &dir).await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap(), "all done");
    assert_eq!(provider.call_count(), 1);
    assert!(
        sink.has(
            |e| matches!(e, EngineEvent::TaskCompleted { result, .. } if result == "all done")
        )
    );
    assert!(sink.has(|e| matches!(e, EngineEvent::TaskStarted { .. })));
}

/// 2. Provider asks for `update_plan`, then returns content. The tool must
///    actually execute and its result must be appended as a Tool message in
///    the next provider request.
#[tokio::test]
async fn tool_call_loop_executes_tool_and_feeds_result_back() {
    let provider = Arc::new(MockProvider::new(vec![
        tool_call_response(
            "update_plan",
            json!({"plan": [{"step": "a", "status": "pending"}]}),
        ),
        ok_response("plan recorded"),
    ]));
    let sink = RecordingSink::default();
    let agent = runtime_with(provider.clone(), AgentConfig::default(), Some(sink.clone()));
    let dir = temp_workdir("tool-loop");

    let result = run(&agent, &dir).await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap(), "plan recorded");
    assert_eq!(provider.call_count(), 2);

    // The second request must contain a Tool-role message carrying the
    // update_plan output ("[plan]" marker produced by UpdatePlanTool).
    let messages = provider.last_request_messages();
    let tool_msg = messages
        .iter()
        .find(|m| m.role == Role::Tool)
        .expect("expected a tool result message in second request");
    assert!(
        tool_msg.content.contains("[plan]"),
        "tool output was: {}",
        tool_msg.content
    );
    assert!(tool_msg.content.contains("a"));

    assert!(sink.has(
        |e| matches!(e, EngineEvent::ToolStarted { tool_name, .. } if tool_name == "update_plan")
    ));
    assert!(sink.has(|e| matches!(
        e,
        EngineEvent::ToolCompleted {
            is_error: false,
            ..
        }
    )));
}

/// 3. Non-retryable provider error → Err and TaskFailed event.
#[tokio::test]
async fn provider_error_returns_err_and_emits_task_failed() {
    let provider = Arc::new(MockProvider::new(vec![Err(ProviderError::Api(
        "fatal boom".into(),
    ))]));
    let sink = RecordingSink::default();
    let agent = runtime_with(provider.clone(), AgentConfig::default(), Some(sink.clone()));
    let dir = temp_workdir("provider-err");

    let result = run(&agent, &dir).await;
    let _ = std::fs::remove_dir_all(&dir);

    let err = result.expect_err("expected provider error");
    assert!(err.contains("provider error"), "unexpected error: {}", err);
    // "fatal boom" is not retryable → exactly one provider call.
    assert_eq!(provider.call_count(), 1);
    assert!(sink.has(
        |e| matches!(e, EngineEvent::TaskFailed { error, .. } if error.contains("fatal boom"))
    ));
}

/// 4. Transient error (HTTP 429) is retried by call_provider_with_retry;
///    the run succeeds on the second attempt.
#[tokio::test]
async fn transient_provider_error_is_retried() {
    let provider = Arc::new(MockProvider::new(vec![
        Err(ProviderError::Api("429 rate limit exceeded".into())),
        ok_response("recovered"),
    ]));
    let agent = runtime_with(provider.clone(), AgentConfig::default(), None);
    let dir = temp_workdir("retry");

    let result = run(&agent, &dir).await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap(), "recovered");
    assert_eq!(provider.call_count(), 2, "expected one retry after 429");
}

/// 5a. Cancel flag already set before the run → immediate Err("cancelled")
///     and TaskCancelled, with no provider call.
#[tokio::test]
async fn pre_cancelled_task_returns_cancelled() {
    let provider = Arc::new(MockProvider::new(vec![ok_response("never")]));
    let sink = RecordingSink::default();
    let agent = runtime_with(provider.clone(), AgentConfig::default(), Some(sink.clone()));
    let dir = temp_workdir("pre-cancel");
    let cancel = Arc::new(AtomicBool::new(true));

    let result = agent
        .run_task(
            None,
            SessionId::new(),
            TaskId::new(),
            "hi".to_string(),
            dir.to_string_lossy().to_string(),
            None,
            vec![],
            Some(cancel),
        )
        .await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap_err(), "cancelled");
    assert_eq!(provider.call_count(), 0);
    assert!(sink.has(|e| matches!(e, EngineEvent::TaskCancelled { .. })));
}

/// 5b. Cancel arrives mid-run: the provider raises the flag during `chat()`
///     and returns a tool call; the per-tool-call cancellation check must
///     abort with Err("cancelled") and TaskCancelled.
#[tokio::test]
async fn cancel_during_tool_loop_aborts_task() {
    let cancel = Arc::new(AtomicBool::new(false));
    let provider = Arc::new(
        MockProvider::new(vec![tool_call_response(
            "update_plan",
            json!({"plan": [{"step": "x", "status": "pending"}]}),
        )])
        .with_flag(cancel.clone()),
    );
    let sink = RecordingSink::default();
    let agent = runtime_with(provider, AgentConfig::default(), Some(sink.clone()));
    let dir = temp_workdir("mid-cancel");

    let result = agent
        .run_task(
            None,
            SessionId::new(),
            TaskId::new(),
            "hi".to_string(),
            dir.to_string_lossy().to_string(),
            None,
            vec![],
            Some(cancel),
        )
        .await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap_err(), "cancelled");
    assert!(sink.has(|e| matches!(e, EngineEvent::TaskCancelled { .. })));
    // The tool must NOT have executed (cancel check precedes execution).
    assert!(!sink.has(|e| matches!(e, EngineEvent::ToolStarted { .. })));
}

/// 6. strict_policy makes write_file require approval; a RejectAll handler
///     causes the tool result "approval rejected" to be fed back, and the
///     task continues to completion on the next provider turn.
#[tokio::test]
async fn rejected_approval_feeds_rejection_back_and_task_continues() {
    let provider = Arc::new(MockProvider::new(vec![
        tool_call_response("write_file", json!({"path": "out.txt", "content": "hi"})),
        ok_response("could not write"),
    ]));
    let sink = RecordingSink::default();
    let mut agent = runtime_with(
        provider.clone(),
        AgentConfig {
            strict_policy: true,
            ..AgentConfig::default()
        },
        Some(sink.clone()),
    );
    agent.approvals = Arc::new(RejectAll);
    let dir = temp_workdir("reject-all");

    let result = run(&agent, &dir).await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap(), "could not write");
    assert!(sink.has(|e| matches!(e, EngineEvent::ApprovalRequired { .. })));

    // The rejection must appear as a Tool message in the follow-up request.
    let messages = provider.last_request_messages();
    let tool_msg = messages
        .iter()
        .find(|m| m.role == Role::Tool)
        .expect("expected tool result message");
    assert_eq!(tool_msg.content, "approval rejected");

    // And the file must not have been written.
    assert!(!dir.join("out.txt").exists());
}

/// 7. History larger than MAX_CONTEXT_MESSAGES (40) triggers compaction:
///    the agent first asks the provider for a summary, then runs the turn.
#[tokio::test]
async fn large_history_triggers_compaction_then_completes() {
    let provider = Arc::new(MockProvider::new(vec![
        // Consumed by compact_context's summarization request.
        ok_response("condensed earlier conversation"),
        // Consumed by the actual turn.
        ok_response("done after compaction"),
    ]));
    let agent = runtime_with(provider.clone(), AgentConfig::default(), None);
    let dir = temp_workdir("compact");

    let mut history = Vec::new();
    for i in 0..45 {
        if i % 2 == 0 {
            history.push(Message::user(format!("question {}", i)));
        } else {
            history.push(Message::assistant(format!("answer {}", i)));
        }
    }

    let result = agent
        .run_task(
            None,
            SessionId::new(),
            TaskId::new(),
            "final question".to_string(),
            dir.to_string_lossy().to_string(),
            None,
            history,
            None,
        )
        .await;
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(result.unwrap(), "done after compaction");
    // One summary call + one turn call.
    assert_eq!(provider.call_count(), 2);

    // The second (turn) request must contain the compaction summary marker
    // and must be well under the original 47-message size.
    let turn_req = provider
        .requests
        .lock()
        .unwrap()
        .get(1)
        .expect("second provider request")
        .clone();
    assert!(
        turn_req
            .messages
            .iter()
            .any(|m| m.content.contains("[Earlier conversation summary]")),
        "expected compaction summary in turn request"
    );
    assert!(
        turn_req.messages.len() < 47,
        "compacted message list should be smaller, got {}",
        turn_req.messages.len()
    );
}
