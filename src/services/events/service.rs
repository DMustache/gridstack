use std::sync::Arc;

use thiserror::Error;
use uuid::Uuid;

use crate::infrastructure::server_name::ServerName;
use crate::services::events::entities::{
    EventBatchWriteContract, EventDraft, EventHashes, EventIntent, EventKind, EventOriginKind,
    EventSignature, EventWriteContract, MatrixEventContent, PersistedEvent, RoomEventFlow,
    RoomVersionDefinition, RoomVersionRegistry, StateEventKind, SupportedRoomVersion,
};
use crate::services::events::validator::{EventIntentValidationError, validate_event_intent};
use crate::services::traits::Clock;

mod event_write_contract_builder;
use event_write_contract_builder::EventWriteContractBuilder;

#[derive(Clone)]
pub struct EventsService {
    room_version_registry: RoomVersionRegistry,
    server_name: ServerName,
    clock: Arc<dyn Clock>,
}

impl EventsService {
    pub fn new(server_name: &ServerName, clock: Arc<dyn Clock>) -> Self {
        Self {
            room_version_registry: RoomVersionRegistry::default(),
            server_name: server_name.clone(),
            clock,
        }
    }

    pub const fn default_room_version(&self) -> SupportedRoomVersion {
        self.room_version_registry.default_room_version()
    }

    pub fn room_version_is_supported(&self, room_version: SupportedRoomVersion) -> bool {
        self.room_version_registry.resolve(room_version).is_some()
    }

    pub fn compile_room_event_flow(
        &self,
        room_event_flow: RoomEventFlow,
    ) -> Result<EventBatchWriteContract, EventCompilationError> {
        self.compile_event_flow(room_event_flow)
    }

    pub fn compile_single_event_intent(
        &self,
        room_id: String,
        room_version: SupportedRoomVersion,
        event_intent: EventIntent,
    ) -> Result<EventWriteContract, EventCompilationError> {
        let batch = self.compile_event_flow(RoomEventFlow {
            room_id: room_id.clone(),
            room_version,
            intents: vec![event_intent],
        })?;

        batch.event_write_contracts.into_iter().next().ok_or(
            EventCompilationError::EmptyIntentPlan {
                room_id,
                room_version,
            },
        )
    }

    pub fn create_membership_event(
        &self,
        room_id: String,
        room_version: SupportedRoomVersion,
        sender: String,
        target_user_id: String,
        content: MatrixEventContent,
        transaction_id: Option<String>,
    ) -> Result<EventWriteContract, EventCompilationError> {
        let mut event_intent = EventIntent::state(
            room_id.clone(),
            sender,
            StateEventKind::RoomMember,
            target_user_id,
            content,
            EventOriginKind::MembershipChange,
        );
        event_intent.transaction_id = transaction_id;
        self.compile_single_event_intent(room_id, room_version, event_intent)
    }

    pub fn create_state_event(
        &self,
        room_id: String,
        room_version: SupportedRoomVersion,
        sender: String,
        state_event_kind: StateEventKind,
        state_key: String,
        content: MatrixEventContent,
        transaction_id: Option<String>,
    ) -> Result<EventWriteContract, EventCompilationError> {
        let mut event_intent = EventIntent::state(
            room_id.clone(),
            sender,
            state_event_kind,
            state_key,
            content,
            EventOriginKind::StateSet,
        );
        event_intent.transaction_id = transaction_id;
        self.compile_single_event_intent(room_id, room_version, event_intent)
    }

    pub fn create_message_event(
        &self,
        room_id: String,
        room_version: SupportedRoomVersion,
        sender: String,
        message_event_kind: crate::services::events::entities::MessageEventKind,
        content: MatrixEventContent,
        transaction_id: Option<String>,
    ) -> Result<EventWriteContract, EventCompilationError> {
        let event_intent = EventIntent {
            room_id: room_id.clone(),
            sender,
            event_kind: EventKind::Message(message_event_kind),
            state_key: None,
            content,
            unsigned: None,
            origin: EventOriginKind::MessageSend,
            transaction_id,
        };
        self.compile_single_event_intent(room_id, room_version, event_intent)
    }

