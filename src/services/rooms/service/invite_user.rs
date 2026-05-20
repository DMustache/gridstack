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
    pub const fn new(
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

    #[allow(
        clippy::needless_pass_by_value,
        reason = "owned at service boundary to align with command DTO ownership"
    )]
    pub fn execute(
        &self,
        requesting_user_identifier: &AuthorizedUserIdentifier,
        room_id: String,
        command: InviteUserCommand,
    ) -> Result<InvitedUser, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let inviter_user_identifier = requesting_user_identifier.as_existing_user_identifier();
        let room_version = require_joined_room_version(
            self.room_repository,
            self.events_service,
            &room_id,
            inviter_user_identifier,
        )?;

        let (invitee_identifier, reason) = match command {
            InviteUserCommand::MatrixUser {
                invited_user_id,
                reason,
            } => {
                let invitee_identifier = self
                    .authorization_service
                    .require_existing_user_identifier(invited_user_id)
                    .map_err(|_| RoomsApplicationError::InvalidParameter)?;
                (invitee_identifier, reason)
            }
            InviteUserCommand::ThirdPartyIdentifier(_) => {
                // Intentional subset: this homeserver currently supports local Matrix ID invites only.
                return Err(RoomsApplicationError::Forbidden);
            }
        };
        if invitee_identifier.as_str() == inviter_user_identifier.as_str() {
            return Err(RoomsApplicationError::Forbidden);
        }

        let invitee_join_context =
            require_room_membership_context(self.room_repository, &room_id, &invitee_identifier)?;

        if let Some("ban" | "join" | "invite") = invitee_join_context.membership_state.as_deref() {
            return Err(RoomsApplicationError::Forbidden);
        }

        append_membership_change_for_target(
            self.room_repository,
            self.events_service,
            &room_id,
            room_version,
            inviter_user_identifier.as_str(),
            invitee_identifier.as_str(),
            MembershipContent::Invite,
            reason,
        )?;

        Ok(InvitedUser)
    }
}
