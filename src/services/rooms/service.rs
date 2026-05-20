use std::sync::Arc;

use tracing::error;

use crate::{
    infrastructure::server_name::ServerName,
    services::{
        authorization::{
            entities::{AuthorizedUserIdentifier, ExistingUserIdentifier},
            persistence::access_session_storage_unit::AccessSessionStorageUnit,
            service::AuthorizationService,
        },
        errors::DomainError,
        events::{entities::SupportedRoomVersion, service::EventsService},
        rooms::{
            entities::{
                CreateRoomCommand, CreatedRoom, GetPublicRoomsCommand, GetRoomMembersCommand,
                GetRoomMessagesCommand, InviteUserCommand, InvitedUser, JoinRoomCommand,
                JoinedMembers, JoinedRoom, JoinedRooms, LeaveRoomCommand, LeftRoom,
                PublicRoomsChunk, PublicRoomsPage, RoomMembersChunk, RoomMembershipFilter,
                RoomMessagesPage, RoomStateEvent, RoomTimelineEvent, SendReceiptCommand,
                SetReadMarkersCommand,
            },
            errors::RoomsApplicationError,
            persistence::{PublicRoomRosterEntry, RoomRepository},
            service::{
                create_room::CreateRoomUseCase, get_joined_members::GetJoinedMembersUseCase,
                get_room_event::GetRoomEventUseCase, get_room_members::GetRoomMembersUseCase,
                get_room_messages::GetRoomMessagesUseCase,
                get_room_state_with_key::GetRoomStateWithKeyUseCase,
                invite_user::InviteUserUseCase, join_room::JoinRoomUseCase,
                leave_room::LeaveRoomUseCase, room_state_write::SetRoomStateWithKeyUseCase,
                send_room_message::SendRoomMessageEventUseCase,
                send_room_receipt::SendRoomReceiptUseCase, set_read_markers::SetReadMarkersUseCase,
            },
        },
    },
};

pub mod create_room;
pub mod get_joined_members;
pub mod get_room_event;
pub mod get_room_members;
pub mod get_room_messages;
pub mod get_room_state_with_key;
mod history_visibility;
pub mod invite_user;
pub mod join_room;
pub mod leave_room;
mod membership_change;
mod receipt_support;
mod room_access;
pub mod room_state_write;
pub mod send_room_message;
pub mod send_room_receipt;
pub mod set_read_markers;

pub struct RoomsService {
    room_repository: Arc<dyn RoomRepository>,
    events_service: Arc<EventsService>,
    authorization_service: Arc<AuthorizationService>,
    server_name: Arc<ServerName>,
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "service boundary accepts owned handler DTOs for clarity"
)]
impl RoomsService {
    pub fn new(
        room_repository: Arc<dyn RoomRepository>,
        events_service: Arc<EventsService>,
        authorization_service: Arc<AuthorizationService>,
        server_name: &ServerName,
    ) -> Self {
        Self {
            room_repository,
            events_service,
            authorization_service,
            server_name: Arc::new(server_name.clone()),
        }
    }

    pub fn create_room(
        &self,
        creator_user_id: &AuthorizedUserIdentifier,
        command: CreateRoomCommand,
    ) -> Result<CreatedRoom, RoomsApplicationError> {
        CreateRoomUseCase::new(
            self.room_repository.as_ref(),
            self.events_service.as_ref(),
            self.authorization_service.as_ref(),
            self.server_name.as_ref(),
        )
        .execute(creator_user_id, command)
    }

    pub fn join_room_by_id(
        &self,
        joined_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: JoinRoomCommand,
    ) -> Result<JoinedRoom, RoomsApplicationError> {
        JoinRoomUseCase::new(self.room_repository.as_ref(), self.events_service.as_ref()).execute(
            joined_user_id,
            room_id,
            command,
        )
    }

    pub fn get_joined_rooms(
        &self,
        user_id: &AuthorizedUserIdentifier,
    ) -> Result<JoinedRooms, RoomsApplicationError> {
        let room_ids = self
            .room_repository
            .fetch_joined_room_ids_for_user(user_id.as_existing_user_identifier().as_str())
            .map_err(|_| RoomsApplicationError::Internal)?;

        Ok(JoinedRooms { room_ids })
    }

