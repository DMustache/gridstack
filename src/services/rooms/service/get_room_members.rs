use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::rooms::entities::{GetRoomMembersCommand, RoomMembersChunk};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomStateQueryRepository;

use super::room_access::require_room_state_read_access;

pub struct GetRoomMembersUseCase<'a, Repository>
where
    Repository: RoomStateQueryRepository + ?Sized,
{
    room_repository: &'a Repository,
}

impl<'a, Repository> GetRoomMembersUseCase<'a, Repository>
where
    Repository: RoomStateQueryRepository + ?Sized,
{
    pub const fn new(room_repository: &'a Repository) -> Self {
        Self { room_repository }
    }

    pub fn execute(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: &str,
        command: &GetRoomMembersCommand,
    ) -> Result<RoomMembersChunk, RoomsApplicationError> {
        require_room_state_read_access(
            self.room_repository,
            room_id,
            user_id.as_existing_user_identifier(),
        )?;

        let events = match command.at_token.as_deref() {
            Some(token) => {
                let stream_position = super::parse_room_stream_position_token(token)?;
                self.room_repository
                    .fetch_room_member_state_events_at_stream_position(room_id, stream_position)
                    .map_err(|_| RoomsApplicationError::Internal)?
            }
            None => self
                .room_repository
                .fetch_room_state_events(room_id)
                .map_err(|_| RoomsApplicationError::Internal)?,
        };

        let chunk = events
            .into_iter()
            .filter(|event| event.event_type == "m.room.member")
            .filter(|event| {
                let Some(event_membership) = super::room_membership_from_event(event) else {
                    return false;
                };

                match (command.membership, command.not_membership) {
                    (None, None) => true,
                    (Some(membership), None) => event_membership == membership,
                    (None, Some(not_membership)) => event_membership != not_membership,
                    (Some(membership), Some(not_membership)) => {
                        event_membership == membership && event_membership != not_membership
                    }
                }
            })
            .collect::<Vec<_>>();

        Ok(RoomMembersChunk { chunk })
    }
}
