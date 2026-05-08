use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde_json::Value;

use crate::infrastructure::schema::e2ee_device_keys;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = e2ee_device_keys)]
#[diesel(primary_key(user_id, device_id))]
pub struct E2eeDeviceKeyDto {
    pub user_id: String,
    pub device_id: String,
    pub key_data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = e2ee_device_keys)]
pub struct NewE2eeDeviceKeyDto {
    pub user_id: String,
    pub device_id: String,
    pub key_data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
