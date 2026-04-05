use axum::{Extension, Json, Router, middleware::from_fn_with_state, routing::get};

use crate::{
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        messages,
    },
    state::ServerState,
};

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BodyView {
    joined_rooms: Vec<String>,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", get(get_joined_rooms))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/joined_rooms",
    tag = "matrix-client",
    security(("access_token" = [])),
    responses(
        (status = 200, description = "Rooms where the user is joined", body = crate::docs::JoinedRoomsResponseDoc),
        (status = 403, description = "Authentication required", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn get_joined_rooms(
    Extension(user): AuthenticatedUserExtension,
    axum::extract::State(state): axum::extract::State<ServerState>,
) -> models::error::AppResult<Json<BodyView>> {
    let joined_rooms = messages::list_joined_rooms(&state, user.user_id)
        .await?
        .into_iter()
        .map(|membership| membership.room_id)
        .collect();

    Ok(Json(BodyView { joined_rooms }))
}
