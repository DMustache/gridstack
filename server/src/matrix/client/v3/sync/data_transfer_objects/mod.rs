use std::collections::BTreeMap;

use ruma::exports::serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct QueryParameters {
    pub filter: Option<String>,
    pub full_state: Option<bool>,
    pub set_presence: Option<String>,
    pub since: Option<String>,
    pub timeout: Option<u64>,
    pub use_state_after: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BodyView {
    pub next_batch: String,
    #[serde(skip_serializing_if = "EventsSection::is_empty")]
    pub account_data: EventsSection,
    #[serde(skip_serializing_if = "EventsSection::is_empty")]
    pub presence: EventsSection,
    #[serde(skip_serializing_if = "RoomsView::is_empty")]
    pub rooms: RoomsView,
    #[serde(skip_serializing_if = "EventsSection::is_empty")]
    pub to_device: EventsSection,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub device_lists: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub device_one_time_keys_count: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EventsSection {
    pub events: Vec<Value>,
}

impl EventsSection {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RoomsView {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub join: BTreeMap<String, JoinedRoomView>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub invite: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub knock: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub leave: BTreeMap<String, LeftRoomView>,
}

impl RoomsView {
    pub fn is_empty(&self) -> bool {
        self.join.is_empty()
            && self.invite.is_empty()
            && self.knock.is_empty()
            && self.leave.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JoinedRoomView {
    pub timeline: TimelineView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<EventsSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_after: Option<EventsSection>,
    #[serde(skip_serializing_if = "EventsSection::is_empty")]
    pub account_data: EventsSection,
    #[serde(skip_serializing_if = "EventsSection::is_empty")]
    pub ephemeral: EventsSection,
    pub summary: RoomSummaryView,
    pub unread_notifications: UnreadNotificationsView,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeftRoomView {
    pub timeline: TimelineView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<EventsSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_after: Option<EventsSection>,
    #[serde(skip_serializing_if = "EventsSection::is_empty")]
    pub account_data: EventsSection,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineView {
    pub events: Vec<Value>,
    pub limited: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_batch: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoomSummaryView {
    #[serde(rename = "m.heroes")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heroes: Vec<String>,
    #[serde(rename = "m.joined_member_count")]
    pub joined_member_count: i64,
    #[serde(rename = "m.invited_member_count")]
    pub invited_member_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnreadNotificationsView {
    pub highlight_count: u64,
    pub notification_count: u64,
}
