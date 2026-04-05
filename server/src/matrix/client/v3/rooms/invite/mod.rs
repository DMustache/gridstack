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
        messages::{self, AppendRoomEventCommand, ROOM_MEMBERSHIP_JOIN},
        rate_limit::layers::rate_limit,
    },
    state::ServerState,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct BodyInfo {
    reason: Option<String>,
    user_id: String,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", post(post_invite))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/rooms/{roomId}/invite",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier")
    ),
    request_body = crate::docs::InviteRequestDoc,
    responses(
        (status = 200, description = "Invited user or invite already exists", body = crate::docs::EmptyResponse),
        (status = 400, description = "Invalid invite payload", body = crate::docs::MatrixErrorResponse),
        (status = 403, description = "Invite forbidden", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn post_invite(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path(room_id): Path<String>,
    Json(body): Json<BodyInfo>,
) -> AppResult<Json<Value>> {
    ensure_invite_request_is_valid(&state, &room_id, &user.user_id, &body.user_id).await?;

    let existing_membership =
        messages::get_membership(&state, room_id.clone(), body.user_id.clone()).await?;

    if existing_membership
        .as_ref()
        .is_some_and(|membership| membership.membership == "invite")
    {
        return Ok(Json(json!({})));
    }

    let mut content = json!({ "membership": "invite" });
    if let Some(reason) = body.reason
        && let Some(object) = content.as_object_mut()
    {
        object.insert(String::from("reason"), json!(reason));
    }

    messages::append_room_event(
        &state,
        AppendRoomEventCommand {
            room_id,
            sender_user_id: user.user_id,
            event_type: String::from("m.room.member"),
            state_key: Some(body.user_id),
            content,
            unsigned: None,
            transaction_id: None,
        },
    )
    .await?;

    Ok(Json(json!({})))
}

async fn ensure_invite_request_is_valid(
    state: &ServerState,
    room_id: &str,
    inviter_user_id: &str,
    invitee_user_id: &str,
) -> AppResult<()> {
    if !state.rooms.room_exists(room_id.to_owned()).await? {
        return Err(AppError::with_status(
            axum::http::StatusCode::NOT_FOUND,
            ErrorKind::Unknown,
            "Room not found",
        ));
    }

    if !state.users.exists(invitee_user_id.to_owned()).await? {
        return Err(AppError::forbidden(format!(
            "Unknown user cannot be invited: {invitee_user_id}"
        )));
    }

    let inviter_membership =
        messages::get_membership(state, room_id.to_owned(), inviter_user_id.to_owned()).await?;

    if inviter_membership
        .as_ref()
        .is_none_or(|membership| membership.membership != ROOM_MEMBERSHIP_JOIN)
    {
        return Err(AppError::forbidden("You are not joined to this room"));
    }

    let invitee_membership =
        messages::get_membership(state, room_id.to_owned(), invitee_user_id.to_owned()).await?;

    if let Some(membership) = invitee_membership {
        match membership.membership.as_str() {
            "ban" => {
                return Err(AppError::forbidden(format!(
                    "{invitee_user_id} is banned from the room"
                )));
            }
            ROOM_MEMBERSHIP_JOIN => {
                return Err(AppError::forbidden(format!(
                    "{invitee_user_id} is already joined to the room"
                )));
            }
            _ => {}
        }
    }

    Ok(())
}
