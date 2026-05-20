use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::rooms::entities::{RoomReceiptType, SetReadMarkersCommand};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomReceiptRepository;

use super::receipt_support::require_receipt_target_event;
use super::room_access::{require_joined_membership, require_room_identifier};

pub struct SetReadMarkersUseCase<'a, Repository>
where
    Repository: RoomReceiptRepository + ?Sized,
{
    room_repository: &'a Repository,
}

impl<'a, Repository> SetReadMarkersUseCase<'a, Repository>
where
    Repository: RoomReceiptRepository + ?Sized,
{
    pub fn new(room_repository: &'a Repository) -> Self {
        Self { room_repository }
    }

    pub fn execute(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: SetReadMarkersCommand,
    ) -> Result<(), RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let requesting_user_id = user_id.as_existing_user_identifier();
        require_joined_membership(self.room_repository, &room_id, requesting_user_id)?;

        let fully_read_event_id = command.fully_read_event_id.as_deref().map(str::trim);
        let read_event_id = command.read_event_id.as_deref().map(str::trim);
        let private_read_event_id = command.private_read_event_id.as_deref().map(str::trim);

        if let Some(fully_read_event_id) = fully_read_event_id {
            require_receipt_target_event(
                self.room_repository,
                &room_id,
                fully_read_event_id,
                None,
            )?;
        }
        if let Some(read_event_id) = read_event_id {
            require_receipt_target_event(self.room_repository, &room_id, read_event_id, None)?;
        }
        if let Some(private_read_event_id) = private_read_event_id {
            require_receipt_target_event(
                self.room_repository,
                &room_id,
                private_read_event_id,
                None,
            )?;
        }

        if let Some(fully_read_event_id) = fully_read_event_id {
            self.room_repository
                .append_room_fully_read_marker(
                    &room_id,
                    requesting_user_id.as_str(),
                    fully_read_event_id,
                )
                .map_err(super::map_receipt_persistence_error)?;
        }

        if let Some(read_event_id) = read_event_id {
            self.room_repository
                .append_room_receipt(
                    &room_id,
                    requesting_user_id.as_str(),
                    RoomReceiptType::Read.as_str(),
                    read_event_id,
                    None,
                )
                .map_err(super::map_receipt_persistence_error)?;
        }

        if let Some(private_read_event_id) = private_read_event_id {
            self.room_repository
                .append_room_receipt(
                    &room_id,
                    requesting_user_id.as_str(),
                    RoomReceiptType::ReadPrivate.as_str(),
                    private_read_event_id,
                    None,
                )
                .map_err(super::map_receipt_persistence_error)?;
        }

        Ok(())
    }
}
