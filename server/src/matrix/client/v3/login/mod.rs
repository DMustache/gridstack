use std::time::Duration;

use axum::{
    Json, Router, extract::State, middleware::from_fn_with_state, response::IntoResponse,
    routing::get,
};
use models::{
    DeviceId,
    error::{AppError, AppResult},
    password::Password,
};
use persistence::{
    accounts::{Device, NewAccessToken, NewDevice},
    users::User,
};
use ruma::{UserId, api::client::error::ErrorKind};
use uuid::Uuid;

use crate::{
    consts::{
        ACCESS_TOKEN_EXPIRY_SECONDS, PASSWORD_LOGIN_TYPE, REFRESH_TOKEN_EXPIRY_SECONDS,
        USER_IDENTIFIER_TYPE,
    },
    services::{
        authorization::layers::optionally_authenticate_user, rate_limit::layers::rate_limit,
    },
    state::ServerState,
};

mod data_transfer_objects;
use data_transfer_objects::{
    GetLoginResponse, HomeserverInfo, LoginFlow, LoginIdentifier, LoginRequest, LoginResponse,
    WellKnown,
};

pub(super) fn router(state: ServerState) -> axum::Router<ServerState> {
    Router::new()
        .route("/", get(get_login).post(post_login))
        .route_layer(from_fn_with_state(
            state.clone(),
            optionally_authenticate_user,
        ))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/login",
    tag = "matrix-client",
    responses(
        (status = 200, description = "Supported login flows", body = crate::docs::GetLoginResponseDoc),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn get_login() -> Json<GetLoginResponse> {
    Json(GetLoginResponse {
        flows: vec![LoginFlow {
            login_type: PASSWORD_LOGIN_TYPE.to_owned(),
            get_login_token: None,
        }],
    })
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/login",
    tag = "matrix-client",
    request_body = crate::docs::LoginRequestDoc,
    responses(
        (status = 200, description = "Authenticated user session", body = crate::docs::LoginResponseDoc),
        (status = 400, description = "Invalid login payload", body = crate::docs::MatrixErrorResponse),
        (status = 403, description = "Invalid credentials", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn post_login(
    State(state): State<ServerState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    ensure_supported_login_type(&body)?;
    ensure_no_unsupported_legacy_fields(&body)?;

    let user_id = resolve_user_id(&state, &body)?;
    let user = state
        .users
        .get_by_user_id(user_id.clone())
        .await?
        .ok_or_else(|| AppError::forbidden("Invalid username or password"))?;

    verify_user_password(&user, &body)?;

    let device = resolve_device(&state, &body, &user).await?;
    let access_token = Uuid::new_v4().to_string();
    let refresh_token = body
        .refresh_token
        .unwrap_or_default()
        .then(|| Uuid::new_v4().to_string());
    let access_token_expires_at = chrono::Utc::now()
        + chrono::Duration::from_std(Duration::from_secs(ACCESS_TOKEN_EXPIRY_SECONDS)).unwrap();
    let refresh_token_expires_at = chrono::Utc::now()
        + chrono::Duration::from_std(Duration::from_secs(REFRESH_TOKEN_EXPIRY_SECONDS)).unwrap();

    state
        .accounts
        .create_access_token(NewAccessToken {
            access_token: access_token.clone(),
            user_id: user.user_id.clone(),
            device_id: device.device_id.clone(),
            refresh_token: refresh_token
                .clone()
                .or_else(|| Some(Uuid::new_v4().to_string())),
            access_token_expires_at,
            refresh_token_expires_at,
        })
        .await?;

    Ok(Json(LoginResponse {
        access_token,
        device_id: device.device_id,
        expires_in_ms: Some(ACCESS_TOKEN_EXPIRY_SECONDS * 1_000),
        home_server: Some(state.server_name.to_string()),
        refresh_token,
        user_id,
        well_known: Some(WellKnown {
            homeserver: HomeserverInfo {
                base_url: format!("http://{}", state.server_address),
            },
        }),
    }))
}

fn ensure_supported_login_type(body: &LoginRequest) -> AppResult<()> {
    if body.login_type == PASSWORD_LOGIN_TYPE {
        return Ok(());
    }

    Err(AppError::bad_request(ErrorKind::Unknown, "Bad login type."))
}

fn ensure_no_unsupported_legacy_fields(body: &LoginRequest) -> AppResult<()> {
    if body.identifier.is_some() || body.user.is_some() {
        return Ok(());
    }

    if body.address.is_some() || body.medium.is_some() || body.token.is_some() {
        return Err(AppError::bad_request(
            ErrorKind::Unknown,
            "Unsupported login payload.",
        ));
    }

    Err(AppError::missing_parameter("identifier.user is required"))
}

fn resolve_user_id(state: &ServerState, body: &LoginRequest) -> AppResult<String> {
    let user = body
        .identifier
        .as_ref()
        .map(extract_identifier_user)
        .transpose()?
        .or_else(|| body.user.clone())
        .ok_or_else(|| AppError::missing_parameter("identifier.user is required"))?;

    if user.starts_with('@') {
        return Ok(UserId::parse(&user)?.to_string());
    }

    Ok(UserId::parse_with_server_name(user, &state.server_name)?.to_string())
}

fn extract_identifier_user(identifier: &LoginIdentifier) -> AppResult<String> {
    if identifier.identifier_type != USER_IDENTIFIER_TYPE {
        return Err(AppError::bad_request(
            ErrorKind::Unknown,
            "Unsupported login identifier.",
        ));
    }

    identifier
        .user
        .clone()
        .ok_or_else(|| AppError::missing_parameter("identifier.user is required"))
}

fn verify_user_password(user: &User, body: &LoginRequest) -> AppResult<()> {
    let password = body
        .password
        .as_deref()
        .ok_or_else(|| AppError::missing_parameter("password is required"))?;
    let Some(password_hash) = user.password_hash.as_deref() else {
        return Err(AppError::forbidden("Invalid username or password"));
    };

    if Password::verify_raw(password, password_hash)? {
        return Ok(());
    }

    Err(AppError::forbidden("Invalid username or password"))
}

async fn resolve_device(
    state: &ServerState,
    body: &LoginRequest,
    user: &User,
) -> AppResult<Device> {
    let device_id = DeviceId::get_or_generate(body.device_id.clone());

    if let Some(device) = state
        .accounts
        .get_device(user.user_id.clone(), device_id.as_str().to_owned())
        .await?
    {
        return Ok(device);
    }

    state
        .accounts
        .create_device(NewDevice {
            user_id: user.user_id.clone(),
            device_id: device_id.as_str().to_owned(),
            display_name: body.initial_device_display_name.clone(),
        })
        .await?;

    Ok(Device {
        user_id: user.user_id.clone(),
        device_id: device_id.as_str().to_owned(),
        display_name: body.initial_device_display_name.clone(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
}
