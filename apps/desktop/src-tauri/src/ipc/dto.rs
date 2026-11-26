use serde::Serialize;
use sorycode_core::model::{Message, Role, ToolCallRequest};
use sorycode_core::project::Project;
use sorycode_core::session::Session;
use sorycode_core::task::{Task, TaskStatus};
use sorycode_providers::ProviderInfo;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub configured: bool,
    pub free_only: bool,
    pub default_model: String,
    pub models: Vec<String>,
}

impl ProviderDto {
    pub fn from_preset(preset: &ProviderInfo, configured: bool) -> Self {
        Self {
            id: preset.name.to_string(),
            name: preset.name.to_string(),
            description: preset.description.to_string(),
            configured: configured || preset.free_only,
            free_only: preset.free_only,
            default_model: preset.default_model.to_string(),
            models: sorycode_providers::model_catalog(preset.name)
                .iter()
                .map(|model| model.to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionDto {
    pub provider: String,
    pub model: String,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub path: String,
    pub name: String,
    pub is_git: bool,
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Session> for SessionDto {
    fn from(session: Session) -> Self {
        Self {
            id: session.id.to_string(),
            project_id: session.project_id.map(|id| id.to_string()),
            title: session.title,
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Project> for ProjectDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id.to_string(),
            name: project.name,
            root_path: project.root_path,
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub session_id: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Task> for TaskDto {
    fn from(task: Task) -> Self {
        Self {
            id: task.id.to_string(),
            session_id: task.session_id.to_string(),
            description: task.description,
            status: task.status,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageDto {
    pub role: Role,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_calls: Vec<ToolCallRequest>,
}

impl From<Message> for MessageDto {
    fn from(message: Message) -> Self {
        Self {
            role: message.role,
            content: message.content,
            tool_call_id: message.tool_call_id,
            tool_calls: message.tool_calls,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoDto {
    pub version: String,
    pub data_path: String,
    pub credentials_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto {
    pub providers: Vec<ProviderDto>,
    pub selection: SelectionDto,
    pub workspace: Option<WorkspaceDto>,
    pub sessions: Vec<SessionDto>,
}
