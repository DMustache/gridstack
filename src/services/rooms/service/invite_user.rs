use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::authorization::service::AuthorizationService;
use crate::services::events::entities::MembershipContent;
use crate::services::events::service::EventsService;
use crate::services::rooms::entities::{InviteUserCommand, InvitedUser};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomMembershipRepository;

use super::membership_change::append_membership_change_for_target;
use super::room_access::{
    require_joined_room_version, require_room_identifier, require_room_membership_context,
};

pub struct InviteUserUseCase<'a, Repository>
where
    Repository: RoomMembershipRepository + ?Sized,
{
    room_repository: &'a Repository,
    events_service: &'a EventsService,
    authorization_service: &'a AuthorizationService,
}

impl<'a, Repository> InviteUserUseCase<'a, Repository>
where
    Repository: RoomMembershipRepository + ?Sized,
{
    pub fn new(
        room_repository: &'a Repository,
        events_service: &'a EventsService,
        authorization_service: &'a AuthorizationService,
    ) -> Self {
        Self {
            room_repository,
            events_service,
            authorization_service,
        }
    }

    pub fn execute(
        &self,
        inviter_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: InviteUserCommand,
    ) -> Result<InvitedUser, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let inviter_user_id = inviter_user_id.as_existing_user_identifier();
        let room_version = require_joined_room_version(
            self.room_repository,
            self.events_service,
            &room_id,
            inviter_user_id,
        )?;

        let (invited_user_id, reason) = match command {
            InviteUserCommand::MatrixUser {
                invited_user_id,
                reason,
            } => {
                let invited_user_id = self
                    .authorization_service
                    .require_existing_user_identifier(invited_user_id)
                    .map_err(|_| RoomsApplicationError::InvalidParameter)?;
                (invited_user_id, reason)
            }
            InviteUserCommand::ThirdPartyIdentifier(_) => {
                // Intentional subset: this homeserver currently supports local Matrix ID invites only.
                return Err(RoomsApplicationError::Forbidden);
            }
        };
        if invited_user_id.as_str() == inviter_user_id.as_str() {
            return Err(RoomsApplicationError::Forbidden);
        }

        let invitee_join_context =
            require_room_membership_context(self.room_repository, &room_id, &invited_user_id)?;

        match invitee_join_context.membership_state.as_deref() {
            Some("ban" | "join" | "invite") => return Err(RoomsApplicationError::Forbidden),
            _ => {}
        }

        append_membership_change_for_target(
            self.room_repository,
            self.events_service,
            room_id,
            room_version,
            inviter_user_id.as_str(),
            invited_user_id.as_str(),
            MembershipContent::Invite,
            reason,
        )?;

        Ok(InvitedUser)
    }
}
