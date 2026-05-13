use serde::{Deserialize, Serialize};

use crate::services::traits::Id;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let normalized = value.trim();
        if normalized.is_empty() || normalized.len() > 255 {
            return None;
        }
        if normalized.starts_with("ed25519:") {
            return None;
        }
        Some(Self(normalized.to_owned()))
    }

    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Id for DeviceId {
    fn new_id() -> String {
        DeviceId::new().into_inner()
    }
}
