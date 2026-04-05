use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    routing::post,
};
use models::error::{AppError, AppResult};
use ruma::{
    api::client::error::ErrorKind,
    exports::serde_json::{Value, json},
};

use crate::{
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        messages::{self, AppendRoomEventCommand},
        rate_limit::layers::rate_limit,
    },
    state::ServerState,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct BodyInfo {
    reason: Option<String>,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", post(post_leave_room))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/rooms/{roomId}/leave",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier")
    ),
    request_body = crate::docs::LeaveRequestDoc,
    responses(
        (status = 200, description = "Left room", body = crate::docs::EmptyResponse),
        (status = 404, description = "Room not found", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn post_leave_room(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path(room_id): Path<String>,
    Json(body): Json<BodyInfo>,
) -> AppResult<Json<Value>> {
    if !state.rooms.room_exists(room_id.clone()).await? {
        return Err(AppError::with_status(
            axum::http::StatusCode::NOT_FOUND,
            ErrorKind::Unknown,
            "Room not found",
        ));
    }

    let membership =
        messages::get_membership(&state, room_id.clone(), user.user_id.clone()).await?;

    if membership
        .as_ref()
        .is_some_and(|membership| membership.membership == "leave")
        || membership.is_none()
    {
        return Ok(Json(json!({})));
    }

    let mut content = json!({ "membership": "leave" });
    if let Some(reason) = body.reason
        && let Some(object) = content.as_object_mut()
    {
        object.insert(String::from("reason"), json!(reason));
    }

    messages::append_room_event(
        &state,
        AppendRoomEventCommand {
            room_id,
            sender_user_id: user.user_id.clone(),
            event_type: String::from("m.room.member"),
            state_key: Some(user.user_id),
            content,
            unsigned: None,
            transaction_id: None,
        },
    )
    .await?;

    Ok(Json(json!({})))
}