    pub fn get_public_rooms(
        &self,
        command: GetPublicRoomsCommand,
    ) -> Result<PublicRoomsPage, RoomsApplicationError> {
        if let Some(server) = command.server.as_deref() {
            if server.trim().is_empty() || server != self.server_name.as_str() {
                return Err(RoomsApplicationError::InvalidParameter);
            }
        }

        let since_offset = parse_public_rooms_offset(command.since.as_deref())?;

        let total_room_count = self
            .room_repository
            .fetch_public_room_count()
            .map_err(|_| RoomsApplicationError::Internal)?;

        let offset =
            i64::try_from(since_offset).map_err(|_| RoomsApplicationError::InvalidParameter)?;
        let limit =
            i64::try_from(command.limit).map_err(|_| RoomsApplicationError::InvalidParameter)?;

        let public_rooms = self
            .room_repository
            .fetch_public_room_roster_page(offset, limit)
            .map_err(|_| RoomsApplicationError::Internal)?;

        let mut chunk = Vec::with_capacity(public_rooms.len());
        for public_room in public_rooms {
            let state_events = self
                .room_repository
                .fetch_room_state_events(&public_room.room_id)
                .map_err(|_| RoomsApplicationError::Internal)?;

            chunk.push(map_public_room_chunk(public_room, state_events));
        }

        let total_room_count_usize = usize::try_from(total_room_count).ok();
        let next_offset = since_offset.saturating_add(chunk.len());
        let next_batch = if command.limit > 0
            && total_room_count_usize.is_some_and(|value| next_offset < value)
        {
            Some(next_offset.to_string())
        } else {
            None
        };
        let prev_batch = if command.limit > 0 && since_offset > 0 {
            Some(since_offset.saturating_sub(command.limit).to_string())
        } else {
            None
        };

        Ok(PublicRoomsPage {
            chunk,
            next_batch,
            prev_batch,
            total_room_count_estimate: Some(total_room_count),
        })
    }

    pub fn get_joined_members(
        &self,
        access_session: &AccessSessionStorageUnit,
        room_id: String,
    ) -> Result<JoinedMembers, RoomsApplicationError> {
        GetJoinedMembersUseCase::new(
            self.room_repository.as_ref(),
            self.authorization_service.as_ref(),
        )
        .execute(access_session, &room_id)
    }

    pub fn leave_room_by_id(
        &self,
        left_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: LeaveRoomCommand,
    ) -> Result<LeftRoom, RoomsApplicationError> {
        LeaveRoomUseCase::new(self.room_repository.as_ref(), self.events_service.as_ref()).execute(
            left_user_id,
            room_id,
            command,
        )
    }

    pub fn invite_user_to_room(
        &self,
        inviter_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: InviteUserCommand,
    ) -> Result<InvitedUser, RoomsApplicationError> {
        InviteUserUseCase::new(
            self.room_repository.as_ref(),
            self.events_service.as_ref(),
            self.authorization_service.as_ref(),
        )
        .execute(inviter_user_id, room_id, command)
    }

    pub fn get_room_state(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
    ) -> Result<Vec<RoomStateEvent>, RoomsApplicationError> {
        self.require_room_state_read_access(&room_id, user_id.as_existing_user_identifier())?;

        self.room_repository
            .fetch_room_state_events(&room_id)
            .map_err(|_| RoomsApplicationError::Internal)
    }

    pub fn get_room_members(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: GetRoomMembersCommand,
    ) -> Result<RoomMembersChunk, RoomsApplicationError> {
        GetRoomMembersUseCase::new(self.room_repository.as_ref())
            .execute(user_id, &room_id, &command)
    }

    pub fn get_room_state_with_key(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: String,
        state_key: String,
    ) -> Result<RoomStateEvent, RoomsApplicationError> {
        GetRoomStateWithKeyUseCase::new(self.room_repository.as_ref()).execute(
            user_id,
            &room_id,
            &event_type,
            &state_key,
        )
    }

    pub fn get_room_event(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_id: String,
    ) -> Result<RoomTimelineEvent, RoomsApplicationError> {
        GetRoomEventUseCase::new(self.room_repository.as_ref())
            .execute(user_id, &room_id, &event_id)
    }

    pub fn get_room_messages(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: GetRoomMessagesCommand,
    ) -> Result<RoomMessagesPage, RoomsApplicationError> {
        GetRoomMessagesUseCase::new(self.room_repository.as_ref())
            .execute(user_id, &room_id, command)
    }

    pub fn set_room_state_with_key(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: String,
        state_key: String,
        content: serde_json::Value,
    ) -> Result<String, RoomsApplicationError> {
        SetRoomStateWithKeyUseCase::new(
            self.room_repository.as_ref(),
            self.events_service.as_ref(),
            self.server_name.as_ref(),
        )
        .execute(user_id, room_id, event_type, state_key, content)
    }

    pub fn send_room_message_event(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: String,
        transaction_id: String,
        content: serde_json::Value,
    ) -> Result<String, RoomsApplicationError> {
        SendRoomMessageEventUseCase::new(
            self.room_repository.as_ref(),
            self.events_service.as_ref(),
        )
        .execute(user_id, room_id, &event_type, transaction_id, content)
    }

    pub fn send_room_receipt(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: SendReceiptCommand,
    ) -> Result<(), RoomsApplicationError> {
        SendRoomReceiptUseCase::new(self.room_repository.as_ref())
            .execute(user_id, &room_id, &command)
    }

    pub fn set_read_markers(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: SetReadMarkersCommand,
    ) -> Result<(), RoomsApplicationError> {
        SetReadMarkersUseCase::new(self.room_repository.as_ref())
            .execute(user_id, &room_id, &command)
    }
}

impl RoomsService {
    fn require_room_state_read_access(
        &self,
        room_id: &str,
        user_id: &ExistingUserIdentifier,
    ) -> Result<(), RoomsApplicationError> {
        room_access::require_room_state_read_access(self.room_repository.as_ref(), room_id, user_id)
    }
}

