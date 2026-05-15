use axum::{Router, middleware, routing::post};

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
        .route_layer(authorization_layer)
        .route_layer(rate_limit_layer)
}
