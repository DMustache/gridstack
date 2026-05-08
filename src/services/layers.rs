use std::time::Instant;

use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use http::header::AUTHORIZATION;

use crate::services::{
    shared::{MatrixErrorResponse, MatrixRateLimitErrorResponse},
    state::ApplicationState,
};

pub struct AuthorizationLayer;

impl FromRequestParts<ApplicationState> for AuthorizationLayer {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApplicationState,
    ) -> Result<Self, Self::Rejection> {
        let token = try_extract_access_token_from_parts(parts).ok_or_else(|| {
            matrix_error(
                StatusCode::UNAUTHORIZED,
                "M_MISSING_TOKEN",
                "No access token was specified for the request.",
            )
        })?;

        let user_id = state
            .authorization_service
            .authenticate_access_token(&token)
            .map_err(|_| {
                matrix_error(
                    StatusCode::UNAUTHORIZED,
                    "The access token specified was not recognised",
                    "Unknown access token",
                )
            })?;

        parts.extensions.insert(user_id);

        Ok(Self)
    }
}

pub struct RateLimitLayer;

impl FromRequestParts<ApplicationState> for RateLimitLayer {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApplicationState,
    ) -> Result<Self, Self::Rejection> {
        let key = rate_limit_key(parts);
        let allowed = state
            .rate_limiter
            .allow_request(&key, Instant::now())
            .map_err(|_| {
                matrix_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "M_UNKNOWN",
                    "Rate limiter failure",
                )
            })?;
        if !allowed {
            return Err(matrix_rate_limit_error(1));
        }

        Ok(Self)
    }
}

fn try_extract_access_token_from_parts(parts: &Parts) -> Option<String> {
    if let Some(authorization_value) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    {
        let mut tokens = authorization_value.split_whitespace();
        if let (Some(scheme), Some(token)) = (tokens.next(), tokens.next())
            && scheme.eq_ignore_ascii_case("bearer")
            && !token.is_empty()
        {
            return Some(token.to_owned());
        }
    }

    parts.uri.query().and_then(parse_access_token_query)
}

fn parse_access_token_query(query: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        if key == "access_token" && !value.is_empty() {
            return Some(value.to_owned());
        }
    }
    None
}

fn rate_limit_key(parts: &Parts) -> String {
    if let Some(value) = parts
        .headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        let first = value.split(',').next().unwrap_or_default().trim();
        if !first.is_empty() {
            return format!("ip:{first}");
        }
    }

    if let Some(value) = parts
        .headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
    {
        let ip = value.trim();
        if !ip.is_empty() {
            return format!("ip:{ip}");
        }
    }

    "ip:unknown".to_owned()
}

fn matrix_error(status: StatusCode, errcode: &str, message: &str) -> Response {
    (
        status,
        Json(MatrixErrorResponse {
            errcode: errcode.to_owned(),
            error: message.to_owned(),
        }),
    )
        .into_response()
}

fn matrix_rate_limit_error(retry_after: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(MatrixRateLimitErrorResponse {
            base: MatrixErrorResponse {
                errcode: "M_LIMIT_EXCEEDED".to_owned(),
                error: "Too many requests".to_owned(),
            },
            retry_after,
        }),
    )
        .into_response()
}
