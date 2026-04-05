use ruma::IdParseError;
use thiserror::Error;

mod database_error;
pub use database_error::DatabaseError;

#[derive(Debug, Error)]
pub enum InternalError {
    #[error(transparent)]
    Database(#[from] DatabaseError),

    #[error("Invalid user ID")]
    InvalidUserId(#[from] IdParseError),
    #[error("Database error")]
    DatabaseError(DatabaseError),
}
