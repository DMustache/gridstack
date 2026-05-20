use axum::{
    Router,
    routing::{get, post},
};

use crate::services::{identity::handlers, state::ApplicationState};

#[must_use]
pub fn routes(_application_state: &ApplicationState) -> Router<ApplicationState> {
    Router::new()
        .route(
            "/_matrix/identity/v2/pubkey/ephemeral/isvalid",
            get(handlers::key_management::is_ephemeral_public_key_valid),
        )
        .route(
            "/_matrix/identity/v2/pubkey/isvalid",
            get(handlers::key_management::is_long_term_public_key_valid),
        )
        .route(
            "/_matrix/identity/v2/pubkey/{keyId}",
            get(handlers::key_management::get_public_key),
        )
        .route(
            "/_matrix/identity/v2/hash_details",
            get(handlers::association_lookup::get_hash_details),
        )
        .route(
            "/_matrix/identity/v2/lookup",
            post(handlers::association_lookup::lookup_associations),
        )
        .route(
            "/_matrix/identity/v2/store-invite",
            post(handlers::invitation_storage::store_invite),
        )
        .route(
            "/_matrix/identity/v2/sign-ed25519",
            post(handlers::invitation_signing::sign_ed25519),
        )
}
