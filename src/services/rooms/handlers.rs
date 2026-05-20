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
            get_public_rooms::{GetPublicRoomsQuery, GetPublicRoomsView},
            get_room_event::GetRoomEventView,
            get_room_members::{GetRoomMembersQuery, GetRoomMembersView},
            get_room_messages::{GetRoomMessagesQuery, GetRoomMessagesView},
            get_room_state::RoomStateEventView,
            get_room_state_with_key::{GetRoomStateWithKeyFormatQuery, room_state_event_to_value},
            invite_user::{InviteUserCommand, InviteUserInfo, InviteUserView},
            join_room::{JoinRoomCommand, JoinRoomInfo, JoinRoomView},
            joined_members::JoinedMembersView,
            joined_rooms::JoinedRoomsView,
            leave_room::{LeaveRoomCommand, LeaveRoomInfo, LeaveRoomView},
            send_receipt::{SendReceiptInfo, SendReceiptView},
            send_room_event::SendRoomEventView,
            set_read_markers::{SetReadMarkersInfo, SetReadMarkersView},
            set_room_state_with_key::SetRoomStateWithKeyView,
        },
    },
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

pub mod create_room;
pub mod get_public_rooms;
pub mod get_room_event;
pub mod get_room_members;
pub mod get_room_messages;
pub mod get_room_state;
pub mod get_room_state_with_key;
pub mod invite_user;
pub mod join_room;
pub mod joined_members;
pub mod joined_rooms;
pub mod leave_room;
pub mod send_receipt;
pub mod send_room_event;
pub mod set_read_markers;
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
pub enum GetJoinedRoomsResponse {
    #[matrix(status = 200)]
    Ok(Json<JoinedRoomsView>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum GetPublicRoomsResponse {
    #[matrix(status = 200)]
    Ok(Json<GetPublicRoomsView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum GetJoinedMembersResponse {
    #[matrix(status = 200)]
    Ok(Json<JoinedMembersView>),
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
pub enum InviteUserResponse {
    #[matrix(status = 200)]
    Ok(Json<InviteUserView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden), (matrix_error = "M_INVITE_BLOCKED", from = RoomsApplicationError::InviteBlocked)])]
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
pub enum GetRoomMembersResponse {
    #[matrix(status = 200)]
    Ok(Json<GetRoomMembersView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
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
pub enum GetRoomEventResponse {
    #[matrix(status = 200)]
    Ok(Json<GetRoomEventView>),
    #[matrix(status = 404, error = [(matrix_error = "M_NOT_FOUND", from = RoomsApplicationError::Forbidden), (matrix_error = "M_NOT_FOUND", from = RoomsApplicationError::NotFound)])]
    NotFound(Json<MatrixErrorResponse>),
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

#[derive(IntoResponseEnum)]
pub enum SendRoomEventResponse {
    #[matrix(status = 200)]
    Ok(Json<SendRoomEventView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter), (matrix_error = "M_BAD_JSON", from = RoomsApplicationError::InvalidRoomState), (matrix_error = "M_BAD_ALIAS", from = RoomsApplicationError::BadAlias)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden), (matrix_error = "M_INVITE_BLOCKED", from = RoomsApplicationError::InviteBlocked)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum SendReceiptResponse {
    #[matrix(status = 200)]
    Ok(Json<SendReceiptView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = RoomsApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum SetReadMarkersResponse {
    #[matrix(status = 200)]
    Ok(Json<SetReadMarkersView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = RoomsApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
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

pub async fn get_joined_rooms(
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetJoinedRoomsResponse {
    match application_state
        .rooms_service
        .get_joined_rooms(access_session.user_identifier())
        .map(JoinedRoomsView::from)
    {
        Ok(view) => GetJoinedRoomsResponse::Ok(Json(view)),
        Err(error_kind) => {
            error!(error = %error_kind, "get joined rooms failed");
            GetJoinedRoomsResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn get_public_rooms(
    Query(query): Query<GetPublicRoomsQuery>,
    State(application_state): State<ApplicationState>,
) -> GetPublicRoomsResponse {
    let command = match query.try_into() {
        Ok(command) => command,
        Err(error_kind) => return GetPublicRoomsResponse::from_mapped_error(error_kind),
    };

    match application_state
        .rooms_service
        .get_public_rooms(command)
        .map(GetPublicRoomsView::from)
    {
        Ok(view) => GetPublicRoomsResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get public rooms failed with internal error");
            } else {
                info!(error = %error_kind, "get public rooms rejected");
            }
            GetPublicRoomsResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn get_joined_members(
    Path(room_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetJoinedMembersResponse {
    match application_state
        .rooms_service
        .get_joined_members(&access_session, room_id)
        .map(JoinedMembersView::from)
    {
        Ok(view) => GetJoinedMembersResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get joined members failed with internal error");
            } else {
                info!(error = %error_kind, "get joined members rejected");
            }
            GetJoinedMembersResponse::from_mapped_error(error_kind)
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

pub async fn invite_user_to_room(
    Path(room_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<InviteUserInfo>,
) -> InviteUserResponse {
    let command: InviteUserCommand = match request.try_into() {
        Ok(command) => command,
        Err(error_kind) => return InviteUserResponse::from_mapped_error(error_kind),
    };

    match application_state
        .rooms_service
        .invite_user_to_room(access_session.user_identifier(), room_id, command)
        .map(InviteUserView::from)
    {
        Ok(view) => InviteUserResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("invite user failed with internal error");
            } else {
                info!(error = %error_kind, "invite user rejected");
            }
            InviteUserResponse::from_mapped_error(error_kind)
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

pub async fn get_room_members(
    Path(room_id): Path<String>,
    Query(query): Query<GetRoomMembersQuery>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomMembersResponse {
    let command = match query.try_into() {
        Ok(command) => command,
        Err(error_kind) => return GetRoomMembersResponse::from_mapped_error(error_kind),
    };

    match application_state
        .rooms_service
        .get_room_members(access_session.user_identifier(), room_id, command)
        .map(GetRoomMembersView::from)
    {
        Ok(view) => GetRoomMembersResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get room members failed with internal error");
            } else {
                info!(error = %error_kind, "get room members rejected");
            }
            GetRoomMembersResponse::from_mapped_error(error_kind)
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

pub async fn get_room_event(
    Path((room_id, event_id)): Path<(String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomEventResponse {
    match application_state
        .rooms_service
        .get_room_event(access_session.user_identifier(), room_id, event_id)
        .map(GetRoomEventView::from)
    {
        Ok(view) => GetRoomEventResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("get room event failed with internal error");
            } else {
                info!(error = %error_kind, "get room event rejected");
            }
            GetRoomEventResponse::from_mapped_error(error_kind)
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

pub async fn send_room_message_event(
    Path((room_id, event_type, transaction_id)): Path<(String, String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(content): Json<serde_json::Value>,
) -> SendRoomEventResponse {
    let sender = access_session
        .user_identifier()
        .as_existing_user_identifier()
        .as_str()
        .to_owned();
    match application_state
        .rooms_service
        .send_room_message_event(
            access_session.user_identifier(),
            room_id.clone(),
            event_type.clone(),
            transaction_id.clone(),
            content,
        )
        .map(|event_id| SendRoomEventView { event_id })
    {
        Ok(view) => SendRoomEventResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!(
                    room_id = %room_id,
                    event_type = %event_type,
                    transaction_id = %transaction_id,
                    sender = %sender,
                    "send room event failed with internal error"
                );
            } else {
                info!(
                    room_id = %room_id,
                    event_type = %event_type,
                    transaction_id = %transaction_id,
                    sender = %sender,
                    error = %error_kind,
                    "send room event rejected"
                );
            }
            SendRoomEventResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn send_room_receipt(
    Path((room_id, receipt_type, event_id)): Path<(String, String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<SendReceiptInfo>,
) -> SendReceiptResponse {
    let command = match (receipt_type.as_str(), event_id, request).try_into() {
        Ok(command) => command,
        Err(error_kind) => return SendReceiptResponse::from_mapped_error(error_kind),
    };

    match application_state
        .rooms_service
        .send_room_receipt(access_session.user_identifier(), room_id, command)
        .map(|()| SendReceiptView::default())
    {
        Ok(view) => SendReceiptResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("send receipt failed with internal error");
            } else {
                info!(error = %error_kind, "send receipt rejected");
            }
            SendReceiptResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn set_room_read_markers(
    Path(room_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<SetReadMarkersInfo>,
) -> SetReadMarkersResponse {
    let command = request.into();

    match application_state
        .rooms_service
        .set_read_markers(access_session.user_identifier(), room_id, command)
        .map(|()| SetReadMarkersView::default())
    {
        Ok(view) => SetReadMarkersResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("set read markers failed with internal error");
            } else {
                info!(error = %error_kind, "set read markers rejected");
            }
            SetReadMarkersResponse::from_mapped_error(error_kind)
        }
    }
}
