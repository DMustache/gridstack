use crate::{
    infrastructure::device_id::DeviceId,
    services::authorization::entities::{AccessSession, AccessToken, AuthorizedUserIdentifier},
};

#[derive(Clone)]
pub struct AccessSessionStorageUnit {
    access_token: AccessToken,
    user_identifier: AuthorizedUserIdentifier,
    device_id: DeviceId,
    expires_at_seconds: i64,
}

impl AccessSessionStorageUnit {
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
        }
    }
}

impl From<AccessSession> for AccessSessionStorageUnit {
    fn from(value: AccessSession) -> Self {
        Self {
            access_token: value.access_token().clone(),
            user_identifier: value.user_identifier().clone(),
            device_id: value.device_id().clone(),
            expires_at_seconds: value.expires_at_seconds(),
        }
    }
}
