use crate::schema::rooms;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = rooms)]
pub struct Room {
    pub room_id: String,
    pub creator_user_id: String,
    pub room_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = rooms)]
pub struct NewRoom {
    pub room_id: String,
    pub creator_user_id: String,
    pub room_version: String,
}
