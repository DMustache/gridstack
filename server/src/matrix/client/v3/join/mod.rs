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
    third_party_signed: Option<Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BodyView {
    room_id: String,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/{roomIdOrAlias}", post(post_join))
        .route_layer(from_fn_with_state(state.clone(), rate_limit))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

pub(crate) async fn join_room_by_id(
    state: &ServerState,
    user_id: &str,
    room_id: &str,
    reason: Option<String>,
) -> AppResult<String> {
    if !state.rooms.room_exists(room_id.to_owned()).await? {
        return Err(AppError::with_status(
            axum::http::StatusCode::NOT_FOUND,
            ErrorKind::Unknown,
            "Room not found",
        ));
    }

    let membership =
        messages::get_membership(state, room_id.to_owned(), user_id.to_owned()).await?;

    match membership
        .as_ref()
        .map(|membership| membership.membership.as_str())
    {
        Some(ROOM_MEMBERSHIP_JOIN) => return Ok(room_id.to_owned()),
        Some("ban") => return Err(AppError::forbidden("You are banned from this room")),
        Some("invite") => {}
        Some("leave") | None => ensure_public_room_or_invited(state, room_id).await?,
        Some(_) => ensure_public_room_or_invited(state, room_id).await?,
    }

    let mut content = json!({ "membership": ROOM_MEMBERSHIP_JOIN });
    if let Some(reason) = reason
        && let Some(object) = content.as_object_mut()
    {
        object.insert(String::from("reason"), json!(reason));
    }

    messages::append_room_event(
        state,
        AppendRoomEventCommand {
            room_id: room_id.to_owned(),
            sender_user_id: user_id.to_owned(),
            event_type: String::from("m.room.member"),
            state_key: Some(user_id.to_owned()),
            content,
            unsigned: None,
            transaction_id: None,
        },
    )
    .await?;

    Ok(room_id.to_owned())
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/join/{roomIdOrAlias}",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomIdOrAlias" = String, Path, description = "Room ID or alias to join")
    ),
    request_body = crate::docs::JoinRequestDoc,
    responses(
        (status = 200, description = "Joined room", body = crate::docs::JoinResponseDoc),
        (status = 400, description = "Unsupported join payload", body = crate::docs::MatrixErrorResponse),
        (status = 403, description = "Join forbidden", body = crate::docs::MatrixErrorResponse),
        (status = 429, description = "Rate limited", body = crate::docs::RateLimitErrorResponse)
    )
)]
pub(crate) async fn post_join(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path(room_id_or_alias): Path<String>,
    Json(body): Json<BodyInfo>,
) -> AppResult<Json<BodyView>> {
    if body.third_party_signed.is_some() {
        return Err(AppError::bad_request(
            ErrorKind::Unknown,
            "third_party_signed is not supported yet",
        ));
    }

    if !room_id_or_alias.starts_with('!') {
        return Err(AppError::bad_request(
            ErrorKind::Unknown,
            "Room aliases are not supported yet",
        ));
    }

    let room_id = join_room_by_id(&state, &user.user_id, &room_id_or_alias, body.reason).await?;

    Ok(Json(BodyView { room_id }))
}

async fn ensure_public_room_or_invited(state: &ServerState, room_id: &str) -> AppResult<()> {
    let join_rules = messages::get_state_event(
        state,
        room_id.to_owned(),
        String::from("m.room.join_rules"),
        String::new(),
    )
    .await?;

    let is_public = join_rules
        .as_ref()
        .and_then(|event| event.content.get("join_rule"))
        .and_then(Value::as_str)
        .is_some_and(|join_rule| join_rule == "public");

    if is_public {
        return Ok(());
    }

    Err(AppError::forbidden("You are not invited to this room."))
}
