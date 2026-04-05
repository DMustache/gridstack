use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum UserAccountType {
    #[serde(rename = "user")]
    #[default]
    User,
    #[serde(rename = "guest")]
    Guest,
}

impl UserAccountType {
    pub fn is_guest(&self) -> bool {
        matches!(self, UserAccountType::Guest)
    }
}
