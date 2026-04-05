use crate::schema::room_state;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = room_state)]
pub struct RoomState {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = room_state)]
pub struct NewRoomState {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
}
