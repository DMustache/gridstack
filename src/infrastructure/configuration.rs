use std::net::SocketAddr;

use config::{Config, ConfigBuilder, Environment, builder::DefaultState};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::infrastructure::server_name::{ServerName, ServerNamePresentationFormat};

#[derive(Clone, Debug, Deserialize)]
pub struct ApplicationConfiguration {
    pub server: ServerConfiguration,
    pub database: DatabaseConfiguration,
    #[serde(alias = "authentication")]
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
        let configuration_builder = Config::builder()
            .add_source(config::File::with_name(path).required(false))
            .add_source(
                Environment::with_prefix("GRIDSTACK")
                    .separator("__")
                    .try_parsing(true),
            );
        Self::load_from_builder(configuration_builder)
    }

    fn load_from_builder(
        configuration_builder: ConfigBuilder<DefaultState>,
    ) -> anyhow::Result<Self> {
        let configuration = configuration_builder.build()?;
        let application_configuration: Self = configuration.try_deserialize()?;
        application_configuration.server.server_name()?;
        application_configuration.validate_secrets()?;
        Ok(application_configuration)
    }

    fn validate_secrets(&self) -> anyhow::Result<()> {
        if self.database.url.trim().is_empty() {
            anyhow::bail!("[database] url must not be empty");
        }
        validate_secret(
            "[authenification] json_web_token_secret",
            &self.authenification.json_web_token_secret,
        )?;
        validate_secret(
            "[authenification] password_pepper",
            &self.authenification.password_pepper,
        )
    }
}

const MINIMUM_SECRET_LENGTH_BYTES: usize = 32;

/// SHA-256 hex digests of secret values that were committed to git history.
const KNOWN_LEAKED_SECRET_DIGESTS: [&str; 2] = [
    "b3a679dd8fc0fe824fc1cf215d2ceca0dfd297abed5e4aabb4a69a3fe807f453",
    "437b08f45dca9aaa4b410d7c25874e764f9af26265de4e51f6c56e0199ee75af",
];

fn validate_secret(setting_name: &str, secret: &str) -> anyhow::Result<()> {
    validate_secret_against(setting_name, secret, &KNOWN_LEAKED_SECRET_DIGESTS)
}

fn sha256_hex(value: &str) -> String {
    use std::fmt::Write;
    Sha256::digest(value.as_bytes())
        .iter()
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn validate_secret_against(
    setting_name: &str,
    secret: &str,
    leaked_digests: &[&str],
) -> anyhow::Result<()> {
    if secret.is_empty() {
        anyhow::bail!("{setting_name} must be set (use the GRIDSTACK__ environment variable)");
    }
    if secret.len() < MINIMUM_SECRET_LENGTH_BYTES {
        anyhow::bail!("{setting_name} must be at least {MINIMUM_SECRET_LENGTH_BYTES} bytes long");
    }
    if leaked_digests.contains(&sha256_hex(secret).as_str()) {
        anyhow::bail!("{setting_name} is a known leaked value and must be rotated");
    }
    Ok(())
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
    use std::collections::HashMap;

    use config::{Config, Environment, File, FileFormat};

    use super::{ApplicationConfiguration, ServerConfiguration};
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

    const VALID_SECRET: &str = "0123456789abcdef0123456789abcdef-test";

    fn configuration_text(json_web_token_secret_line: &str, pepper: &str) -> String {
        format!(
            "[server]\nhost=\"127.0.0.1\"\nport=3000\nserver_name=\"example.org\"\n\
             [database]\nurl=\"postgres://example\"\n\
             [authenification]\nallow_registration=true\naccess_token_expiry_seconds=1\n\
             refresh_token_expiry_seconds=1\n{json_web_token_secret_line}\npassword_pepper=\"{pepper}\"\n\
             [rate_limit]\nretry_after_ms=1\n"
        )
    }

    fn load(
        text: &str,
        environment: HashMap<String, String>,
    ) -> anyhow::Result<ApplicationConfiguration> {
        let builder = Config::builder()
            .add_source(File::from_str(text, FileFormat::Toml))
            .add_source(
                Environment::with_prefix("GRIDSTACK")
                    .separator("__")
                    .source(Some(environment)),
            );
        ApplicationConfiguration::load_from_builder(builder)
    }

    fn valid_text() -> String {
        configuration_text(
            &format!("json_web_token_secret=\"{VALID_SECRET}\""),
            VALID_SECRET,
        )
    }

    #[test]
    fn environment_overrides_file_secret() {
        let text = configuration_text("json_web_token_secret=\"short\"", VALID_SECRET);
        let environment = HashMap::from([(
            "GRIDSTACK__AUTHENIFICATION__JSON_WEB_TOKEN_SECRET".to_owned(),
            VALID_SECRET.to_owned(),
        )]);
        let loaded = load(&text, environment).expect("environment secret should win");
        assert_eq!(loaded.authenification.json_web_token_secret, VALID_SECRET);
    }

    #[test]
    fn missing_json_web_token_secret_fails_startup() {
        let text = configuration_text("", VALID_SECRET);
        let error = load(&text, HashMap::new()).expect_err("must fail");
        assert!(error.to_string().contains("json_web_token_secret"));
    }

    #[test]
    fn short_secret_fails_startup() {
        let text = configuration_text("json_web_token_secret=\"too-short\"", VALID_SECRET);
        let error = load(&text, HashMap::new()).expect_err("must fail");
        assert!(error.to_string().contains("json_web_token_secret"));
    }

    #[test]
    fn known_leaked_secret_fails_startup() {
        let digest = super::sha256_hex(VALID_SECRET);
        let error = super::validate_secret_against("[x] y", VALID_SECRET, &[digest.as_str()])
            .expect_err("must fail");
        assert!(error.to_string().contains("known leaked"));
        assert!(!error.to_string().contains(VALID_SECRET));
    }

    #[test]
    fn error_message_does_not_contain_secret_value() {
        let secret = "short-but-recognizable-secret";
        let text = configuration_text(&format!("json_web_token_secret=\"{secret}\""), VALID_SECRET);
        let error = load(&text, HashMap::new()).expect_err("must fail");
        assert!(!format!("{error:#}").contains(secret));
        assert!(load(&valid_text(), HashMap::new()).is_ok());
    }
}
