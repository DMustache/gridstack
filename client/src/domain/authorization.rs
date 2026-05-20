use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct RegistrationFlowView {
    pub stages: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetAuthMetadataView {
    #[serde(rename = "authorization_endpoint")]
    pub authorization_endpoint: String,
    #[serde(rename = "code_challenge_methods_supported")]
    pub code_challenge_methods_supported: Vec<String>,
    #[serde(rename = "grant_types_supported")]
    pub grant_types_supported: Vec<String>,
    pub issuer: String,
    #[serde(rename = "registration_endpoint")]
    pub registration_endpoint: String,
    #[serde(rename = "response_modes_supported")]
    pub response_modes_supported: Vec<String>,
    #[serde(rename = "response_types_supported")]
    pub response_types_supported: Vec<String>,
    #[serde(rename = "revocation_endpoint")]
    pub revocation_endpoint: String,
    #[serde(rename = "token_endpoint")]
    pub token_endpoint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginFlow {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "get_login_token", skip_serializing_if = "Option::is_none")]
    pub get_login_token: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetLoginFlowsView {
    pub flows: Vec<LoginFlow>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckUsernameAvailableView {
    pub available: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    #[default]
    User,
    Guest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationDataInfo {
    pub session: Option<String>,
    #[serde(rename = "type")]
    pub authentication_type: Option<String>,
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticationFlowView {
    pub stages: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiaaResponseView {
    pub completed: Vec<String>,
    pub flows: Vec<AuthenticationFlowView>,
    pub params: Value,
    pub session: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginType {
    #[serde(rename = "m.login.password")]
    Password,
    #[serde(rename = "m.login.token")]
    Token,
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhoAmIView {
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "is_guest")]
    pub is_guest: bool,
    #[serde(rename = "device_id", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogoutUserView {}

#[derive(Clone, Debug)]
pub struct SessionRecord {
    pub server_url: String,
    pub username: String,
    pub access_token: String,
    pub user_id: String,
    pub device_id: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in_milliseconds: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ServerCredentials {
    pub server_url: String,
    pub username: String,
    pub password: String,
}
