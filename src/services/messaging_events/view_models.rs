use serde::Serialize;
use serde_json::Value;

use crate::services::messaging_events::entities::RoomEvent;

#[derive(Clone, Debug, Serialize)]
pub struct SendRoomMessageResponseViewModel {
    #[serde(rename = "event_id")]
    pub event_identifier: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomMessagesResponseViewModel {
    pub chunk: Vec<RoomEventEnvelopeViewModel>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomEventEnvelopeViewModel {
    #[serde(rename = "event_id")]
    pub event_identifier: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "sender")]
    pub sender_identifier: String,
    #[serde(rename = "origin_server_ts")]
    pub timestamp_milliseconds: i64,
    pub content: Value,
}

impl From<RoomEvent> for RoomEventEnvelopeViewModel {
    fn from(value: RoomEvent) -> Self {
        Self {
            event_identifier: value.event_identifier.into_inner(),
            event_type: value.event_type,
            sender_identifier: value.sender_identifier,
            timestamp_milliseconds: value.timestamp_milliseconds,
            content: value.content,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncResponseViewModel {
    #[serde(rename = "next_batch")]
    pub next_batch: String,
    pub rooms: Value,
}
