use axum::{Json, extract::State};
use axum_extra::routing::TypedPath;

use crate::services::{
    identity::{
        contracts::{StoreInviteRequest, StoreInviteResponse, ThirdPartyInvitePublicKey},
        endpoints::{
            IdentityV2PublicKeyEphemeralIsValidPath, IdentityV2PublicKeyIsValidPath,
        },
        entities::{
            MatrixUserIdentifier, RoomIdentifier, ThirdPartyIdentifierAddress,
            ThirdPartyIdentifierMedium,
        },
        errors::IdentityServiceError,
        service::StoreThirdPartyInviteCommand,
    },
    state::ApplicationState,
};

pub async fn store_invite(
    State(application_state): State<ApplicationState>,
    Json(request): Json<StoreInviteRequest>,
) -> Result<Json<StoreInviteResponse>, IdentityServiceError> {
    let medium = ThirdPartyIdentifierMedium::parse(request.medium)?;
    let address = ThirdPartyIdentifierAddress::parse(request.address)?;
    let room_id = RoomIdentifier::parse(request.room_id)?;
    let sender = MatrixUserIdentifier::parse(request.sender)?;

    let command = StoreThirdPartyInviteCommand {
        medium,
        address,
        room_id,
        sender,
    };

    let stored_invite = application_state
        .identity_invitation_storage_service
        .store_invite(&command)?;

    let public_keys = stored_invite
        .public_keys
        .into_iter()
        .map(|public_key| {
            let key_validity_url = match public_key.usage {
                crate::services::identity::entities::IdentitySigningKeyUsage::LongTerm => {
                    format!(
                        "https://{}{}",
                        application_state.server_name,
                        IdentityV2PublicKeyIsValidPath.to_uri().to_string()
                    )
                }
                crate::services::identity::entities::IdentitySigningKeyUsage::EphemeralInvite => {
                    format!(
                        "https://{}{}",
                        application_state.server_name,
                        IdentityV2PublicKeyEphemeralIsValidPath.to_uri().to_string()
                    )
                }
            };

            ThirdPartyInvitePublicKey {
                key_validity_url,
                public_key: public_key.public_key.as_str().to_owned(),
            }
        })
        .collect();

    Ok(Json(StoreInviteResponse {
        display_name: stored_invite.display_name,
        public_keys,
        token: stored_invite.token.as_str().to_owned(),
    }))
}
