use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct CreateRoomRequestViewModel {
    pub name: Option<String>,
    pub visibility: Option<String>,
    #[serde(rename = "room_alias_name")]
    pub room_alias_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateRoomResponseViewModel {
    #[serde(rename = "room_id")]
    pub room_identifier: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct JoinRoomResponseViewModel {
    #[serde(rename = "room_id")]
    pub room_identifier: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicRoomsResponseViewModel {
    pub chunk: Vec<PublicRoomViewModel>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicRoomViewModel {
    #[serde(rename = "room_id")]
    pub room_identifier: String,
    pub name: String,
    #[serde(rename = "num_joined_members")]
    pub joined_member_count: usize,
}
