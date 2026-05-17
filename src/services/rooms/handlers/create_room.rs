use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct RoomStateEventInfo {
    pub content: serde_json::Value,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct InviteThirdPartyIdentifierInfo {
    pub id_server: String,
    pub id_access_token: String,
    pub medium: String,
    pub address: String,
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
    pub creation_content: Option<serde_json::Value>,
    pub initial_state: Option<Vec<RoomStateEventInfo>>,
    pub invite: Option<Vec<String>>,
    pub invite_3pid: Option<Vec<InviteThirdPartyIdentifierInfo>>,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    #[serde(default)]
    pub power_level_content_override: Option<serde_json::Value>,
    #[serde(default)]
    pub preset: RoomPreset,
    pub room_alias_name: Option<String>,
    pub room_version: Option<String>,
    pub topic: Option<String>,
    #[serde(default)]
    pub visibility: RoomVisibility,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateRoomView {
    pub room_id: String,
}
