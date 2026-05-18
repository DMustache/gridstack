use std::sync::Arc;

use tracing::error;

use crate::{
    infrastructure::{server_name::ServerName, user_identifier::UserIdentifier},
    services::{
        authorization::{
            entities::{AuthorizedUserIdentifier, ExistingUserIdentifier},
            service::AuthorizationService,
        },
        errors::DomainError,
        events::{
            entities::{
                EventBatchWriteContract, MembershipContent, RoomEventFlow, RoomMemberContent,
                StateEventKind, SupportedRoomVersion,
            },
            service::EventsService,
        },
        rooms::{
            entities::{
                CreateRoomCommand, CreatedRoom, GetRoomMessagesCommand, JoinedRoom,
                JoinRoomCommand, LeaveRoomCommand, LeftRoom, RoomCreationFlow, RoomFactoryEvent,
                RoomIdentifier, RoomMessageDirection, RoomMessagesPage, RoomStateEvent,
                RoomValidationError, ValidatedCreateRoomInput, parse_room_state_event_content,
            },
            errors::RoomsApplicationError,
            persistence::RoomRepository,
            service::create_room::{CreateRoomValidationInput, RoomCreationFactory},
        },
    },
};

pub mod create_room;

pub struct RoomsService {
    room_repository: Arc<dyn RoomRepository>,
    events_service: Arc<EventsService>,
    authorization_service: Arc<AuthorizationService>,
    server_name: Arc<ServerName>,
}

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
        let resolved_invite = self.resolve_invited_users(command.invite.clone())?;
        let validated_input: ValidatedCreateRoomInput = CreateRoomValidationInput {
            creator: creator_user_id.as_existing_user_identifier().clone(),
            command,
            resolved_invite,
            default_room_version: self.events_service.default_room_version(),
        }
        .try_into()?;

        if !self
            .events_service
            .room_version_is_supported(validated_input.room_version)
        {
            return Err(RoomsApplicationError::UnsupportedRoomVersion);
        }

        let room_creation_flow: RoomCreationFlow =
            RoomCreationFactory::new(&validated_input, &self.server_name)
                .add_event(RoomFactoryEvent::RequiredCreateEvent)
                .add_event(RoomFactoryEvent::CreatorJoinEvent)
                .add_event(RoomFactoryEvent::DefaultPowerLevelsEvent)
                .add_event(RoomFactoryEvent::CanonicalAliasEventIfNeeded)
                .add_event(RoomFactoryEvent::PresetEvents)
                .add_event(RoomFactoryEvent::InitialStateEvents)
                .add_event(RoomFactoryEvent::NameAndTopicEvents)
                .add_event(RoomFactoryEvent::InviteEvents)
                .build_event_flow();

        let event_batch_write_contract: EventBatchWriteContract =
            self.events_service.compile_room_event_flow(RoomEventFlow {
                room_id: room_creation_flow.room_shell_intent.room_id.clone(),
                room_version: room_creation_flow.room_shell_intent.room_version.clone(),
                intents: room_creation_flow.ordered_event_intents.clone(),
            })?;

        self.room_repository
            .create_room_with_initial_events(&room_creation_flow, &event_batch_write_contract)
            .map_err(|error| {
                map_room_persistence_error(
                    error,
                    room_creation_flow.room_shell_intent.room_id.clone(),
                )
            })?;

        Ok(CreatedRoom {
            room_id: room_creation_flow.room_shell_intent.room_id,
        })
    }

    pub fn join_room_by_id(
        &self,
        joined_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: JoinRoomCommand,
    ) -> Result<JoinedRoom, RoomsApplicationError> {
        self.require_room_identifier(&room_id)?;

        if command.third_party_signed.is_some() {
            return Err(RoomsApplicationError::Forbidden);
        }

        let room_join_context =
            self.require_room_membership_context(&room_id, joined_user_id.as_existing_user_identifier())?;

        if room_join_context.membership_state.as_deref() == Some("join") {
            return Ok(JoinedRoom { room_id });
        }

        if room_join_context.visibility.as_deref() != Some("public")
            && room_join_context.membership_state.as_deref() != Some("invite")
        {
            return Err(RoomsApplicationError::Forbidden);
        }

        let room_version = room_join_context
            .room_version
            .unwrap_or_else(|| self.events_service.default_room_version().to_string());
        let room_version = room_version
            .parse::<SupportedRoomVersion>()
            .map_err(|_| RoomsApplicationError::Internal)?;

        self.append_membership_change(
            room_id.clone(),
            room_version,
            joined_user_id.as_existing_user_identifier().as_str(),
            MembershipContent::Join,
            command.reason,
        )?;

        Ok(JoinedRoom { room_id })
    }

    pub fn leave_room_by_id(
        &self,
        left_user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: LeaveRoomCommand,
    ) -> Result<LeftRoom, RoomsApplicationError> {
        self.require_room_identifier(&room_id)?;

        let room_join_context =
            self.require_room_membership_context(&room_id, left_user_id.as_existing_user_identifier())?;

        let Some(membership_state) = room_join_context.membership_state.as_deref() else {
            return Err(RoomsApplicationError::Forbidden);
        };
        if !matches!(membership_state, "join" | "invite" | "leave") {
            return Err(RoomsApplicationError::Forbidden);
        }
        if membership_state == "leave" {
            return Ok(LeftRoom);
        }

        let room_version = room_join_context
            .room_version
            .unwrap_or_else(|| self.events_service.default_room_version().to_string());
        let room_version = room_version
            .parse::<SupportedRoomVersion>()
            .map_err(|_| RoomsApplicationError::Internal)?;

        self.append_membership_change(
            room_id,
            room_version,
            left_user_id.as_existing_user_identifier().as_str(),
            MembershipContent::Leave,
            command.reason,
        )?;

        Ok(LeftRoom)
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

    pub fn get_room_state_with_key(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: String,
        state_key: String,
    ) -> Result<RoomStateEvent, RoomsApplicationError> {
        self.require_room_state_read_access(&room_id, user_id.as_existing_user_identifier())?;

        self.room_repository
            .fetch_room_state_event_by_type_and_key(&room_id, &event_type, &state_key)
            .map_err(|_| RoomsApplicationError::Internal)?
            .ok_or(RoomsApplicationError::NotFound)
    }

    pub fn get_room_messages(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        command: GetRoomMessagesCommand,
    ) -> Result<RoomMessagesPage, RoomsApplicationError> {
        self.require_room_state_read_access(&room_id, user_id.as_existing_user_identifier())?;

        let parse_stream_position = |token: &str| -> Result<i64, RoomsApplicationError> {
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
        };

        let encode_stream_token = |stream_position: i64| -> String {
            format!("s{stream_position}_0_0")
        };

        let from_stream_position = command
            .from_token
            .as_deref()
            .map(parse_stream_position)
            .transpose()?;
        let to_stream_position = command
            .to_token
            .as_deref()
            .map(parse_stream_position)
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

    pub fn set_room_state_with_key(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: String,
        state_key: String,
        content: serde_json::Value,
    ) -> Result<String, RoomsApplicationError> {
        self.require_room_identifier(&room_id)?;
        if !content.is_object() {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        let room_join_context =
            self.require_room_membership_context(&room_id, user_id.as_existing_user_identifier())?;
        if room_join_context.membership_state.as_deref() != Some("join") {
            return Err(RoomsApplicationError::Forbidden);
        }

        let room_version = room_join_context
            .room_version
            .unwrap_or_else(|| self.events_service.default_room_version().to_string())
            .parse::<SupportedRoomVersion>()
            .map_err(|_| RoomsApplicationError::Internal)?;

        let state_event_kind = event_type
            .parse::<StateEventKind>()
            .map_err(|_| RoomsApplicationError::InvalidParameter)?;
        self.validate_canonical_alias_state_if_needed(&room_id, &state_event_kind, &content)?;
        let event_content = parse_room_state_event_content(state_event_kind.clone(), content)
            .map_err(|_| RoomsApplicationError::InvalidRoomState)?;

        let event_write_contract = self
            .events_service
            .create_state_event(
                room_id.clone(),
                room_version,
                user_id
                    .as_existing_user_identifier()
                    .as_str()
                    .to_owned(),
                state_event_kind,
                state_key,
                event_content,
                None,
            )
            .map_err(RoomsApplicationError::from)?;

        let event_id = event_write_contract
            .event_rows
            .first()
            .map(|event| event.event_id.clone())
            .ok_or(RoomsApplicationError::Internal)?;

        self.room_repository
            .append_room_event(&event_write_contract)
            .map_err(|error| map_room_persistence_error(error, room_id))?;

        Ok(event_id)
    }
}

impl RoomsService {
    fn require_room_identifier(&self, room_id: &str) -> Result<(), RoomsApplicationError> {
        if RoomIdentifier::parse(room_id.to_owned()).is_none() {
            return Err(RoomsApplicationError::Forbidden);
        }
        Ok(())
    }

    fn require_room_membership_context(
        &self,
        room_id: &str,
        user_id: &ExistingUserIdentifier,
    ) -> Result<crate::services::rooms::persistence::RoomJoinContext, RoomsApplicationError> {
        self.room_repository
            .fetch_join_context(room_id, user_id.as_str())
            .map_err(|_| RoomsApplicationError::Internal)?
            .ok_or(RoomsApplicationError::Forbidden)
    }

    fn require_room_state_read_access(
        &self,
        room_id: &str,
        user_id: &ExistingUserIdentifier,
    ) -> Result<(), RoomsApplicationError> {
        self.require_room_identifier(room_id)?;
        let room_join_context = self.require_room_membership_context(room_id, user_id)?;
        let Some(membership_state) = room_join_context.membership_state.as_deref() else {
            return Err(RoomsApplicationError::Forbidden);
        };
        if !matches!(membership_state, "join" | "leave") {
            return Err(RoomsApplicationError::Forbidden);
        }
        Ok(())
    }

    fn append_membership_change(
        &self,
        room_id: String,
        room_version: SupportedRoomVersion,
        user_identifier: &str,
        membership_content: MembershipContent,
        reason: Option<String>,
    ) -> Result<(), RoomsApplicationError> {
        let event_write_contract = self
            .events_service
            .create_membership_event(
                room_id.clone(),
                room_version,
                user_identifier.to_owned(),
                user_identifier.to_owned(),
                crate::services::events::entities::MatrixEventContent::RoomMember(
                    RoomMemberContent {
                        membership: membership_content,
                        is_direct: None,
                    },
                ),
                None,
            )
            .map_err(RoomsApplicationError::from)?;

        let _ = reason;

        self.room_repository
            .append_room_event(&event_write_contract)
            .map_err(|error| map_room_persistence_error(error, room_id))?;
        Ok(())
    }

    fn resolve_invited_users(
        &self,
        invited_user_ids: Vec<UserIdentifier>,
    ) -> Result<Vec<ExistingUserIdentifier>, RoomsApplicationError> {
        let mut invite = Vec::with_capacity(invited_user_ids.len());

        for invited_user_id in invited_user_ids {
            let existing_user = self
                .authorization_service
                .require_existing_user_identifier(invited_user_id.clone())
                .map_err(|_| {
                    RoomsApplicationError::from(RoomValidationError::InvalidInviteUserIdentifier {
                        user_id: invited_user_id.to_string(),
                    })
                })?;
            invite.push(existing_user);
        }

        Ok(invite)
    }

    fn validate_canonical_alias_state_if_needed(
        &self,
        room_id: &str,
        state_event_kind: &StateEventKind,
        content: &serde_json::Value,
    ) -> Result<(), RoomsApplicationError> {
        if !matches!(state_event_kind, StateEventKind::RoomCanonicalAlias) {
            return Ok(());
        }

        let mut aliases = Vec::new();
        if let Some(alias_value) = content.get("alias") {
            let alias = alias_value
                .as_str()
                .ok_or(RoomsApplicationError::InvalidParameter)?;
            aliases.push(alias.to_owned());
        }
        if let Some(alt_aliases_value) = content.get("alt_aliases") {
            let alt_aliases = alt_aliases_value
                .as_array()
                .ok_or(RoomsApplicationError::InvalidParameter)?;
            for alias_value in alt_aliases {
                let alias = alias_value
                    .as_str()
                    .ok_or(RoomsApplicationError::InvalidParameter)?;
                aliases.push(alias.to_owned());
            }
        }

        for alias in aliases {
            let alias_localpart = parse_room_alias_localpart_for_local_server(
                alias.as_str(),
                self.server_name.as_str(),
            )
            .ok_or(RoomsApplicationError::InvalidParameter)?;

            let bound_room_id = self
                .room_repository
                .fetch_room_id_by_alias_localpart(alias_localpart)
                .map_err(|_| RoomsApplicationError::Internal)?;
            if bound_room_id.as_deref() != Some(room_id) {
                return Err(RoomsApplicationError::BadAlias);
            }
        }

        Ok(())
    }
}

fn map_room_persistence_error(error: DomainError, room_id: String) -> RoomsApplicationError {
    match error {
        DomainError::AlreadyExists => RoomsApplicationError::RoomInUse,
        DomainError::InvalidRequest(reason) => {
            if reason.contains("room_aliases") || reason.contains("alias_localpart") {
                RoomsApplicationError::RoomInUse
            } else {
                error!(room_id, reason, "failed to persist room creation contract");
                RoomsApplicationError::Internal
            }
        }
        DomainError::NotFound | DomainError::InvalidCredentials => RoomsApplicationError::Internal,
    }
}

fn parse_room_alias_localpart_for_local_server<'a>(
    room_alias: &'a str,
    server_name: &str,
) -> Option<&'a str> {
    let alias_without_prefix = room_alias.strip_prefix('#')?;
    let (localpart, alias_server_name) = alias_without_prefix.split_once(':')?;
    if localpart.trim().is_empty()
        || alias_server_name.trim().is_empty()
        || alias_server_name != server_name
    {
        return None;
    }
    Some(localpart)
}

#[allow(dead_code)]
fn _supported_room_versions() -> [SupportedRoomVersion; 2] {
    [SupportedRoomVersion::V1, SupportedRoomVersion::V2]
}
