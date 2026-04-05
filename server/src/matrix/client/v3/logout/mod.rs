use axum::{
    Extension, Json, Router, extract::State, middleware::from_fn_with_state, routing::post,
};
use models::error::AppResult;
use ruma::exports::serde_json::{Value, json};

use crate::{
    services::authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
    state::ServerState,
};

pub(crate) mod all;

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", post(post_logout))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .nest("/all", all::router(state.clone()))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/logout",
    tag = "matrix-client",
    security(("access_token" = [])),
    responses(
        (status = 200, description = "Logged out current device", body = crate::docs::EmptyResponse),
        (status = 403, description = "Authentication required", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn post_logout(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
) -> AppResult<Json<Value>> {
    state
        .accounts
        .logout_by_access_token(user.access_token.clone())
        .await?;

    Ok(Json(json!({})))
}
