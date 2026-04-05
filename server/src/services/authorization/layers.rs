use axum::{
    Extension,
    extract::State,
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use models::error::{AppError, AppResult};
use persistence::accounts::AuthenticatedSession;

use crate::state::ServerState;

#[derive(Debug, Clone)]
pub(crate) struct AuthenticatedUser {
    pub(crate) user_id: String,
    pub(crate) device_id: String,
    pub(crate) device_display_name: Option<String>,
    pub(crate) access_token: String,
    pub(crate) is_guest: bool,
}

impl From<AuthenticatedSession> for AuthenticatedUser {
    fn from(value: AuthenticatedSession) -> Self {
        Self {
            user_id: value.user_id,
            device_id: value.device_id,
            device_display_name: value.device_display_name,
            access_token: value.access_token,
            is_guest: value.is_guest,
        }
    }
}

pub(crate) async fn require_authenticated_user(
    State(state): State<ServerState>,
    mut request: axum::extract::Request,
    next: Next,
) -> AppResult<Response> {
    let user = try_authenticate_user(&state, request.headers())
        .await?
        .ok_or_else(|| AppError::forbidden("Authentication is required for this endpoint"))?;

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

pub(crate) type AuthenticatedUserExtension = Extension<AuthenticatedUser>;
pub(crate) type OptionalAuthenticatedUserExtension = Extension<Option<AuthenticatedUser>>;

pub(crate) async fn optionally_authenticate_user(
    State(state): State<ServerState>,
    mut request: axum::extract::Request,
    next: Next,
) -> AppResult<Response> {
    let user = try_authenticate_user(&state, request.headers()).await?;
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

async fn try_authenticate_user(
    state: &ServerState,
    headers: &HeaderMap,
) -> AppResult<Option<AuthenticatedUser>> {
    let Some(access_token) = try_extract_bearer_token(headers) else {
        return Ok(None);
    };

    let Some(session) = state
        .accounts
        .try_get_authenticated_session_by_access_token(access_token)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(session.into()))
}

pub(crate) fn try_extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let authorization = headers.get(AUTHORIZATION)?.to_str().ok()?;

    authorization
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
}
