use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::rooms::{RoomStateEventView, RoomTimelineEventView};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefineFilterInfo {
    #[serde(flatten)]
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefineFilterView {
    pub filter_id: String,
}

#[derive(Clone, Debug)]
pub struct SyncQuery {
    pub filter: Option<String>,
    pub since: Option<String>,
    pub full_state: bool,
    pub set_presence: Option<String>,
    pub timeout_milliseconds: Option<u64>,
    pub use_state_after: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncResponseView {
    pub next_batch: String,
    pub rooms: SyncRoomsResponseView,
    pub presence: SyncEventsEnvelopeView,
    pub account_data: SyncEventsEnvelopeView,
    pub to_device: SyncEventsEnvelopeView,
    pub device_lists: SyncDeviceListsView,
    #[serde(default)]
    pub device_one_time_keys_count: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncRoomsResponseView {
    #[serde(default)]
    pub join: BTreeMap<String, SyncJoinedRoomView>,
    #[serde(default)]
    pub invite: BTreeMap<String, Value>,
    #[serde(default)]
    pub leave: BTreeMap<String, Value>,
    #[serde(default)]
    pub knock: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncJoinedRoomView {
    #[serde(default)]
    pub timeline: SyncTimelineView,
    #[serde(default)]
    pub state: SyncStateView,
    #[serde(default)]
    pub ephemeral: SyncEventsEnvelopeView,
    #[serde(default)]
    pub account_data: SyncEventsEnvelopeView,
    #[serde(default)]
    pub unread_notifications: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncTimelineView {
    #[serde(default)]
    pub events: Vec<RoomTimelineEventView>,
    #[serde(default)]
    pub limited: bool,
    #[serde(default)]
    pub prev_batch: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncStateView {
    #[serde(default)]
    pub events: Vec<RoomStateEventView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncEventsEnvelopeView {
    #[serde(default)]
    pub events: Vec<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncDeviceListsView {
    #[serde(default)]
    pub changed: Vec<String>,
    #[serde(default)]
    pub left: Vec<String>,
}
