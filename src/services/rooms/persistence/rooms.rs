use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::infrastructure::schema::rooms;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = rooms)]
#[diesel(primary_key(room_id))]
pub struct RoomDto {
    pub room_id: String,
    pub creator_user_id: String,
    pub room_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = rooms)]
pub struct NewRoomDto {
    pub room_id: String,
    pub creator_user_id: String,
    pub room_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
