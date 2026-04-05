use crate::schema::devices;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = devices)]
pub struct Device {
    pub user_id: String,
    pub device_id: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = devices)]
pub struct NewDevice {
    pub user_id: String,
    pub device_id: String,
    pub display_name: Option<String>,
}
