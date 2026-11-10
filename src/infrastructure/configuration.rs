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
    #[serde(default)]
    pub sync: SyncConfiguration,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfiguration {
    pub host: String,
    pub port: u16,
    pub server_name: String,
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
pub struct SyncConfiguration {
    /// Upper bound for the `/sync` `timeout` query parameter. Larger requested
    /// values are clamped to it, not rejected.
    #[serde(default = "default_maximum_sync_timeout_milliseconds")]
    pub maximum_timeout_milliseconds: u64,
}

impl Default for SyncConfiguration {
    fn default() -> Self {
        Self {
            maximum_timeout_milliseconds: default_maximum_sync_timeout_milliseconds(),
        }
    }
}

const fn default_maximum_sync_timeout_milliseconds() -> u64 {
    30_000
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
        let application_configuration: Self = configuration.try_deserialize()?;
        application_configuration.server.server_name()?;
        Ok(application_configuration)
    }
}

impl ServerConfiguration {
    pub fn socket_address(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse()?)
    }

    pub fn server_name(&self) -> anyhow::Result<ServerName> {
        ServerName::try_new(self.server_name.as_str()).ok_or_else(|| {
            anyhow::anyhow!(
                "invalid [server] server_name \"{}\": expected a Matrix server name such as \"example.org\" or \"example.org:8448\"",
                self.server_name
            )
        })
    }

    pub fn presented_server_name(&self) -> anyhow::Result<String> {
        Ok(self.server_name()?.present(self.server_name_presentation))
    }
}

#[cfg(test)]
mod tests {
    use super::ServerConfiguration;
    use crate::infrastructure::server_name::ServerNamePresentationFormat;

    fn server_configuration(host: &str, port: u16, server_name: &str) -> ServerConfiguration {
        ServerConfiguration {
            host: host.to_owned(),
            port,
            server_name: server_name.to_owned(),
            server_name_presentation: ServerNamePresentationFormat::default(),
        }
    }

    #[test]
    fn server_name_is_independent_of_bind_address() {
        let first = server_configuration("127.0.0.1", 3000, "example.org");
        let second = server_configuration("0.0.0.0", 9999, "example.org");
        assert_eq!(first.server_name().unwrap(), second.server_name().unwrap());
        assert_eq!(first.server_name().unwrap().as_str(), "example.org");
    }

    #[test]
    fn ipv6_bind_address_does_not_panic() {
        let configuration = server_configuration("::", 3000, "example.org");
        assert_eq!(configuration.server_name().unwrap().as_str(), "example.org");
    }

    #[test]
    fn invalid_server_name_fails_configuration_loading() {
        let path = std::env::temp_dir().join("gridstack_invalid_server_name_test.toml");
        std::fs::write(
            &path,
            "[server]\nhost = \"127.0.0.1\"\nport = 3000\nserver_name = \"bad name:\"\n\n[database]\nurl = \"x\"\n\n[authenification]\nallow_registration = true\naccess_token_expiry_seconds = 1\nrefresh_token_expiry_seconds = 1\njson_web_token_secret = \"x\"\npassword_pepper = \"x\"\n\n[rate_limit]\nretry_after_ms = 1\n",
        )
        .unwrap();
        let result = super::ApplicationConfiguration::load_from_path(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("invalid [server] server_name \"bad name:\""),
            "{message}"
        );
    }
}
