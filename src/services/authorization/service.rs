use serde_json::json;
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tracing::error;
use uuid::Uuid;

use crate::{
    infrastructure::{
        configuration::AuthenticationConfiguration, device_id::DeviceId,
        password_hash::PasswordHash, server_name::ServerName, user_identifier::UserIdentifier,
        username::Username,
    },
    services::{
        authorization::{
            entities::{AccessSession, AccountKind, ExpirationClock, UiaaFlowType},
            persistence::access_session_storage_unit::AccessSessionStorageUnit,
        },
        repositories::{SessionRepository, UserRepository},
        traits::{Clock, JsonWebTokenAdapter},
    },
};

use super::{
    entities::{AccessToken, ExistingUserIdentifier, LoginType, UserAccount},
    errors::AuthorizationApplicationError,
    handlers::{
        check_username_available::CheckUsernameAvailableView,
        get_auth_metadata::GetAuthMetadataView,
        get_login_flows::{GetLoginFlowsView, LoginFlow},
        login_user::{LoginIdentifierInfo, LoginUserInfo, LoginUserView},
        logout_user::LogoutUserView,
        register_user::{AuthenticationFlowView, UiaaResponseView},
        register_user::{RegisterUserInfo, RegisterUserQueryInfo, RegisterUserView},
        who_am_i::WhoAmIView,
    },
};

pub struct AuthorizationService {
    user_repository: Arc<dyn UserRepository>,
    session_repository: Arc<dyn SessionRepository>,
    clock: Arc<dyn Clock>,
    json_web_token_adapter: Arc<dyn JsonWebTokenAdapter>,

    server_name: Arc<ServerName>,
    configuration: AuthenticationConfiguration,
    registration_sessions: RwLock<HashSet<String>>,
}

