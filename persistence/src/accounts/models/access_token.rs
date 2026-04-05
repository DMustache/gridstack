use crate::schema::access_tokens;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = access_tokens)]
pub struct AccessToken {
    pub access_token: String,
    pub user_id: String,
    pub device_id: String,
    pub refresh_token: String,
    pub access_token_created_at: DateTime<Utc>,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token_created_at: DateTime<Utc>,
    pub refresh_token_expires_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = access_tokens)]
pub struct NewAccessToken {
    pub access_token: String,
    pub user_id: String,
    pub device_id: String,
    pub refresh_token: Option<String>,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token_expires_at: DateTime<Utc>,
}
