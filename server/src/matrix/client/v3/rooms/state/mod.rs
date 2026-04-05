use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    middleware::from_fn_with_state,
    routing::get,
};
use models::error::{AppError, AppResult};
use persistence::rooms::RoomEvent;
use ruma::{
    api::client::error::ErrorKind,
    exports::serde_json::{Map, Value, json},
};

use crate::{
    services::{
        authorization::layers::{
            AuthenticatedUser, AuthenticatedUserExtension, require_authenticated_user,
        },
        messages,
    },
    state::ServerState,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct QueryParameters {
    format: Option<String>,
}

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", get(get_room_state))
        .route(
            "/{eventType}",
            get(get_room_state_event_with_empty_state_key),
        )
        .route("/{eventType}/{*stateKey}", get(get_room_state_event))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/rooms/{roomId}/state",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier")
    ),
    responses(
        (status = 200, description = "Current room state", body = Vec<crate::docs::ClientEventDoc>),
        (status = 403, description = "No access to room state", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn get_room_state(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path(room_id): Path<String>,
) -> AppResult<Json<Vec<Value>>> {
    let membership = ensure_can_access_room_state(&state, &room_id, &user.user_id).await?;
    let events = messages::get_current_state(&state, room_id).await?;

    Ok(Json(
        events
            .iter()
            .map(|event| room_event_to_client_event(event, &user, &membership))
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/rooms/{roomId}/state/{eventType}",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier"),
        ("eventType" = String, Path, description = "State event type"),
        ("format" = Option<String>, Query, description = "content or event")
    ),
    responses(
        (status = 200, description = "State event content or full event", body = std::collections::BTreeMap<String, Value>),
        (status = 403, description = "No access to room state", body = crate::docs::MatrixErrorResponse),
        (status = 404, description = "State event not found", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn get_room_state_event_with_empty_state_key(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path((room_id, event_type)): Path<(String, String)>,
    Query(query): Query<QueryParameters>,
) -> AppResult<Json<Value>> {
    get_room_state_event_inner(state, user, room_id, event_type, String::new(), query).await
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/rooms/{roomId}/state/{eventType}/{stateKey}",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("roomId" = String, Path, description = "Room identifier"),
        ("eventType" = String, Path, description = "State event type"),
        ("stateKey" = String, Path, description = "State key"),
        ("format" = Option<String>, Query, description = "content or event")
    ),
    responses(
        (status = 200, description = "State event content or full event", body = std::collections::BTreeMap<String, Value>),
        (status = 403, description = "No access to room state", body = crate::docs::MatrixErrorResponse),
        (status = 404, description = "State event not found", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn get_room_state_event(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Path((room_id, event_type, state_key)): Path<(String, String, String)>,
    Query(query): Query<QueryParameters>,
) -> AppResult<Json<Value>> {
    get_room_state_event_inner(state, user, room_id, event_type, state_key, query).await
}

async fn get_room_state_event_inner(
    state: ServerState,
    user: AuthenticatedUser,
    room_id: String,
    event_type: String,
    state_key: String,
    query: QueryParameters,
) -> AppResult<Json<Value>> {
    let membership = ensure_can_access_room_state(&state, &room_id, &user.user_id).await?;
    let event = messages::get_state_event(&state, room_id, event_type, state_key)
        .await?
        .ok_or_else(|| {
            AppError::with_status(
                axum::http::StatusCode::NOT_FOUND,
                ErrorKind::Unknown,
                "State event not found",
            )
        })?;

    match query.format.as_deref() {
        Some("event") => Ok(Json(room_event_to_client_event(&event, &user, &membership))),
        None | Some("content") => Ok(Json(event.content)),
        Some(_) => Err(AppError::bad_request(
            ErrorKind::InvalidParam,
            "format must be either 'content' or 'event'",
        )),
    }
}

async fn ensure_can_access_room_state(
    state: &ServerState,
    room_id: &str,
    user_id: &str,
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
    let Some(membership) = membership else {
        return Err(AppError::forbidden(
            "You are not a member of this room and were not previously a member",
        ));
    };

    match membership.membership.as_str() {
        "join" | "leave" | "ban" => Ok(membership.membership),
        _ => Err(AppError::forbidden(
            "You are not a member of this room and were not previously a member",
        )),
    }
}

fn room_event_to_client_event(
    event: &RoomEvent,
    user: &AuthenticatedUser,
    membership: &str,
) -> Value {
    let age = (chrono::Utc::now() - event.origin_server_ts)
        .num_milliseconds()
        .max(0);

    let mut unsigned = event.unsigned.clone().unwrap_or_else(|| json!({}));
    if let Some(unsigned_object) = unsigned.as_object_mut() {
        unsigned_object.insert(String::from("age"), json!(age));
        unsigned_object.insert(String::from("membership"), json!(membership));

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
