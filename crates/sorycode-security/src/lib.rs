use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sorycode_core::tool::ToolContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadFile,
    WriteFile,
    ExecuteCommand,
    NetworkAccess,
    GitWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub tool_name: String,
    pub permission: Permission,
    pub risk: RiskLevel,
    pub description: String,
}

#[async_trait]
pub trait PolicyEngine: Send + Sync {
    async fn check(&self, ctx: &ToolContext, req: &ActionRequest) -> PolicyDecision;
}

#[derive(Debug, Clone)]
pub enum PolicyDecision {
    Allow,
    Deny(String),
    RequireApproval(ApprovalInfo),
}

#[derive(Debug, Clone)]
pub struct ApprovalInfo {
    pub action: String,
    pub reason: String,
}

pub struct DefaultPolicy {
    pub allow_write: bool,
    pub allow_exec: bool,
    pub allow_network: bool,
}

impl Default for DefaultPolicy {
    fn default() -> Self {
        Self {
            allow_write: true,
            allow_exec: true,
            allow_network: false,
        }
    }
}

pub mod exec_policy;
pub use exec_policy::{ExecAction, ExecDecision, ExecPolicy, ExecRule};

#[async_trait]
impl PolicyEngine for DefaultPolicy {
    async fn check(&self, _ctx: &ToolContext, req: &ActionRequest) -> PolicyDecision {
        match req.permission {
            Permission::ReadFile => PolicyDecision::Allow,
            Permission::WriteFile if self.allow_write => PolicyDecision::Allow,
            Permission::ExecuteCommand if self.allow_exec => PolicyDecision::Allow,
            Permission::NetworkAccess if self.allow_network => PolicyDecision::Allow,
            Permission::GitWrite if self.allow_write => PolicyDecision::Allow,
            _ => PolicyDecision::RequireApproval(ApprovalInfo {
                action: req.description.clone(),
                reason: format!("permission {:?} not granted by policy", req.permission),
            }),
        }
    }
}
