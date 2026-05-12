use serde::Deserialize;

use crate::services::traits::Id;

#[derive(Clone, Debug, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Id for DeviceId {
    fn new(&self) -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
