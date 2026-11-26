use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sorycode_core::ids::*;
use sorycode_core::model::Message;
use sorycode_core::project::Project;
use sorycode_core::session::Session;
use sorycode_core::task::Task;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serde(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub session_id: SessionId,
    pub message: Message,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    sessions: Vec<Session>,
    #[serde(default)]
    tasks: Vec<Task>,
    #[serde(default)]
    messages: Vec<StoredMessage>,
}

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: &Project) -> StorageResult<()>;
    async fn get(&self, id: ProjectId) -> StorageResult<Option<Project>>;
    async fn list(&self) -> StorageResult<Vec<Project>>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: &Session) -> StorageResult<()>;
    async fn get(&self, id: SessionId) -> StorageResult<Option<Session>>;
    async fn list(&self, project_id: Option<ProjectId>) -> StorageResult<Vec<Session>>;
    async fn update(&self, session: &Session) -> StorageResult<()>;
}

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: &Task) -> StorageResult<()>;
    async fn get(&self, id: TaskId) -> StorageResult<Option<Task>>;
    async fn update(&self, task: &Task) -> StorageResult<()>;
    async fn list_by_session(&self, session_id: SessionId) -> StorageResult<Vec<Task>>;
}

#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn append(&self, session_id: SessionId, message: &Message) -> StorageResult<()>;
    async fn list(&self, session_id: SessionId) -> StorageResult<Vec<Message>>;
    async fn clear(&self, session_id: SessionId) -> StorageResult<()>;
}

pub struct Storage {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Storage {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub async fn connect_file(path: &Path) -> StorageResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        Ok(Self::new(path.to_path_buf()))
    }

    async fn load(&self) -> StorageResult<Store> {
        if !self.path.exists() {
            return Ok(Store::default());
        }
        let data = fs::read_to_string(&self.path)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        if data.trim().is_empty() {
            return Ok(Store::default());
        }
        serde_json::from_str(&data).map_err(|e| StorageError::Serde(e.to_string()))
    }

    async fn save(&self, store: &Store) -> StorageResult<()> {
        let data =
            serde_json::to_string_pretty(store).map_err(|e| StorageError::Serde(e.to_string()))?;
        // Atomic write: serialize to a temp file then rename, so a crash
        // mid-write can never leave a truncated/corrupt database.
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, data)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        fs::rename(&tmp, &self.path)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))
    }
}

#[async_trait]
impl ProjectRepository for Storage {
    async fn create(&self, project: &Project) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        store.projects.push(project.clone());
        self.save(&store).await
    }
    async fn get(&self, id: ProjectId) -> StorageResult<Option<Project>> {
        let store = self.load().await?;
        Ok(store.projects.into_iter().find(|p| p.id == id))
    }
    async fn list(&self) -> StorageResult<Vec<Project>> {
        let store = self.load().await?;
        Ok(store.projects)
    }
}

#[async_trait]
impl SessionRepository for Storage {
    async fn create(&self, session: &Session) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        store.sessions.push(session.clone());
        self.save(&store).await
    }
    async fn get(&self, id: SessionId) -> StorageResult<Option<Session>> {
        let store = self.load().await?;
        Ok(store.sessions.into_iter().find(|s| s.id == id))
    }
    async fn list(&self, project_id: Option<ProjectId>) -> StorageResult<Vec<Session>> {
        let store = self.load().await?;
        Ok(store
            .sessions
            .into_iter()
            .filter(|s| project_id.map_or(true, |p| s.project_id == Some(p)))
            .collect())
    }
    async fn update(&self, session: &Session) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        if let Some(x) = store.sessions.iter_mut().find(|s| s.id == session.id) {
            *x = session.clone();
        }
        self.save(&store).await
    }
}

#[async_trait]
impl TaskRepository for Storage {
    async fn create(&self, task: &Task) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        store.tasks.push(task.clone());
        self.save(&store).await
    }
    async fn get(&self, id: TaskId) -> StorageResult<Option<Task>> {
        let store = self.load().await?;
        Ok(store.tasks.into_iter().find(|t| t.id == id))
    }
    async fn update(&self, task: &Task) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        if let Some(x) = store.tasks.iter_mut().find(|t| t.id == task.id) {
            *x = task.clone();
        }
        self.save(&store).await
    }
    async fn list_by_session(&self, session_id: SessionId) -> StorageResult<Vec<Task>> {
        let store = self.load().await?;
        Ok(store
            .tasks
            .into_iter()
            .filter(|t| t.session_id == session_id)
            .collect())
    }
}

#[async_trait]
impl MessageRepository for Storage {
    async fn append(&self, session_id: SessionId, message: &Message) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        store.messages.push(StoredMessage {
            session_id,
            message: message.clone(),
            created_at: chrono::Utc::now(),
        });
        self.save(&store).await
    }
    async fn list(&self, session_id: SessionId) -> StorageResult<Vec<Message>> {
        let store = self.load().await?;
        Ok(store
            .messages
            .into_iter()
            .filter(|m| m.session_id == session_id)
            .map(|m| m.message)
            .collect())
    }
    async fn clear(&self, session_id: SessionId) -> StorageResult<()> {
        let _g = self.lock.lock().await;
        let mut store = self.load().await?;
        store.messages.retain(|m| m.session_id != session_id);
        self.save(&store).await
    }
}

// backwards-compat alias
pub type SqliteStorage = Storage;
