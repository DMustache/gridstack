mod request_invalid_info;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use request_invalid_info::RequestInvalidInfo;
use ruma::api::client::error::ErrorKind;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
#[error("MatrixError: {status} {body:?}")]
pub struct MatrixError {
    status: StatusCode,
    body: RequestInvalidInfo,
}

impl MatrixError {
    pub(crate) fn bad_request(error_code: ErrorKind, error: impl Into<String>) -> Self {
        Self::with_status(StatusCode::BAD_REQUEST, error_code, error)
    }

    pub(crate) fn forbidden(error: impl Into<String>) -> Self {
        Self::with_status(StatusCode::FORBIDDEN, ErrorKind::forbidden(), error)
    }

    pub(crate) fn guest_access_forbidden(error: impl Into<String>) -> Self {
        Self::with_status(
            StatusCode::FORBIDDEN,
            ErrorKind::GuestAccessForbidden,
            error,
        )
    }

    pub(crate) fn weak_password(error: impl Into<String>) -> Self {
        Self::with_status(StatusCode::BAD_REQUEST, ErrorKind::WeakPassword, error)
    }

    pub(crate) fn with_status(
        status: StatusCode,
        error_code: ErrorKind,
        error: impl Into<String>,
    ) -> Self {
        Self {
            status,
            body: RequestInvalidInfo {
                error_code,
                error: error.into(),
                retry_after_ms: None,
            },
        }
    }

    pub fn missing_parameter(error: &str) -> Self {
        Self::with_status(
            StatusCode::UNAUTHORIZED,
            ErrorKind::MissingParam,
            error.to_string(),
        )
    }

    pub fn invalid_username(error: impl Into<String>) -> Self {
        Self::with_status(
            StatusCode::BAD_REQUEST,
            ErrorKind::InvalidUsername,
            error.into(),
        )
    }

    pub(crate) fn internal_error(error_code: ErrorKind) -> Self {
        Self::with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            error_code,
            "Internal Server Error",
        )
    }

    pub(crate) fn unauthorized(error: impl Into<String>) -> MatrixError {
        Self::with_status(
            StatusCode::UNAUTHORIZED,
            ErrorKind::Unauthorized,
            error.into(),
        )
    }
}

impl IntoResponse for MatrixError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
