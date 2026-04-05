use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GetLoginResponse {
    pub flows: Vec<LoginFlow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LoginFlow {
    #[serde(rename = "type")]
    pub login_type: String,
    #[serde(rename = "get_login_token")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_login_token: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoginRequest {
    pub address: Option<String>,
    #[serde(rename = "device_id")]
    pub device_id: Option<String>,
    pub identifier: Option<LoginIdentifier>,
    #[serde(rename = "initial_device_display_name")]
    pub initial_device_display_name: Option<String>,
    pub medium: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "refresh_token")]
    pub refresh_token: Option<bool>,
    pub token: Option<String>,
    #[serde(rename = "type")]
    pub login_type: String,
    pub user: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoginIdentifier {
    #[serde(rename = "type")]
    pub identifier_type: String,
    pub user: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LoginResponse {
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "device_id")]
    pub device_id: String,
    #[serde(rename = "expires_in_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_ms: Option<u64>,
    #[serde(rename = "home_server")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_server: Option<String>,
    #[serde(rename = "refresh_token")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "well_known")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub well_known: Option<WellKnown>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WellKnown {
    #[serde(rename = "m.homeserver")]
    pub homeserver: HomeserverInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HomeserverInfo {
    #[serde(rename = "base_url")]
    pub base_url: String,
}
