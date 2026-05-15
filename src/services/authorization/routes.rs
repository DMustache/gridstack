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
    let rate_limit_layer =
        middleware::from_extractor_with_state::<RateLimitLayer, ApplicationState>(state.clone());
    let authorization_layer = middleware::from_extractor_with_state::<
        AuthorizationLayer,
        ApplicationState,
    >(state.clone());

    let public_routes = Router::new()
        .route("/_matrix/client/v3/register", post(handlers::register_user))
        .route(
            "/_matrix/client/v3/register/available",
            get(handlers::check_username_available),
        )
        .route(
            "/_matrix/client/v3/login",
            get(handlers::get_login_flows).post(handlers::login_user),
        )
        .route(
            "/_matrix/client/v1/auth_metadata",
            get(handlers::get_auth_metadata),
        );

    let protected_routes = Router::new()
        .route("/_matrix/client/v3/account/whoami", get(handlers::who_am_i))
        .route_layer(authorization_layer);

    let logout_routes =
        Router::new().route("/_matrix/client/v3/logout", post(handlers::logout_user));

    public_routes
        .route_layer(rate_limit_layer.clone())
        .merge(protected_routes.route_layer(rate_limit_layer.clone()))
        .merge(logout_routes.route_layer(rate_limit_layer))
}
