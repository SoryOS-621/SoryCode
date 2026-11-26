use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sorycode_core::model::{Message, ToolCallRequest};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("http error: {0}")]
    Http(String),
    #[error("api error: {0}")]
    Api(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("not configured: {0}")]
    NotConfigured(String),
    #[error("cancelled")]
    Cancelled,
    #[error("timeout")]
    Timeout,
}
pub type ProviderResult<T> = Result<T, ProviderError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCallRequest>,
    pub usage: Usage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StreamEvent {
    pub delta: String,
    pub tool_calls: Vec<ToolCallRequest>,
    pub done: bool,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat(&self, req: ChatRequest) -> ProviderResult<ChatResponse>;
    /// Stream a chat response. Text deltas are sent as `StreamEvent`s with
    /// `done = false`; the terminal event has `done = true` and carries the
    /// fully accumulated `tool_calls`.
    async fn chat_stream(
        &self,
        _req: ChatRequest,
        _tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> ProviderResult<()> {
        Err(ProviderError::Api("streaming not supported".into()))
    }
    fn supports_tools(&self) -> bool {
        true
    }
    fn supports_streaming(&self) -> bool {
        false
    }
}

pub mod credentials;
pub mod gemini;
pub mod openai_compat;
pub mod presets;
pub use gemini::GeminiProvider;
pub use openai_compat::OpenAiCompatProvider;
pub use presets::*;

pub struct ProviderManager {
    providers: std::collections::HashMap<String, Box<dyn Provider>>,
}
impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Default::default(),
        }
    }
    pub fn register(&mut self, name: impl Into<String>, p: Box<dyn Provider>) {
        self.providers.insert(name.into(), p);
    }
    pub fn get(&self, name: &str) -> Option<&dyn Provider> {
        self.providers.get(name).map(|b| b.as_ref())
    }
    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}
impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}
