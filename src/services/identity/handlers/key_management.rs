use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
};

use crate::services::{
    identity::{
        contracts::{PublicKeyResponse, PublicKeyValidityQuery, PublicKeyValidityResponse},
        entities::{EncodedPublicKey, IdentitySigningKeyId},
        errors::IdentityServiceError,
    },
    state::ApplicationState,
};

pub async fn get_public_key(
    State(application_state): State<ApplicationState>,
    Path(key_id): Path<String>,
) -> Result<Json<PublicKeyResponse>, IdentityServiceError> {
    let key_id = IdentitySigningKeyId::parse(key_id)?;
    let public_key = application_state
        .identity_key_management_service
        .get_public_key(&key_id)?;

    Ok(Json(PublicKeyResponse {
        public_key: public_key.as_str().to_owned(),
    }))
}

pub async fn is_long_term_public_key_valid(
    State(application_state): State<ApplicationState>,
    query: Result<Query<PublicKeyValidityQuery>, QueryRejection>,
) -> Result<Json<PublicKeyValidityResponse>, IdentityServiceError> {
    evaluate_public_key_validity(query, |public_key| {
        application_state
            .identity_key_management_service
            .is_long_term_public_key_valid(public_key)
    })
}

pub async fn is_ephemeral_public_key_valid(
    State(application_state): State<ApplicationState>,
    query: Result<Query<PublicKeyValidityQuery>, QueryRejection>,
) -> Result<Json<PublicKeyValidityResponse>, IdentityServiceError> {
    evaluate_public_key_validity(query, |public_key| {
        application_state
            .identity_key_management_service
            .is_ephemeral_public_key_valid(public_key)
    })
}

fn evaluate_public_key_validity<F>(
    query: Result<Query<PublicKeyValidityQuery>, QueryRejection>,
    validator: F,
) -> Result<Json<PublicKeyValidityResponse>, IdentityServiceError>
where
    F: FnOnce(&EncodedPublicKey) -> Result<bool, IdentityServiceError>,
{
    let query = query.map_err(|error| map_public_key_query_rejection(&error))?;
    let Query(query) = query;
    let public_key = EncodedPublicKey::parse(query.public_key)?;
    let valid = validator(&public_key)?;

    Ok(Json(PublicKeyValidityResponse { valid }))
}

fn map_public_key_query_rejection(error: &QueryRejection) -> IdentityServiceError {
    let message = error.body_text();
    if message.contains("missing field `public_key`") {
        return IdentityServiceError::MissingParameters(
            "Missing required query parameter: public_key".to_owned(),
        );
    }

    IdentityServiceError::InvalidRequest(message)
}
