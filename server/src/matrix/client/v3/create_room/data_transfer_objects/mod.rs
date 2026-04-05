use ruma::exports::serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct BodyInfo {
    pub creation_content: Option<Value>,
    pub initial_state: Option<Vec<StateEvent>>,
    pub invite: Option<Vec<String>>,
    pub invite_3pid: Option<Vec<Invite3pid>>,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub power_level_content_override: Option<Value>,
    pub preset: Option<String>,
    pub room_alias_name: Option<String>,
    pub room_version: Option<String>,
    pub topic: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StateEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Invite3pid {
    pub address: String,
    pub id_access_token: String,
    pub id_server: String,
    pub medium: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BodyView {
    pub room_id: String,
}
