use crate::services::identity::{
    entities::{
        IdentitySigningKey, MatrixUserIdentifier, RoomIdentifier, StoredThirdPartyInvite,
        ThirdPartyIdentifierAddress, ThirdPartyIdentifierMedium,
    },
    errors::IdentityServiceError,
};

pub struct StoreThirdPartyInviteCommand {
    pub medium: ThirdPartyIdentifierMedium,
    pub address: ThirdPartyIdentifierAddress,
    pub room_id: RoomIdentifier,
    pub sender: MatrixUserIdentifier,
}

/// A freshly generated ephemeral key together with the private seed that must be persisted.
pub struct GeneratedEphemeralKey {
    pub key: IdentitySigningKey,
    pub private_seed: String,
}

pub trait IdentityInvitationRepository: Send + Sync {
    fn store_invite(
        &self,
        command: &StoreThirdPartyInviteCommand,
        ephemeral_keys: Vec<GeneratedEphemeralKey>,
    ) -> Result<StoredThirdPartyInvite, IdentityServiceError>;
}

pub trait IdentityEphemeralKeyGenerator: Send + Sync {
    fn generate_invite_key(&self) -> Result<GeneratedEphemeralKey, IdentityServiceError>;
}

pub struct InvitationStorageService<Repository, KeyGenerator>
where
    Repository: IdentityInvitationRepository,
    KeyGenerator: IdentityEphemeralKeyGenerator,
{
    invitation_repository: Repository,
    ephemeral_key_generator: KeyGenerator,
}

impl<Repository, KeyGenerator> InvitationStorageService<Repository, KeyGenerator>
where
    Repository: IdentityInvitationRepository,
    KeyGenerator: IdentityEphemeralKeyGenerator,
{
    #[must_use]
    pub const fn new(
        invitation_repository: Repository,
        ephemeral_key_generator: KeyGenerator,
    ) -> Self {
        Self {
            invitation_repository,
            ephemeral_key_generator,
        }
    }

    pub fn store_invite(
        &self,
        command: &StoreThirdPartyInviteCommand,
    ) -> Result<StoredThirdPartyInvite, IdentityServiceError> {
        let ephemeral_key = self.ephemeral_key_generator.generate_invite_key()?;
        self.invitation_repository
            .store_invite(command, vec![ephemeral_key])
    }
}
