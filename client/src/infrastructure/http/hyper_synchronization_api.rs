use async_trait::async_trait;
use http::Method;

use crate::{
    application::ports::SynchronizationApi,
    domain::{
        error::ClientError,
        synchronization::{DefineFilterInfo, DefineFilterView, SyncQuery, SyncResponseView},
    },
    infrastructure::http::matrix_http_client::MatrixHttpClient,
};

pub struct HyperSynchronizationApi {
    matrix_http_client: MatrixHttpClient,
}

impl HyperSynchronizationApi {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            matrix_http_client: MatrixHttpClient::new(base_url),
        }
    }
}

#[async_trait]
impl SynchronizationApi for HyperSynchronizationApi {
    async fn define_filter(
        &self,
        access_token: &str,
        user_id: &str,
        request: &DefineFilterInfo,
    ) -> Result<DefineFilterView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/user/{}/filter",
            percent_encode_path_segment(user_id)
        );
        self.matrix_http_client
            .send_json(Method::POST, &path, Some(access_token), Some(request))
            .await
    }

    async fn get_filter(
        &self,
        access_token: &str,
        user_id: &str,
        filter_id: &str,
    ) -> Result<serde_json::Value, ClientError> {
        let path = format!(
            "/_matrix/client/v3/user/{}/filter/{}",
            percent_encode_path_segment(user_id),
            percent_encode_path_segment(filter_id)
        );
        self.matrix_http_client
            .send_json::<(), serde_json::Value>(Method::GET, &path, Some(access_token), None)
            .await
    }

    async fn sync(
        &self,
        access_token: &str,
        query: &SyncQuery,
    ) -> Result<SyncResponseView, ClientError> {
        let mut query_fields: Vec<String> = Vec::new();
        if let Some(filter) = query.filter.as_deref() {
            query_fields.push(format!("filter={}", percent_encode_query_value(filter)));
        }
        if let Some(since) = query.since.as_deref() {
            query_fields.push(format!("since={}", percent_encode_query_value(since)));
        }
        if query.full_state {
            query_fields.push("full_state=true".to_owned());
        }
        if let Some(set_presence) = query.set_presence.as_deref() {
            query_fields.push(format!(
                "set_presence={}",
                percent_encode_query_value(set_presence)
            ));
        }
        if let Some(timeout_milliseconds) = query.timeout_milliseconds {
            query_fields.push(format!("timeout={timeout_milliseconds}"));
        }
        if query.use_state_after {
            query_fields.push("use_state_after=true".to_owned());
        }
        let path = if query_fields.is_empty() {
            "/_matrix/client/v3/sync".to_owned()
        } else {
            format!("/_matrix/client/v3/sync?{}", query_fields.join("&"))
        };

        self.matrix_http_client
            .send_json_with_query::<(), SyncResponseView>(Method::GET, &path, Some(access_token), None)
            .await
    }
}

fn percent_encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}
