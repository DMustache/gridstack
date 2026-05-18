use std::{str::FromStr, sync::Arc};
use tracing::error;

use crate::{
    infrastructure::server_name::ServerName,
    services::{
        authorization::entities::AuthorizedUserIdentifier,
        authorization::service::AuthorizationService,
        errors::DomainError,
        events::{
            entities::{EventIntent, EventOriginKind, RoomEventIntentPlan},
            service::{EventCompilationError, EventsService},
        },
        rooms::{
            entities::{CreateRoomStateEventPayload, CreatedRoom, Room, versions::RoomVersion},
            errors::RoomsApplicationError,
            handlers::create_room::CreateRoomInfo,
            persistence::RoomRepository,
            service::create_room::ValidatedCreateRoomRequest,
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
        request: CreateRoomInfo,
    ) -> Result<CreatedRoom, RoomsApplicationError> {
        let info =
            ValidatedCreateRoomRequest::try_from_request(request, &self.authorization_service)?;

        let contract = Room::try_create_contract(creator_user_id, info, &self.server_name)
            .map_err(|error| {
                error!(error = %error, "failed to create room contract");
                RoomsApplicationError::from(error)
            })?;

        let room_event_intent_plan = build_room_event_intent_plan(
            &contract.persistence_payload.room_id,
            &contract.persistence_payload.room_version,
            &contract.persistence_payload.initial_state,
            creator_user_id.as_user_identifier().as_str(),
        )?;
        let event_batch_write_contract = self
            .events_service
            .compile_event_batch(room_event_intent_plan)
            .map_err(map_event_compilation_error)?;

        if let Some(room_alias_name) = contract.persistence_payload.room_alias_name.as_deref()
            && !self
                .room_repository
                .reserve_room_alias(room_alias_name)
                .map_err(|error| {
                    error!(error = %error, "failed to reserve room alias");
                    RoomsApplicationError::Internal
                })?
        {
            return Err(RoomsApplicationError::RoomInUse);
        }

        self.room_repository
            .save_room(&contract.persistence_payload, &event_batch_write_contract)
            .map_err(|error| {
                error!(error = %error, "failed to persist room");
                if matches!(error, DomainError::AlreadyExists) {
                    RoomsApplicationError::RoomInUse
                } else {
                    RoomsApplicationError::Internal
                }
            })?;

        Ok(CreatedRoom {
            room_id: contract.persistence_payload.room_id,
        })
    }
}

fn build_room_event_intent_plan(
    room_id: &str,
    room_version: &str,
    planned_room_state_events: &[CreateRoomStateEventPayload],
    creator_user_id: &str,
) -> Result<RoomEventIntentPlan, RoomsApplicationError> {
    let parsed_room_version = RoomVersion::from_str(room_version)
        .map_err(|_| RoomsApplicationError::UnsupportedRoomVersion)?;
    let intents = planned_room_state_events
        .iter()
        .map(|event| EventIntent {
            room_id: room_id.to_owned(),
            sender: creator_user_id.to_owned(),
            event_type: event.event_type.clone(),
            state_key: Some(event.state_key.clone()),
            content: event.content.clone(),
            unsigned: None,
            origin_kind: infer_origin_kind(event),
            transaction_id: None,
            client_visible: true,
        })
        .collect::<Vec<_>>();

    Ok(RoomEventIntentPlan {
        room_id: room_id.to_owned(),
        room_version: parsed_room_version,
        intents,
    })
}

fn infer_origin_kind(event: &CreateRoomStateEventPayload) -> EventOriginKind {
    match event.event_type.as_str() {
        "m.room.create" => EventOriginKind::CreateRoom,
        "m.room.power_levels" => EventOriginKind::DefaultPowerLevels,
        "m.room.canonical_alias" => EventOriginKind::CanonicalAlias,
        "m.room.name" => EventOriginKind::Name,
        "m.room.topic" => EventOriginKind::Topic,
        "m.room.member"
            if event
                .content
                .get("membership")
                .and_then(|value| value.as_str())
                == Some("join") =>
        {
            EventOriginKind::CreatorJoin
        }
        "m.room.member"
            if event
                .content
                .get("membership")
                .and_then(|value| value.as_str())
                == Some("invite") =>
        {
            EventOriginKind::Invite
        }
        "m.room.member" => EventOriginKind::MembershipChange,
        _ => EventOriginKind::InitialState,
    }
}

fn map_event_compilation_error(error: EventCompilationError) -> RoomsApplicationError {
    match error {
        EventCompilationError::UnsupportedRoomVersion { .. } => {
            RoomsApplicationError::UnsupportedRoomVersion
        }
        EventCompilationError::InvalidEventType
        | EventCompilationError::InvalidEventContentShape { .. }
        | EventCompilationError::MissingStateKeyForStateEvent { .. }
        | EventCompilationError::CreateEventMustBeFirst
        | EventCompilationError::CreatorJoinMustBeSecond
        | EventCompilationError::MissingSenderMembershipForAuth { .. }
        | EventCompilationError::EmptyIntentPlan => RoomsApplicationError::InvalidRoomState,
    }
}
