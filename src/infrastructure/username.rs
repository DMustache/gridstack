use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref USERNAME_REGULAR_EXPRESSION: Regex = Regex::new(r"^[a-z0-9\-._=/]+$").unwrap();
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    pub fn try_new(username: impl Into<String>) -> Option<Self> {
        let username = username.into();
        if !USERNAME_REGULAR_EXPRESSION.is_match(&username) {
            return None;
        }

        Some(Self(username))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
