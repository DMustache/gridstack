use thiserror::Error;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct MatrixErrorResponse {
    pub errcode: String,
    pub error: String,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("http transport failure: {0}")]
    Transport(String),
    #[error("serialization failure: {0}")]
    Serialization(String),
    #[error("unexpected http status: {status}")]
    UnexpectedStatus { status: u16, body: String },
    #[error("matrix error {errcode}: {message}")]
    Matrix {
        status: u16,
        errcode: String,
        message: String,
    },
    #[error("database failure: {0}")]
    Database(String),
}
