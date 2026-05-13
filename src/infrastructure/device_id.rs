use serde::Deserialize;

use crate::services::traits::Id;

#[derive(Clone, Debug, Deserialize)]
pub struct DeviceId;

impl DeviceId {
    pub fn parse(value: impl Into<String>) -> Option<String> {
        let value = value.into();
        if value.trim().is_empty() {
            return None;
        }
        Some(value)
    }
}

impl Id for DeviceId {
    fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