    fn compile_event_flow(
        &self,
        room_event_flow: RoomEventFlow,
    ) -> Result<EventBatchWriteContract, EventCompilationError> {
        if room_event_flow.intents.is_empty() {
            return Err(EventCompilationError::EmptyIntentPlan {
                room_id: room_event_flow.room_id,
                room_version: room_event_flow.room_version,
            });
        }

        let room_version_definition = self
            .room_version_registry
            .resolve(room_event_flow.room_version)
            .ok_or(EventCompilationError::UnsupportedRoomVersion {
                room_version: room_event_flow.room_version.to_string(),
            })?;

        let mut event_write_contracts = Vec::with_capacity(room_event_flow.intents.len());
        let mut compilation_state = CompilationState::new();
        let is_room_bootstrap_flow = room_event_flow
            .intents
            .first()
            .map(|event_intent| {
                event_intent.event_type() == "m.room.create"
                    && event_intent.state_key.as_deref() == Some("")
            })
            .unwrap_or(false);

        for (event_index, event_intent) in room_event_flow.intents.iter().enumerate() {
            validate_event_intent(event_intent)
                .map_err(map_validation_error_to_compilation_error)?;

            let prev_events = self.select_prev_events(&compilation_state, &room_version_definition);
            let auth_events =
                self.select_auth_events(event_intent, &compilation_state, &room_version_definition);
            let depth = self.select_depth(&prev_events, &compilation_state);

            let event_draft = EventDraft {
                room_id: event_intent.room_id.clone(),
                room_version: room_event_flow.room_version,
                sender: event_intent.sender.clone(),
                event_type: event_intent.event_type().to_owned(),
                state_key: event_intent.state_key.clone(),
                content: event_intent.content.clone(),
                unsigned: event_intent.unsigned.clone(),
                prev_events: prev_events.clone(),
                auth_events: auth_events.clone(),
                depth,
                origin_server_ts: self.clock.now_unix_milliseconds() as u64,
                redacts: None,
            };

            self.authorize_event(
                event_index,
                &event_draft,
                &compilation_state,
                is_room_bootstrap_flow,
            )?;

            let persisted_event = self.persist_event(event_draft, room_event_flow.room_version);
            let event_write_contract = self.plan_event_write_contract(
                event_intent,
                &persisted_event,
                prev_events,
                auth_events,
                &mut compilation_state,
            );
            event_write_contracts.push(event_write_contract);
        }

        Ok(EventBatchWriteContract {
            room_id: room_event_flow.room_id,
            room_version: room_event_flow.room_version,
            event_write_contracts,
        })
    }

    fn select_prev_events(
        &self,
        compilation_state: &CompilationState,
        room_version_definition: &RoomVersionDefinition,
    ) -> Vec<String> {
        let mut prev_events = compilation_state
            .accepted_event_ids
            .last()
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        prev_events.truncate(room_version_definition.max_prev_events);
        prev_events
    }

    fn select_auth_events(
        &self,
        event_intent: &EventIntent,
        compilation_state: &CompilationState,
        room_version_definition: &RoomVersionDefinition,
    ) -> Vec<String> {
        let mut auth_events = Vec::new();

        if let Some(create_event_id) = compilation_state.current_state_event_id("m.room.create", "")
        {
            auth_events.push(create_event_id);
        }
        if let Some(power_levels_event_id) =
            compilation_state.current_state_event_id("m.room.power_levels", "")
        {
            auth_events.push(power_levels_event_id);
        }
        if let Some(sender_membership_event_id) =
            compilation_state.current_state_event_id("m.room.member", &event_intent.sender)
        {
            auth_events.push(sender_membership_event_id);
        }

        auth_events.truncate(room_version_definition.max_auth_events);
        auth_events
    }

    fn select_depth(&self, prev_events: &[String], compilation_state: &CompilationState) -> u64 {
        prev_events
            .first()
            .and_then(|event_id| compilation_state.event_depth(event_id))
            .unwrap_or(0)
            + 1
    }

    fn authorize_event(
        &self,
        event_index: usize,
        event_draft: &EventDraft,
        compilation_state: &CompilationState,
        is_room_bootstrap_flow: bool,
    ) -> Result<(), EventCompilationError> {
        if event_index == 0 && !is_room_bootstrap_flow {
            return Ok(());
        }

        if event_index == 0 {
            if event_draft.event_type != "m.room.create"
                || event_draft.state_key.as_deref() != Some("")
            {
                return Err(EventCompilationError::CreateEventMustBeFirst);
            }
            return Ok(());
        }

        if event_index == 1 && is_room_bootstrap_flow {
            if event_draft.event_type != "m.room.member"
                || event_draft.state_key.as_deref() != Some(event_draft.sender.as_str())
            {
                return Err(EventCompilationError::CreatorJoinMustBeSecond);
            }
            return Ok(());
        }

        let sender_membership =
            compilation_state.current_state_event_id("m.room.member", &event_draft.sender);
        if sender_membership.is_none() {
            return Err(EventCompilationError::MissingSenderMembershipForAuth {
                sender: event_draft.sender.clone(),
                event_type: event_draft.event_type.clone(),
            });
        }

        Ok(())
    }

    fn persist_event(
        &self,
        event_draft: EventDraft,
        room_version: SupportedRoomVersion,
    ) -> PersistedEvent {
        let event_id = format!("${}:{}", Uuid::new_v4().simple(), self.server_name.as_str());
        PersistedEvent {
            event_id,
            room_id: event_draft.room_id,
            room_version,
            sender: event_draft.sender,
            event_type: event_draft.event_type,
            state_key: event_draft.state_key,
            content: event_draft.content,
            unsigned: event_draft.unsigned,
            prev_events: event_draft.prev_events,
            auth_events: event_draft.auth_events,
            depth: event_draft.depth,
            origin_server_ts: event_draft.origin_server_ts,
            hashes: EventHashes {
                sha256: Uuid::new_v4().simple().to_string(),
            },
            signatures: vec![EventSignature {
                server_name: self.server_name.as_str().to_owned(),
                key_id: "ed25519:auto".to_owned(),
                signature: Uuid::new_v4().simple().to_string(),
            }],
            rejected: false,
            soft_failed: false,
        }
    }

