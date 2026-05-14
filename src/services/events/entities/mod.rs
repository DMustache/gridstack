use std::default;

use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::{
    infrastructure::user_identifier::UserIdentifier, services::rooms::entities::RoomIdentifier,
};

pub enum EventType {
    State(StateEvent),
    Message(MessageEvent),
}

struct Event {
    pub event_type: EventType,
    pub state_key: Option<String>,
}

trait EventDefenition {
    fn event_type(&self) -> EventType;
    fn state_key(&self) -> Option<String>;
}

pub enum StateEvent {
    RoomCreate(RoomCreateEvent),
}

pub struct RoomCreateEvent {
    state_key: Option<String>,
    content: RoomCreateContent,
}

#[derive(Deserialize)]
pub struct RoomCreateContent {
    aditional_creators: Option<Vec<UserIdentifier>>,
    creator: Option<String>,
    // Whether users on other servers can join this room. Defaults to true if key does not exist.
    // m.federate name
    is_federative: bool,
    predecessor: Option<PreviousRoom>,
    room_version: RoomVersion,
    // type
    room_type: Option<String>,
}

#[derive(Default, EnumString, PartialEq, Eq, Ord, PartialOrd, Deserialize)]
pub enum RoomVersion {
    #[default]
    #[strum(serialize = "1")]
    V1 = 1,
    #[strum(serialize = "2")]
    V2 = 2,
    #[strum(serialize = "3")]
    V3 = 3,
    #[strum(serialize = "4")]
    V4 = 4,
    #[strum(serialize = "5")]
    V5 = 5,
    #[strum(serialize = "6")]
    V6 = 6,
    #[strum(serialize = "7")]
    V7 = 7,
    #[strum(serialize = "8")]
    V8 = 8,
    #[strum(serialize = "9")]
    V9 = 9,
    #[strum(serialize = "10")]
    V10 = 10,
    #[strum(serialize = "11")]
    V11 = 11,
    #[strum(serialize = "12")]
    V12 = 12,
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct EventIdentifier(String);

#[derive(Deserialize)]
pub struct PreviousRoom {
    event_id: Option<EventIdentifier>,
    room_id: Option<RoomIdentifier>,
}

pub enum MessageEvent {}
