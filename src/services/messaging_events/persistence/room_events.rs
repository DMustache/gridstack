use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde_json::Value;

use crate::infrastructure::schema::room_events;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = room_events)]
#[diesel(primary_key(event_id))]
pub struct RoomEventDto {
    pub event_id: String,
    pub room_id: String,
    pub sender_user_id: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub origin_server_ts: DateTime<Utc>,
    pub depth: i64,
    pub stream_ordering: i64,
    pub transaction_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = room_events)]
pub struct NewRoomEventDto {
    pub event_id: String,
    pub room_id: String,
    pub sender_user_id: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub origin_server_ts: DateTime<Utc>,
    pub depth: i64,
    pub stream_ordering: i64,
    pub transaction_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
