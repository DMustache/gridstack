use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    middleware::from_fn_with_state,
    routing::get,
};
use models::error::{AppError, AppResult};
use persistence::rooms::{PaginationDirection, RoomEvent};
use ruma::{
    api::client::error::ErrorKind,
    exports::serde_json::{Map, Value, json},
};

use crate::{
    services::{
        authorization::layers::{
            AuthenticatedUser, AuthenticatedUserExtension, require_authenticated_user,
        },
        messages::{self, ROOM_MEMBERSHIP_JOIN},
    },
    state::ServerState,
};

mod data_transfer_objects;
use data_transfer_objects::{BodyView, QueryParameters};

const DEFAULT_LIMIT: i64 = 10;
const MAX_LIMIT: i64 = 100;

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", get(get_messages))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/rooms/{roomId}/messages",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier"),
        ("dir" = String, Query, description = "Pagination direction: b or f"),
        ("filter" = Option<String>, Query, description = "Room event filter"),
        ("from" = Option<String>, Query, description = "Pagination token to start from"),
        ("limit" = Option<i64>, Query, description = "Maximum number of events"),
        ("to" = Option<String>, Query, description = "Pagination token to stop at")
    ),
    responses(
        (status = 200, description = "Paginated room messages", body = crate::docs::RoomMessagesResponseDoc),
        (status = 403, description = "User is not joined to the room", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn get_messages(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path(room_id): Path<String>,
    Query(query): Query<QueryParameters>,
) -> AppResult<Json<BodyView>> {
    ensure_messages_request_is_valid(&state, &room_id, &user.user_id).await?;

    let direction = parse_direction(&query.dir)?;
    let from = resolve_from_token(&state, &room_id, direction, query.from.as_deref()).await?;
    let to = parse_token(query.to.as_deref())?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let _filter = query.filter;

    let events = state
        .rooms
        .list_events_paginated(room_id.clone(), Some(from), to, direction, limit + 1)
        .await?;

    let has_more = events.len() as i64 > limit;
    let chunk_events = events.into_iter().take(limit as usize).collect::<Vec<_>>();
    let end = build_end_token(&chunk_events, direction, has_more);

    Ok(Json(BodyView {
        chunk: chunk_events
            .iter()
            .map(|event| room_event_to_client_event(event, &user))
            .collect(),
        start: format!("s{from}"),
        end,
        state: Vec::new(),
    }))
}

async fn ensure_messages_request_is_valid(
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
        return Err(AppError::forbidden("You are not a member of this room"));
    }

    Ok(())
}

fn parse_direction(direction: &str) -> AppResult<PaginationDirection> {
    match direction {
        "f" => Ok(PaginationDirection::Forward),
        "b" => Ok(PaginationDirection::Backward),
        _ => Err(AppError::bad_request(
            ErrorKind::InvalidParam,
            "dir must be either 'b' or 'f'",
        )),
    }
}

async fn resolve_from_token(
    state: &ServerState,
    room_id: &str,
    direction: PaginationDirection,
    token: Option<&str>,
) -> AppResult<i64> {
    if let Some(token) = token {
        return parse_token(Some(token))?
            .ok_or_else(|| AppError::bad_request(ErrorKind::InvalidParam, "Invalid from token"));
    }

    match direction {
        PaginationDirection::Forward => Ok(0),
        PaginationDirection::Backward => Ok(state
            .rooms
            .get_latest_stream_ordering_for_room(room_id.to_owned())
            .await?
            .unwrap_or_default()
            + 1),
    }
}

fn parse_token(token: Option<&str>) -> AppResult<Option<i64>> {
    let Some(token) = token else {
        return Ok(None);
    };

    token
        .strip_prefix('s')
        .and_then(|value| value.parse::<i64>().ok())
        .map(Some)
        .ok_or_else(|| AppError::bad_request(ErrorKind::InvalidParam, "Invalid pagination token"))
}

fn build_end_token(
    events: &[RoomEvent],
    _direction: PaginationDirection,
    has_more: bool,
) -> Option<String> {
    if !has_more {
        return None;
    }

    events
        .last()
        .map(|event| format!("s{}", event.stream_ordering))
}

fn room_event_to_client_event(event: &RoomEvent, user: &AuthenticatedUser) -> Value {
    let age = (chrono::Utc::now() - event.origin_server_ts)
        .num_milliseconds()
        .max(0);

    let mut unsigned = event.unsigned.clone().unwrap_or_else(|| json!({}));
    if let Some(unsigned_object) = unsigned.as_object_mut() {
        unsigned_object.insert(String::from("age"), json!(age));
        unsigned_object.insert(String::from("membership"), json!(ROOM_MEMBERSHIP_JOIN));

        if event.sender_user_id == user.user_id
            && let Some(transaction_id) = &event.transaction_id
        {
            unsigned_object.insert(String::from("transaction_id"), json!(transaction_id));
        }
    }

    let mut object = Map::new();
    object.insert(String::from("content"), event.content.clone());
    object.insert(String::from("event_id"), json!(event.event_id));
    object.insert(
        String::from("origin_server_ts"),
        json!(event.origin_server_ts.timestamp_millis()),
    );
    object.insert(String::from("room_id"), json!(event.room_id));
    object.insert(String::from("sender"), json!(event.sender_user_id));
    object.insert(String::from("type"), json!(event.event_type));
    object.insert(String::from("unsigned"), unsigned);

    if let Some(state_key) = &event.state_key {
        object.insert(String::from("state_key"), json!(state_key));
    }

    Value::Object(object)
}
