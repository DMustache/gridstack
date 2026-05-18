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
                SupportedRoomVersion,
            },
            service::EventsService,
        },
        rooms::{
            entities::{
                CreateRoomCommand, CreatedRoom, JoinRoomCommand, JoinedRoom, LeaveRoomCommand,
                LeftRoom, RoomCreationFlow, RoomFactoryEvent, RoomIdentifier, RoomValidationError,
                ValidatedCreateRoomInput,
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
        if RoomIdentifier::parse(room_id.clone()).is_none() {
            return Err(RoomsApplicationError::Forbidden);
        }

        if command.third_party_signed.is_some() {
            return Err(RoomsApplicationError::Forbidden);
        }

        let room_join_context = self
            .room_repository
            .fetch_join_context(
                &room_id,
                joined_user_id.as_existing_user_identifier().as_str(),
            )
            .map_err(|_| RoomsApplicationError::Internal)?
            .ok_or(RoomsApplicationError::Forbidden)?;

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
        if RoomIdentifier::parse(room_id.clone()).is_none() {
            return Err(RoomsApplicationError::Forbidden);
        }

        let room_join_context = self
            .room_repository
            .fetch_join_context(
                &room_id,
                left_user_id.as_existing_user_identifier().as_str(),
            )
            .map_err(|_| RoomsApplicationError::Internal)?
            .ok_or(RoomsApplicationError::Forbidden)?;

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
}

impl RoomsService {
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
            .append_membership_event(&event_write_contract)
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

#[allow(dead_code)]
fn _supported_room_versions() -> [SupportedRoomVersion; 2] {
    [SupportedRoomVersion::V1, SupportedRoomVersion::V2]
}
