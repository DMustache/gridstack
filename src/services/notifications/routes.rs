use axum::{Router, routing::get};

use crate::services::{notifications::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new().route(
        "/_matrix/client/v3/notifications",
        get(handlers::get_notifications),
    )
}
