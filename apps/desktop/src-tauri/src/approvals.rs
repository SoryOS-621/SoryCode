//! Interactive approval bridge for the desktop shell.
//!
//! The engine emits `ApprovalRequired` events, but those carry an
//! `ApprovalId` the approval handler never sees. Instead, `TauriApproval`
//! emits its own `sorycode://approval` event with a fresh UUID; the frontend
//! answers via the `respond_approval` command. Pending requests time out
//! after 5 minutes and default to deny.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use sorycode_agent::ApprovalHandler;
use sorycode_security::ApprovalInfo;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;
use uuid::Uuid;

pub const APPROVAL_CHANNEL: &str = "sorycode://approval";
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

pub struct TauriApproval {
    app: AppHandle,
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl TauriApproval {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve a pending approval from the frontend.
    pub fn respond(&self, approval_id: &str, approved: bool) -> Result<(), String> {
        let tx = self
            .pending
            .lock()
            .map_err(|_| "approval lock poisoned".to_string())?
            .remove(approval_id)
            .ok_or_else(|| format!("no pending approval: {approval_id}"))?;
        let _ = tx.send(approved);
        Ok(())
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for TauriApproval {
    async fn approve(&self, info: ApprovalInfo) -> bool {
        let approval_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<bool>();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(approval_id.clone(), tx);

        let payload = serde_json::json!({
            "approvalId": approval_id,
            "action": info.action,
            "reason": info.reason,
        });
        if let Err(error) = self.app.emit(APPROVAL_CHANNEL, payload) {
            tracing::warn!(%error, "failed to emit approval request");
            self.pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&approval_id);
            return false;
        }

        match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
            Ok(Ok(approved)) => approved,
            _ => {
                self.pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&approval_id);
                false
            }
        }
    }
}
