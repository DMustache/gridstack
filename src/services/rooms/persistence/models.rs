use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable, Selectable};
use serde_json::Value;
use uuid::Uuid;

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::rooms)]
pub struct CreateRoomModel {
    pub room_id: String,
    pub room_version: Option<String>,
    pub creator_user_id: String,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub visibility: String,
    pub preset: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_aliases)]
pub struct CreateRoomAliasModel {
    pub alias_localpart: String,
    pub room_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_state_events)]
pub struct CreateRoomStateEventModel {
    pub id: Uuid,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub content: Value,
    pub ordering: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::infrastructure::schema::room_aliases)]
pub struct RoomAliasModel {
    pub alias_localpart: String,
    pub room_id: String,
    pub created_at: NaiveDateTime,
}
