use anyhow::Error as AnyhowError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::services::shared::MatrixErrorResponse;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("entity already exists")]
    AlreadyExists,
    #[error("entity not found")]
    NotFound,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("resource already exists")]
    Conflict,
    #[error("resource not found")]
    NotFound,
    #[error("authentication required")]
    AuthenticationRequired,
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("forbidden")]
    Forbidden,
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("internal failure")]
    Internal(#[source] AnyhowError),
}

impl From<DomainError> for ApplicationError {
    fn from(value: DomainError) -> Self {
        match value {
            DomainError::AlreadyExists => Self::Conflict,
            DomainError::NotFound => Self::NotFound,
            DomainError::InvalidCredentials => Self::AuthenticationFailed,
            DomainError::InvalidRequest(message) => Self::InvalidInput(message),
        }
    }
}

impl ApplicationError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let status_code = match &self {
            ApplicationError::Conflict => StatusCode::CONFLICT,
            ApplicationError::NotFound => StatusCode::NOT_FOUND,
            ApplicationError::AuthenticationRequired => StatusCode::UNAUTHORIZED,
            ApplicationError::AuthenticationFailed => StatusCode::UNAUTHORIZED,
            ApplicationError::Forbidden => StatusCode::FORBIDDEN,
            ApplicationError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            ApplicationError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            ApplicationError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let (errcode, error) = match self {
            ApplicationError::Conflict => (
                "M_USER_IN_USE".to_owned(),
                "Resource already exists".to_owned(),
            ),
            ApplicationError::NotFound => {
                ("M_NOT_FOUND".to_owned(), "Resource not found".to_owned())
            }
            ApplicationError::AuthenticationRequired => (
                "M_MISSING_TOKEN".to_owned(),
                "Authentication required".to_owned(),
            ),
            ApplicationError::AuthenticationFailed => {
                ("M_FORBIDDEN".to_owned(), "Invalid credentials".to_owned())
            }
            ApplicationError::Forbidden => ("M_FORBIDDEN".to_owned(), "Forbidden".to_owned()),
            ApplicationError::NotImplemented(message) => ("M_UNRECOGNIZED".to_owned(), message),
            ApplicationError::InvalidInput(message) => ("M_BAD_JSON".to_owned(), message),
            ApplicationError::Internal(error) => ("M_UNKNOWN".to_owned(), error.to_string()),
        };

        (status_code, Json(MatrixErrorResponse { errcode, error })).into_response()
    }
}
