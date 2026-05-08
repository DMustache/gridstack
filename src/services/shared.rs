#[derive(serde::Serialize)]
pub struct MatrixErrorResponse {
    pub errcode: String,
    pub error: String,
}

#[derive(serde::Serialize)]
pub struct MatrixRateLimitErrorResponse {
    #[serde(flatten)]
    pub base: MatrixErrorResponse,
    pub retry_after: u64,
}
