use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventType {
    Mouse {
        x: f64,
        y: f64,
        click: bool,
    },
    Keyboard {
        text: String,
    },
    Window {
        title: String,
        app: String,
    },
    Clipboard {
        content: String,
    },
    Notification {
        app: String,
        title: String,
        body: String,
    },
    Screenshot {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchMeEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String, // e.g., "macos-accessibility"
    pub event_type: EventType,
}

impl CatchMeEvent {
    pub fn new(source: impl Into<String>, event_type: EventType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            source: source.into(),
            event_type,
        }
    }
}
