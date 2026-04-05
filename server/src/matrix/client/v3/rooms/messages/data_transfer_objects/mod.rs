use ruma::exports::serde_json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct QueryParameters {
    pub dir: String,
    pub filter: Option<String>,
    pub from: Option<String>,
    pub limit: Option<i64>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BodyView {
    pub chunk: Vec<Value>,
    pub start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<Value>,
}
