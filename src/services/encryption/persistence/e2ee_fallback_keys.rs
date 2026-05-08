use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde_json::Value;

use crate::infrastructure::schema::e2ee_fallback_keys;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = e2ee_fallback_keys)]
#[diesel(primary_key(user_id, device_id, key_algorithm))]
pub struct E2eeFallbackKeyDto {
    pub user_id: String,
    pub device_id: String,
    pub key_algorithm: String,
    pub key_id: String,
    pub key_data: Value,
    pub is_used: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = e2ee_fallback_keys)]
pub struct NewE2eeFallbackKeyDto {
    pub user_id: String,
    pub device_id: String,
    pub key_algorithm: String,
    pub key_id: String,
    pub key_data: Value,
    pub is_used: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
