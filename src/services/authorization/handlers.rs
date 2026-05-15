use axum::{
    Extension, Json,
    extract::{Query, State},
    http::{HeaderMap, header::AUTHORIZATION},
};
use response_derive::IntoResponseEnum;
use tracing::{error, info};

use crate::services::{
    authorization::{
        errors::AuthorizationApplicationError,
        handlers::{
            check_username_available::{CheckUsernameAvailableInfo, CheckUsernameAvailableView},
            get_auth_metadata::GetAuthMetadataView,
            get_login_flows::GetLoginFlowsView,
            login_user::{LoginUserInfo, LoginUserView},
            logout_user::LogoutUserView,
            register_user::{
                RegisterUserInfo, RegisterUserQueryInfo, RegisterUserView, UiaaResponseView,
            },
            who_am_i::WhoAmIView,
        },
        persistence::access_session_storage_unit::AccessSessionStorageUnit,
    },
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

pub mod check_username_available;
pub mod get_auth_metadata;
pub mod get_login_flows;
pub mod login_user;
pub mod logout_user;
pub mod register_user;
pub mod who_am_i;

#[derive(IntoResponseEnum)]
pub enum GetAuthMetadataResponse {
    #[matrix(status = 200)]
    Ok(Json<GetAuthMetadataView>),
    #[matrix(status = 404, error = [(matrix_error = "M_UNRECOGNIZED", from = AuthorizationApplicationError::OAuthAuthorizationUnsupported)])]
    Unrecognized(Json<MatrixErrorResponse>),
}

pub async fn get_auth_metadata(
    State(application_state): State<ApplicationState>,
) -> GetAuthMetadataResponse {
    match application_state.authorization_service.get_auth_metadata() {
        Ok(view) => {
            info!("authorization metadata requested");
            GetAuthMetadataResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            info!(error = %error_kind, "authorization metadata request failed");
            GetAuthMetadataResponse::from_mapped_error(error_kind)
        }
    }
}

#[derive(IntoResponseEnum)]
pub enum GetLoginFlowsResponse {
    #[matrix(status = 200)]
    Ok(Json<GetLoginFlowsView>),
    #[matrix(status = 404, error = [(matrix_error = "M_UNRECOGNIZED", from = AuthorizationApplicationError::LegacyAuthorizationUnsupported)])]
    BadRequest(Json<MatrixErrorResponse>),
}

pub async fn get_login_flows(
    State(application_state): State<ApplicationState>,
) -> GetLoginFlowsResponse {
    match application_state.authorization_service.get_login_flows() {
        Ok(view) => {
            info!("login flows requested");
            GetLoginFlowsResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            info!(error = %error_kind, "login flows request failed");
            GetLoginFlowsResponse::from_mapped_error(error_kind)
        }
    }
}

#[derive(IntoResponseEnum)]
pub enum CheckUsernameAvailableResponse {
    #[matrix(status = 200)]
    Ok(Json<CheckUsernameAvailableView>),
    #[matrix(
        status = 400,
        error = [
            (matrix_error = "M_INVALID_USERNAME", from = AuthorizationApplicationError::InvalidUsername),
            (matrix_error = "M_USER_IN_USE", from = AuthorizationApplicationError::UserInUse),
            (matrix_error = "M_EXCLUSIVE", from = AuthorizationApplicationError::Exclusive),
        ]
    )]
    BadRequest(Json<MatrixErrorResponse>),
}

pub async fn check_username_available(
    State(application_state): State<ApplicationState>,
    Query(query_info): Query<CheckUsernameAvailableInfo>,
) -> CheckUsernameAvailableResponse {
    match application_state
        .authorization_service
        .check_username_available(query_info.username)
    {
        Ok(view) => {
            info!("username availability checked");
            CheckUsernameAvailableResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            info!(error = %error_kind, "username availability check failed");
            CheckUsernameAvailableResponse::from_mapped_error(error_kind)
        }
    }
}

#[derive(IntoResponseEnum)]
pub enum RegisterUserResponse {
    #[matrix(status = 200)]
    Ok(Json<RegisterUserView>),
    #[matrix(status = 400, error = [
        (matrix_error = "M_INVALID_USERNAME", from = AuthorizationApplicationError::InvalidUsername),
        (matrix_error = "M_USER_IN_USE", from = AuthorizationApplicationError::UserInUse),
        (matrix_error = "M_EXCLUSIVE", from = AuthorizationApplicationError::Exclusive),
        (matrix_error = "M_INVALID_PARAM", from = AuthorizationApplicationError::InvalidCredentials)])]
    InvalidUsername(Json<MatrixErrorResponse>),
    #[matrix(status = 401)]
    Unauthorized(Json<UiaaResponseView>),
    #[matrix(status = 403, error = [
        (matrix_error = "M_FORBIDDEN", from = AuthorizationApplicationError::RegistrationDisabled),
        (matrix_error = "M_FORBIDDEN", from = AuthorizationApplicationError::GuestRegistrationDisabled)
    ])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [
        (matrix_error = "M_BAD_JSON", from = AuthorizationApplicationError::RegisterInvalidParameters),
        (matrix_error = "M_UNKNOWN", from = AuthorizationApplicationError::Internal)
    ])]
    BadJson(Json<MatrixErrorResponse>),
}

