use axum::{Extension, Json, extract::State};
use response_derive::IntoResponseEnum;

use crate::services::{
    authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
    rooms::{
        errors::RoomsApplicationError,
        handlers::create_room::{CreateRoomInfo, CreateRoomView},
    },
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

pub mod create_room;

#[derive(IntoResponseEnum)]
pub enum CreateRoomResponse {
    #[matrix(status = 200)]
    Ok(Json<CreateRoomView>),
    #[matrix(status = 400, error = [
        (matrix_error = "M_ROOM_IN_USE", from = RoomsApplicationError::RoomInUse),
        (matrix_error = "M_INVALID_ROOM_STATE", from = RoomsApplicationError::InvalidRoomState),
        (matrix_error = "M_UNSUPPORTED_ROOM_VERSION", from = RoomsApplicationError::UnsupportedRoomVersion),
        (matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)
    ])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = RoomsApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

pub async fn create_room(
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<CreateRoomInfo>,
) -> CreateRoomResponse {
    CreateRoomResponse::from_result(
        application_state
            .rooms_service
            .create_room(access_session.user_identifier(), request)
            .map(CreateRoomView::from),
    )
}
