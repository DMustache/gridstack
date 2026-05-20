use axum::Json;

use crate::services::identity::{
    contracts::{SignEd25519Request, SignEd25519Response},
    entities::{EncodedPrivateKey, InvitationToken, MatrixUserIdentifier},
    errors::IdentityServiceError,
};

pub async fn sign_ed25519(
    Json(request): Json<SignEd25519Request>,
) -> Result<Json<SignEd25519Response>, IdentityServiceError> {
    let _mxid = MatrixUserIdentifier::parse(request.mxid)?;
    let _token = InvitationToken::parse(request.token)?;
    let _private_key = EncodedPrivateKey::parse(request.private_key)?;

    Err(IdentityServiceError::NotImplemented)
}
