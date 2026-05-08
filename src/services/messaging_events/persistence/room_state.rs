use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::infrastructure::schema::room_state;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = room_state)]
#[diesel(primary_key(room_id, event_type, state_key))]
pub struct RoomStateDto {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = room_state)]
pub struct NewRoomStateDto {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
    pub updated_at: DateTime<Utc>,
}
