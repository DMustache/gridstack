use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct LoginIdentifierInfo {
    #[serde(rename = "type")]
    pub identifier_type: String,
    pub user: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoginUserInfo {
    #[serde(rename = "type")]
    pub login_type: Option<String>,
    pub identifier: Option<LoginIdentifierInfo>,
    pub user: Option<String>,
    pub password: String,
    #[serde(rename = "device_id")]
    pub device_identifier: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoginUserView {
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "device_id")]
    pub device_id: String,
    #[serde(rename = "expires_in_ms")]
    pub expires_in_ms: i64,
    #[serde(rename = "refresh_token")]
    pub refresh_token: String,
    #[serde(rename = "user_id")]
    pub user_id: String,
}
