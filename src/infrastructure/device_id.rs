use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Id for DeviceId {
    fn new_id() -> String {
        Self::default().into_inner()
    }
}
