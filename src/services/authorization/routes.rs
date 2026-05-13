use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::services::{
    authorization::handlers,
    layers::{AuthorizationLayer, RateLimitLayer},
    state::ApplicationState,
};

pub fn routes(state: &ApplicationState) -> Router<ApplicationState> {
    let public_routes = Router::new()
        .route("/_matrix/client/v3/register", post(handlers::register_user))
        .layer(middleware::from_extractor_with_state::<
            RateLimitLayer,
            ApplicationState,
        >(state.clone()))
        .route(
            "/_matrix/client/v3/register/available",
            get(handlers::check_username_available),
        )
        .layer(middleware::from_extractor_with_state::<
            RateLimitLayer,
            ApplicationState,
        >(state.clone()))
        .route(
            "/_matrix/client/v3/login",
            get(handlers::get_login_flows).post(handlers::login_user),
        )
        .layer(middleware::from_extractor_with_state::<
            RateLimitLayer,
            ApplicationState,
        >(state.clone()))
        .route(
            "/_matrix/client/v1/auth_metadata",
            get(handlers::get_auth_metadata),
        );

    let protected_routes = Router::new()
        .route("/_matrix/client/v3/account/whoami", get(handlers::who_am_i))
        .layer(middleware::from_extractor_with_state::<
            RateLimitLayer,
            ApplicationState,
        >(state.clone()))
        .layer(middleware::from_extractor_with_state::<
            AuthorizationLayer,
            ApplicationState,
        >(state.clone()));

    Router::new().merge(public_routes).merge(protected_routes)
}
