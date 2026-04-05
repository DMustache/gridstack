use axum::{Extension, Json, Router, middleware::from_fn_with_state, routing::get};

use crate::{
    matrix::client::v3::account::whoami::data_transfer_objects::GetViewModel,
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        rate_limit::layers::rate_limit,
    },
    state::ServerState,
};

mod data_transfer_objects;

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", get(get_whoami))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/account/whoami",
    tag = "matrix-client",
    security(("access_token" = [])),
    responses(
        (status = 200, description = "Current authenticated Matrix user", body = crate::docs::WhoAmIResponseDoc),
        (status = 403, description = "Authentication required", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn get_whoami(Extension(user): AuthenticatedUserExtension) -> Json<GetViewModel> {
    Json(user.into())
}
