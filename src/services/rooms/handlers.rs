use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use response_derive::IntoResponseEnum;
use tracing::{error, info};

use crate::services::{
    authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
    rooms::{
        errors::RoomsApplicationError,
        handlers::{
            create_room::{CreateRoomCommand, CreateRoomInfo, CreateRoomView},
            get_room_messages::{GetRoomMessagesQuery, GetRoomMessagesView},
            get_room_state::RoomStateEventView,
            get_room_state_with_key::{GetRoomStateWithKeyFormatQuery, room_state_event_to_value},
            join_room::{JoinRoomCommand, JoinRoomInfo, JoinRoomView},
            leave_room::{LeaveRoomCommand, LeaveRoomInfo, LeaveRoomView},
            set_room_state_with_key::SetRoomStateWithKeyView,
        },
    },
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

pub mod create_room;
pub mod get_room_messages;
pub mod get_room_state;
pub mod get_room_state_with_key;
pub mod join_room;
pub mod leave_room;
pub mod set_room_state_with_key;

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

#[derive(IntoResponseEnum)]
pub enum GetRoomStateResponse {
    #[matrix(status = 200)]
    Ok(Json<Vec<RoomStateEventView>>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum GetRoomStateWithKeyResponse {
    #[matrix(status = 200)]
    Ok(Json<serde_json::Value>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 404, error = [(matrix_error = "M_NOT_FOUND", from = RoomsApplicationError::NotFound)])]
    NotFound(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum GetRoomMessagesResponse {
    #[matrix(status = 200)]
    Ok(Json<GetRoomMessagesView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum SetRoomStateWithKeyResponse {
    #[matrix(status = 200)]
    Ok(Json<SetRoomStateWithKeyView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter), (matrix_error = "M_BAD_JSON", from = RoomsApplicationError::InvalidRoomState), (matrix_error = "M_BAD_ALIAS", from = RoomsApplicationError::BadAlias)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden), (matrix_error = "M_INVITE_BLOCKED", from = RoomsApplicationError::InviteBlocked)])]
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

pub async fn get_room_state(
    Path(room_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomStateResponse {
    match application_state
        .rooms_service
        .get_room_state(access_session.user_identifier(), room_id)
        .map(|events| {
            events
                .into_iter()
                .map(RoomStateEventView::from)
                .collect::<Vec<_>>()
        }) {
        Ok(view) => GetRoomStateResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get room state failed with internal error");
            } else {
                info!(error = %error_kind, "get room state rejected");
            }
            GetRoomStateResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn get_room_state_with_key(
    Path((room_id, event_type, state_key)): Path<(String, String, String)>,
    Query(format_query): Query<GetRoomStateWithKeyFormatQuery>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomStateWithKeyResponse {
    let include_full_event = match format_query.request_full_event() {
        Ok(value) => value,
        Err(error_kind) => return GetRoomStateWithKeyResponse::from_mapped_error(error_kind),
    };

    match application_state.rooms_service.get_room_state_with_key(
        access_session.user_identifier(),
        room_id,
        event_type,
        state_key,
    ) {
        Ok(event) => {
            if include_full_event {
                GetRoomStateWithKeyResponse::Ok(Json(room_state_event_to_value(
                    RoomStateEventView::from(event),
                )))
            } else {
                GetRoomStateWithKeyResponse::Ok(Json(event.content))
            }
        }
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get room state with key failed with internal error");
            } else {
                info!(error = %error_kind, "get room state with key rejected");
            }
            GetRoomStateWithKeyResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn get_room_state_with_empty_key(
    Path((room_id, event_type)): Path<(String, String)>,
    Query(format_query): Query<GetRoomStateWithKeyFormatQuery>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomStateWithKeyResponse {
    let include_full_event = match format_query.request_full_event() {
        Ok(value) => value,
        Err(error_kind) => return GetRoomStateWithKeyResponse::from_mapped_error(error_kind),
    };

    match application_state.rooms_service.get_room_state_with_key(
        access_session.user_identifier(),
        room_id,
        event_type,
        String::new(),
    ) {
        Ok(event) => {
            if include_full_event {
                GetRoomStateWithKeyResponse::Ok(Json(room_state_event_to_value(
                    RoomStateEventView::from(event),
                )))
            } else {
                GetRoomStateWithKeyResponse::Ok(Json(event.content))
            }
        }
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get room state with empty key failed with internal error");
            } else {
                info!(error = %error_kind, "get room state with empty key rejected");
            }
            GetRoomStateWithKeyResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn get_room_messages(
    Path(room_id): Path<String>,
    Query(query): Query<GetRoomMessagesQuery>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomMessagesResponse {
    let command = match query.try_into() {
        Ok(command) => command,
        Err(error_kind) => return GetRoomMessagesResponse::from_mapped_error(error_kind),
    };

    match application_state
        .rooms_service
        .get_room_messages(access_session.user_identifier(), room_id, command)
        .map(GetRoomMessagesView::from)
    {
        Ok(view) => GetRoomMessagesResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get room messages failed with internal error");
            } else {
                info!(error = %error_kind, "get room messages rejected");
            }
            GetRoomMessagesResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn set_room_state_with_key(
    Path((room_id, event_type, state_key)): Path<(String, String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(content): Json<serde_json::Value>,
) -> SetRoomStateWithKeyResponse {
    match application_state
        .rooms_service
        .set_room_state_with_key(
            access_session.user_identifier(),
            room_id,
            event_type,
            state_key,
            content,
        )
        .map(|event_id| SetRoomStateWithKeyView { event_id })
    {
        Ok(view) => SetRoomStateWithKeyResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("set room state with key failed with internal error");
            } else {
                info!(error = %error_kind, "set room state with key rejected");
            }
            SetRoomStateWithKeyResponse::from_mapped_error(error_kind)
        }
    }
}
