use axum::{Router, middleware, routing::post};

use crate::services::{layers::AuthorizationLayer, rooms::handlers, state::ApplicationState};

pub fn routes(state: &ApplicationState) -> Router<ApplicationState> {
    Router::new()
        .route("/_matrix/client/v3/createRoom", post(handlers::create_room))
        .layer(middleware::from_extractor_with_state::<
            AuthorizationLayer,
            ApplicationState,
        >(state.clone()))
}
