use axum::{Extension, Json, Router, middleware::from_fn_with_state, routing::post};
use models::error::AppResult;
use ruma::exports::serde_json::{Value, json};

use crate::{
    services::authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
    state::ServerState,
};

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", post(post_logout_all))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/logout/all",
    tag = "matrix-client",
    security(("access_token" = [])),
    responses(
        (status = 200, description = "Logged out all devices", body = crate::docs::EmptyResponse),
        (status = 403, description = "Authentication required", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn post_logout_all(
    state: axum::extract::State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
) -> AppResult<Json<Value>> {
    state.accounts.logout_all_for_user(user.user_id).await?;

    Ok(Json(json!({})))
}
