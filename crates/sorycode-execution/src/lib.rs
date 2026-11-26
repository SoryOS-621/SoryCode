use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: std::collections::HashMap<String, String>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    async fn exec(&self, req: ExecRequest) -> Result<ExecResult, String>;
    async fn read_file(&self, path: &str) -> Result<String, String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), String>;
    async fn list_dir(&self, path: &str) -> Result<Vec<String>, String>;
}

pub struct LocalEnvironment { pub base_dir: std::path::PathBuf }

impl LocalEnvironment {
    pub fn new(base_dir: impl Into<std::path::PathBuf>) -> Self { Self { base_dir: base_dir.into() } }
}

#[async_trait]
impl ExecutionEnvironment for LocalEnvironment {
    async fn exec(&self, req: ExecRequest) -> Result<ExecResult, String> {
        use tokio::process::Command;
        let start = std::time::Instant::now();
        let mut cmd = Command::new(&req.command);
        cmd.args(&req.args);
        if let Some(cwd) = &req.cwd { cmd.current_dir(cwd); } else { cmd.current_dir(&self.base_dir); }
        for (k, v) in &req.env { cmd.env(k, v); }
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
    async fn read_file(&self, path: &str) -> Result<String, String> {
        let p = self.base_dir.join(path);
        tokio::fs::read_to_string(p).await.map_err(|e| e.to_string())
    }
    async fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        let p = self.base_dir.join(path);
        if let Some(parent) = p.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
        tokio::fs::write(p, content).await.map_err(|e| e.to_string())
    }
    async fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let p = self.base_dir.join(path);
        let mut rd = tokio::fs::read_dir(p).await.map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(e) = rd.next_entry().await.map_err(|e| e.to_string())? {
            out.push(e.file_name().to_string_lossy().to_string());
        }
        Ok(out)
    }
}
