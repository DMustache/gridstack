use axum::{Router, routing::get};

use crate::services::{discovery::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new()
        .route(
            "/.well-known/matrix/client",
            get(handlers::get_well_known_client),
        )
        .route(
            "/.well-known/matrix/policy_server",
            get(handlers::get_well_known_policy_server),
        )
        .route(
            "/.well-known/matrix/support",
            get(handlers::get_well_known_support),
        )
}
