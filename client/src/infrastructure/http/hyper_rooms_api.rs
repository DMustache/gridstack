use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, Request, header};
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::TokioExecutor;

use crate::{
    application::ports::RoomsApi,
    domain::{
        error::{ClientError, MatrixErrorResponse},
        rooms::{CreateRoomInfo, CreateRoomView},
    },
};

pub struct HyperRoomsApi {
    base_url: String,
    http_client: Client<HttpConnector, Full<Bytes>>,
}

impl HyperRoomsApi {
    pub fn new(base_url: impl Into<String>) -> Self {
        let connector = HttpConnector::new();
        let http_client = Client::builder(TokioExecutor::new()).build(connector);
        Self {
            base_url: base_url.into(),
            http_client,
        }
    }
}

#[async_trait]
impl RoomsApi for HyperRoomsApi {
    async fn create_room(
        &self,
        access_token: &str,
        request: &CreateRoomInfo,
    ) -> Result<CreateRoomView, ClientError> {
        let uri = format!("{}/_matrix/client/v3/createRoom", self.base_url);
        let body_bytes = serde_json::to_vec(request)
            .map_err(|error| ClientError::Serialization(error.to_string()))?;
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
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
            return serde_json::from_slice::<CreateRoomView>(&response_bytes)
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
