use crate::{
    ChatRequest, ChatResponse, Provider, ProviderError, ProviderResult, StreamEvent,
    ToolCallRequest, ToolDefinition, Usage,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use sorycode_core::model::Message;

pub struct OpenAiCompatProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub client: Client,
    pub extra_headers: Vec<(String, String)>,
}

impl OpenAiCompatProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .connect_timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
            extra_headers: vec![],
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.extra_headers = headers;
        self
    }
}

fn messages_to_openai(msgs: &[Message]) -> Vec<Value> {
    msgs.iter()
        .map(|m| {
            let role = match m.role {
                sorycode_core::model::Role::System => "system",
                sorycode_core::model::Role::User => "user",
                sorycode_core::model::Role::Assistant => "assistant",
                sorycode_core::model::Role::Tool => "tool",
            };
            let mut o = json!({"role": role, "content": m.content});
            if m.role == sorycode_core::model::Role::Tool {
                if let Some(id) = &m.tool_call_id {
                    o["tool_call_id"] = json!(id);
                }
            }
            if !m.tool_calls.is_empty() {
                let tcs: Vec<Value> = m
                    .tool_calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id, "type": "function",
                            "function": {"name": tc.name, "arguments": tc.arguments.to_string()}
                        })
                    })
                    .collect();
                o["tool_calls"] = json!(tcs);
            }
            o
        })
        .collect()
}

fn tools_to_openai(tools: &[ToolDefinition]) -> Vec<Value> {
    tools.iter().map(|t| json!({
        "type": "function",
        "function": { "name": t.name, "description": t.description, "parameters": t.parameters }
    })).collect()
}

#[derive(Default)]
struct ToolCallAcc {
    id: String,
    name: String,
    arguments: String,
}

fn finish_tool_calls(calls: &[ToolCallAcc]) -> Vec<ToolCallRequest> {
    calls
        .iter()
        .map(|a| {
            let arguments = if a.arguments.trim().is_empty() {
                Value::Object(Default::default())
            } else {
                serde_json::from_str(&a.arguments).unwrap_or(Value::Object(Default::default()))
            };
            ToolCallRequest {
                id: if a.id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    a.id.clone()
                },
                name: a.name.clone(),
                arguments,
            }
        })
        .collect()
}

async fn send_done(
    tx: &tokio::sync::mpsc::Sender<StreamEvent>,
    tool_calls: Vec<ToolCallRequest>,
) -> ProviderResult<()> {
    tx.send(StreamEvent {
        delta: String::new(),
        tool_calls,
        done: true,
    })
    .await
    .map_err(|_| ProviderError::Cancelled)
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn name(&self) -> &str {
        &self.name
    }
    async fn chat(&self, req: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = json!({ "model": req.model, "messages": messages_to_openai(&req.messages) });
        if !req.tools.is_empty() {
            body["tools"] = json!(tools_to_openai(&req.tools));
            body["tool_choice"] = json!("auto");
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(m) = req.max_tokens {
            body["max_tokens"] = json!(m);
        }
        let mut request = self.client.post(&url);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }
        for (k, v) in &self.extra_headers {
            request = request.header(k, v);
        }
        let resp = request
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
        let choice = v
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| ProviderError::Parse("no choices".into()))?;
        let msg = choice
            .get("message")
            .ok_or_else(|| ProviderError::Parse("no message".into()))?;
        let content = msg
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let mut tool_calls = Vec::new();
        if let Some(tcs) = msg.get("tool_calls").and_then(|t| t.as_array()) {
            for tc in tcs {
                let id = tc
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let func = tc.get("function").cloned().unwrap_or(Value::Null);
                let name = func
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = func
                    .get("arguments")
                    .and_then(|x| x.as_str())
                    .unwrap_or("{}");
                let arguments: Value =
                    serde_json::from_str(args_str).unwrap_or(Value::Object(Default::default()));
                tool_calls.push(ToolCallRequest {
                    id,
                    name,
                    arguments,
                });
            }
        }
        let usage = v
            .get("usage")
            .map(|u| Usage {
                prompt_tokens: u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u
                    .get("completion_tokens")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            })
            .unwrap_or_default();
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            finish_reason,
        })
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn chat_stream(
        &self,
        req: ChatRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> ProviderResult<()> {
        use futures::StreamExt;
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = json!({
            "model": req.model,
            "messages": messages_to_openai(&req.messages),
            "stream": true,
        });
        if !req.tools.is_empty() {
            body["tools"] = json!(tools_to_openai(&req.tools));
            body["tool_choice"] = json!("auto");
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(m) = req.max_tokens {
            body["max_tokens"] = json!(m);
        }
        let mut request = self.client.post(&url);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }
        for (k, v) in &self.extra_headers {
            request = request.header(k, v);
        }
        let resp = request
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("{}: {}", status, text)));
        }
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut calls: Vec<ToolCallAcc> = Vec::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| ProviderError::Http(e.to_string()))?;
            buf.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim().to_string();
                buf.drain(..=pos);
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    return send_done(&tx, finish_tool_calls(&calls)).await;
                }
                if data.is_empty() {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                let Some(choice) = v.get("choices").and_then(|c| c.get(0)) else {
                    continue;
                };
                let Some(delta) = choice.get("delta") else {
                    continue;
                };
                if let Some(c) = delta.get("content").and_then(|x| x.as_str()) {
                    if !c.is_empty() {
                        tx.send(StreamEvent {
                            delta: c.to_string(),
                            tool_calls: vec![],
                            done: false,
                        })
                        .await
                        .map_err(|_| ProviderError::Cancelled)?;
                    }
                }
                if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tcs {
                        let idx = tc.get("index").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                        while calls.len() <= idx {
                            calls.push(ToolCallAcc::default());
                        }
                        let acc = &mut calls[idx];
                        if let Some(id) = tc.get("id").and_then(|x| x.as_str()) {
                            if !id.is_empty() {
                                acc.id = id.to_string();
                            }
                        }
                        if let Some(f) = tc.get("function") {
                            if let Some(n) = f.get("name").and_then(|x| x.as_str()) {
                                acc.name.push_str(n);
                            }
                            if let Some(a) = f.get("arguments").and_then(|x| x.as_str()) {
                                acc.arguments.push_str(a);
                            }
                        }
                    }
                }
            }
        }
        send_done(&tx, finish_tool_calls(&calls)).await
    }
}
