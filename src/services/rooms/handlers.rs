use axum::{
    Extension, Json,
    extract::{Path, State},
};
use response_derive::IntoResponseEnum;
use tracing::{error, info};

use crate::services::{
    authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
    rooms::{
        errors::RoomsApplicationError,
        handlers::{
            create_room::{CreateRoomCommand, CreateRoomInfo, CreateRoomView},
            join_room::{JoinRoomCommand, JoinRoomInfo, JoinRoomView},
            leave_room::{LeaveRoomCommand, LeaveRoomInfo, LeaveRoomView},
        },
    },
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

pub mod create_room;
pub mod join_room;
pub mod leave_room;

#[derive(IntoResponseEnum)]
pub enum CreateRoomResponse {
    #[matrix(status = 200)]
    Ok(Json<CreateRoomView>),
    #[matrix(status = 400, error = [
        (matrix_error = "M_ROOM_IN_USE", from = RoomsApplicationError::RoomInUse),
        (matrix_error = "M_INVALID_ROOM_STATE", from = RoomsApplicationError::InvalidRoomState),
        (matrix_error = "M_UNSUPPORTED_ROOM_VERSION", from = RoomsApplicationError::UnsupportedRoomVersion),
        (matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter),
        (matrix_error = "M_INVITE_BLOCKED", from = RoomsApplicationError::InviteBlocked)
    ])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = RoomsApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum JoinRoomResponse {
    #[matrix(status = 200)]
    Ok(Json<JoinRoomView>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum LeaveRoomResponse {
    #[matrix(status = 200)]
    Ok(Json<LeaveRoomView>),
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
    let command: CreateRoomCommand = match request.try_into() {
        Ok(command) => command,
        Err(error) => {
            return CreateRoomResponse::from_mapped_error(RoomsApplicationError::from(error));
        }
    };

    match application_state
        .rooms_service
        .create_room(access_session.user_identifier(), command)
        .map(CreateRoomView::from)
    {
        Ok(view) => {
            info!(room_id = %view.room_id, "room creation completed");
            CreateRoomResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("room creation failed with internal error");
            } else {
                info!(error = %error_kind, "room creation rejected");
            }
            CreateRoomResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn join_room_by_id(
    Path(room_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<JoinRoomInfo>,
) -> JoinRoomResponse {
    let command: JoinRoomCommand = request.into();

    match application_state
        .rooms_service
        .join_room_by_id(access_session.user_identifier(), room_id, command)
        .map(JoinRoomView::from)
    {
        Ok(view) => JoinRoomResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("join room failed with internal error");
            } else {
                info!(error = %error_kind, "join room rejected");
            }
            JoinRoomResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn leave_room_by_id(
    Path(room_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<LeaveRoomInfo>,
) -> LeaveRoomResponse {
    let command: LeaveRoomCommand = request.into();

    match application_state
        .rooms_service
        .leave_room_by_id(access_session.user_identifier(), room_id, command)
        .map(LeaveRoomView::from)
    {
        Ok(view) => LeaveRoomResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("leave room failed with internal error");
            } else {
                info!(error = %error_kind, "leave room rejected");
            }
            LeaveRoomResponse::from_mapped_error(error_kind)
        }
    }
}
