use serde::{Deserialize, Serialize};
use sorycode_core::ids::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreCommand {
    CreateProject {
        name: String,
        root_path: String,
    },
    OpenProject {
        project_id: ProjectId,
    },
    CreateSession {
        project_id: Option<ProjectId>,
        title: String,
    },
    SendMessage {
        session_id: SessionId,
        content: String,
    },
    CancelTask {
        task_id: TaskId,
    },
    ApproveAction {
        approval_id: ApprovalId,
    },
    RejectAction {
        approval_id: ApprovalId,
        reason: Option<String>,
    },
    GetSession {
        session_id: SessionId,
    },
    GetSessionHistory {
        session_id: SessionId,
    },
    ClearSessionHistory {
        session_id: SessionId,
    },
    ListSessions {
        project_id: Option<ProjectId>,
    },
    GetTaskStatus {
        task_id: TaskId,
    },
    ExecuteTool {
        session_id: SessionId,
        tool_name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreResponse {
    Ok,
    ProjectCreated {
        project_id: ProjectId,
    },
    SessionCreated {
        session_id: SessionId,
    },
    TaskStarted {
        task_id: TaskId,
    },
    Session {
        session: sorycode_core::session::Session,
    },
    Sessions {
        sessions: Vec<sorycode_core::session::Session>,
    },
    History {
        messages: Vec<sorycode_core::model::Message>,
    },
    TaskStatus {
        task_id: TaskId,
        status: sorycode_core::task::TaskStatus,
    },
    ToolResult {
        output: sorycode_core::tool::ToolOutput,
    },
    Error {
        message: String,
    },
}
