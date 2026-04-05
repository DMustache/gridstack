use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::post,
};
use models::{
    error::{AppError, AppResult},
    register::uiaa::AuthorizationData,
    username::Localpart,
};
use persistence::{
    accounts::{NewAccessToken, NewDevice, NewRegistrationSession},
    users::NewUser,
};
use ruma::{
    OwnedUserId, UserId,
    api::client::error::ErrorKind,
    exports::serde_json::{Value, json},
};
use uuid::Uuid;

use crate::{
    consts::{
        ACCESS_TOKEN_EXPIRY_SECONDS, REFRESH_TOKEN_EXPIRY_SECONDS, SUPPORTED_AUTHORIZATION_TYPES,
    },
    services::rate_limit::layers::rate_limit,
    state::ServerState,
};

mod data_transfer_objects;
use data_transfer_objects::{BodyInfo, BodyView, QueryParameters, UserParameters};

pub(crate) fn router(state: ServerState) -> axum::Router<ServerState> {
    Router::new()
        .route("/", post(post_register))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/register",
    tag = "matrix-client",
    params(
        ("kind" = Option<String>, Query, description = "Registration kind. Guest registration is currently rejected.")
    ),
    request_body = crate::docs::RegistrationRequestDoc,
    responses(
        (status = 200, description = "Registered user", body = crate::docs::RegistrationResponseDoc),
        (status = 401, description = "UIAA authentication flow or auth failure", body = crate::docs::RegistrationUiaaResponseDoc),
        (status = 403, description = "Registration forbidden", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn post_register(
    State(state): State<ServerState>,
    Query(query_parameters): Query<QueryParameters>,
    Json(body_parameters): Json<BodyInfo>,
) -> AppResult<Response> {
    ensure_registration_allowed(&state)?;

    if query_parameters.kind.is_guest() {
        block_guest_registration()?;
    }

    validate_requested_username(&state, body_parameters.username.as_deref()).await?;

    let authorization = match body_parameters.authorization.as_ref() {
        Some(authorization_parameter) => authorization_parameter,
        None => return Ok(provide_authorization_flows(&state).await?.into_response()),
    };

    let session_id = authorization
        .get_session_id()
        .ok_or_else(|| AppError::missing_parameter("auth.session is required"))?;

    let account_id = state
        .accounts
        .get_account_by_registration_session(session_id)
        .await
        .map_err(|_| {
            AppError::with_status(
                StatusCode::UNAUTHORIZED,
                ErrorKind::Unknown,
                "Unknown or expired UIAA session",
            )
        })?;

    validate_registration_token(&state, authorization)?;

    let user_parameters = UserParameters::try_from((body_parameters, &state))?;
    let body_view = register_user(&state, account_id, user_parameters).await?;

    Ok(Json(body_view).into_response())
}

fn ensure_registration_allowed(state: &ServerState) -> AppResult<()> {
    if state.is_registration_allowed {
        return Ok(());
    }

    Err(AppError::forbidden("Registration disabled").into())
}

async fn provide_authorization_flows(state: &ServerState) -> AppResult<RegistrationUiaaInfo> {
    let session_id = Uuid::new_v4().to_string();

    state
        .accounts
        .create_pending_registration_session(NewRegistrationSession {
            session_id: session_id.clone(),
            completed_stages: json!([]),
        })
        .await?;

    Ok(RegistrationUiaaInfo {
        completed: Vec::new(),
        flows: vec![AuthFlow {
            stages: SUPPORTED_AUTHORIZATION_TYPES
                .iter()
                .map(|authorization_type| authorization_type.to_string())
                .collect(),
        }],
        params: json!({}),
        session: session_id,
    })
}

fn block_guest_registration() -> AppResult<()> {
    Err(AppError::forbidden("Guest registration disabled").into())
}

async fn validate_requested_username(state: &ServerState, username: Option<&str>) -> AppResult<()> {
    let Some(username) = username else {
        return Ok(());
    };

    let localpart = Localpart::try_new(username.to_owned())?;
    let user_id = UserId::parse_with_server_name(localpart.as_str(), &state.server_name)?;

    if state.users.exists(user_id.to_string()).await? {
        return Err(AppError::bad_request(
            ErrorKind::UserInUse,
            "User ID is already taken",
        ));
    }

    Ok(())
}

fn validate_registration_token(
    state: &ServerState,
    authorization: &AuthorizationData,
) -> AppResult<()> {
    match authorization {
        AuthorizationData::RegistrationToken(registration_token) => {
            if registration_token.token == state.registration_token {
                return Ok(());
            }

            Err(AppError::with_status(
                StatusCode::UNAUTHORIZED,
                ErrorKind::forbidden(),
                "Invalid registration token",
            ))
        }
    }
}

async fn register_user(
    state: &ServerState,
    account_id: Uuid,
    user_parameters: UserParameters,
) -> AppResult<BodyView> {
    let user_id = map_username_to_user_id(state, &user_parameters)?;

    if state.users.exists(user_id.to_string()).await? {
        return Err(AppError::bad_request(
            ErrorKind::UserInUse,
            "User ID is already taken",
        ));
    }

    state
        .users
        .create_user(NewUser {
            user_id: user_id.to_string(),
            account_id,
            is_guest: false,
            password_hash: Some(user_parameters.password.to_hash()?),
        })
        .await?;

    if user_parameters.inhibit_login {
        return Ok(BodyView {
            access_token: None,
            device_id: None,
            expires_in_ms: None,
            home_server: Some(state.server_name.to_string()),
            refresh_token: None,
            user_id: user_id.to_string(),
        });
    }

    let device_id = user_parameters.device_id.as_str().to_owned();
    state
        .accounts
        .create_device(NewDevice {
            user_id: user_id.to_string(),
            device_id: device_id.clone(),
            display_name: user_parameters.initial_device_display_name,
        })
        .await?;

    let access_token = Uuid::new_v4().to_string();
    let refresh_token = user_parameters
        .refresh_token
        .then(|| Uuid::new_v4().to_string());
    let access_token_expires_at = chrono::Utc::now()
        + chrono::Duration::from_std(Duration::from_secs(ACCESS_TOKEN_EXPIRY_SECONDS)).unwrap();
    let refresh_token_expires_at = chrono::Utc::now()
        + chrono::Duration::from_std(Duration::from_secs(REFRESH_TOKEN_EXPIRY_SECONDS)).unwrap();

    state
        .accounts
        .create_access_token(NewAccessToken {
            access_token: access_token.clone(),
            user_id: user_id.to_string(),
            device_id: device_id.clone(),
            refresh_token: refresh_token
                .clone()
                .or_else(|| Some(Uuid::new_v4().to_string())),
            access_token_expires_at,
            refresh_token_expires_at,
        })
        .await?;

    Ok(BodyView {
        access_token: Some(access_token),
        device_id: Some(device_id),
        expires_in_ms: None,
        home_server: Some(state.server_name.to_string()),
        refresh_token,
        user_id: user_id.to_string(),
    })
}

fn map_username_to_user_id(
    state: &ServerState,
    user_parameters: &UserParameters,
) -> AppResult<OwnedUserId> {
    Ok(UserId::parse_with_server_name(
        user_parameters.username.as_str(),
        &state.server_name,
    )?)
}

#[derive(Debug, Clone, serde::Serialize)]
struct AuthFlow {
    stages: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RegistrationUiaaInfo {
    completed: Vec<String>,
    flows: Vec<AuthFlow>,
    params: Value,
    session: String,
}

impl IntoResponse for RegistrationUiaaInfo {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json(self)).into_response()
    }
}
