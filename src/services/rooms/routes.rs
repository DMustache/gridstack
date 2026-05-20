use axum::{
    Router, middleware,
    routing::{get, post, put},
};

use crate::services::{
    layers::{AuthorizationLayer, RateLimitLayer},
    rooms::handlers,
    state::ApplicationState,
};

pub fn routes(state: &ApplicationState) -> Router<ApplicationState> {
    let authorization_layer = middleware::from_extractor_with_state::<
        AuthorizationLayer,
        ApplicationState,
    >(state.clone());
    let rate_limit_layer =
        middleware::from_extractor_with_state::<RateLimitLayer, ApplicationState>(state.clone());

    Router::new()
        .route("/_matrix/client/v3/createRoom", post(handlers::create_room))
        .route(
            "/_matrix/client/v3/rooms/{roomId}/join",
            post(handlers::join_room_by_id),
        )
        .route(
            "/_matrix/client/v3/joined_rooms",
            get(handlers::get_joined_rooms),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/joined_members",
            get(handlers::get_joined_members),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/leave",
            post(handlers::leave_room_by_id),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/invite",
            post(handlers::invite_user_to_room),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/state",
            get(handlers::get_room_state),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/members",
            get(handlers::get_room_members),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/state/{eventType}/{stateKey}",
            get(handlers::get_room_state_with_key).put(handlers::set_room_state_with_key),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/state/{eventType}",
            get(handlers::get_room_state_with_empty_key),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/messages",
            get(handlers::get_room_messages),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/event/{eventId}",
            get(handlers::get_room_event),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}",
            put(handlers::send_room_message_event),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/receipt/{receiptType}/{eventId}",
            post(handlers::send_room_receipt),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/read_markers",
            post(handlers::set_room_read_markers),
        )
        .route_layer(authorization_layer)
        .route_layer(rate_limit_layer)
}
