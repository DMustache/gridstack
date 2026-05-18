use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncronizationApplicationError {
    #[error("missing or invalid access token")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("invalid request parameter")]
    InvalidParameter,
    #[error("internal error")]
    Internal,
}
