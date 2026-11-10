use anyhow::Error as AnyhowError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use response_derive::IntoResponseEnum;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tracing::{error, info};

use crate::services::{
    layers::{AuthorizationLayerError, RateLimitLayerError},
    shared::{MatrixErrorResponse, MatrixRateLimitErrorResponse},
};

static RATE_LIMIT_RETRY_AFTER_MS: AtomicU64 = AtomicU64::new(2000);

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
    #[error("infrastructure unavailable")]
    Unavailable(#[source] AnyhowError),
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
            DomainError::Unavailable(source) => Self::Internal(source),
        }
    }
}

impl ApplicationError {
    pub fn invalid_input<T: Into<String>>(message: T) -> Self {
        Self::InvalidInput(message.into())
    }
}

pub fn set_rate_limit_retry_after_ms(value: u64) {
    RATE_LIMIT_RETRY_AFTER_MS.store(value, Ordering::Relaxed);
}

fn configured_rate_limit_retry_after_ms() -> u64 {
    RATE_LIMIT_RETRY_AFTER_MS.load(Ordering::Relaxed)
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        match &self {
            Self::Internal(internal_error) => {
                error!(error = %format!("{internal_error:#}"), "application internal error");
            }
            _ => {
                info!(error = %self, "application request failed");
            }
        }

        let status_code = match self {
            Self::Conflict => StatusCode::CONFLICT,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AuthenticationRequired | Self::AuthenticationFailed => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let (errcode, error) = match self {
            Self::Conflict => (
                "M_USER_IN_USE".to_owned(),
                "Resource already exists".to_owned(),
            ),
            Self::NotFound => ("M_NOT_FOUND".to_owned(), "Resource not found".to_owned()),
            Self::AuthenticationRequired => (
                "M_MISSING_TOKEN".to_owned(),
                "Authentication required".to_owned(),
            ),
            Self::AuthenticationFailed => {
                ("M_FORBIDDEN".to_owned(), "Invalid credentials".to_owned())
            }
            Self::Forbidden => ("M_FORBIDDEN".to_owned(), "Forbidden".to_owned()),
            Self::NotImplemented(message) => ("M_UNRECOGNIZED".to_owned(), message),
            Self::InvalidInput(message) => ("M_BAD_JSON".to_owned(), message),
            Self::Internal(_) => ("M_UNKNOWN".to_owned(), "Internal server error".to_owned()),
        };

        (status_code, Json(MatrixErrorResponse { errcode, error })).into_response()
    }
}

#[derive(IntoResponseEnum)]
pub enum AuthorizationLayerResponse {
    #[matrix(
        status = 401,
        error = [
            (matrix_error = "M_MISSING_TOKEN", from = AuthorizationLayerError::MissingToken),
            (matrix_error = "M_UNKNOWN_TOKEN", from = AuthorizationLayerError::UnknownToken),
        ]
    )]
    Unauthorized(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum RateLimitLayerResponse {
    #[matrix(
        status = 429,
        error = [(matrix_error = "M_LIMIT_EXCEEDED", from = RateLimitLayerError::RateLimited)]
    )]
    RateLimited(Json<MatrixRateLimitErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RateLimitLayerError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

impl IntoResponse for AuthorizationLayerError {
    fn into_response(self) -> Response {
        info!(error = %self, "authorization layer rejected request");
        match self {
            Self::MissingToken => (
                StatusCode::UNAUTHORIZED,
                Json(MatrixErrorResponse {
                    errcode: "M_MISSING_TOKEN".to_owned(),
                    error: "No access token was specified for the request.".to_owned(),
                }),
            )
                .into_response(),
            Self::UnknownToken => (
                StatusCode::UNAUTHORIZED,
                Json(MatrixErrorResponse {
                    errcode: "M_UNKNOWN_TOKEN".to_owned(),
                    error: "Unknown access token".to_owned(),
                }),
            )
                .into_response(),
        }
    }
}

impl IntoResponse for RateLimitLayerError {
    fn into_response(self) -> Response {
        match &self {
            Self::Internal => error!("rate limit layer internal failure"),
            Self::RateLimited => info!("rate limit layer rejected request"),
        }
        match self {
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(MatrixRateLimitErrorResponse {
                    base: MatrixErrorResponse {
                        errcode: "M_LIMIT_EXCEEDED".to_owned(),
                        error: "Too many requests".to_owned(),
                    },
                    retry_after_ms: configured_rate_limit_retry_after_ms(),
                }),
            )
                .into_response(),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(MatrixErrorResponse {
                    errcode: "M_UNKNOWN".to_owned(),
                    error: "Rate limiter failure".to_owned(),
                }),
            )
                .into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn response_parts(error: DomainError) -> (StatusCode, serde_json::Value) {
        let response = ApplicationError::from(error).into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn unavailable_maps_to_500_m_unknown() {
        let (status, body) =
            response_parts(DomainError::Unavailable(anyhow::anyhow!("db down"))).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["errcode"], "M_UNKNOWN");
        assert_eq!(body["error"], "Internal server error");
    }

    #[tokio::test]
    async fn internal_error_body_does_not_contain_source_text() {
        let source = anyhow::anyhow!("relation \"accounts\" violates constraint accounts_pkey");
        let (_, body) = response_parts(DomainError::Unavailable(source)).await;
        let rendered = body.to_string();
        assert!(!rendered.contains("accounts_pkey"));
        assert!(!rendered.contains("relation"));
    }

    #[tokio::test]
    async fn invalid_request_still_maps_to_400() {
        let (status, body) = response_parts(DomainError::InvalidRequest("bad".to_owned())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["errcode"], "M_BAD_JSON");
        assert_eq!(body["error"], "bad");
    }
}
