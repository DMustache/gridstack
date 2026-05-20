use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::events::entities::{MembershipContent, SupportedRoomVersion};
use crate::services::events::service::EventsService;
use crate::services::rooms::entities::{LeaveRoomCommand, LeftRoom};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomMembershipRepository;

use super::membership_change::append_membership_change_for_target;
use super::room_access::{require_room_identifier, require_room_membership_context};

pub struct LeaveRoomUseCase<'a, Repository>
where
    Repository: RoomMembershipRepository + ?Sized,
{
    room_repository: &'a Repository,
    events_service: &'a EventsService,
}

impl<'a, Repository> LeaveRoomUseCase<'a, Repository>
where
    Repository: RoomMembershipRepository + ?Sized,
{
    pub const fn new(room_repository: &'a Repository, events_service: &'a EventsService) -> Self {
        Self {
            room_repository,
            events_service,
        }
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "owned at service boundary to align with command DTO ownership"
    )]
    pub fn execute(
        &self,
        left_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: LeaveRoomCommand,
    ) -> Result<LeftRoom, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let left_user_id = left_user_id.as_existing_user_identifier();
        let room_join_context =
            require_room_membership_context(self.room_repository, &room_id, left_user_id)?;

        let Some(membership_state) = room_join_context.membership_state.as_deref() else {
            return Err(RoomsApplicationError::Forbidden);
        };
        if !matches!(membership_state, "join" | "invite" | "leave") {
            return Err(RoomsApplicationError::Forbidden);
        }
        if membership_state == "leave" {
            return Ok(LeftRoom);
        }

        let room_version = room_join_context
            .room_version
            .unwrap_or_else(|| self.events_service.default_room_version().to_string())
            .parse::<SupportedRoomVersion>()
            .map_err(|_| RoomsApplicationError::Internal)?;

        append_membership_change_for_target(
            self.room_repository,
            self.events_service,
            &room_id,
            room_version,
            left_user_id.as_str(),
            left_user_id.as_str(),
            MembershipContent::Leave,
            command.reason,
        )?;

        Ok(LeftRoom)
    }
}
