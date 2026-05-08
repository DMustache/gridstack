use axum::{
    Router,
    routing::{get, post},
};

use crate::services::{rooms::handlers, state::ApplicationState};

pub fn routes() -> Router<ApplicationState> {
    Router::new()
        .route("/_matrix/client/v3/createRoom", post(handlers::create_room))
        .route(
            "/_matrix/client/v3/join/{roomIdOrAlias}",
            post(handlers::join_room),
        )
        .route(
            "/_matrix/client/v3/publicRooms",
            get(handlers::get_public_rooms).post(handlers::post_public_rooms),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/state",
            get(handlers::get_room_state),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/state/{eventType}/{stateKey}",
            get(handlers::get_room_state).put(handlers::put_room_state),
        )
}
