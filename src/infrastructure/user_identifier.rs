use serde::{Deserialize, Serialize};

use crate::infrastructure::server_name;

/// format @localpart:domain
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserIdentifier(String);

impl UserIdentifier {
    fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let candidate = value.trim();
        if candidate.is_empty() {
            return None;
        }

        if !candidate.starts_with('@') {
            return None;
        }

        let (localpart, _) = server_name::split_localpart_and_server_name(&candidate[1..])?;
        if localpart.is_empty() {
            return None;
        }

        Some(Self(candidate.to_owned()))
    }

    pub fn from_localpart_and_server(localpart: &str, server_name: &str) -> Option<Self> {
        Self::parse(format!("@{localpart}:{server_name}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn server_name(&self) -> Option<&str> {
        let (_, server_name) = self.0.split_once(':')?;
        server_name::is_valid_server_name(server_name).then_some(server_name)
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for UserIdentifier {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or("invalid user identifier")
    }
}
