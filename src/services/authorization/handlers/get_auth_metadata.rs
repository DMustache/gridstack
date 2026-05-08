use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct RegistrationFlowView {
    pub stages: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
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

pub enum AuthorizationMetadataResponse {
    Ok(Json<GetAuthMetadataView>),
}

impl IntoResponse for AuthorizationMetadataResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Ok(payload) => payload.into_response(),
        }
    }
}
