use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::services::{events::entities::PreviousRoom, rooms::entities::RoomStateEventInfo};

#[derive(Clone, Debug, Deserialize)]
pub struct InviteThirdPartyIdentifierInfo {
    pub id_server: String,
    pub id_access_token: String,
    pub medium: String,
    pub address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RoomPowerLevelNotifications {
    pub room: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RoomPowerLevelsContentOverride {
    pub ban: Option<i64>,
    pub events: Option<BTreeMap<String, i64>>,
    pub events_default: Option<i64>,
    pub invite: Option<i64>,
    pub kick: Option<i64>,
    pub notifications: Option<RoomPowerLevelNotifications>,
    pub redact: Option<i64>,
    pub state_default: Option<i64>,
    pub users: Option<BTreeMap<String, i64>>,
    pub users_default: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoomPreset {
    PrivateChat,
    PublicChat,
    TrustedPrivateChat,
}

impl From<RoomVisibility> for RoomPreset {
    fn from(value: RoomVisibility) -> Self {
        match value {
            RoomVisibility::Public => Self::PublicChat,
            RoomVisibility::Private => Self::PrivateChat,
        }
    }
}

impl Default for RoomPreset {
    fn default() -> Self {
        RoomVisibility::default().into()
    }
}

#[derive(Clone, Debug, Deserialize, Default, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RoomVisibility {
    Public,
    #[default]
    Private,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateRoomInfo {
    /// additional info to include in the m.room.create request
    pub creation_content: Option<CreationContentInfo>,
    pub initial_state: Option<Vec<RoomStateEventInfo>>,
    pub invite: Option<Vec<String>>,
    pub invite_3pid: Option<Vec<InviteThirdPartyIdentifierInfo>>,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    #[serde(default)]
    pub power_level_content_override: Option<RoomPowerLevelsContentOverride>,
    #[serde(default)]
    pub preset: RoomPreset,
    pub room_alias_name: Option<String>,
    pub room_version: Option<String>,
    pub topic: Option<String>,
    #[serde(default)]
    pub visibility: RoomVisibility,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct CreationContentInfo {
    #[serde(default)]
    pub additional_creators: Option<Vec<String>>,
    #[serde(default, rename = "m.federate")]
    pub federate: Option<bool>,
    pub predecessor: Option<PreviousRoom>,
    #[serde(rename = "type")]
    pub room_type: Option<String>,
    // Server-owned keys accepted in input but ignored.
    pub creator: Option<String>,
    pub room_version: Option<String>,
    #[serde(flatten)]
    pub extra_content: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateRoomView {
    pub room_id: String,
}
