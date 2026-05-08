use axum::{Router, routing::get};

use crate::services::{administration::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new().route(
        "/_matrix/client/v3/admin/whois/{userId}",
        get(handlers::get_admin_whois),
    )
}
