use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
}

#[derive(Clone)]
pub struct EventBroadcaster {
    pub sender: broadcast::Sender<WsEvent>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.sender.subscribe()
    }

    pub fn broadcast(&self, event: WsEvent) {
        let _ = self.sender.send(event);
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
