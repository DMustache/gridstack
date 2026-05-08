use axum::{Router, routing::post};

use crate::services::{encryption::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new().route(
        "/_matrix/client/v3/keys/upload",
        post(handlers::upload_keys),
    )
}
