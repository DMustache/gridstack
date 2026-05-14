use serde::Deserialize;

use crate::services::rooms::handlers::create_room::CreateRoomView;

#[derive(Clone, Debug, Deserialize)]
pub struct RoomStateEventInfo {
    pub content: serde_json::Value,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Clone, Debug)]
pub struct CreatedRoom {
    pub room_id: String,
}

impl From<CreatedRoom> for CreateRoomView {
    fn from(value: CreatedRoom) -> Self {
        Self {
            room_id: value.room_id,
        }
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RoomIdentifier(String);

enum RoomType {}

pub enum Spaces {}
