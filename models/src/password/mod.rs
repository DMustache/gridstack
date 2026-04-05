use std::env;

use argon2::{
    Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use ruma::api::client::error::ErrorKind;

use crate::error::MatrixError;

const PASSWORD_PEPPER: &str = "PASSWORD_PEPPER";
const MIN_PASSWORD_LENGTH: usize = 8;

#[derive(Debug, Clone)]
pub struct Password(String);

impl Password {
    pub fn try_new(password: String) -> Result<Self, MatrixError> {
        Self::validate(&password)?;
        Ok(Self(password))
    }

    fn is_valid(password: &str) -> bool {
        password.len() >= MIN_PASSWORD_LENGTH
    }

    fn validate(password: &str) -> Result<(), MatrixError> {
        if Self::is_valid(password) {
            return Ok(());
        }
        Err(MatrixError::weak_password(format!(
            "Password must be at least {MIN_PASSWORD_LENGTH} characters long"
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_hash(&self) -> Result<String, MatrixError> {
        let salt = SaltString::generate(&mut OsRng);
        let pepper = env::var(PASSWORD_PEPPER)
            .map_err(|_| MatrixError::internal_error(ErrorKind::Unknown))?;
        let argon2 = Argon2::new_with_secret(
            pepper.as_bytes(),
            Algorithm::Argon2id,
            Version::V0x13,
            Params::default(),
        )
        .map_err(|_| MatrixError::internal_error(ErrorKind::Unknown))?;

        Ok(argon2
            .hash_password(self.0.as_bytes(), &salt)
            .map_err(|_| MatrixError::internal_error(ErrorKind::Unknown))?
            .to_string())
    }

    pub fn verify(&self, password_hash: &str) -> Result<bool, MatrixError> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|_| MatrixError::internal_error(ErrorKind::Unknown))?;
        let pepper = env::var(PASSWORD_PEPPER)
            .map_err(|_| MatrixError::internal_error(ErrorKind::Unknown))?;
        let argon2 = Argon2::new_with_secret(
            pepper.as_bytes(),
            Algorithm::Argon2id,
            Version::V0x13,
            Params::default(),
        )
        .map_err(|_| MatrixError::internal_error(ErrorKind::Unknown))?;

        match argon2.verify_password(self.0.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(_) => Err(MatrixError::internal_error(ErrorKind::Unknown)),
        }
    }

    pub fn verify_raw(password: &str, password_hash: &str) -> Result<bool, MatrixError> {
        Self(password.to_owned()).verify(password_hash)
    }
}
