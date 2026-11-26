use serde::{Deserialize, Serialize};
use sorycode_core::ids::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    TaskStarted { project_id: Option<ProjectId>, session_id: SessionId, task_id: TaskId },
    TaskProgress { session_id: SessionId, task_id: TaskId, message: String },
    MessageDelta { session_id: SessionId, task_id: Option<TaskId>, delta: String },
    ToolStarted { session_id: SessionId, task_id: TaskId, tool_call_id: ToolCallId, tool_name: String },
    ToolOutput { session_id: SessionId, task_id: TaskId, tool_call_id: ToolCallId, chunk: String },
    ToolCompleted { session_id: SessionId, task_id: TaskId, tool_call_id: ToolCallId, is_error: bool, summary: String },
    ApprovalRequired { session_id: SessionId, task_id: TaskId, approval_id: ApprovalId, action: String, reason: String },
    TaskCompleted { session_id: SessionId, task_id: TaskId, result: String },
    TaskFailed { session_id: SessionId, task_id: TaskId, error: String },
    TaskCancelled { session_id: SessionId, task_id: TaskId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event: EngineEvent,
}

impl EventEnvelope {
    pub fn new(event: EngineEvent) -> Self {
        Self { id: Uuid::new_v4(), timestamp: Utc::now(), event }
    }
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn emit(&self, envelope: EventEnvelope);
}
