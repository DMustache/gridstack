use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::rooms::entities::{
    GetRoomMessagesCommand, RoomMessageDirection, RoomMessagesPage,
};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::{RoomMembershipRepository, RoomTimelineQueryRepository};

use super::room_access::require_room_state_read_access;

pub struct GetRoomMessagesUseCase<'a, Repository>
where
    Repository: RoomTimelineQueryRepository + RoomMembershipRepository + ?Sized,
{
    room_repository: &'a Repository,
}

impl<'a, Repository> GetRoomMessagesUseCase<'a, Repository>
where
    Repository: RoomTimelineQueryRepository + RoomMembershipRepository + ?Sized,
{
    pub fn new(room_repository: &'a Repository) -> Self {
        Self { room_repository }
    }

    pub fn execute(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: GetRoomMessagesCommand,
    ) -> Result<RoomMessagesPage, RoomsApplicationError> {
        require_room_state_read_access(
            self.room_repository,
            &room_id,
            user_id.as_existing_user_identifier(),
        )?;

        let encode_stream_token =
            |stream_position: i64| -> String { format!("s{stream_position}_0_0") };

        let from_stream_position = command
            .from_token
            .as_deref()
            .map(super::parse_room_stream_position_token)
            .transpose()?;
        let to_stream_position = command
            .to_token
            .as_deref()
            .map(super::parse_room_stream_position_token)
            .transpose()?;

        let filter = command.filter;
        let page = self
            .room_repository
            .fetch_room_timeline_events(
                &room_id,
                from_stream_position,
                to_stream_position,
                command.limit,
                matches!(command.direction, RoomMessageDirection::Backward),
                filter,
            )
            .map_err(|_| RoomsApplicationError::Internal)?;

        let start_stream_position = page
            .start
            .parse::<i64>()
            .map_err(|_| RoomsApplicationError::Internal)?;
        let end_token = page
            .end
            .map(|value| {
                value
                    .parse::<i64>()
                    .map(encode_stream_token)
                    .map_err(|_| RoomsApplicationError::Internal)
            })
            .transpose()?;

        Ok(RoomMessagesPage {
            start: encode_stream_token(start_stream_position),
            end: end_token,
            chunk: page.chunk,
            state: page.state,
        })
    }
}
