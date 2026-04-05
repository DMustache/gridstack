use ruma::api::client::error::ErrorKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[error("RequestInvalidInfo: {error_code}, {error}")]
pub struct RequestInvalidInfo {
    #[serde(rename = "errcode")]
    pub error_code: ErrorKind,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}
