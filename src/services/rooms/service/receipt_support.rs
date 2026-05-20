use crate::services::rooms::entities::RoomReceiptType;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomReceiptRepository;

pub(super) fn validate_receipt_thread_id(
    receipt_type: RoomReceiptType,
    thread_id: Option<&str>,
) -> Result<Option<String>, RoomsApplicationError> {
    let Some(thread_id) = thread_id else {
        return Ok(None);
    };

    let trimmed_thread_id = thread_id.trim();
    if trimmed_thread_id.is_empty() {
        return Err(RoomsApplicationError::InvalidParameter);
    }
    if matches!(receipt_type, RoomReceiptType::FullyRead) {
        return Err(RoomsApplicationError::InvalidParameter);
    }
    if trimmed_thread_id != "main" && !trimmed_thread_id.starts_with('$') {
        return Err(RoomsApplicationError::InvalidParameter);
    }

    Ok(Some(trimmed_thread_id.to_owned()))
}

pub(super) fn require_receipt_target_event(
    room_repository: &(impl RoomReceiptRepository + ?Sized),
    room_id: &str,
    event_id: &str,
    thread_id: Option<&str>,
) -> Result<(), RoomsApplicationError> {
    let trimmed_event_id = event_id.trim();
    if trimmed_event_id.is_empty() || !trimmed_event_id.starts_with('$') {
        return Err(RoomsApplicationError::InvalidParameter);
    }

    let event_exists_in_room = room_repository
        .fetch_room_timeline_event_by_id(room_id, trimmed_event_id)
        .map_err(|_| RoomsApplicationError::Internal)?
        .is_some();
    if !event_exists_in_room {
        return Err(RoomsApplicationError::InvalidParameter);
    }

    if let Some(thread_id) = thread_id {
        let event_matches_thread = room_repository
            .room_event_matches_thread(room_id, trimmed_event_id, thread_id)
            .map_err(|_| RoomsApplicationError::Internal)?;
        if !event_matches_thread {
            return Err(RoomsApplicationError::InvalidParameter);
        }
    }

    Ok(())
}
