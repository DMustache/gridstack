use axum::{
    Router, middleware,
    routing::{get, put},
};

use crate::services::{
    layers::{AuthorizationLayer, RateLimitLayer},
    messages::handlers,
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
        .route(
            "/_matrix/client/v3/rooms/{roomId}/event/{eventId}",
            get(handlers::get_room_event),
        )
        .route(
            "/_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}",
            put(handlers::send_message_event),
        )
        .route_layer(authorization_layer)
        .route_layer(rate_limit_layer)
}
