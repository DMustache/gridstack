use serde::{Deserialize, Serialize};

use crate::services::events::entities::event_kinds::state_events::create::RoomCreateEvent;

pub mod create;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateEvent {
    RoomCreate(RoomCreateEvent),
}
