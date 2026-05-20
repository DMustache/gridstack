use crate::services::authorization::entities::ExistingUserIdentifier;
use crate::services::events::entities::SupportedRoomVersion;
use crate::services::events::service::EventsService;
use crate::services::rooms::entities::RoomIdentifier;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::{RoomJoinContext, RoomMembershipRepository};

pub(super) fn require_room_identifier(room_id: &str) -> Result<(), RoomsApplicationError> {
    if RoomIdentifier::parse(room_id.to_owned()).is_none() {
        return Err(RoomsApplicationError::Forbidden);
    }
    Ok(())
}

pub(super) fn require_room_membership_context(
    room_repository: &(impl RoomMembershipRepository + ?Sized),
    room_id: &str,
    user_id: &ExistingUserIdentifier,
) -> Result<RoomJoinContext, RoomsApplicationError> {
    room_repository
        .fetch_join_context(room_id, user_id.as_str())
        .map_err(|_| RoomsApplicationError::Internal)?
        .ok_or(RoomsApplicationError::Forbidden)
}

pub(super) fn require_joined_room_version(
    room_repository: &(impl RoomMembershipRepository + ?Sized),
    events_service: &EventsService,
    room_id: &str,
    user_id: &ExistingUserIdentifier,
) -> Result<SupportedRoomVersion, RoomsApplicationError> {
    let room_join_context = require_joined_membership(room_repository, room_id, user_id)?;

    room_join_context
        .room_version
        .unwrap_or_else(|| events_service.default_room_version().to_string())
        .parse::<SupportedRoomVersion>()
        .map_err(|_| RoomsApplicationError::Internal)
}

pub(super) fn require_joined_membership(
    room_repository: &(impl RoomMembershipRepository + ?Sized),
    room_id: &str,
    user_id: &ExistingUserIdentifier,
) -> Result<RoomJoinContext, RoomsApplicationError> {
    require_room_identifier(room_id)?;
    let room_join_context = require_room_membership_context(room_repository, room_id, user_id)?;

    if room_join_context.membership_state.as_deref() != Some("join") {
        return Err(RoomsApplicationError::Forbidden);
    }

    Ok(room_join_context)
}

pub(super) fn require_room_state_read_access(
    room_repository: &(impl RoomMembershipRepository + ?Sized),
    room_id: &str,
    user_id: &ExistingUserIdentifier,
) -> Result<(), RoomsApplicationError> {
    require_room_identifier(room_id)?;
    let room_join_context = require_room_membership_context(room_repository, room_id, user_id)?;
    let Some(membership_state) = room_join_context.membership_state.as_deref() else {
        return Err(RoomsApplicationError::Forbidden);
    };
    if !matches!(membership_state, "join" | "leave") {
        return Err(RoomsApplicationError::Forbidden);
    }
    Ok(())
}
