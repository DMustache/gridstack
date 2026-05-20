use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreateRoomInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_content: Option<CreateRoomCreationContentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_3pid: Option<Vec<InviteThirdPartyInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_alias_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_level_content_override: Option<RoomPowerLevelsOverrideInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRoomView {
    pub room_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinedRoomsView {
    pub joined_rooms: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct JoinRoomInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinRoomView {
    pub room_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LeaveRoomInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LeaveRoomView {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomStateEventView {
    pub content: Value,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    pub state_key: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomTimelineEventView {
    pub content: Value,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomMessageDirection {
    Backward,
    Forward,
}

impl RoomMessageDirection {
    pub const fn as_query_value(self) -> &'static str {
        match self {
            Self::Backward => "b",
            Self::Forward => "f",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GetRoomMessagesQuery {
    pub from_token: Option<String>,
    pub to_token: Option<String>,
    pub direction: RoomMessageDirection,
    pub limit: usize,
    pub filter: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetRoomMessagesView {
    pub start: String,
    #[serde(default)]
    pub chunk: Vec<RoomTimelineEventView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<RoomTimelineEventView>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendRoomEventView {
    pub event_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreateRoomCreationContentInfo {
    #[serde(rename = "m.federate", skip_serializing_if = "Option::is_none")]
    pub federate: Option<bool>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub room_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InviteThirdPartyInfo {
    pub id_server: String,
    pub id_access_token: String,
    pub medium: String,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RoomPowerLevelsOverrideInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kick: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redact: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users_default: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct RoomListItem {
    pub server_url: String,
    pub user_id: String,
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
}
