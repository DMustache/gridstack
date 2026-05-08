use axum::{Router, routing::get};

use crate::services::{media::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new().route(
        "/_matrix/client/v1/media/config",
        get(handlers::get_media_config),
    )
}
