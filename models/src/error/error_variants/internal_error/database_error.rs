use deadpool::managed::PoolError;
use deadpool_diesel::{Error as DeadpoolDieselError, InteractError};
use diesel::result::Error as DieselError;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Deadpool Diesel Pool Error: {0}")]
    DeadpoolDieselPoolError(#[from] PoolError<DeadpoolDieselError>),
    #[error("InteractError: {0}")]
    InteractError(#[from] InteractError),
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),
    #[error("Connection error: {0}")]
    ConnectionError(#[from] diesel::ConnectionError),
    #[error("Database manager error: {0}")]
    DatabaseManagerError(#[from] deadpool::managed::BuildError),
    #[error("Migration error: {0}")]
    MigrationError(Box<dyn StdError + Send + Sync>),
    #[error("Diesel error: {0}")]
    DieselError(#[from] DieselError),
}
