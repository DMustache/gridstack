use serde::{Deserialize, Serialize};

pub mod message_events;
pub mod state_events;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomEventKind {
    State,
    Message,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventDefinitionKey {
    RoomCreate,
    RoomAliases,
    RoomMessage,
    GenericStateEvent,
}

impl EventDefinitionKey {
    pub const ALL: [Self; 4] = [
        Self::RoomCreate,
        Self::RoomAliases,
        Self::RoomMessage,
        Self::GenericStateEvent,
    ];
}
