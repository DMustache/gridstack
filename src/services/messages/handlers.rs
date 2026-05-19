use axum::{
    Extension, Json,
    extract::{Path, State},
};
use response_derive::IntoResponseEnum;
use tracing::{error, info};

use crate::services::{
    authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
    messages::{entities::SendMessageEventView, service},
    rooms::entities::RoomTimelineEventView,
    rooms::errors::RoomsApplicationError,
    shared::MatrixErrorResponse,
    state::ApplicationState,
};

#[derive(IntoResponseEnum)]
pub enum SendMessageEventResponse {
    #[matrix(status = 200)]
    Ok(Json<SendMessageEventView>),
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
    Ok(Json<RoomTimelineEventView>),
    #[matrix(status = 404, error = [(matrix_error = "M_NOT_FOUND", from = RoomsApplicationError::NotFound)])]
    NotFound(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = RoomsApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

pub async fn get_room_event(
    Path((room_id, event_id)): Path<(String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetRoomEventResponse {
    match service::get_room_event(
        &application_state,
        access_session.user_identifier(),
        room_id,
        event_id,
    )
    .map(RoomTimelineEventView::from)
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

pub async fn send_message_event(
    Path((room_id, event_type, transaction_id)): Path<(String, String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(content): Json<serde_json::Value>,
) -> SendMessageEventResponse {
    match service::send_message_event(
        &application_state,
        access_session.user_identifier(),
        room_id,
        event_type,
        transaction_id,
        content,
    )
    .map(|event_id| SendMessageEventView { event_id })
    {
        Ok(view) => SendMessageEventResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, RoomsApplicationError::Internal) {
                error!("send message event failed with internal error");
            } else {
                info!(error = %error_kind, "send message event rejected");
            }
            SendMessageEventResponse::from_mapped_error(error_kind)
        }
    }
}
