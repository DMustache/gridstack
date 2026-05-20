use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::{error, info};

use crate::services::identity::entities::IdentityEntityError;
use crate::services::shared::MatrixErrorResponse;

#[derive(Debug, thiserror::Error)]
pub enum IdentityServiceError {
    #[error("identity resource not found")]
    NotFound,
    #[error("identity public key not found")]
    PublicKeyNotFound,
    #[error("identity authentication required")]
    AuthenticationRequired,
    #[error("identity request is forbidden")]
    Forbidden,
    #[error("invalid identity request: {0}")]
    InvalidRequest(String),
    #[error("identity request is missing parameters: {0}")]
    MissingParameters(String),
    #[error("identity operation is not implemented")]
    NotImplemented,
    #[error("identity internal failure")]
    Internal(#[source] anyhow::Error),
}

impl From<IdentityEntityError> for IdentityServiceError {
    fn from(value: IdentityEntityError) -> Self {
        Self::InvalidRequest(value.to_string())
    }
}

impl IntoResponse for IdentityServiceError {
    fn into_response(self) -> Response {
        match &self {
            Self::Internal(internal_error) => {
                error!(error = %internal_error, "identity service internal error");
            }
            _ => {
                info!(error = %self, "identity service request failed");
            }
        }

        let status_code = match self {
            Self::NotFound | Self::PublicKeyNotFound => StatusCode::NOT_FOUND,
            Self::AuthenticationRequired => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::InvalidRequest(_) | Self::MissingParameters(_) => StatusCode::BAD_REQUEST,
            Self::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let (errcode, error) = match self {
            Self::NotFound => (
                "M_NOT_FOUND".to_owned(),
                "The identity resource was not found".to_owned(),
            ),
            Self::PublicKeyNotFound => (
                "M_NOT_FOUND".to_owned(),
                "The public key was not found".to_owned(),
            ),
            Self::AuthenticationRequired => (
                "M_UNAUTHORIZED".to_owned(),
                "Authentication is required".to_owned(),
            ),
            Self::Forbidden => ("M_FORBIDDEN".to_owned(), "Forbidden".to_owned()),
            Self::InvalidRequest(message) => ("M_INVALID_PARAM".to_owned(), message),
            Self::MissingParameters(message) => ("M_MISSING_PARAMS".to_owned(), message),
            Self::NotImplemented => (
                "M_UNRECOGNIZED".to_owned(),
                "Identity endpoint is not implemented".to_owned(),
            ),
            Self::Internal(error) => ("M_UNKNOWN".to_owned(), error.to_string()),
        };

        (status_code, Json(MatrixErrorResponse { errcode, error })).into_response()
    }
}
