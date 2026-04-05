use linkme::distributed_slice;
use models::error::AppResult;
use ruma::OwnedServerName;
use std::sync::Arc;
use uuid::Uuid;

use persistence::{accounts::Accounts, rooms::Rooms, users::Users};

use crate::services::rate_limit::layers::ClientRateLimiter;

#[derive(Clone)]
pub struct UnstableFeature {
    pub name: &'static str,
    pub is_authorization_required: bool,
}

#[derive(Clone)]
pub struct ServerState {
    pub(crate) is_registration_allowed: bool,
    pub(crate) is_rate_limiting_enabled: bool,
    pub server_address: String,
    pub(crate) registration_token: String,
    pub(crate) server_name: OwnedServerName,

    pub(crate) accounts: Arc<Accounts>,
    pub(crate) rooms: Arc<Rooms>,
    pub(crate) users: Arc<Users>,
    pub(crate) rate_limiter: Arc<ClientRateLimiter>,

    unstable_features: Vec<UnstableFeature>,
    versions: Vec<String>,
}

#[distributed_slice]
pub static UNSTABLE_FEATURES: [UnstableFeature];

pub static VERSIONS: [&'static str; 1] = ["v1.18"];

impl ServerState {
    pub fn try_new() -> AppResult<Self> {
        Ok(Self {
            is_registration_allowed: true,
            is_rate_limiting_enabled: true,
            server_address: String::from("127.0.0.1:3000"),
            registration_token: Uuid::new_v4().to_string(),
            server_name: OwnedServerName::parse("127.0.0.1".to_string()).unwrap(),
            accounts: Arc::new(Accounts::try_new()?),
            rooms: Arc::new(Rooms::try_new()?),
            users: Arc::new(Users::try_new()?),
            rate_limiter: Arc::new(ClientRateLimiter::default()),

            unstable_features: UNSTABLE_FEATURES
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
            versions: VERSIONS
                .iter()
                .map(|version| (*version).to_owned())
                .collect(),
        })
    }

    pub(crate) fn get_unstable_features(&self) -> Vec<UnstableFeature> {
        self.unstable_features.clone()
    }

    pub(crate) fn get_versions(&self) -> Vec<String> {
        self.versions.clone()
    }
}
