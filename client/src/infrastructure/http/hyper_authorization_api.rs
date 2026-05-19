use async_trait::async_trait;
use http::{Method, StatusCode};

use crate::{
    application::ports::{AuthorizationApi, RegisterUserResult},
    domain::{
        authorization::{
            AccountKind, CheckUsernameAvailableView, GetAuthMetadataView, GetLoginFlowsView,
            LoginUserInfo, LoginUserView, LogoutUserView, RegisterUserInfo, RegisterUserView,
            UiaaResponseView, WhoAmIView,
        },
        error::{ClientError, MatrixErrorResponse},
    },
    infrastructure::http::matrix_http_client::MatrixHttpClient,
};

pub struct HyperAuthorizationApi {
    matrix_http_client: MatrixHttpClient,
}

impl HyperAuthorizationApi {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            matrix_http_client: MatrixHttpClient::new(base_url),
        }
    }
}

#[async_trait]
impl AuthorizationApi for HyperAuthorizationApi {
    async fn get_auth_metadata(&self) -> Result<GetAuthMetadataView, ClientError> {
        self.matrix_http_client
            .send_json::<(), GetAuthMetadataView>(
            Method::GET,
            "/_matrix/client/v1/auth_metadata",
            None,
            None,
        )
        .await
    }

    async fn get_login_flows(&self) -> Result<GetLoginFlowsView, ClientError> {
        self.matrix_http_client
            .send_json::<(), GetLoginFlowsView>(
            Method::GET,
            "/_matrix/client/v3/login",
            None,
            None,
        )
        .await
    }

    async fn check_username_available(
        &self,
        username: &str,
    ) -> Result<CheckUsernameAvailableView, ClientError> {
        let path = format!("/_matrix/client/v3/register/available?username={username}");
        self.matrix_http_client
            .send_json::<(), CheckUsernameAvailableView>(Method::GET, &path, None, None)
            .await
    }

    async fn register_user(
        &self,
        kind: AccountKind,
        request: &RegisterUserInfo,
    ) -> Result<RegisterUserResult, ClientError> {
        let kind_value = match kind {
            AccountKind::User => "user",
            AccountKind::Guest => "guest",
        };
        let path = format!("/_matrix/client/v3/register?kind={kind_value}");

        let (status, response_bytes) = self
            .matrix_http_client
            .send_json_with_status(Method::POST, &path, None, Some(request))
            .await?;

        if status == StatusCode::UNAUTHORIZED {
            let uiaa = serde_json::from_slice::<UiaaResponseView>(&response_bytes)
                .map_err(|error| ClientError::Serialization(error.to_string()))?;
            return Ok(RegisterUserResult::AuthenticationRequired(uiaa));
        }

        if status.is_success() {
            let registered = serde_json::from_slice::<RegisterUserView>(&response_bytes)
                .map_err(|error| ClientError::Serialization(error.to_string()))?;
            return Ok(RegisterUserResult::Registered(registered));
        }

        if let Ok(matrix_error) = serde_json::from_slice::<MatrixErrorResponse>(&response_bytes) {
            return Err(ClientError::Matrix {
                status: status.as_u16(),
                errcode: matrix_error.errcode,
                message: matrix_error.error,
            });
        }

        Err(ClientError::UnexpectedStatus {
            status: status.as_u16(),
            body: String::from_utf8_lossy(&response_bytes).to_string(),
        })
    }

    async fn login_user(&self, request: &LoginUserInfo) -> Result<LoginUserView, ClientError> {
        self.matrix_http_client
            .send_json(Method::POST, "/_matrix/client/v3/login", None, Some(request))
            .await
    }

    async fn who_am_i(&self, access_token: &str) -> Result<WhoAmIView, ClientError> {
        self.matrix_http_client
            .send_json::<(), WhoAmIView>(
            Method::GET,
            "/_matrix/client/v3/account/whoami",
            Some(access_token),
            None,
        )
        .await
    }

    async fn logout_user(&self, access_token: &str) -> Result<LogoutUserView, ClientError> {
        self.matrix_http_client
            .send_json::<(), LogoutUserView>(
            Method::POST,
            "/_matrix/client/v3/logout",
            Some(access_token),
            None,
        )
        .await
    }
}
