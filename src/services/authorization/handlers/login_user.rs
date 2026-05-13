use serde::{Deserialize, Serialize};

use crate::services::authorization::entities::LoginType;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum LoginIdentifierInfo {
    #[serde(rename = "m.id.user")]
    MatrixUser { user: String },
    #[serde(rename = "m.id.thirdparty")]
    ThirdParty { medium: String, address: String },
    #[serde(rename = "m.id.phone")]
    PhoneNumber { country: String, phone: String },
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoginUserInfo {
    #[serde(rename = "type")]
    pub login_type: LoginType,
    pub identifier: Option<LoginIdentifierInfo>,
    pub user: Option<String>,
    pub password: String,
    #[serde(rename = "device_id")]
    pub device_identifier: Option<String>,
    #[serde(default)]
    pub refresh_token: bool,
    pub token: Option<String>,
    pub medium: Option<String>,
    pub address: Option<String>,
    #[serde(rename = "initial_device_display_name")]
    pub initial_device_display_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoginUserView {
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "device_id")]
    pub device_id: String,
    #[serde(rename = "expires_in_ms", skip_serializing_if = "Option::is_none")]
    pub expires_in_milliseconds: Option<u64>,
    #[serde(rename = "home_server", skip_serializing_if = "Option::is_none")]
    pub home_server: Option<String>,
    #[serde(rename = "refresh_token", skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(rename = "user_id")]
    pub user_id: String,
}
