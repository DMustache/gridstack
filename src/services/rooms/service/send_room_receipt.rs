use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::rooms::entities::SendReceiptCommand;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomReceiptRepository;

use super::receipt_support::{require_receipt_target_event, validate_receipt_thread_id};
use super::room_access::{require_joined_membership, require_room_identifier};

pub struct SendRoomReceiptUseCase<'a, Repository>
where
    Repository: RoomReceiptRepository + ?Sized,
{
    room_repository: &'a Repository,
}

impl<'a, Repository> SendRoomReceiptUseCase<'a, Repository>
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
        command: SendReceiptCommand,
    ) -> Result<(), RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let event_id = command.event_id.trim();
        if event_id.is_empty() || !event_id.starts_with('$') {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        let requesting_user_id = user_id.as_existing_user_identifier();
        require_joined_membership(self.room_repository, &room_id, requesting_user_id)?;

        let thread_id =
            validate_receipt_thread_id(command.receipt_type, command.thread_id.as_deref())?;
        require_receipt_target_event(
            self.room_repository,
            &room_id,
            event_id,
            thread_id.as_deref(),
        )?;

        match command.receipt_type {
            crate::services::rooms::entities::RoomReceiptType::FullyRead => self
                .room_repository
                .append_room_fully_read_marker(&room_id, requesting_user_id.as_str(), event_id)
                .map_err(super::map_receipt_persistence_error),
            crate::services::rooms::entities::RoomReceiptType::Read
            | crate::services::rooms::entities::RoomReceiptType::ReadPrivate => self
                .room_repository
                .append_room_receipt(
                    &room_id,
                    requesting_user_id.as_str(),
                    command.receipt_type.as_str(),
                    event_id,
                    thread_id.as_deref(),
                )
                .map_err(super::map_receipt_persistence_error),
        }
    }
}
