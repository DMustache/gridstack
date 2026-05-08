use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct RegisterUserQueryInfo {
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegisterUserInfo {
    pub auth: Option<Value>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "device_id")]
    pub device_identifier: Option<String>,
    pub inhibit_login: Option<bool>,
    pub refresh_token: Option<bool>,
    pub initial_device_display_name: Option<String>,
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

impl UiaaResponseView {
    pub fn password_auth_challenge() -> Self {
        Self {
            completed: vec![],
            flows: vec![AuthenticationFlowView {
                stages: vec!["m.login.password".to_owned()],
            }],
            params: json!({
                "m.login.password": {
                    "identifier_types": [
                        "m.id.user",
                    ]
                }
            }),
            session: Uuid::new_v4().to_string(),
        }
    }
}