pub async fn register_user(
    State(application_state): State<ApplicationState>,
    Query(query_info): Query<RegisterUserQueryInfo>,
    Json(register_user_info): Json<RegisterUserInfo>,
) -> RegisterUserResponse {
    match application_state
        .authorization_service
        .register_user(&query_info, &register_user_info)
    {
        Ok(response) => {
            info!("user registration completed");
            RegisterUserResponse::Ok(Json(response))
        }
        Err(AuthorizationApplicationError::Unauthorized) => {
            info!("user registration requires additional auth");
            RegisterUserResponse::Unauthorized(Json(
                application_state
                    .authorization_service
                    .registration_uiaa_challenge(),
            ))
        }
        Err(error_kind) => {
            match error_kind {
                AuthorizationApplicationError::Internal => {
                    error!("user registration failed with internal error");
                }
                _ => info!(error = %error_kind, "user registration rejected"),
            }
            RegisterUserResponse::from_mapped_error(error_kind)
        }
    }
}

#[derive(IntoResponseEnum)]
pub enum LoginUserResponse {
    #[matrix(status = 200)]
    Ok(Json<LoginUserView>),
    #[matrix(status = 400, error = [(matrix_error = "M_UNKNOWN", from = AuthorizationApplicationError::Unrecognized)])]
    Unrecognized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [
        (matrix_error = "M_FORBIDDEN", from = AuthorizationApplicationError::InvalidCredentials),
        (matrix_error = "M_FORBIDDEN", from = AuthorizationApplicationError::InvalidUsername),
        (matrix_error = "M_FORBIDDEN", from = AuthorizationApplicationError::Forbidden)
    ])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = AuthorizationApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

pub async fn login_user(
    State(application_state): State<ApplicationState>,
    Json(login_user_info): Json<LoginUserInfo>,
) -> LoginUserResponse {
    match application_state
        .authorization_service
        .login_user(login_user_info)
    {
        Ok(view) => {
            info!("user login completed");
            LoginUserResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            match error_kind {
                AuthorizationApplicationError::Internal => {
                    error!("user login failed with internal error");
                }
                _ => info!(error = %error_kind, "user login rejected"),
            }
            LoginUserResponse::from_mapped_error(error_kind)
        }
    }
}

#[derive(IntoResponseEnum)]
pub enum WhoAmIResponse {
    #[matrix(status = 200)]
    Ok(Json<WhoAmIView>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = AuthorizationApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = AuthorizationApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
}

pub async fn who_am_i(
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> WhoAmIResponse {
    match application_state
        .authorization_service
        .who_am_i_from_session(&access_session)
    {
        Ok(view) => {
            info!("whoami requested");
            WhoAmIResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            info!(error = %error_kind, "whoami request failed");
            WhoAmIResponse::from_mapped_error(error_kind)
        }
    }
}

#[derive(IntoResponseEnum)]
pub enum LogoutUserResponse {
    #[matrix(status = 200)]
    Ok(Json<LogoutUserView>),
}

pub async fn logout_user(
    State(application_state): State<ApplicationState>,
    Query(query_info): Query<LogoutQueryInfo>,
    headers: HeaderMap,
) -> LogoutUserResponse {
    if let Some(raw_access_token) = extract_logout_access_token(&headers, &query_info)
        && let Some(access_token) =
            crate::services::authorization::entities::AccessToken::parse(raw_access_token)
    {
        match application_state
            .authorization_service
            .authenticate_access_token(&access_token)
        {
            Ok(_) => match application_state
                .authorization_service
                .logout_user(&access_token)
            {
                Ok(_) => info!("user logout completed for valid token"),
                Err(error_kind) => match error_kind {
                    AuthorizationApplicationError::Internal => {
                        error!("user logout failed with internal error");
                    }
                    _ => {
                        info!(error = %error_kind, "user logout rejected after auth success");
                    }
                },
            },
            Err(error_kind) => match error_kind {
                AuthorizationApplicationError::Unauthorized => {
                    info!("user logout called with invalid or expired token");
                }
                _ => {
                    error!(error = %error_kind, "user logout token validation failed");
                }
            },
        }
    } else {
        info!("user logout called without token; returning success");
    }

    LogoutUserResponse::Ok(Json(LogoutUserView {}))
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct LogoutQueryInfo {
    pub access_token: Option<String>,
}

fn extract_logout_access_token(
    headers: &HeaderMap,
    query_info: &LogoutQueryInfo,
) -> Option<String> {
    if let Some(value) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    {
        let mut tokens = value.split_whitespace();
        if let (Some(scheme), Some(token)) = (tokens.next(), tokens.next())
            && scheme.eq_ignore_ascii_case("bearer")
            && !token.is_empty()
        {
            return Some(token.to_owned());
        }
    }

    query_info
        .access_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}
