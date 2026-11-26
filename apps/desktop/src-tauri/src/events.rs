//! Bridge between the engine event stream and the React application.

use sorycode_events::{EventEnvelope, EventSink};
use tauri::{AppHandle, Emitter};

/// Channel listened to by the frontend.
pub const EVENT_CHANNEL: &str = "sorycode://event";

pub struct TauriSink {
    app: AppHandle,
}

impl TauriSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl EventSink for TauriSink {
    async fn emit(&self, envelope: EventEnvelope) {
        if let Err(error) = self.app.emit(EVENT_CHANNEL, envelope) {
            tracing::warn!(%error, "failed to emit engine event");
        }
    }
}
