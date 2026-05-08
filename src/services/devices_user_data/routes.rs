use axum::{Router, routing::get};

use crate::services::{devices_user_data::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new().route("/_matrix/client/v3/devices", get(handlers::get_devices))
}
