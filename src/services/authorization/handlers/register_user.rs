use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct RegisterUserQueryInfo {
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthenticationDataInfo {
    pub session: Option<String>,
    #[serde(rename = "type")]
    pub auth_type: Option<String>,
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegisterUserInfo {
    pub auth: Option<AuthenticationDataInfo>,
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
            completed: vec!["example.type.foo".to_owned()],
            flows: vec![AuthenticationFlowView {
                stages: vec!["example.type.foo".to_owned()],
            }],
            params: json!({
                "example.type.baz": {
                    "example_key": "foobar",
                }
            }),
            session: format!("{}xyz", &Uuid::new_v4().to_string()[..5]),
        }
    }
}
