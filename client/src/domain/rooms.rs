use serde::{Deserialize, Serialize};

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
