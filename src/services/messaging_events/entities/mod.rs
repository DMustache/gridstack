use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomEvent {
    pub event_identifier: EventId,
    pub event_type: String,
    pub sender_identifier: String,
    pub content: Value,
    pub timestamp_milliseconds: i64,
}
