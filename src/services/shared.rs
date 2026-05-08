#[derive(serde::Serialize)]
pub struct MatrixErrorResponse {
    pub errcode: String,
    pub error: String,
}

#[derive(serde::Serialize)]
pub struct MatrixRateLimitErrorResponse {
    #[serde(flatten)]
    pub base: MatrixErrorResponse,
    #[serde(rename = "retry_after_ms")]
    pub retry_after_ms: u64,
}
