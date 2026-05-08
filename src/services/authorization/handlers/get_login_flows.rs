use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct LoginFlowView {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "get_login_token")]
    pub get_login_token: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GetLoginFlowsView {
    pub flows: Vec<LoginFlowView>,
}
