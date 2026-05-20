use crate::services::identity::{
    entities::{EncodedPublicKey, IdentitySigningKey, IdentitySigningKeyId},
    errors::IdentityServiceError,
};

pub trait IdentitySigningKeyRepository: Send + Sync {
    fn find_long_term_key(
        &self,
        key_id: &IdentitySigningKeyId,
    ) -> Result<Option<IdentitySigningKey>, IdentityServiceError>;

    fn is_long_term_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError>;

    fn is_ephemeral_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError>;
}

pub struct KeyManagementService<Repository>
where
    Repository: IdentitySigningKeyRepository,
{
    signing_key_repository: Repository,
}

impl<Repository> KeyManagementService<Repository>
where
    Repository: IdentitySigningKeyRepository,
{
    #[must_use]
    pub const fn new(signing_key_repository: Repository) -> Self {
        Self {
            signing_key_repository,
        }
    }

    pub fn get_public_key(
        &self,
        key_id: &IdentitySigningKeyId,
    ) -> Result<EncodedPublicKey, IdentityServiceError> {
        self.signing_key_repository
            .find_long_term_key(key_id)?
            .map(|key| key.public_key)
            .ok_or(IdentityServiceError::PublicKeyNotFound)
    }

    pub fn is_long_term_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError> {
        self.signing_key_repository
            .is_long_term_public_key_valid(public_key)
    }

    pub fn is_ephemeral_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError> {
        self.signing_key_repository
            .is_ephemeral_public_key_valid(public_key)
    }
}
