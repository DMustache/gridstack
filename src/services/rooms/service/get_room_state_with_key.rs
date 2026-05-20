use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::rooms::entities::RoomStateEvent;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomStateQueryRepository;

use super::room_access::require_room_state_read_access;

pub struct GetRoomStateWithKeyUseCase<'a, Repository>
where
    Repository: RoomStateQueryRepository + ?Sized,
{
    room_repository: &'a Repository,
}

impl<'a, Repository> GetRoomStateWithKeyUseCase<'a, Repository>
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
        event_type: &str,
        state_key: &str,
    ) -> Result<RoomStateEvent, RoomsApplicationError> {
        require_room_state_read_access(
            self.room_repository,
            room_id,
            user_id.as_existing_user_identifier(),
        )?;

        self.room_repository
            .fetch_room_state_event_by_type_and_key(room_id, event_type, state_key)
            .map_err(|_| RoomsApplicationError::Internal)?
            .ok_or(RoomsApplicationError::NotFound)
    }
}
