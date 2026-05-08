use axum::{
    Router,
    routing::{get, put},
};

use crate::services::{messaging_events::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new()
        .route(
            "/_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}",
            put(handlers::send_room_message),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/messages",
            get(handlers::get_room_messages),
        )
        .route("/_matrix/client/v3/sync", get(handlers::sync_events))
        .route("/_matrix/client/v3/events", get(handlers::get_events))
        .route(
            "/_matrix/client/v3/events/{eventId}",
            get(handlers::get_event_by_identifier),
        )
}
