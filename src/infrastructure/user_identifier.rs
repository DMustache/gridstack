use serde::{Deserialize, Serialize};

/// format @localpart:domain
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserIdentifier(String);

impl UserIdentifier {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        let candidate = value.trim();
        if candidate.is_empty() {
            return None;
        }

        if !candidate.starts_with('@') {
            return None;
        }

        let (localpart, server_name) = candidate[1..].split_once(':')?;

        if localpart.is_empty() || server_name.is_empty() {
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

    pub fn into_inner(self) -> String {
        self.0
    }
}
