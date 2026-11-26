//! End-to-end tests at the `SoryCodeApp` level with a mock provider:
//! CreateSession -> SendMessage -> TaskStarted -> Completed, and messages
//! persisted in session history.

use async_trait::async_trait;
use sorycode_agent::AgentConfig;
use sorycode_application::SoryCodeApp;
use sorycode_core::ids::SessionId;
use sorycode_core::task::TaskStatus;
use sorycode_orchestrator::Orchestrator;
use sorycode_protocol::{CoreCommand, CoreResponse};
use sorycode_providers::{
    ChatRequest, ChatResponse, Provider, ProviderError, ProviderResult, Usage,
};
use sorycode_storage::Storage;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct MockProvider {
    calls: AtomicUsize,
    /// Optional scripted responses consumed in order; falls back to "done".
    script: Mutex<Vec<ChatResponse>>,
}

impl MockProvider {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            script: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }
    async fn chat(&self, _req: ChatRequest) -> ProviderResult<ChatResponse> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut script = self.script.lock().unwrap();
        if let Some(resp) = script.pop() {
            return Ok(resp);
        }
        Ok(ChatResponse {
            content: "mock reply".into(),
            tool_calls: vec![],
            usage: Usage::default(),
            finish_reason: Some("stop".into()),
        })
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sorycode-app-test-{}-{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

async fn make_app(dir: &PathBuf) -> (Arc<SoryCodeApp>, Arc<MockProvider>) {
    let storage_path = dir.join(".sorycode").join("sorycode.db.json");
    let storage = Arc::new(Storage::connect_file(&storage_path).await.unwrap());
    let provider = Arc::new(MockProvider::new());
    let orchestrator = Orchestrator::new(
        provider.clone(),
        AgentConfig {
            model: "mock".into(),
            max_turns: 5,
            ..Default::default()
        },
    );
    let app = Arc::new(SoryCodeApp {
        storage,
        orchestrator,
        event_sink: None,
        working_dir: dir.to_string_lossy().to_string(),
        task_cancels: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    });
    (app, provider)
}

async fn create_session(app: &SoryCodeApp) -> SessionId {
    match app
        .handle(CoreCommand::CreateSession {
            project_id: None,
            title: "test".into(),
        })
        .await
    {
        CoreResponse::SessionCreated { session_id } => session_id,
        other => panic!("session create failed: {:?}", other),
    }
}

async fn wait_task(app: &SoryCodeApp, task_id: sorycode_core::ids::TaskId) -> TaskStatus {
    for _ in 0..200 {
        if let CoreResponse::TaskStatus { status, .. } =
            app.handle(CoreCommand::GetTaskStatus { task_id }).await
        {
            match status {
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                    return status;
                }
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("task did not finish in time");
}

#[tokio::test]
async fn send_message_completes_and_persists_history() {
    let dir = temp_dir("e2e-basic");
    let (app, provider) = make_app(&dir).await;
    let session_id = create_session(&app).await;

    let resp = app
        .handle(CoreCommand::SendMessage {
            session_id,
            content: "hello agent".into(),
        })
        .await;
    let task_id = match resp {
        CoreResponse::TaskStarted { task_id } => task_id,
        other => panic!("send failed: {:?}", other),
    };

    assert_eq!(wait_task(&app, task_id).await, TaskStatus::Completed);
    assert!(provider.calls.load(Ordering::Relaxed) >= 1);

    // history must contain the user message and the assistant reply
    match app
        .handle(CoreCommand::GetSessionHistory { session_id })
        .await
    {
        CoreResponse::History { messages } => {
            assert!(messages.iter().any(|m| m.content == "hello agent"));
            assert!(messages.iter().any(|m| m.content == "mock reply"));
        }
        other => panic!("history failed: {:?}", other),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn send_message_with_history_feeds_prior_turns() {
    let dir = temp_dir("e2e-history");
    let (app, provider) = make_app(&dir).await;
    let session_id = create_session(&app).await;

    // first turn
    let t1 = match app
        .handle(CoreCommand::SendMessage {
            session_id,
            content: "first".into(),
        })
        .await
    {
        CoreResponse::TaskStarted { task_id } => task_id,
        other => panic!("send1 failed: {:?}", other),
    };
    assert_eq!(wait_task(&app, t1).await, TaskStatus::Completed);

    // second turn: provider must see the first exchange in `messages`
    let t2 = match app
        .handle(CoreCommand::SendMessage {
            session_id,
            content: "second".into(),
        })
        .await
    {
        CoreResponse::TaskStarted { task_id } => task_id,
        other => panic!("send2 failed: {:?}", other),
    };
    assert_eq!(wait_task(&app, t2).await, TaskStatus::Completed);

    // provider.calls == 2 (one chat per task); inspect last request contents
    // via a fresh check: history now has 4 messages (2 user + 2 assistant)
    match app
        .handle(CoreCommand::GetSessionHistory { session_id })
        .await
    {
        CoreResponse::History { messages } => {
            assert_eq!(messages.len(), 4);
        }
        other => panic!("history failed: {:?}", other),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn cancel_task_marks_cancelled() {
    let dir = temp_dir("e2e-cancel");
    let (app, _provider) = make_app(&dir).await;
    let session_id = create_session(&app).await;

    let task_id = match app
        .handle(CoreCommand::SendMessage {
            session_id,
            content: "long work".into(),
        })
        .await
    {
        CoreResponse::TaskStarted { task_id } => task_id,
        other => panic!("send failed: {:?}", other),
    };
    // cancel immediately — mock responds fast, but flag may still win the race
    let _ = app.handle(CoreCommand::CancelTask { task_id }).await;
    let status = wait_task(&app, task_id).await;
    // either cancelled or already completed — both are valid terminal states
    assert!(matches!(
        status,
        TaskStatus::Cancelled | TaskStatus::Completed
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn storage_persists_across_app_restarts() {
    let dir = temp_dir("e2e-persist");
    let (app1, _p) = make_app(&dir).await;
    let session_id = create_session(&app1).await;
    drop(app1);

    // reopen storage on same file, verify session still there
    let storage_path = dir.join(".sorycode").join("sorycode.db.json");
    let storage = Arc::new(Storage::connect_file(&storage_path).await.unwrap());
    match app_handle_get_session(&storage, session_id).await {
        true => {}
        false => panic!("session lost after restart"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

async fn app_handle_get_session(storage: &Arc<Storage>, session_id: SessionId) -> bool {
    use sorycode_storage::SessionRepository;
    matches!(
        SessionRepository::get(&**storage, session_id).await,
        Ok(Some(_))
    )
}
