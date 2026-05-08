use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::infrastructure::schema::room_memberships;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = room_memberships)]
#[diesel(primary_key(room_id, user_id))]
pub struct RoomMembershipDto {
    pub room_id: String,
    pub user_id: String,
    pub membership: String,
    pub membership_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = room_memberships)]
pub struct NewRoomMembershipDto {
    pub room_id: String,
    pub user_id: String,
    pub membership: String,
    pub membership_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
