use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::rooms::entities::RoomTimelineEvent;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomTimelineQueryRepository;

use super::history_visibility::{EventVisibilityEvaluationInput, evaluate_event_visibility};
use super::room_access::require_room_identifier;

pub struct GetRoomEventUseCase<'a, Repository>
where
    Repository: RoomTimelineQueryRepository + ?Sized,
{
    room_repository: &'a Repository,
}

impl<'a, Repository> GetRoomEventUseCase<'a, Repository>
where
    Repository: RoomTimelineQueryRepository + ?Sized,
{
    pub fn new(room_repository: &'a Repository) -> Self {
        Self { room_repository }
    }

    pub fn execute(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_id: String,
    ) -> Result<RoomTimelineEvent, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let room_event = self
            .room_repository
            .fetch_room_timeline_event_by_id(&room_id, &event_id)
            .map_err(|_| RoomsApplicationError::Internal)?
            .ok_or(RoomsApplicationError::NotFound)?;

        let user_id = user_id.as_existing_user_identifier();
        let membership_at_event = self
            .room_repository
            .fetch_user_membership_at_stream_position(
                &room_id,
                user_id.as_str(),
                room_event.stream_position,
            )
            .map_err(|_| RoomsApplicationError::Internal)?;
        let user_joined_since_event = self
            .room_repository
            .user_joined_since_stream_position(
                &room_id,
                user_id.as_str(),
                room_event.stream_position,
            )
            .map_err(|_| RoomsApplicationError::Internal)?;

        let history_visibility_at_event = self
            .room_repository
            .fetch_room_history_visibility_at_stream_position(&room_id, room_event.stream_position)
            .map_err(|_| RoomsApplicationError::Internal)?;
        let mut history_visibility_before_event = None;
        let mut history_visibility_after_event = None;

        if room_event.event_type == "m.room.history_visibility"
            && room_event.state_key.as_deref() == Some("")
        {
            history_visibility_before_event = self
                .room_repository
                .fetch_room_history_visibility_before_stream_position(
                    &room_id,
                    room_event.stream_position,
                )
                .map_err(|_| RoomsApplicationError::Internal)?;

            history_visibility_after_event =
                super::history_visibility_from_timeline_event(&room_event)
                    .or(history_visibility_at_event.clone());
        }

        let mut membership_before_event = None;
        let mut membership_after_event = None;
        if room_event.event_type == "m.room.member"
            && room_event.state_key.as_deref() == Some(user_id.as_str())
        {
            membership_before_event = self
                .room_repository
                .fetch_user_membership_before_stream_position(
                    &room_id,
                    user_id.as_str(),
                    room_event.stream_position,
                )
                .map_err(|_| RoomsApplicationError::Internal)?;
            membership_after_event = super::room_membership_from_timeline_event(&room_event)
                .map(str::to_owned)
                .or(membership_at_event.clone());
        }

        let event_visible = evaluate_event_visibility(&EventVisibilityEvaluationInput {
            event_type: room_event.event_type.as_str(),
            event_state_key: room_event.state_key.as_deref(),
            requesting_user_id: user_id.as_str(),
            history_visibility_at_event: history_visibility_at_event.as_deref(),
            history_visibility_before_event: history_visibility_before_event.as_deref(),
            history_visibility_after_event: history_visibility_after_event.as_deref(),
            membership_at_event: membership_at_event.as_deref(),
            membership_before_event: membership_before_event.as_deref(),
            membership_after_event: membership_after_event.as_deref(),
            user_joined_since_event,
        });

        if !event_visible {
            return Err(RoomsApplicationError::Forbidden);
        }

        Ok(room_event)
    }
}
