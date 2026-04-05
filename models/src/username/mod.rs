use regex::Regex;

use crate::error::MatrixError;

#[allow(dead_code)]
pub struct UserId {
    localpart: Localpart,
    server_name: String,
}

#[derive(Debug, Clone)]
pub struct Localpart(String);

impl Localpart {
    pub fn try_new(localpart: String) -> Result<Self, MatrixError> {
        Self::validate(&localpart)?;
        Ok(Self(localpart))
    }

    fn is_valid(localpart: &str) -> bool {
        let regex = Regex::new(r"(?mu)^[a-z0-9\-./=_]+$").unwrap();
        regex.is_match(localpart)
    }

    fn validate(localpart: &str) -> Result<(), MatrixError> {
        if Self::is_valid(localpart) {
            return Ok(());
        }
        Err(MatrixError::invalid_username("username invalid"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
