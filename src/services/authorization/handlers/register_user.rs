use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::services::authorization::entities::AccountKind;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RegisterUserQueryInfo {
    #[serde(default)]
    pub kind: AccountKind,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthenticationDataInfo {
    pub session: Option<String>,
    #[serde(rename = "type")]
    pub authentication_type: Option<String>,
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RegisterUserInfo {
    #[serde(rename = "auth")]
    pub authentication: Option<AuthenticationDataInfo>,
    #[serde(rename = "device_id")]
    pub device_identifier: Option<String>,
    #[serde(default)]
    pub inhibit_login: bool,
    pub initial_device_display_name: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub refresh_token: bool,
    pub username: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegisterUserView {
    #[serde(rename = "user_id")]
    pub user_identifier: String,
    #[serde(rename = "access_token", skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(rename = "device_id", skip_serializing_if = "Option::is_none")]
    pub device_identifier: Option<String>,
    #[serde(rename = "home_server", skip_serializing_if = "Option::is_none")]
    pub home_server_name: Option<String>,
    #[serde(rename = "expires_in_ms", skip_serializing_if = "Option::is_none")]
    pub expires_in_milliseconds: Option<u64>,
    #[serde(rename = "refresh_token", skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthenticationFlowView {
    pub stages: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UiaaResponseView {
    pub completed: Vec<String>,
    pub flows: Vec<AuthenticationFlowView>,
    pub params: Value,
    pub session: String,
}
