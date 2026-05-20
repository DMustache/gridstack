use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentitySigningKeyId {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentitySigningAlgorithm {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentitySigningKeyIdentifier {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedPublicKey {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedPrivateKey {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityServerAccessToken {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupPepper {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityHashAlgorithm {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThirdPartyIdentifierMedium {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThirdPartyIdentifierAddress {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixUserIdentifier {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomIdentifier {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvitationToken {
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentitySigningKey {
    pub key_id: IdentitySigningKeyId,
    pub public_key: EncodedPublicKey,
    pub usage: IdentitySigningKeyUsage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentitySigningKeyUsage {
    LongTerm,
    EphemeralInvite,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredThirdPartyInvite {
    pub medium: ThirdPartyIdentifierMedium,
    pub address: ThirdPartyIdentifierAddress,
    pub room_id: RoomIdentifier,
    pub sender: MatrixUserIdentifier,
    pub display_name: String,
    pub token: InvitationToken,
    pub public_keys: Vec<IdentitySigningKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityAssociation {
    pub medium: ThirdPartyIdentifierMedium,
    pub address: ThirdPartyIdentifierAddress,
    pub mxid: MatrixUserIdentifier,
}

impl IdentitySigningKeyId {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        let value = value.into();
        let (algorithm, identifier) = value
            .split_once(':')
            .ok_or(IdentityEntityError::InvalidSigningKeyId)?;

        IdentitySigningAlgorithm::parse(algorithm)?;
        IdentitySigningKeyIdentifier::parse(identifier)?;

        Ok(Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl Display for IdentitySigningKeyId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl IdentitySigningAlgorithm {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidSigningAlgorithm)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn ed25519() -> Self {
        Self {
            value: "ed25519".to_owned(),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl IdentitySigningKeyIdentifier {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidSigningKeyId)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl EncodedPublicKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidPublicKey)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl EncodedPrivateKey {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidPrivateKey)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl IdentityServerAccessToken {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidAccessToken)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl LookupPepper {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidLookupPepper)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl IdentityHashAlgorithm {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidHashAlgorithm)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn sha256() -> Self {
        Self {
            value: "sha256".to_owned(),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl ThirdPartyIdentifierMedium {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidThirdPartyMedium)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl ThirdPartyIdentifierAddress {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidThirdPartyAddress)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl MatrixUserIdentifier {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(
            value.into(),
            IdentityEntityError::InvalidMatrixUserIdentifier,
        )
        .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl RoomIdentifier {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidRoomIdentifier)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

impl InvitationToken {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdentityEntityError> {
        non_empty_value(value.into(), IdentityEntityError::InvalidInvitationToken)
            .map(|value| Self { value })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdentityEntityError {
    #[error("invalid identity signing key id")]
    InvalidSigningKeyId,
    #[error("invalid identity signing algorithm")]
    InvalidSigningAlgorithm,
    #[error("invalid identity public key")]
    InvalidPublicKey,
    #[error("invalid identity private key")]
    InvalidPrivateKey,
    #[error("invalid identity server access token")]
    InvalidAccessToken,
    #[error("invalid identity lookup pepper")]
    InvalidLookupPepper,
    #[error("invalid identity hash algorithm")]
    InvalidHashAlgorithm,
    #[error("invalid third-party identifier medium")]
    InvalidThirdPartyMedium,
    #[error("invalid third-party identifier address")]
    InvalidThirdPartyAddress,
    #[error("invalid Matrix user identifier")]
    InvalidMatrixUserIdentifier,
    #[error("invalid Matrix room identifier")]
    InvalidRoomIdentifier,
    #[error("invalid invitation token")]
    InvalidInvitationToken,
}

fn non_empty_value(
    value: String,
    error: IdentityEntityError,
) -> Result<String, IdentityEntityError> {
    if value.trim().is_empty() {
        return Err(error);
    }

    Ok(value)
}
