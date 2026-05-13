use std::net::SocketAddr;

use config::Config;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ApplicationConfiguration {
    pub server: ServerConfiguration,
    pub database: DatabaseConfiguration,
    pub authenification: AuthenticationConfiguration,
    pub rate_limit: RateLimitConfiguration,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfiguration {
    pub host: String,
    pub port: u16,
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
pub struct RateLimitConfiguration {
    pub retry_after_ms: u64,
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
}