impl AuthorizationService {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        session_repository: Arc<dyn SessionRepository>,
        json_web_token_adapter: Arc<dyn JsonWebTokenAdapter>,
        clock: Arc<dyn Clock>,
        server_name: &ServerName,
        configuration: AuthenticationConfiguration,
    ) -> Self {
        Self {
            user_repository,
            session_repository,
            clock,
            json_web_token_adapter,

            server_name: Arc::new(server_name.clone()),

            configuration,
            registration_sessions: RwLock::new(HashSet::new()),
        }
    }

    pub const fn get_auth_metadata(
        &self,
    ) -> Result<GetAuthMetadataView, AuthorizationApplicationError> {
        Err(AuthorizationApplicationError::OAuthAuthorizationUnsupported)
    }

    pub fn get_login_flows(&self) -> Result<GetLoginFlowsView, AuthorizationApplicationError> {
        Ok(GetLoginFlowsView {
            flows: Self::registration_uiaa_flow_types()
                .iter()
                .map(|flow_type| LoginFlow {
                    type_field: flow_type.to_string(),
                    get_login_token: None,
                })
                .collect(),
        })
    }

    pub fn registration_uiaa_challenge(&self) -> UiaaResponseView {
        let session = Uuid::new_v4().to_string();
        if let Ok(mut sessions) = self.registration_sessions.write() {
            sessions.insert(session.clone());
        }

        UiaaResponseView {
            completed: Vec::new(),
            flows: vec![AuthenticationFlowView {
                stages: Self::registration_uiaa_flow_types()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            }],
            params: json!({}),
            session,
        }
    }

    #[allow(clippy::needless_pass_by_value, reason = "handler boundary ownership")]
    pub fn check_username_available(
        &self,
        username: String,
    ) -> Result<CheckUsernameAvailableView, AuthorizationApplicationError> {
        let user_identifier = self.try_parse_user_id(&username)?;
        Ok(CheckUsernameAvailableView {
            available: !self.user_repository.user_exists(&user_identifier),
        })
    }

    pub fn register_user(
        &self,
        query: &RegisterUserQueryInfo,
        info: &RegisterUserInfo,
    ) -> Result<RegisterUserView, AuthorizationApplicationError> {
        if !self.configuration.allow_registration {
            return Err(AuthorizationApplicationError::RegistrationDisabled);
        }

        let is_guest = query.kind.eq(&AccountKind::Guest);
        if is_guest {
            return Err(AuthorizationApplicationError::GuestRegistrationDisabled);
        }

        let authentication = info
            .authentication
            .as_ref()
            .ok_or(AuthorizationApplicationError::Unauthorized)?;

        let session_identifier = authentication
            .session
            .as_deref()
            .ok_or(AuthorizationApplicationError::Unauthorized)?;
        if !self.is_known_registration_session(session_identifier) {
            return Err(AuthorizationApplicationError::Unauthorized);
        }

        let authentication_type = UiaaFlowType::from_str(
            authentication
                .authentication_type
                .as_deref()
                .unwrap_or_default(),
        )
        .map_err(|_| AuthorizationApplicationError::Unauthorized)?;
        if !Self::is_supported_registration_uiaa_flow(&authentication_type) {
            return Err(AuthorizationApplicationError::Unauthorized);
        }

        let username = info
            .username
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map_or_else(|| Self::new_generated_username("user"), str::to_owned);

        let password = info
            .password
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or(AuthorizationApplicationError::InvalidCredentials)?;

        let user_identifier = self.try_parse_user_id(&username)?;
        if self.user_repository.user_exists(&user_identifier) {
            return Err(AuthorizationApplicationError::UserInUse);
        }

        let encoded_password = PasswordHash::encode(password, &self.configuration.password_pepper)
            .map_err(|encoding_error| {
                error!(error = %encoding_error, "failed to encode password");
                AuthorizationApplicationError::Internal
            })?;

        self.user_repository
            .create_user(UserAccount {
                user_identifier: user_identifier.clone(),
                password_hash: encoded_password,
                display_name: user_identifier.to_string(),
                is_guest,
            })
            .map_err(|repository_error| {
                error!(error = %repository_error, "failed to create user");
                AuthorizationApplicationError::Internal
            })?;

        self.remove_registration_session(session_identifier);

        let refresh_token = info
            .refresh_token
            .then(|| self.generate_refresh_token())
            .transpose()?;

        if info.inhibit_login {
            return Ok(RegisterUserView {
                user_identifier: user_identifier.into_inner(),
                access_token: None,
                device_identifier: None,
                home_server_name: None,
                expires_in_milliseconds: None,
                refresh_token: None,
            });
        }

        let (raw_access_token, access_token_expiry_seconds) = self.generate_access_token()?;
        let access_token =
            AccessToken::parse(raw_access_token).ok_or(AuthorizationApplicationError::Internal)?;

        let device_id = info
            .device_identifier
            .as_deref()
            .and_then(DeviceId::parse)
            .unwrap_or_default();

        self.session_repository
            .create_session(AccessSessionStorageUnit::from(AccessSession::new(
                access_token.clone(),
                super::entities::AuthorizedUserIdentifier::new(ExistingUserIdentifier::new(
                    user_identifier.clone(),
                )),
                device_id.clone(),
                access_token_expiry_seconds,
            )))
            .map_err(|repository_error| {
                error!(
                    error = %repository_error,
                    "failed to create registration session"
                );
                AuthorizationApplicationError::Internal
            })?;

        Ok(RegisterUserView {
            user_identifier: user_identifier.into_inner(),
            access_token: Some(access_token.into_inner()),
            device_identifier: Some(device_id.into_inner()),
            home_server_name: Some(self.server_name.as_str().to_string()),
            expires_in_milliseconds: Some(
                self.configuration
                    .access_token_expiry_as_milliseconds()
                    .saturating_mul(1000),
            ),
            refresh_token,
        })
    }

    pub fn login_user(
        &self,
        request: LoginUserInfo,
    ) -> Result<LoginUserView, AuthorizationApplicationError> {
        if request.login_type != LoginType::Password {
            return Err(AuthorizationApplicationError::Unrecognized);
        }

        let requested_user = request
            .identifier
            .map(|login_identifier_info| match login_identifier_info {
                LoginIdentifierInfo::MatrixUser { user } => Ok(user),
                LoginIdentifierInfo::ThirdParty { .. }
                | LoginIdentifierInfo::PhoneNumber { .. } => {
                    Err(AuthorizationApplicationError::InvalidCredentials)
                }
                LoginIdentifierInfo::Unsupported => {
                    Err(AuthorizationApplicationError::Unrecognized)
                }
            })
            .transpose()?
            .or(request.user)
            .ok_or(AuthorizationApplicationError::InvalidUsername)?;

        let user_identifier = self.try_parse_user_id(&requested_user)?;
        let user_account = self
            .user_repository
            .find_user_by_identifier(&user_identifier)
            .ok_or(AuthorizationApplicationError::InvalidCredentials)?;

        if !PasswordHash::verify(
            &request.password,
            &self.configuration.password_pepper,
            &user_account.password_hash,
        ) {
            return Err(AuthorizationApplicationError::InvalidCredentials);
        }

        let (raw_access_token, access_token_expiry_seconds) = self.generate_access_token()?;
        let access_token =
            AccessToken::parse(raw_access_token).ok_or(AuthorizationApplicationError::Internal)?;

        let device_id = request
            .device_identifier
            .as_deref()
            .and_then(DeviceId::parse)
            .unwrap_or_default();

        self.session_repository
            .create_session(AccessSessionStorageUnit::from(AccessSession::new(
                access_token.clone(),
                super::entities::AuthorizedUserIdentifier::new(ExistingUserIdentifier::new(
                    user_identifier.clone(),
                )),
                device_id.clone(),
                access_token_expiry_seconds,
            )))
            .map_err(|repository_error| {
                error!(error = %repository_error, "failed to create login session");
                AuthorizationApplicationError::Internal
            })?;

        let refresh_token = request
            .refresh_token
            .then(|| self.generate_refresh_token())
            .transpose()?;

        Ok(LoginUserView {
            access_token: access_token.into_inner(),
            device_id: device_id.into_inner(),
            expires_in_milliseconds: Some(self.configuration.access_token_expiry_as_milliseconds()),
            home_server: Some(self.server_name.as_str().to_string()),
            refresh_token,
            user_id: user_identifier.into_inner(),
        })
    }

    pub fn who_am_i_from_session(
        &self,
        access_session: &AccessSessionStorageUnit,
    ) -> Result<WhoAmIView, AuthorizationApplicationError> {
        let user_account = self
            .user_repository
            .find_user_by_identifier(access_session.user_identifier().as_user_identifier())
            .ok_or(AuthorizationApplicationError::Unauthorized)?;

        Ok(WhoAmIView {
            user_id: access_session
                .user_identifier()
                .clone()
                .into_inner()
                .into_inner()
                .into_inner(),
            is_guest: user_account.is_guest,
            device_id: Some(access_session.device_id().clone().into_inner()),
        })
    }

    pub fn logout_user(
        &self,
        access_token: &AccessToken,
    ) -> Result<LogoutUserView, AuthorizationApplicationError> {
        self.session_repository
            .delete_session_by_access_token(access_token)
            .map_err(|repository_error| {
                error!(error = %repository_error, "failed to delete session");
                AuthorizationApplicationError::Internal
            })?;

        Ok(LogoutUserView {})
    }

    pub fn authenticate_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Result<AccessSessionStorageUnit, AuthorizationApplicationError> {
        if !self.json_web_token_adapter.is_token_valid(
            access_token.as_str(),
            &self.configuration.json_web_token_secret,
        ) {
            return Err(AuthorizationApplicationError::Unauthorized);
        }

        let session = self
            .session_repository
            .find_session_by_access_token(access_token)
            .ok_or(AuthorizationApplicationError::Unauthorized)?;

        Ok(session)
    }

    pub fn require_existing_user_identifier(
        &self,
        user_identifier: UserIdentifier,
    ) -> Result<ExistingUserIdentifier, AuthorizationApplicationError> {
        if !self.user_repository.user_exists(&user_identifier) {
            return Err(AuthorizationApplicationError::Unauthorized);
        }

        Ok(ExistingUserIdentifier::new(user_identifier))
    }

    pub fn require_authorized_user(
        &self,
        access_session: &AccessSessionStorageUnit,
        requested_user_identifier: &UserIdentifier,
    ) -> Result<super::entities::AuthorizedUserIdentifier, AuthorizationApplicationError> {
        if access_session.user_identifier().as_user_identifier() != requested_user_identifier {
            return Err(AuthorizationApplicationError::Forbidden);
        }

        Ok(access_session.user_identifier().clone())
    }

    fn try_parse_user_id(
        &self,
        username_or_identifier: &str,
    ) -> Result<UserIdentifier, AuthorizationApplicationError> {
        let candidate = username_or_identifier.trim();
        if candidate.is_empty() {
            return Err(AuthorizationApplicationError::InvalidUsername);
        }
        if candidate.starts_with('@') && candidate.contains(':') {
            return UserIdentifier::try_from(candidate.to_owned())
                .map_err(|_| AuthorizationApplicationError::InvalidUsername);
        }

        let username =
            Username::try_new(candidate).ok_or(AuthorizationApplicationError::InvalidUsername)?;
        UserIdentifier::from_localpart_and_server(username.as_str(), self.server_name.as_str())
            .ok_or(AuthorizationApplicationError::InvalidUsername)
    }

    const fn registration_uiaa_flow_types() -> &'static [UiaaFlowType] {
        &[UiaaFlowType::Password]
    }

    fn is_supported_registration_uiaa_flow(flow_type: &UiaaFlowType) -> bool {
        Self::registration_uiaa_flow_types().contains(flow_type)
    }

    fn new_generated_username(prefix: &str) -> String {
        let suffix = Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>();
        format!("{prefix}_{suffix}")
    }

    fn generate_access_token(&self) -> Result<(String, i64), AuthorizationApplicationError> {
        let expires_at_seconds =
            self.expiry_from_now(self.configuration.access_token_expiry_seconds)?;
        let expiry_clock = ExpirationClock::new(expires_at_seconds);
        let token = self
            .json_web_token_adapter
            .encode(&expiry_clock, &self.configuration.json_web_token_secret)
            .map_err(|token_error| {
                error!(error = %token_error, "failed to generate access token");
                AuthorizationApplicationError::Internal
            })?;
        Ok((token, expires_at_seconds))
    }

    fn generate_refresh_token(&self) -> Result<String, AuthorizationApplicationError> {
        let expires_at_seconds =
            self.expiry_from_now(self.configuration.refresh_token_expiry_seconds)?;
        let expiry_clock = ExpirationClock::new(expires_at_seconds);
        self.json_web_token_adapter
            .encode(&expiry_clock, &self.configuration.json_web_token_secret)
            .map_err(|token_error| {
                error!(error = %token_error, "failed to generate refresh token");
                AuthorizationApplicationError::Internal
            })
    }

    fn expiry_from_now(
        &self,
        valid_for_seconds: u64,
    ) -> Result<i64, AuthorizationApplicationError> {
        Ok((self.clock.as_seconds()).saturating_add(
            i64::try_from(valid_for_seconds)
                .map_err(|_| AuthorizationApplicationError::Internal)?,
        ))
    }

    fn is_known_registration_session(&self, session_identifier: &str) -> bool {
        self.registration_sessions
            .read()
            .map(|sessions| sessions.contains(session_identifier))
            .unwrap_or(false)
    }

    fn remove_registration_session(&self, session_identifier: &str) {
        if let Ok(mut sessions) = self.registration_sessions.write() {
            sessions.remove(session_identifier);
        }
    }
}
