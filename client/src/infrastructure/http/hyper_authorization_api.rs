use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, Request, StatusCode, header};
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::TokioExecutor;

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
};

pub struct HyperAuthorizationApi {
    base_url: String,
    http_client: Client<HttpConnector, Full<Bytes>>,
}

impl HyperAuthorizationApi {
    pub fn new(base_url: impl Into<String>) -> Self {
        let connector = HttpConnector::new();
        let http_client = Client::builder(TokioExecutor::new()).build(connector);
        Self {
            base_url: base_url.into(),
            http_client,
        }
    }

    async fn send_json<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        bearer_token: Option<&str>,
        body: Option<&Req>,
    ) -> Result<Res, ClientError> {
        let uri = format!("{}{}", self.base_url, path);

        let body_bytes = if let Some(body) = body {
            serde_json::to_vec(body).map_err(|error| ClientError::Serialization(error.to_string()))?
        } else {
            Vec::new()
        };

        let mut request_builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::ACCEPT, "application/json");

        if body_bytes.is_empty() {
            request_builder = request_builder.header(header::CONTENT_LENGTH, "0");
        } else {
            request_builder = request_builder.header(header::CONTENT_TYPE, "application/json");
        }

        if let Some(token) = bearer_token {
            request_builder = request_builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }

        let request = request_builder
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let response = self
            .http_client
            .request(request)
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let status = response.status();
        let response_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?
            .to_bytes();

        if status.is_success() {
            if response_bytes.is_empty() {
                return serde_json::from_str("{}")
                    .map_err(|error| ClientError::Serialization(error.to_string()));
            }

            return serde_json::from_slice(&response_bytes)
                .map_err(|error| ClientError::Serialization(error.to_string()));
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
}

#[async_trait]
impl AuthorizationApi for HyperAuthorizationApi {
    async fn get_auth_metadata(&self) -> Result<GetAuthMetadataView, ClientError> {
        self.send_json::<(), GetAuthMetadataView>(
            Method::GET,
            "/_matrix/client/v1/auth_metadata",
            None,
            None,
        )
        .await
    }

    async fn get_login_flows(&self) -> Result<GetLoginFlowsView, ClientError> {
        self.send_json::<(), GetLoginFlowsView>(
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
        self.send_json::<(), CheckUsernameAvailableView>(Method::GET, &path, None, None)
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

        let uri = format!("{}{}", self.base_url, path);
        let body_bytes = serde_json::to_vec(request)
            .map_err(|error| ClientError::Serialization(error.to_string()))?;
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let response = self
            .http_client
            .request(request)
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        let status = response.status();
        let response_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?
            .to_bytes();

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
        self.send_json(Method::POST, "/_matrix/client/v3/login", None, Some(request))
            .await
    }

    async fn who_am_i(&self, access_token: &str) -> Result<WhoAmIView, ClientError> {
        self.send_json::<(), WhoAmIView>(
            Method::GET,
            "/_matrix/client/v3/account/whoami",
            Some(access_token),
            None,
        )
        .await
    }

    async fn logout_user(&self, access_token: &str) -> Result<LogoutUserView, ClientError> {
        self.send_json::<(), LogoutUserView>(
            Method::POST,
            "/_matrix/client/v3/logout",
            Some(access_token),
            None,
        )
        .await
    }
}
