use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::infrastructure::{device_id::DeviceId, user_identifier::UserIdentifier};

use crate::services::traits::Clock;

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
    pub user_identifier: UserIdentifier,
    pub password_hash: String,
    pub display_name: String,
    pub is_guest: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExistingUserIdentifier(UserIdentifier);

impl ExistingUserIdentifier {
    pub const fn as_user_identifier(&self) -> &UserIdentifier {
        &self.0
    }

    pub fn into_inner(self) -> UserIdentifier {
        self.0
    }

    pub(in crate::services::authorization) const fn new(user_identifier: UserIdentifier) -> Self {
        Self(user_identifier)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthorizedUserIdentifier(ExistingUserIdentifier);

impl AuthorizedUserIdentifier {
    pub const fn as_existing_user_identifier(&self) -> &ExistingUserIdentifier {
        &self.0
    }

    pub const fn as_user_identifier(&self) -> &UserIdentifier {
        self.0.as_user_identifier()
    }

    pub fn into_inner(self) -> ExistingUserIdentifier {
        self.0
    }

    pub(in crate::services::authorization) const fn new(
        existing_user_identifier: ExistingUserIdentifier,
    ) -> Self {
        Self(existing_user_identifier)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppserviceIdentity {
    pub appservice_id: String,
    pub controlled_user_id_patterns: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionPrincipal {
    #[default]
    User,
    Appservice {
        appservice_id: String,
        controlled_user_id_patterns: Vec<String>,
    },
}

/// Available via `Extension(access_session)`: `Extension<AccessSession>` by layer
/// ```rust
/// .layer(middleware::from_extractor_with_state::<
///     RateLimitLayer,
///     ApplicationState,
/// >(state.clone()))
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessSession {
    access_token: AccessToken,
    user_identifier: AuthorizedUserIdentifier,
    device_id: DeviceId,
    expires_at_seconds: i64,
    #[serde(default)]
    principal: SessionPrincipal,
}

impl AccessSession {
    pub(in crate::services::authorization) const fn new(
        access_token: AccessToken,
        user_identifier: AuthorizedUserIdentifier,
        device_id: DeviceId,
        expires_at_seconds: i64,
    ) -> Self {
        Self {
            access_token,
            user_identifier,
            device_id,
            expires_at_seconds,
            principal: SessionPrincipal::User,
        }
    }

    pub(in crate::services::authorization) fn new_appservice(
        access_token: AccessToken,
        user_identifier: AuthorizedUserIdentifier,
        device_id: DeviceId,
        expires_at_seconds: i64,
        appservice_identity: AppserviceIdentity,
    ) -> Self {
        Self {
            access_token,
            user_identifier,
            device_id,
            expires_at_seconds,
            principal: SessionPrincipal::Appservice {
                appservice_id: appservice_identity.appservice_id,
                controlled_user_id_patterns: appservice_identity.controlled_user_id_patterns,
            },
        }
    }

    pub const fn access_token(&self) -> &AccessToken {
        &self.access_token
    }

    pub const fn user_identifier(&self) -> &AuthorizedUserIdentifier {
        &self.user_identifier
    }

    pub const fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub const fn expires_at_seconds(&self) -> i64 {
        self.expires_at_seconds
    }

    pub const fn principal(&self) -> &SessionPrincipal {
        &self.principal
    }
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum LoginType {
    #[serde(rename = "m.login.password")]
    Password,
    #[serde(rename = "m.login.token")]
    Token,
    #[serde(other)]
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum UserIdentifierType {
    #[serde(rename = "m.id.user")]
    MatrixUser,
    #[serde(rename = "m.id.thirdparty")]
    ThirdParty,
    #[serde(rename = "m.id.phone")]
    PhoneNumber,
    #[serde(other)]
    Unsupported,
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
