use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::infrastructure::schema::devices;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = devices)]
#[diesel(primary_key(user_id, device_id))]
pub struct DeviceDto {
    pub user_id: String,
    pub device_id: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = devices)]
pub struct NewDeviceDto {
    pub user_id: String,
    pub device_id: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
