use bytes::Bytes;
use http::{Method, Request, StatusCode, header};
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::TokioExecutor;

use crate::domain::error::{ClientError, MatrixErrorResponse};

pub struct MatrixHttpClient {
    base_url: String,
    http_client: Client<HttpConnector, Full<Bytes>>,
}

impl MatrixHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let connector = HttpConnector::new();
        let http_client = Client::builder(TokioExecutor::new()).build(connector);
        Self {
            base_url: base_url.into(),
            http_client,
        }
    }

    pub async fn send_json<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        bearer_token: Option<&str>,
        body: Option<&Req>,
    ) -> Result<Res, ClientError> {
        let response = self
            .send_request(method, path, bearer_token, body, None)
            .await?;
        self.decode_json_response(response).await
    }

    pub async fn send_json_with_status<Req: serde::Serialize>(
        &self,
        method: Method,
        path: &str,
        bearer_token: Option<&str>,
        body: Option<&Req>,
    ) -> Result<(StatusCode, Bytes), ClientError> {
        let response = self
            .send_request(method, path, bearer_token, body, None)
            .await?;
        self.read_response_bytes(response).await
    }

    pub async fn send_json_with_query<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path_with_query: &str,
        bearer_token: Option<&str>,
        body: Option<&Req>,
    ) -> Result<Res, ClientError> {
        let response = self
            .send_request(method, path_with_query, bearer_token, body, None)
            .await?;
        self.decode_json_response(response).await
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn send_request<Req: serde::Serialize>(
        &self,
        method: Method,
        path: &str,
        bearer_token: Option<&str>,
        body: Option<&Req>,
        content_type: Option<&str>,
    ) -> Result<hyper::Response<hyper::body::Incoming>, ClientError> {
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
            request_builder =
                request_builder.header(header::CONTENT_TYPE, content_type.unwrap_or("application/json"));
        }

        if let Some(token) = bearer_token {
            request_builder = request_builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }

        let request = request_builder
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        self.http_client
            .request(request)
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }

    async fn read_response_bytes(
        &self,
        response: hyper::Response<hyper::body::Incoming>,
    ) -> Result<(StatusCode, Bytes), ClientError> {
        let status = response.status();
        let response_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?
            .to_bytes();
        Ok((status, response_bytes))
    }

    async fn decode_json_response<Res: serde::de::DeserializeOwned>(
        &self,
        response: hyper::Response<hyper::body::Incoming>,
    ) -> Result<Res, ClientError> {
        let (status, response_bytes) = self.read_response_bytes(response).await?;

        if status.is_success() {
            if response_bytes.is_empty() {
                return serde_json::from_str("{}")
                    .map_err(|error| ClientError::Serialization(error.to_string()));
            }

            return serde_json::from_slice(&response_bytes)
                .map_err(|error| ClientError::Serialization(error.to_string()));
        }

        Self::decode_non_success(status, &response_bytes)
    }

    pub fn decode_non_success<T>(status: StatusCode, response_bytes: &Bytes) -> Result<T, ClientError> {
        if let Ok(matrix_error) = serde_json::from_slice::<MatrixErrorResponse>(response_bytes) {
            return Err(ClientError::Matrix {
                status: status.as_u16(),
                errcode: matrix_error.errcode,
                message: matrix_error.error,
            });
        }

        Err(ClientError::UnexpectedStatus {
            status: status.as_u16(),
            body: String::from_utf8_lossy(response_bytes).to_string(),
        })
    }
}
