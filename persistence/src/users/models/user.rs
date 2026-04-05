use crate::schema::users;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = users)]
pub struct User {
    pub user_id: String,
    pub account_id: Uuid,
    pub password_hash: Option<String>,
    pub is_guest: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub user_id: String,
    pub account_id: Uuid,
    pub is_guest: bool,
    pub password_hash: Option<String>,
}
