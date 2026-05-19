use std::sync::Arc;

use crate::{
    application::ports::{AuthorizationApi, RegisterUserResult, SessionRepository},
    domain::{
        authorization::{
            AccountKind, CheckUsernameAvailableView, GetAuthMetadataView, GetLoginFlowsView,
            LoginUserInfo, LoginUserView, LogoutUserView, RegisterUserInfo, SessionRecord,
            WhoAmIView,
        },
        error::ClientError,
    },
};

pub struct AuthorizationClientService {
    server_url: String,
    authorization_api: Arc<dyn AuthorizationApi>,
    session_repository: Arc<dyn SessionRepository>,
}

impl AuthorizationClientService {
    pub fn new(
        server_url: impl Into<String>,
        authorization_api: Arc<dyn AuthorizationApi>,
        session_repository: Arc<dyn SessionRepository>,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            authorization_api,
            session_repository,
        }
    }

    pub async fn get_auth_metadata(&self) -> Result<GetAuthMetadataView, ClientError> {
        self.authorization_api.get_auth_metadata().await
    }

    pub async fn get_login_flows(&self) -> Result<GetLoginFlowsView, ClientError> {
        self.authorization_api.get_login_flows().await
    }

    pub async fn check_username_available(
        &self,
        username: &str,
    ) -> Result<CheckUsernameAvailableView, ClientError> {
        self.authorization_api
            .check_username_available(username)
            .await
    }

    pub async fn register_user(
        &self,
        kind: AccountKind,
        request: &RegisterUserInfo,
    ) -> Result<RegisterUserResult, ClientError> {
        let response = self.authorization_api.register_user(kind, request).await?;

        if let RegisterUserResult::Registered(registered) = &response
            && let Some(access_token) = &registered.access_token
        {
            let session = SessionRecord {
                server_url: self.server_url.clone(),
                access_token: access_token.clone(),
                user_id: registered.user_identifier.clone(),
                device_id: registered.device_identifier.clone(),
                refresh_token: registered.refresh_token.clone(),
                expires_in_milliseconds: registered.expires_in_milliseconds,
            };
            self.session_repository.upsert_session(&session).await?;
        }

        Ok(response)
    }

    pub async fn login_user(&self, request: &LoginUserInfo) -> Result<LoginUserView, ClientError> {
        let response = self.authorization_api.login_user(request).await?;
        let session = SessionRecord {
            server_url: self.server_url.clone(),
            access_token: response.access_token.clone(),
            user_id: response.user_id.clone(),
            device_id: Some(response.device_id.clone()),
            refresh_token: response.refresh_token.clone(),
            expires_in_milliseconds: response.expires_in_milliseconds,
        };
        self.session_repository.upsert_session(&session).await?;
        Ok(response)
    }

    pub async fn who_am_i_from_saved_session(&self) -> Result<WhoAmIView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        self.authorization_api.who_am_i(&session.access_token).await
    }

    pub async fn logout_from_saved_session(&self) -> Result<LogoutUserView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        let response = self.authorization_api.logout_user(&session.access_token).await?;
        self.session_repository
            .clear_session_by_server(&self.server_url)
            .await?;
        Ok(response)
    }
}
