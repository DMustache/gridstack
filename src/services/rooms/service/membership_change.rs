use crate::services::events::entities::{
    MatrixEventContent, MembershipContent, RoomMemberContent, SupportedRoomVersion,
};
use crate::services::events::service::EventsService;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomMembershipRepository;

pub(super) fn append_membership_change_for_target(
    room_repository: &(impl RoomMembershipRepository + ?Sized),
    events_service: &EventsService,
    room_id: String,
    room_version: SupportedRoomVersion,
    sender_user_identifier: &str,
    target_user_identifier: &str,
    membership_content: MembershipContent,
    reason: Option<String>,
) -> Result<(), RoomsApplicationError> {
    let event_write_contract = events_service
        .create_membership_event(
            room_id.clone(),
            room_version,
            sender_user_identifier.to_owned(),
            target_user_identifier.to_owned(),
            MatrixEventContent::RoomMember(RoomMemberContent {
                membership: membership_content,
                is_direct: None,
            }),
            None,
        )
        .map_err(super::map_membership_compilation_error)?;

    let _ = reason;

    room_repository
        .append_room_event(&event_write_contract)
        .map_err(|error| super::map_room_persistence_error(error, room_id))?;
    Ok(())
}