fn history_visibility_from_timeline_event(event: &RoomTimelineEvent) -> Option<String> {
    event
        .content
        .get("history_visibility")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn parse_public_rooms_offset(value: Option<&str>) -> Result<usize, RoomsApplicationError> {
    let token = match value {
        Some(value) => value.trim(),
        None => return Ok(0),
    };
    if token.is_empty() {
        return Err(RoomsApplicationError::InvalidParameter);
    }

    token
        .parse::<usize>()
        .map_err(|_| RoomsApplicationError::InvalidParameter)
}

fn map_public_room_chunk(
    public_room: PublicRoomRosterEntry,
    state_events: Vec<RoomStateEvent>,
) -> PublicRoomsChunk {
    let create_content = room_state_content(&state_events, "m.room.create");
    let join_rules_content = room_state_content(&state_events, "m.room.join_rules");
    let history_visibility_content = room_state_content(&state_events, "m.room.history_visibility");
    let guest_access_content = room_state_content(&state_events, "m.room.guest_access");
    let canonical_alias_content = room_state_content(&state_events, "m.room.canonical_alias");
    let name_content = room_state_content(&state_events, "m.room.name");
    let topic_content = room_state_content(&state_events, "m.room.topic");
    let avatar_content = room_state_content(&state_events, "m.room.avatar");

    PublicRoomsChunk {
        room_id: public_room.room_id,
        num_joined_members: public_room.num_joined_members,
        world_readable: json_string_field(history_visibility_content, "history_visibility")
            .is_some_and(|value| value == "world_readable"),
        guest_can_join: json_string_field(guest_access_content, "guest_access")
            .is_some_and(|value| value == "can_join"),
        canonical_alias: json_string_field(canonical_alias_content, "alias"),
        name: json_string_field(name_content, "name").or(public_room.fallback_name),
        topic: json_string_field(topic_content, "topic").or(public_room.fallback_topic),
        avatar_url: json_string_field(avatar_content, "url"),
        join_rule: json_string_field(join_rules_content, "join_rule"),
        room_type: json_string_field(create_content, "type"),
    }
}

fn room_state_content<'a>(
    state_events: &'a [RoomStateEvent],
    event_type: &str,
) -> Option<&'a serde_json::Value> {
    state_events
        .iter()
        .find(|event| event.event_type == event_type && event.state_key.is_empty())
        .map(|event| &event.content)
}

fn json_string_field(value: Option<&serde_json::Value>, field: &str) -> Option<String> {
    value
        .and_then(|value| value.get(field))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn room_membership_from_timeline_event(event: &RoomTimelineEvent) -> Option<&str> {
    event
        .content
        .get("membership")
        .and_then(serde_json::Value::as_str)
}

fn map_room_persistence_error(error: DomainError, room_id: &str) -> RoomsApplicationError {
    match error {
        DomainError::AlreadyExists => RoomsApplicationError::RoomInUse,
        DomainError::InvalidRequest(reason) => {
            if reason.contains("room_aliases") || reason.contains("alias_localpart") {
                RoomsApplicationError::RoomInUse
            } else {
                error!(room_id, reason, "failed to persist room event contract");
                RoomsApplicationError::Internal
            }
        }
        DomainError::NotFound | DomainError::InvalidCredentials => RoomsApplicationError::Internal,
    }
}

fn map_membership_compilation_error(
    error: crate::services::events::service::EventCompilationError,
) -> RoomsApplicationError {
    match error {
        crate::services::events::service::EventCompilationError::MissingSenderMembershipForAuth {
            ..
        } => RoomsApplicationError::Forbidden,
        _ => RoomsApplicationError::from(error),
    }
}

fn map_receipt_persistence_error(error: DomainError) -> RoomsApplicationError {
    match error {
        DomainError::NotFound => RoomsApplicationError::InvalidParameter,
        DomainError::AlreadyExists | DomainError::InvalidCredentials => {
            RoomsApplicationError::Internal
        }
        DomainError::InvalidRequest(reason) => {
            error!(reason, "failed to persist receipt");
            RoomsApplicationError::Internal
        }
    }
}

fn room_membership_from_event(event: &RoomStateEvent) -> Option<RoomMembershipFilter> {
    let membership = event.content.get("membership")?.as_str()?;
    RoomMembershipFilter::parse(membership)
}

fn parse_room_stream_position_token(token: &str) -> Result<i64, RoomsApplicationError> {
    if let Ok(value) = token.parse::<i64>() {
        return Ok(value);
    }

    let first_numeric_fragment = token
        .trim_start_matches(|character: char| character.is_ascii_alphabetic())
        .split(|character: char| !character.is_ascii_digit())
        .find(|fragment| !fragment.is_empty())
        .ok_or(RoomsApplicationError::InvalidParameter)?;

    first_numeric_fragment
        .parse::<i64>()
        .map_err(|_| RoomsApplicationError::InvalidParameter)
}

#[allow(dead_code)]
const fn _supported_room_versions() -> [SupportedRoomVersion; 2] {
    [SupportedRoomVersion::V1, SupportedRoomVersion::V2]
}
