use serde::{Deserialize, Serialize};

use crate::services::messaging_events::entities::RoomEvent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRoom {
    pub room_identifier: String,
    pub room_alias: Option<String>,
    pub name: String,
    pub creator_identifier: String,
    pub is_public: bool,
    pub members: Vec<String>,
    pub events: Vec<RoomEvent>,
}
