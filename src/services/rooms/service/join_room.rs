use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::events::entities::{MembershipContent, SupportedRoomVersion};
use crate::services::events::service::EventsService;
use crate::services::rooms::entities::{JoinRoomCommand, JoinedRoom};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomMembershipRepository;

use super::membership_change::append_membership_change_for_target;
use super::room_access::{require_room_identifier, require_room_membership_context};

pub struct JoinRoomUseCase<'a, Repository>
where
    Repository: RoomMembershipRepository + ?Sized,
{
    room_repository: &'a Repository,
    events_service: &'a EventsService,
}

impl<'a, Repository> JoinRoomUseCase<'a, Repository>
where
    Repository: RoomMembershipRepository + ?Sized,
{
    pub fn new(room_repository: &'a Repository, events_service: &'a EventsService) -> Self {
        Self {
            room_repository,
            events_service,
        }
    }

    pub fn execute(
        &self,
        joined_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: JoinRoomCommand,
    ) -> Result<JoinedRoom, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        if command.third_party_signed.is_some() {
            return Err(RoomsApplicationError::Forbidden);
        }

        let joined_user_id = joined_user_id.as_existing_user_identifier();
        let room_join_context =
            require_room_membership_context(self.room_repository, &room_id, joined_user_id)?;

        if room_join_context.membership_state.as_deref() == Some("join") {
            return Ok(JoinedRoom { room_id });
        }

        if room_join_context.visibility.as_deref() != Some("public")
            && room_join_context.membership_state.as_deref() != Some("invite")
        {
            return Err(RoomsApplicationError::Forbidden);
        }

        let room_version = room_join_context
            .room_version
            .unwrap_or_else(|| self.events_service.default_room_version().to_string())
            .parse::<SupportedRoomVersion>()
            .map_err(|_| RoomsApplicationError::Internal)?;

        append_membership_change_for_target(
            self.room_repository,
            self.events_service,
            room_id.clone(),
            room_version,
            joined_user_id.as_str(),
            joined_user_id.as_str(),
            MembershipContent::Join,
            command.reason,
        )?;

        Ok(JoinedRoom { room_id })
    }
}
