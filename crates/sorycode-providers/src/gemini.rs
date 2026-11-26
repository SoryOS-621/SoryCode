use crate::{
    ChatRequest, ChatResponse, Provider, ProviderError, ProviderResult, ToolCallRequest, Usage,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct GeminiProvider {
    pub api_key: String,
    pub base_url: String,
    pub client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .connect_timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }
    async fn chat(&self, req: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, req.model, self.api_key
        );
        let mut contents = Vec::new();
        for m in &req.messages {
            if m.role == sorycode_core::model::Role::System {
                contents.push(json!({"role":"user","parts":[{"text": m.content}]}));
                continue;
            }
            if m.role == sorycode_core::model::Role::Tool {
                let name = m.tool_call_id.clone().unwrap_or_else(|| "tool".to_string());
                contents.push(json!({"role":"function","parts":[{"functionResponse":{"name": name, "response":{"result": m.content}}}] }));
                continue;
            }
            let mut parts = vec![json!({"text": m.content})];
            if !m.tool_calls.is_empty() {
                for tc in &m.tool_calls {
                    parts.push(json!({"functionCall":{"name": tc.name, "args": tc.arguments}}));
                }
            }
            let role = match m.role {
                sorycode_core::model::Role::User => "user",
                _ => "model",
            };
            contents.push(json!({"role": role, "parts": parts}));
        }
        let mut body = json!({"contents": contents});
        if !req.tools.is_empty() {
            let decls: Vec<Value> = req.tools.iter().map(|t| json!({"name": t.name, "description": t.description, "parameters": t.parameters})).collect();
            body["tools"] = json!([{"functionDeclarations": decls}]);
        }
        if let Some(m) = req.max_tokens {
            body["generationConfig"] = json!({"maxOutputTokens": m});
        }
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if !status.is_success() {
            return Err(ProviderError::Api(format!("{}: {}", status, text)));
        }
        let v: Value =
            serde_json::from_str(&text).map_err(|e| ProviderError::Parse(e.to_string()))?;
        let cand = v
            .get("candidates")
            .and_then(|c| c.get(0))
            .ok_or_else(|| ProviderError::Parse("no candidates".into()))?;
        let content_obj = cand.get("content").cloned().unwrap_or(Value::Null);
        let parts = content_obj
            .get("parts")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for p in parts {
            if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                content.push_str(t);
            }
            if let Some(fc) = p.get("functionCall") {
                let name = fc
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = fc
                    .get("args")
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));
                tool_calls.push(ToolCallRequest {
                    id: Uuid::new_v4().to_string(),
                    name,
                    arguments: args,
                });
            }
        }
        let usage = v
            .get("usageMetadata")
            .map(|u| Usage {
                prompt_tokens: u
                    .get("promptTokenCount")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: u
                    .get("candidatesTokenCount")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u
                    .get("totalTokenCount")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as u32,
            })
            .unwrap_or_default();
        let finish_reason = cand
            .get("finishReason")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            finish_reason,
        })
    }
}
