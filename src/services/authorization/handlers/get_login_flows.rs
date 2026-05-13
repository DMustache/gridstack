use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct LoginFlow {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "get_login_token", skip_serializing_if = "Option::is_none")]
    pub get_login_token: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GetLoginFlowsView {
    pub flows: Vec<LoginFlow>,
}
