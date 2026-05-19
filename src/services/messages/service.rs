use crate::services::{
    authorization::entities::AuthorizedUserIdentifier,
    rooms::{entities::RoomTimelineEvent, errors::RoomsApplicationError},
    state::ApplicationState,
};

pub fn get_room_event(
    application_state: &ApplicationState,
    user_id: &AuthorizedUserIdentifier,
    room_id: String,
    event_id: String,
) -> Result<RoomTimelineEvent, RoomsApplicationError> {
    application_state
        .rooms_service
        .get_room_event(user_id, room_id, event_id)
        .map_err(|error| match error {
            RoomsApplicationError::Forbidden | RoomsApplicationError::InvalidParameter => {
                RoomsApplicationError::NotFound
            }
            other => other,
        })
}

pub fn send_message_event(
    application_state: &ApplicationState,
    user_id: &AuthorizedUserIdentifier,
    room_id: String,
    event_type: String,
    transaction_id: String,
    content: serde_json::Value,
) -> Result<String, RoomsApplicationError> {
    if event_type.trim().is_empty() {
        return Err(RoomsApplicationError::InvalidParameter);
    }
    if !content.is_object() {
        return Err(RoomsApplicationError::InvalidParameter);
    }
    application_state.rooms_service.send_message_event(
        user_id,
        room_id,
        event_type,
        transaction_id,
        content,
    )
}
