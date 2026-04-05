use crate::schema::room_events;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use serde_json::Value;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = room_events)]
pub struct RoomEvent {
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

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = room_events)]
pub struct NewRoomEvent {
    pub event_id: String,
    pub room_id: String,
    pub sender_user_id: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub origin_server_ts: DateTime<Utc>,
    pub depth: i64,
    pub transaction_id: Option<String>,
}
