use std::{convert::Infallible, time::Instant};

use axum::{extract::FromRequestParts, http::request::Parts};
use http::header::AUTHORIZATION;
use thiserror::Error;

use crate::services::{authorization::entities::AccessToken, state::ApplicationState};

pub struct AuthorizationLayer;
pub struct OptionalAuthorizationLayer;

#[derive(Debug, Error)]
pub enum AuthorizationLayerError {
    #[error("access token is missing")]
    MissingToken,
    #[error("access token is invalid")]
    UnknownToken,
}

impl FromRequestParts<ApplicationState> for AuthorizationLayer {
    type Rejection = AuthorizationLayerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApplicationState,
    ) -> Result<Self, Self::Rejection> {
        let raw_token = Self::try_extract_access_token_from_parts(parts)
            .ok_or(AuthorizationLayerError::MissingToken)?;
        let access_token =
            AccessToken::parse(raw_token).ok_or(AuthorizationLayerError::UnknownToken)?;

        let access_session = state
            .authorization_service
            .authenticate_access_token(&access_token)
            .map_err(|_| AuthorizationLayerError::UnknownToken)?;

        parts.extensions.insert(access_session);

        Ok(Self)
    }
}

impl FromRequestParts<ApplicationState> for OptionalAuthorizationLayer {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApplicationState,
    ) -> Result<Self, Self::Rejection> {
        let authorized_user_identifier =
            AuthorizationLayer::try_extract_access_token_from_parts(parts)
                .and_then(AccessToken::parse)
                .and_then(|access_token| {
                    state
                        .authorization_service
                        .authenticate_access_token(&access_token)
                        .ok()
                        .map(|access_session| access_session.user_identifier().clone())
                });

        parts.extensions.insert(authorized_user_identifier);

        Ok(Self)
    }
}

impl AuthorizationLayer {
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

        parts.uri.query().and_then(Self::parse_access_token_query)
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
}

pub struct RateLimitLayer;

#[derive(Debug, Error)]
pub enum RateLimitLayerError {
    #[error("rate limit exceeded")]
    RateLimited,
    #[error("rate limiter failure")]
    Internal,
}

impl FromRequestParts<ApplicationState> for RateLimitLayer {
    type Rejection = RateLimitLayerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApplicationState,
    ) -> Result<Self, Self::Rejection> {
        let key = Self::rate_limit_key(parts);
        let allowed = state
            .rate_limiter
            .allow_request(&key, Instant::now())
            .map_err(|_| RateLimitLayerError::Internal)?;
        if !allowed {
            return Err(RateLimitLayerError::RateLimited);
        }

        Ok(Self)
    }
}

impl RateLimitLayer {
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
}
