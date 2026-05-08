use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::Value;

use crate::services::{
    authorization::entities::UserId,
    rooms::view_models::{
        CreateRoomRequestViewModel, CreateRoomResponseViewModel, JoinRoomResponseViewModel,
        PublicRoomsResponseViewModel,
    },
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

pub enum CreateRoomResponse {
    Ok(Json<CreateRoomResponseViewModel>),
    Unauthorized(Json<MatrixErrorResponse>),
    TooManyRequests(Json<MatrixErrorResponse>),
    Error(Response),
}

impl IntoResponse for CreateRoomResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Ok(payload) => payload.into_response(),
            Self::Unauthorized(payload) => (StatusCode::UNAUTHORIZED, payload).into_response(),
            Self::TooManyRequests(payload) => {
                (StatusCode::TOO_MANY_REQUESTS, payload).into_response()
            }
            Self::Error(response) => response,
        }
    }
}

pub enum JoinRoomResponse {
    Ok(Json<JoinRoomResponseViewModel>),
    Unauthorized(Json<MatrixErrorResponse>),
    TooManyRequests(Json<MatrixErrorResponse>),
    Error(Response),
}

impl IntoResponse for JoinRoomResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Ok(payload) => payload.into_response(),
            Self::Unauthorized(payload) => (StatusCode::UNAUTHORIZED, payload).into_response(),
            Self::TooManyRequests(payload) => {
                (StatusCode::TOO_MANY_REQUESTS, payload).into_response()
            }
            Self::Error(response) => response,
        }
    }
}

pub async fn create_room(
    State(application_state): State<ApplicationState>,
    Extension(user_id): Extension<UserId>,
    Json(create_room_request_view_model): Json<CreateRoomRequestViewModel>,
) -> CreateRoomResponse {
    match application_state
        .rooms_service
        .create_room(user_id, create_room_request_view_model)
    {
        Ok(response) => CreateRoomResponse::Ok(Json(response)),
        Err(application_error) => {
            CreateRoomResponse::Error(application_error.into_response())
        }
    }
}

pub async fn join_room(
    State(application_state): State<ApplicationState>,
    Extension(user_id): Extension<UserId>,
    Path(room_identifier_or_alias): Path<String>,
) -> JoinRoomResponse {
    match application_state
        .rooms_service
        .join_room(user_id, &room_identifier_or_alias)
    {
        Ok(response) => JoinRoomResponse::Ok(Json(response)),
        Err(application_error) => JoinRoomResponse::Error(application_error.into_response()),
    }
}

pub async fn get_public_rooms(
    State(application_state): State<ApplicationState>,
) -> Json<PublicRoomsResponseViewModel> {
    Json(application_state.rooms_service.get_public_rooms())
}

pub async fn post_public_rooms(
    State(application_state): State<ApplicationState>,
) -> Json<PublicRoomsResponseViewModel> {
    Json(application_state.rooms_service.get_public_rooms())
}

pub async fn get_room_state(
    State(application_state): State<ApplicationState>,
    Extension(_user_id): Extension<UserId>,
    Path(room_identifier): Path<String>,
) -> Response {
    match application_state
        .rooms_service
        .get_room_state(&room_identifier)
    {
        Ok(response) => Json(response).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}

pub async fn put_room_state(
    State(_application_state): State<ApplicationState>,
    Extension(_user_id): Extension<UserId>,
    _path: Path<(String, String, String)>,
    Json(_content): Json<Value>,
) -> Response {
    crate::services::errors::ApplicationError::NotImplemented(
        "room state persistence was moved behind domain persistence service".to_owned(),
    )
    .into_response()
}
