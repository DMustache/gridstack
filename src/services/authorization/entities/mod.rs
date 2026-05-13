use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::services::traits::Clock;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return None;
        }
        Some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_identifier: UserId,
    pub password_hash: String,
    pub display_name: String,
    pub is_guest: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessSession {
    pub access_token: AccessToken,
    pub user_identifier: UserId,
}

#[derive(Debug, Default, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    #[default]
    User,
    Guest,
}

#[derive(Debug, PartialEq, Eq, Display, EnumString)]
pub enum UiaaFlowType {
    #[strum(serialize = "m.login.password")]
    Password,
}

pub struct ExpirationClock {
    expires_at_seconds: i64,
}

impl ExpirationClock {
    pub fn new(expires_at_seconds: i64) -> Self {
        Self {
            expires_at_seconds: (Utc::now() + Duration::seconds(expires_at_seconds)).timestamp(),
        }
    }
}

impl Clock for ExpirationClock {
    fn now_unix_milliseconds(&self) -> i64 {
        self.expires_at_seconds.saturating_mul(1000)
    }

    fn as_seconds(&self) -> i64 {
        self.expires_at_seconds
    }
}
