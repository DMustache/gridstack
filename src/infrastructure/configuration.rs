use std::net::SocketAddr;

use config::Config;
use serde::Deserialize;

use crate::infrastructure::server_name::{ServerName, ServerNamePresentationFormat};

#[derive(Clone, Debug, Deserialize)]
pub struct ApplicationConfiguration {
    pub server: ServerConfiguration,
    pub database: DatabaseConfiguration,
    pub authenification: AuthenticationConfiguration,
    #[serde(default)]
    pub identity: IdentityConfiguration,
    pub rate_limit: RateLimitConfiguration,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfiguration {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub server_name_presentation: ServerNamePresentationFormat,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfiguration {
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthenticationConfiguration {
    pub allow_registration: bool,
    pub access_token_expiry_seconds: u64,
    pub refresh_token_expiry_seconds: u64,
    pub json_web_token_secret: String,
    pub password_pepper: String,
    #[serde(default)]
    pub appservices: Vec<AppserviceConfiguration>,
}

impl AuthenticationConfiguration {
    pub const fn refresh_token_expiry_as_milliseconds(&self) -> u64 {
        self.refresh_token_expiry_seconds * 1000
    }

    pub const fn access_token_expiry_as_milliseconds(&self) -> u64 {
        self.access_token_expiry_seconds * 1000
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppserviceConfiguration {
    pub id: String,
    pub as_token: String,
    pub sender_localpart: String,
    #[serde(default)]
    pub controlled_user_id_patterns: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RateLimitConfiguration {
    pub retry_after_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IdentityConfiguration {
    #[serde(default = "default_long_term_key_id")]
    pub long_term_key_id: String,
    pub long_term_public_key: Option<String>,
    pub long_term_private_key: Option<String>,
    #[serde(default = "default_ephemeral_key_validity_seconds")]
    pub ephemeral_key_validity_seconds: u64,
}

impl Default for IdentityConfiguration {
    fn default() -> Self {
        Self {
            long_term_key_id: default_long_term_key_id(),
            long_term_public_key: None,
            long_term_private_key: None,
            ephemeral_key_validity_seconds: default_ephemeral_key_validity_seconds(),
        }
    }
}

fn default_long_term_key_id() -> String {
    "ed25519:0".to_owned()
}

const fn default_ephemeral_key_validity_seconds() -> u64 {
    60 * 60 * 24 * 7
}

impl ApplicationConfiguration {
    pub fn load_from_path(path: &str) -> anyhow::Result<Self> {
        let configuration_builder = Config::builder().add_source(config::File::with_name(path));
        let configuration = configuration_builder.build()?;
        Ok(configuration.try_deserialize()?)
    }
}

impl ServerConfiguration {
    pub fn socket_address(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse()?)
    }

    pub fn server_name(&self) -> anyhow::Result<ServerName> {
        ServerName::try_new(format!("{}:{}", self.host, self.port))
            .ok_or_else(|| anyhow::anyhow!("invalid server name in server.host/server.port"))
    }

    pub fn presented_server_name(&self) -> anyhow::Result<String> {
        Ok(self.server_name()?.present(self.server_name_presentation))
    }
}
