use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    routing::post,
};
use models::error::{AppError, AppResult};
use ruma::{api::client::error::ErrorKind, exports::serde_json::Value};

use crate::{
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        rate_limit::layers::rate_limit,
    },
    state::ServerState,
};

use crate::matrix::client::v3::join::join_room_by_id;

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct BodyInfo {
    reason: Option<String>,
    third_party_signed: Option<Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BodyView {
    room_id: String,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", post(post_join_room))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/rooms/{roomId}/join",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier")
    ),
    request_body = crate::docs::JoinRequestDoc,
    responses(
        (status = 200, description = "Joined room", body = crate::docs::JoinResponseDoc),
        (status = 400, description = "Unsupported join payload", body = crate::docs::MatrixErrorResponse),
        (status = 403, description = "Join forbidden", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn post_join_room(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path(room_id): Path<String>,
    Json(body): Json<BodyInfo>,
) -> AppResult<Json<BodyView>> {
    if body.third_party_signed.is_some() {
        return Err(AppError::bad_request(
            ErrorKind::Unknown,
            "third_party_signed is not supported yet",
        ));
    }

    let room_id = join_room_by_id(&state, &user.user_id, &room_id, body.reason).await?;

    Ok(Json(BodyView { room_id }))
}
