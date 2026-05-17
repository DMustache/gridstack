use serde::{Deserialize, Serialize};
use strum::EnumString;

pub mod message_events;
pub mod state_events;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomEventKind {
    State,
    Message,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EnumString,
)]
pub enum EventDefinitionKey {
    #[strum(serialize = "m.room.create")]
    RoomCreate,
    #[strum(serialize = "m.room.aliases")]
    RoomAliases,
    #[strum(serialize = "m.room.message")]
    RoomMessage,
}
