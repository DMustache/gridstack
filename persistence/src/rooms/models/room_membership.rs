use crate::schema::room_memberships;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = room_memberships)]
pub struct RoomMembership {
    pub room_id: String,
    pub user_id: String,
    pub membership: String,
    pub membership_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = room_memberships)]
pub struct NewRoomMembership {
    pub room_id: String,
    pub user_id: String,
    pub membership: String,
    pub membership_event_id: Option<String>,
}
