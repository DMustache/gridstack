use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::infrastructure::{server_name::ServerName, username::Username};

/// format @localpart:domain
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserIdentifier {
    localpart: Username,
    server_name: ServerName,

    full_identifier: Arc<String>,
}

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

        let (localpart, server_name) =
            ServerName::split_localpart_and_server_name(&candidate[1..])?;
        Self::from_localpart_and_server(localpart, server_name)
    }

    fn user_idetifier_format(username: &Username, server_name: &ServerName) -> String {
        format!("@{}:{}", username.as_str(), server_name.as_str())
    }

    pub fn from_localpart_and_server(localpart: &str, server_name: &str) -> Option<Self> {
        let localpart = Username::try_new(localpart)?;
        let server_name = ServerName::try_new(server_name)?;
        Some(Self {
            full_identifier: Arc::new(Self::user_idetifier_format(&localpart, &server_name)),
            localpart,
            server_name,
        })
    }

    pub const fn localpart(&self) -> &Username {
        &self.localpart
    }

    pub const fn server_name_value(&self) -> &ServerName {
        &self.server_name
    }

    pub fn server_name(&self) -> Option<&str> {
        Some(self.server_name.as_str())
    }

    pub fn as_str(&self) -> &str {
        &self.full_identifier
    }

    pub fn into_inner(self) -> String {
        self.full_identifier.to_string()
    }
}

impl TryFrom<String> for UserIdentifier {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or("invalid user identifier")
    }
}

impl std::fmt::Display for UserIdentifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.as_str())
    }
}

impl Serialize for UserIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UserIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}
