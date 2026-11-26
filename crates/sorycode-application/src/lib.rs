use sorycode_core::ids::*;
use sorycode_core::model::Message;
use sorycode_core::project::Project;
use sorycode_core::session::Session;
use sorycode_core::task::{Task, TaskStatus};
use sorycode_events::EventSink;
use sorycode_orchestrator::Orchestrator;
use sorycode_protocol::{CoreCommand, CoreResponse};
use sorycode_storage::{
    MessageRepository, ProjectRepository, SessionRepository, Storage, TaskRepository,
};

use chrono::Utc;
use std::sync::Arc;

pub struct SoryCodeApp {
    pub storage: Arc<Storage>,
    pub orchestrator: Orchestrator,
    pub event_sink: Option<Arc<dyn EventSink>>,
    pub working_dir: String,
    /// Cancellation flags for in-flight tasks (set by CancelTask).
    pub task_cancels: Arc<
        std::sync::Mutex<std::collections::HashMap<TaskId, Arc<std::sync::atomic::AtomicBool>>>,
    >,
}

impl SoryCodeApp {
    pub async fn handle(&self, cmd: CoreCommand) -> CoreResponse {
        match cmd {
            CoreCommand::CreateProject { name, root_path } => {
                let now = Utc::now();
                let p = Project {
                    id: ProjectId::new(),
                    name,
                    root_path,
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = ProjectRepository::create(&*self.storage, &p).await {
                    return CoreResponse::Error {
                        message: e.to_string(),
                    };
                }
                CoreResponse::ProjectCreated { project_id: p.id }
            }
            CoreCommand::OpenProject { project_id } => {
                match ProjectRepository::get(&*self.storage, project_id).await {
                    Ok(Some(_)) => CoreResponse::Ok,
                    Ok(None) => CoreResponse::Error {
                        message: "project not found".into(),
                    },
                    Err(e) => CoreResponse::Error {
                        message: e.to_string(),
                    },
                }
            }
            CoreCommand::CreateSession { project_id, title } => {
                let now = Utc::now();
                let s = Session {
                    id: SessionId::new(),
                    project_id,
                    title,
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = SessionRepository::create(&*self.storage, &s).await {
                    return CoreResponse::Error {
                        message: e.to_string(),
                    };
                }
                CoreResponse::SessionCreated { session_id: s.id }
            }
            CoreCommand::GetSession { session_id } => {
                match SessionRepository::get(&*self.storage, session_id).await {
                    Ok(Some(s)) => CoreResponse::Session { session: s },
                    Ok(None) => CoreResponse::Error {
                        message: "session not found".into(),
                    },
                    Err(e) => CoreResponse::Error {
                        message: e.to_string(),
                    },
                }
            }
            CoreCommand::GetSessionHistory { session_id } => {
                match MessageRepository::list(&*self.storage, session_id).await {
                    Ok(messages) => CoreResponse::History { messages },
                    Err(e) => CoreResponse::Error {
                        message: e.to_string(),
                    },
                }
            }
            CoreCommand::ClearSessionHistory { session_id } => {
                match MessageRepository::clear(&*self.storage, session_id).await {
                    Ok(()) => CoreResponse::Ok,
                    Err(e) => CoreResponse::Error {
                        message: e.to_string(),
                    },
                }
            }
            CoreCommand::ListSessions { project_id } => {
                match SessionRepository::list(&*self.storage, project_id).await {
                    Ok(sessions) => CoreResponse::Sessions { sessions },
                    Err(e) => CoreResponse::Error {
                        message: e.to_string(),
                    },
                }
            }
            CoreCommand::SendMessage {
                session_id,
                content,
            } => {
                let history = MessageRepository::list(&*self.storage, session_id)
                    .await
                    .unwrap_or_default();
                if let Err(e) = MessageRepository::append(
                    &*self.storage,
                    session_id,
                    &Message::user(content.clone()),
                )
                .await
                {
                    return CoreResponse::Error {
                        message: e.to_string(),
                    };
                }
                let now = Utc::now();
                let task = Task {
                    id: TaskId::new(),
                    session_id,
                    description: content.clone(),
                    status: TaskStatus::Running,
                    created_at: now,
                    updated_at: now,
                };
                if let Err(e) = TaskRepository::create(&*self.storage, &task).await {
                    return CoreResponse::Error {
                        message: e.to_string(),
                    };
                }
                let task_id = task.id;
                let wd = self.working_dir.clone();
                let storage = self.storage.clone();
                let orchestrator = self.orchestrator.clone_for_run();
                let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
                self.task_cancels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(task_id, cancel.clone());
                let cancels = self.task_cancels.clone();
                let ctx_sys = Some(
                    sorycode_context::ContextBuilder::build(std::path::Path::new(&wd))
                        .to_system_prompt(),
                );
                tokio::spawn(async move {
                    let res = orchestrator
                        .run_main_task(
                            None,
                            session_id,
                            task_id,
                            content,
                            wd,
                            ctx_sys,
                            history,
                            Some(cancel),
                        )
                        .await;
                    cancels
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&task_id);
                    if let Ok(text) = &res {
                        let _ = MessageRepository::append(
                            &*storage,
                            session_id,
                            &Message::assistant(text.clone()),
                        )
                        .await;
                    }
                    if let Ok(Some(mut t)) = TaskRepository::get(&*storage, task_id).await {
                        t.status = if res.is_ok() {
                            TaskStatus::Completed
                        } else if res.as_deref() == Ok("cancelled") {
                            TaskStatus::Cancelled
                        } else {
                            TaskStatus::Failed
                        };
                        t.updated_at = Utc::now();
                        let _ = TaskRepository::update(&*storage, &t).await;
                    }
                });
                CoreResponse::TaskStarted { task_id }
            }
            CoreCommand::CancelTask { task_id } => {
                if let Some(flag) = self
                    .task_cancels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&task_id)
                    .cloned()
                {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if let Ok(Some(mut t)) = TaskRepository::get(&*self.storage, task_id).await {
                    t.status = TaskStatus::Cancelled;
                    t.updated_at = Utc::now();
                    let _ = TaskRepository::update(&*self.storage, &t).await;
                }
                CoreResponse::Ok
            }
            CoreCommand::GetTaskStatus { task_id } => {
                match TaskRepository::get(&*self.storage, task_id).await {
                    Ok(Some(t)) => CoreResponse::TaskStatus {
                        task_id,
                        status: t.status,
                    },
                    Ok(None) => CoreResponse::Error {
                        message: "task not found".into(),
                    },
                    Err(e) => CoreResponse::Error {
                        message: e.to_string(),
                    },
                }
            }
            CoreCommand::ApproveAction { .. } | CoreCommand::RejectAction { .. } => {
                CoreResponse::Ok
            }
            CoreCommand::ExecuteTool {
                session_id,
                tool_name,
                input,
            } => {
                let _ = (session_id, tool_name, input);
                CoreResponse::Error {
                    message: "ExecuteTool not implemented in app layer".into(),
                }
            }
        }
    }
}

trait CloneForRun {
    fn clone_for_run(&self) -> Orchestrator;
}
impl CloneForRun for Orchestrator {
    fn clone_for_run(&self) -> Orchestrator {
        Orchestrator {
            provider: self.provider.clone(),
            base_config: self.base_config.clone(),
            event_sink: self.event_sink.clone(),
            approvals: self.approvals.clone(),
        }
    }
}
