use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "m.login.registration_token")]
pub struct RegistrationTokenAuth {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}
