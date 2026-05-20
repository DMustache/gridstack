use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::events::entities::StateEventKind;
use crate::services::events::service::EventsService;
use crate::services::rooms::entities::parse_room_message_event_content;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomMessageWriteRepository;
use tracing::{info, warn};

use super::room_access::{require_joined_room_version, require_room_identifier};

pub struct SendRoomMessageEventUseCase<'a, Repository>
where
    Repository: RoomMessageWriteRepository + ?Sized,
{
    room_repository: &'a Repository,
    events_service: &'a EventsService,
}

impl<'a, Repository> SendRoomMessageEventUseCase<'a, Repository>
where
    Repository: RoomMessageWriteRepository + ?Sized,
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
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: &str,
        transaction_id: String,
        content: serde_json::Value,
    ) -> Result<String, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        if event_type.trim().is_empty() || transaction_id.trim().is_empty() || !content.is_object()
        {
            info!(
                room_id = %room_id,
                event_type = %event_type,
                transaction_id = %transaction_id,
                "send room event rejected due to invalid parameter shape"
            );
            return Err(RoomsApplicationError::InvalidParameter);
        }
        if !matches!(
            event_type.parse::<StateEventKind>(),
            Ok(StateEventKind::Custom(_))
        ) {
            info!(
                room_id = %room_id,
                event_type = %event_type,
                transaction_id = %transaction_id,
                "send room event rejected because event_type is not a message/custom event"
            );
            return Err(RoomsApplicationError::InvalidParameter);
        }

        let sender_user_id = user_id.as_existing_user_identifier().as_str().to_owned();
        if let Some(existing_event_id) = self
            .room_repository
            .fetch_room_event_id_by_transaction_id(
                &room_id,
                &sender_user_id,
                event_type,
                &transaction_id,
            )
            .map_err(|_| RoomsApplicationError::Internal)?
        {
            return Ok(existing_event_id);
        }

        let room_version = require_joined_room_version(
            self.room_repository,
            self.events_service,
            &room_id,
            user_id.as_existing_user_identifier(),
        )?;
        let (message_event_kind, event_content) =
            parse_room_message_event_content(event_type, content).map_err(|_| {
                warn!(
                    room_id = %room_id,
                    event_type = %event_type,
                    sender_user_id = %sender_user_id,
                    "send room event rejected because message content could not be parsed"
                );
                RoomsApplicationError::InvalidRoomState
            })?;

        let event_write_contract = self
            .events_service
            .create_message_event(
                room_id.clone(),
                room_version,
                sender_user_id.clone(),
                message_event_kind,
                event_content,
                Some(transaction_id),
            )
            .map_err(|error| {
                warn!(
                    room_id = %room_id,
                    event_type = %event_type,
                    sender_user_id = %sender_user_id,
                    error = %error,
                    "send room event rejected by event authorization/compilation"
                );
                super::map_membership_compilation_error(error)
            })?;

        let event_id = event_write_contract
            .event_rows
            .first()
            .map(|event| event.event_id.clone())
            .ok_or(RoomsApplicationError::Internal)?;

        self.room_repository
            .append_room_event(&event_write_contract)
            .map_err(|error| {
                warn!(
                    room_id = %room_id,
                    event_type = %event_type,
                    sender_user_id = %sender_user_id,
                    error = ?error,
                    "send room event failed during persistence"
                );
                super::map_room_persistence_error(error, &room_id)
            })?;

        Ok(event_id)
    }
}
