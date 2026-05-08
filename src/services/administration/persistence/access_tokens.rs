use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::infrastructure::schema::access_tokens;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = access_tokens)]
#[diesel(primary_key(access_token))]
pub struct AccessTokenModel {
    pub access_token: String,
    pub user_id: String,
    pub device_id: String,
    pub refresh_token: String,
    pub access_token_created_at: DateTime<Utc>,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token_created_at: DateTime<Utc>,
    pub refresh_token_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = access_tokens)]
pub struct NewAccessTokenCreateModel {
    pub access_token: String,
    pub user_id: String,
    pub device_id: String,
    pub refresh_token: String,
    pub access_token_created_at: DateTime<Utc>,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token_created_at: DateTime<Utc>,
    pub refresh_token_expires_at: DateTime<Utc>,
}
