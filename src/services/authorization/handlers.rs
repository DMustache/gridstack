use axum::{
    Extension, Json,
    extract::{Query, State},
};
use response_derive::IntoResponseEnum;

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
    },
    entities::AccessSession,
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
    GetAuthMetadataResponse::from_result(
        application_state.authorization_service.get_auth_metadata(),
    )
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
    GetLoginFlowsResponse::from_result(application_state.authorization_service.get_login_flows())
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
    CheckUsernameAvailableResponse::from_result(
        application_state
            .authorization_service
            .check_username_available(query_info.username),
    )
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
    match application_state.authorization_service.register_user(
        &query_info,
        register_user_info,
        application_state.allow_registration,
    ) {
        Ok(response) => RegisterUserResponse::Ok(Json(response)),
        Err(AuthorizationApplicationError::Unauthorized) => {
            RegisterUserResponse::Unauthorized(Json(
                application_state
                    .authorization_service
                    .registration_uiaa_challenge(),
            ))
        }
        Err(error) => RegisterUserResponse::from_mapped_error(error),
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
    LoginUserResponse::from_result(
        application_state
            .authorization_service
            .login_user(login_user_info),
    )
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
    Extension(access_session): Extension<AccessSession>,
) -> WhoAmIResponse {
    WhoAmIResponse::from_result(
        application_state
            .authorization_service
            .who_am_i_from_session(access_session),
    )
}

#[derive(IntoResponseEnum)]
pub enum LogoutUserResponse {
    #[matrix(status = 200)]
    Ok(Json<LogoutUserView>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = AuthorizationApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
}

pub async fn logout_user(
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSession>,
) -> LogoutUserResponse {
    LogoutUserResponse::from_result(
        application_state
            .authorization_service
            .logout_user(&access_session.access_token),
    )
}
