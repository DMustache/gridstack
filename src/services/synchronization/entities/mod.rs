use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncSetPresence {
    Offline,
    Online,
    Unavailable,
}

#[derive(Clone, Debug)]
pub enum SyncFilterSelection {
    StoredFilterIdentifier(String),
    InlineFilterDefinition(serde_json::Value),
}

#[derive(Clone, Debug)]
pub struct SyncRequest {
    pub filter: Option<SyncFilterSelection>,
    pub since: Option<String>,
    pub full_state: bool,
    pub set_presence: Option<SyncSetPresence>,
    pub timeout_milliseconds: u64,
    pub use_state_after: bool,
    pub timeline_limit: usize,
}

#[derive(Clone, Debug)]
pub struct SyncBatch {
    pub next_batch: String,
    pub rooms: SyncRoomsBatch,
    pub presence: SyncEventsBatch,
    pub account_data: SyncEventsBatch,
    pub to_device: SyncEventsBatch,
    pub device_lists: SyncDeviceListsBatch,
    pub device_one_time_keys_count: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct SyncEventsBatch {
    pub events: Vec<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct SyncRoomsBatch {
    pub join: serde_json::Map<String, serde_json::Value>,
    pub invite: serde_json::Map<String, serde_json::Value>,
    pub leave: serde_json::Map<String, serde_json::Value>,
    pub knock: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct SyncDeviceListsBatch {
    pub changed: Vec<String>,
    pub left: Vec<String>,
}
