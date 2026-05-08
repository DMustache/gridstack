use axum::{
    Extension, Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value};

use crate::services::{
    authorization::entities::UserId, state::ApplicationState,
};

pub async fn send_room_message(
    State(application_state): State<ApplicationState>,
    Extension(user_id): Extension<UserId>,
    Path((room_identifier, event_type, _transaction_identifier)): Path<(String, String, String)>,
    Json(content): Json<Value>,
) -> Response {
    let object_content = match content {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    match application_state
        .messaging_events_service
        .send_room_message(user_id, &room_identifier, event_type, object_content)
    {
        Ok(response) => Json(response).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}

pub async fn get_room_messages(
    State(application_state): State<ApplicationState>,
    Extension(_user_id): Extension<UserId>,
    Path(room_identifier): Path<String>,
) -> Response {
    match application_state
        .messaging_events_service
        .get_room_messages(&room_identifier)
    {
        Ok(response) => Json(response).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}

pub async fn sync_events(
    State(application_state): State<ApplicationState>,
    Extension(_user_id): Extension<UserId>,
) -> Response {
    Json(application_state.messaging_events_service.sync()).into_response()
}

pub async fn get_events(
    State(application_state): State<ApplicationState>,
    Extension(_user_id): Extension<UserId>,
) -> Response {
    Json(application_state.messaging_events_service.get_events()).into_response()
}

pub async fn get_event_by_identifier(
    State(application_state): State<ApplicationState>,
    Extension(_user_id): Extension<UserId>,
    Path(event_identifier): Path<String>,
) -> Response {
    match application_state
        .messaging_events_service
        .get_event_by_identifier(&event_identifier)
    {
        Ok(response) => Json(response).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}
