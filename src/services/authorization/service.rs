use std::sync::Arc;

use crate::{
    infrastructure::configuration::AuthenticationConfiguration,
    services::{
        repositories::{SessionRepository, UserRepository},
        traits::IdGenerator,
    },
};

use super::{
    entities::{AccessSession, AccessToken, UserAccount, UserId},
    errors::AuthorizationApplicationError,
    handlers::{
        check_username_available::CheckUsernameAvailableView,
        get_auth_metadata::GetAuthMetadataView,
        get_login_flows::{GetLoginFlowsView, LoginFlowView},
        login_user::{LoginUserInfo, LoginUserView},
        register_user::{RegisterUserInfo, RegisterUserQueryInfo, RegisterUserView},
        who_am_i::WhoAmIView,
    },
};

pub struct AuthorizationService {
    user_repository: Arc<dyn UserRepository>,
    session_repository: Arc<dyn SessionRepository>,
    id_generator: Arc<dyn IdGenerator>,
    home_server_name: String,
    configuration: AuthenticationConfiguration,
}

impl AuthorizationService {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        session_repository: Arc<dyn SessionRepository>,
        id_generator: Arc<dyn IdGenerator>,
        home_server_name: String,
        configuration: AuthenticationConfiguration,
    ) -> Self {
        Self {
            user_repository,
            session_repository,
            id_generator,
            home_server_name,
            configuration,
        }
    }

    pub fn get_auth_metadata(&self) -> Result<GetAuthMetadataView, AuthorizationApplicationError> {
        Err(AuthorizationApplicationError::Unrecognized)
    }

    pub fn get_login_flows(&self) -> Result<GetLoginFlowsView, AuthorizationApplicationError> {
        Ok(GetLoginFlowsView {
            flows: vec![LoginFlowView {
                type_field: "m.login.password".to_owned(),
                get_login_token: Some(false),
            }],
        })
    }

    pub fn check_username_available(
        &self,
        username: String,
    ) -> Result<CheckUsernameAvailableView, AuthorizationApplicationError> {
        let user_identifier = self.try_get_user_id(&username)?;
        Ok(CheckUsernameAvailableView {
            available: !self.user_repository.user_exists(&user_identifier),
        })
    }

    pub fn register_user(
        &self,
        query: RegisterUserQueryInfo,
        info: RegisterUserInfo,
        allow_registration: bool,
    ) -> Result<RegisterUserView, AuthorizationApplicationError> {
        let _ = (
            &info.auth,
            &info.refresh_token,
            &info.initial_device_display_name,
        );

        if !allow_registration {
            return Err(AuthorizationApplicationError::Forbidden);
        }

        if query.kind.as_deref().unwrap_or("user") == "guest" {
            return Err(AuthorizationApplicationError::Forbidden);
        }

        let username = info
            .username
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(AuthorizationApplicationError::InvalidUsername)?;
        let password = info
            .password
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or(AuthorizationApplicationError::InvalidCredentials)?;

        let user_identifier = self.try_get_user_id(username)?;
        if self.user_repository.user_exists(&user_identifier) {
            return Err(AuthorizationApplicationError::UserInUse);
        }

        self.user_repository
            .create_user(UserAccount {
                user_identifier: user_identifier.clone(),
                password_hash: password.to_owned(),
                display_name: user_identifier.as_str().to_owned(),
            })
            .map_err(|error| AuthorizationApplicationError::Internal(error.to_string()))?;

        if info.inhibit_login.unwrap_or(false) {
            return Ok(RegisterUserView {
                user_identifier: user_identifier.into_inner(),
                access_token: None,
                device_identifier: None,
                home_server_name: None,
            });
        }

        let access_token = AccessToken::parse(
            self.id_generator
                .next_access_token(user_identifier.as_str()),
        )
        .ok_or_else(|| {
            AuthorizationApplicationError::Internal("token generation failed".to_owned())
        })?;

        self.session_repository
            .create_session(AccessSession {
                access_token: access_token.clone(),
                user_identifier: user_identifier.clone(),
            })
            .map_err(|error| AuthorizationApplicationError::Internal(error.to_string()))?;

        Ok(RegisterUserView {
            user_identifier: user_identifier.into_inner(),
            access_token: Some(access_token.into_inner()),
            device_identifier: Some(
                info.device_identifier
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "DEVICEGRIDSTACK".to_owned()),
            ),
            home_server_name: Some(self.home_server_name.clone()),
        })
    }

    pub fn login_user(
        &self,
        request: LoginUserInfo,
    ) -> Result<LoginUserView, AuthorizationApplicationError> {
        let requested_user = request
            .identifier
            .map(|v| v.user)
            .or(request.user)
            .ok_or(AuthorizationApplicationError::InvalidUsername)?;

        let user_identifier = self.try_get_user_id(&requested_user)?;
        let user_account = self
            .user_repository
            .find_user_by_identifier(&user_identifier)
            .ok_or(AuthorizationApplicationError::InvalidCredentials)?;

        if user_account.password_hash != request.password {
            return Err(AuthorizationApplicationError::InvalidCredentials);
        }

        let access_token = AccessToken::parse(
            self.id_generator
                .next_access_token(user_identifier.as_str()),
        )
        .ok_or_else(|| {
            AuthorizationApplicationError::Internal("token generation failed".to_owned())
        })?;

        self.session_repository
            .create_session(AccessSession {
                access_token: access_token.clone(),
                user_identifier: user_identifier.clone(),
            })
            .map_err(|error| AuthorizationApplicationError::Internal(error.to_string()))?;

        Ok(LoginUserView {
            access_token: access_token.into_inner(),
            device_id: request
                .device_identifier
                .unwrap_or_else(|| "DEVICEGRIDSTACK".to_owned()),
            expires_in_ms: 0,
            refresh_token: String::new(),
            user_id: user_identifier.into_inner(),
        })
    }

    pub fn who_am_i_from_user_id(
        &self,
        user_identifier: UserId,
    ) -> Result<WhoAmIView, AuthorizationApplicationError> {
        if self
            .user_repository
            .find_user_by_identifier(&user_identifier)
            .is_none()
        {
            return Err(AuthorizationApplicationError::Unauthorized);
        }

        Ok(WhoAmIView {
            user_id: user_identifier.into_inner(),
            is_guest: false,
            device_id: None,
        })
    }

    pub fn authenticate_access_token(
        &self,
        access_token: &str,
    ) -> Result<UserId, AuthorizationApplicationError> {
        let access_token = AccessToken::parse(access_token.to_owned())
            .ok_or(AuthorizationApplicationError::Unauthorized)?;
        let session = self
            .session_repository
            .find_session_by_access_token(&access_token)
            .ok_or(AuthorizationApplicationError::Unauthorized)?;

        Ok(session.user_identifier)
    }

    fn try_get_user_id(
        &self,
        username_or_identifier: &str,
    ) -> Result<UserId, AuthorizationApplicationError> {
        let candidate = username_or_identifier.trim();
        if candidate.is_empty() {
            return Err(AuthorizationApplicationError::InvalidUsername);
        }
        if candidate.starts_with('@') && candidate.contains(":") {
            return UserId::parse(candidate.to_owned())
                .ok_or(AuthorizationApplicationError::InvalidUsername);
        }

        UserId::parse(format!("@{}:{}", candidate, self.home_server_name))
            .ok_or(AuthorizationApplicationError::InvalidUsername)
    }
}
