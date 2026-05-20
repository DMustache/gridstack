use serde_json::Value;

use crate::services::identity::{
    entities::{EncodedPrivateKey, InvitationToken, MatrixUserIdentifier},
    errors::IdentityServiceError,
};

pub struct SignThirdPartyInviteCommand {
    pub mxid: MatrixUserIdentifier,
    pub token: InvitationToken,
    pub private_key: EncodedPrivateKey,
}

pub struct SignedThirdPartyInvite {
    pub mxid: MatrixUserIdentifier,
    pub sender: MatrixUserIdentifier,
    pub token: InvitationToken,
    pub signatures: Value,
}

pub trait IdentityInvitationSigningRepository: Send + Sync {
    fn find_sender_for_invitation(
        &self,
        token: &InvitationToken,
    ) -> Result<Option<MatrixUserIdentifier>, IdentityServiceError>;
}

pub trait IdentityJsonSigner: Send + Sync {
    fn sign_third_party_invite(
        &self,
        command: &SignThirdPartyInviteCommand,
        sender: &MatrixUserIdentifier,
    ) -> Result<Value, IdentityServiceError>;
}

pub struct InvitationSigningService<Repository, JsonSigner>
where
    Repository: IdentityInvitationSigningRepository,
    JsonSigner: IdentityJsonSigner,
{
    invitation_repository: Repository,
    json_signer: JsonSigner,
}

impl<Repository, JsonSigner> InvitationSigningService<Repository, JsonSigner>
where
    Repository: IdentityInvitationSigningRepository,
    JsonSigner: IdentityJsonSigner,
{
    #[must_use]
    pub const fn new(invitation_repository: Repository, json_signer: JsonSigner) -> Self {
        Self {
            invitation_repository,
            json_signer,
        }
    }

    pub fn sign_invite(
        &self,
        command: &SignThirdPartyInviteCommand,
    ) -> Result<SignedThirdPartyInvite, IdentityServiceError> {
        let sender = self
            .invitation_repository
            .find_sender_for_invitation(&command.token)?
            .ok_or(IdentityServiceError::NotFound)?;
        let signatures = self.json_signer.sign_third_party_invite(command, &sender)?;

        Ok(SignedThirdPartyInvite {
            mxid: command.mxid.clone(),
            sender,
            token: command.token.clone(),
            signatures,
        })
    }
}
