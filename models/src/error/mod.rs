use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ruma::{IdParseError, api::client::error::ErrorKind};
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

mod error_variants;
pub use error_variants::{InternalError, MatrixError, internal_error};

use crate::error::internal_error::DatabaseError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Matrix(#[from] MatrixError),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

impl AppError {
    pub fn bad_request(error_code: ErrorKind, error: impl Into<String>) -> Self {
        Self::Matrix(MatrixError::bad_request(error_code, error))
    }

    pub fn unauthorized(error: impl Into<String>) -> Self {
        Self::Matrix(MatrixError::unauthorized(error))
    }

    pub fn forbidden(error: impl Into<String>) -> Self {
        Self::Matrix(MatrixError::forbidden(error))
    }

    pub fn guest_access_forbidden(error: impl Into<String>) -> Self {
        Self::Matrix(MatrixError::guest_access_forbidden(error))
    }

    pub fn weak_password(error: impl Into<String>) -> Self {
        Self::Matrix(MatrixError::weak_password(error))
    }

    pub fn with_status(
        status: StatusCode,
        error_code: ErrorKind,
        error: impl Into<String>,
    ) -> Self {
        Self::Matrix(MatrixError::with_status(status, error_code, error))
    }

    pub fn missing_parameter(error: &str) -> Self {
        Self::Matrix(MatrixError::missing_parameter(error))
    }

    pub fn invalid_username(error: impl Into<String>) -> Self {
        Self::Matrix(MatrixError::invalid_username(error))
    }

    pub fn internal_error(error_code: ErrorKind) -> Self {
        Self::Matrix(MatrixError::internal_error(error_code))
    }
}

impl From<IdParseError> for AppError {
    fn from(value: IdParseError) -> Self {
        AppError::Internal(InternalError::InvalidUserId(value))
    }
}

impl From<DatabaseError> for AppError {
    fn from(value: DatabaseError) -> Self {
        AppError::Internal(InternalError::DatabaseError(value))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Matrix(error) => error.into_response(),
            AppError::Internal(error) => {
                tracing::error!(error = ?error, "Unexpected error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
