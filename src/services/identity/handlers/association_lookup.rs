use axum::Json;

use crate::services::identity::{
    contracts::{AssociationLookupRequest, AssociationLookupResponse, HashDetailsResponse},
    entities::{IdentityHashAlgorithm, LookupPepper},
    errors::IdentityServiceError,
};

pub async fn get_hash_details() -> Result<Json<HashDetailsResponse>, IdentityServiceError> {
    Err(IdentityServiceError::NotImplemented)
}

pub async fn lookup_associations(
    Json(request): Json<AssociationLookupRequest>,
) -> Result<Json<AssociationLookupResponse>, IdentityServiceError> {
    let _algorithm = IdentityHashAlgorithm::parse(request.algorithm)?;
    let _pepper = LookupPepper::parse(request.pepper)?;
    if request.addresses.is_empty() {
        return Err(IdentityServiceError::InvalidRequest(
            "lookup requires at least one address".to_owned(),
        ));
    }

    Err(IdentityServiceError::NotImplemented)
}
