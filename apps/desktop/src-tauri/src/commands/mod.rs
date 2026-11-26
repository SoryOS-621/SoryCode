//! Tauri commands exposed to the React application.

pub mod chat;
pub mod projects;
pub mod providers;
pub mod sessions;
pub mod system;

use sorycode_core::ids::{ProjectId, SessionId, TaskId};
use uuid::Uuid;

pub fn parse_session_id(raw: &str) -> Result<SessionId, String> {
    Uuid::parse_str(raw)
        .map(SessionId)
        .map_err(|error| format!("invalid session id: {error}"))
}

pub fn parse_task_id(raw: &str) -> Result<TaskId, String> {
    Uuid::parse_str(raw)
        .map(TaskId)
        .map_err(|error| format!("invalid task id: {error}"))
}

pub fn parse_project_id(raw: &str) -> Result<ProjectId, String> {
    Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|error| format!("invalid project id: {error}"))
}
