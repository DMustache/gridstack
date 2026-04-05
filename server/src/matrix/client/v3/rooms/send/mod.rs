use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    routing::put,
};
use models::error::{AppError, AppResult};
use ruma::{api::client::error::ErrorKind, exports::serde_json::Value};

use crate::{
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        messages::{self, AppendRoomEventCommand, ROOM_MEMBERSHIP_JOIN},
    },
    state::ServerState,
};

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BodyView {
    event_id: String,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/{eventType}/{txnId}", put(put_send_event))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    put,
    path = "/_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier"),
        ("eventType" = String, Path, description = "Event type to send"),
        ("txnId" = String, Path, description = "Idempotency transaction id")
    ),
    request_body = std::collections::BTreeMap<String, Value>,
    responses(
        (status = 200, description = "Sent event id", body = crate::docs::SendEventResponseDoc),
        (status = 400, description = "Invalid event content", body = crate::docs::MatrixErrorResponse),
        (status = 403, description = "User is not joined to the room", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn put_send_event(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path((room_id, event_type, txn_id)): Path<(String, String, String)>,
    Json(content): Json<Value>,
) -> AppResult<Json<BodyView>> {
    ensure_send_request_is_valid(&state, &room_id, &user.user_id).await?;

    if let Some(existing_event) = messages::get_event_by_transaction_id(
        &state,
        room_id.clone(),
        user.user_id.clone(),
        txn_id.clone(),
    )
    .await?
    {
        return Ok(Json(BodyView {
            event_id: existing_event.event_id,
        }));
    }

    let event = messages::append_room_event(
        &state,
        AppendRoomEventCommand {
            room_id,
            sender_user_id: user.user_id,
            event_type,
            state_key: None,
            content: ensure_event_content(content)?,
            unsigned: None,
            transaction_id: Some(txn_id),
        },
    )
    .await?;

    Ok(Json(BodyView {
        event_id: event.event_id,
    }))
}

async fn ensure_send_request_is_valid(
    state: &ServerState,
    room_id: &str,
    user_id: &str,
) -> AppResult<()> {
    if !state.rooms.room_exists(room_id.to_owned()).await? {
        return Err(AppError::with_status(
            axum::http::StatusCode::NOT_FOUND,
            ErrorKind::Unknown,
            "Room not found",
        ));
    }

    let membership =
        messages::get_membership(state, room_id.to_owned(), user_id.to_owned()).await?;

    if membership
        .as_ref()
        .is_none_or(|membership| membership.membership != ROOM_MEMBERSHIP_JOIN)
    {
        return Err(AppError::forbidden(
            "You must be joined to the room to send events",
        ));
    }

    Ok(())
}

fn ensure_event_content(content: Value) -> AppResult<Value> {
    if content.is_object() {
        return Ok(content);
    }

    Err(AppError::bad_request(
        ErrorKind::BadJson,
        "Event content must be a JSON object",
    ))
}