    fn plan_event_write_contract(
        &self,
        event_intent: &EventIntent,
        persisted_event: &PersistedEvent,
        prev_events: Vec<String>,
        auth_events: Vec<String>,
        compilation_state: &mut CompilationState,
    ) -> EventWriteContract {
        EventWriteContractBuilder::new(
            event_intent,
            persisted_event,
            prev_events,
            auth_events,
            compilation_state,
        )
        .build()
    }
}

fn map_validation_error_to_compilation_error(
    error: EventIntentValidationError,
) -> EventCompilationError {
    match error {
        EventIntentValidationError::InvalidEventType => EventCompilationError::InvalidEventType,
        EventIntentValidationError::InvalidEventContentShape { event_type } => {
            EventCompilationError::InvalidEventContentShape { event_type }
        }
        EventIntentValidationError::MissingStateKeyForStateEvent { event_type } => {
            EventCompilationError::MissingStateKeyForStateEvent { event_type }
        }
        EventIntentValidationError::MessageEventCannotHaveStateKey { event_type } => {
            EventCompilationError::MessageEventCannotHaveStateKey { event_type }
        }
        EventIntentValidationError::StateKeyMustBeEmpty { event_type } => {
            EventCompilationError::StateKeyMustBeEmpty { event_type }
        }
        EventIntentValidationError::InvalidMembershipStateKey { state_key } => {
            EventCompilationError::InvalidMembershipStateKey { state_key }
        }
    }
}

struct CompilationState {
    accepted_event_ids: Vec<String>,
    current_state: Vec<CurrentStateEntry>,
    depth_by_event_id: Vec<EventDepthEntry>,
    stream_position: i64,
}

impl CompilationState {
    fn new() -> Self {
        Self {
            accepted_event_ids: Vec::new(),
            current_state: Vec::new(),
            depth_by_event_id: Vec::new(),
            stream_position: 0,
        }
    }

    fn current_state_event_id(&self, event_type: &str, state_key: &str) -> Option<String> {
        self.current_state
            .iter()
            .find(|entry| entry.event_type == event_type && entry.state_key == state_key)
            .map(|entry| entry.event_id.clone())
    }

    fn set_current_state_event_id(
        &mut self,
        event_type: String,
        state_key: String,
        event_id: String,
    ) {
        if let Some(entry) = self
            .current_state
            .iter_mut()
            .find(|entry| entry.event_type == event_type && entry.state_key == state_key)
        {
            entry.event_id = event_id;
            return;
        }
        self.current_state.push(CurrentStateEntry {
            event_type,
            state_key,
            event_id,
        });
    }

    fn event_depth(&self, event_id: &str) -> Option<u64> {
        self.depth_by_event_id
            .iter()
            .find(|entry| entry.event_id == event_id)
            .map(|entry| entry.depth)
    }

    fn set_event_depth(&mut self, event_id: String, depth: u64) {
        if let Some(entry) = self
            .depth_by_event_id
            .iter_mut()
            .find(|entry| entry.event_id == event_id)
        {
            entry.depth = depth;
            return;
        }
        self.depth_by_event_id
            .push(EventDepthEntry { event_id, depth });
    }
}

struct CurrentStateEntry {
    event_type: String,
    state_key: String,
    event_id: String,
}

struct EventDepthEntry {
    event_id: String,
    depth: u64,
}

#[derive(Debug, Error)]
pub enum EventCompilationError {
    #[error("event intent plan is empty for room `{room_id}` / version `{room_version}`")]
    EmptyIntentPlan {
        room_id: String,
        room_version: SupportedRoomVersion,
    },
    #[error("unsupported room version `{room_version}` for event compilation")]
    UnsupportedRoomVersion { room_version: String },
    #[error("event type cannot be empty")]
    InvalidEventType,
    #[error("event `{event_type}` must have object content")]
    InvalidEventContentShape { event_type: String },
    #[error("state event `{event_type}` must include state_key")]
    MissingStateKeyForStateEvent { event_type: String },
    #[error("message event `{event_type}` cannot include state_key")]
    MessageEventCannotHaveStateKey { event_type: String },
    #[error("state event `{event_type}` requires empty state_key")]
    StateKeyMustBeEmpty { event_type: String },
    #[error("m.room.member state_key `{state_key}` is not a valid user identifier")]
    InvalidMembershipStateKey { state_key: String },
    #[error("m.room.create with empty state_key must be first event in a room")]
    CreateEventMustBeFirst,
    #[error("creator join membership must be second event in a room creation sequence")]
    CreatorJoinMustBeSecond,
    #[error("missing joined membership for sender `{sender}` when authorizing `{event_type}`")]
    MissingSenderMembershipForAuth { sender: String, event_type: String },
}
