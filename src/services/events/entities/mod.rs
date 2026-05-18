use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    infrastructure::{server_name::ServerName, user_identifier::UserIdentifier},
    services::rooms::entities::{RoomIdentifier, versions::RoomVersion},
};

pub mod event_kinds;

use event_kinds::RoomEventKind;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomEvent {
    pub content: Value,
    pub event_id: EventIdentifier,
    pub kind: RoomEventKind,
    pub origin_server_ts: u64,
    pub room_id: RoomIdentifier,
    pub sender: UserIdentifier,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub unsigned: Option<Value>,
}

impl RoomEvent {
    pub fn validate(&self, room_version: RoomVersion) -> Result<(), RoomEventValidationError> {
        self.event_id.validate_for_version(room_version)?;
        self.room_id
            .validate_for_room_version(room_version.as_number())?;

        if self.kind == RoomEventKind::State && self.state_key.is_none() {
            return Err(RoomEventValidationError::MissingStateKeyForStateEvent {
                event_type: self.event_type.clone(),
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventIdentifier(String);

impl EventIdentifier {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        if !candidate.starts_with('$') || candidate.len() < 2 {
            return None;
        }

        Some(Self(candidate))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate_for_version(
        &self,
        room_version: RoomVersion,
    ) -> Result<(), RoomEventValidationError> {
        match room_version {
            RoomVersion::V1 | RoomVersion::V2 => {
                let suffix = self.0.strip_prefix('$').and_then(|identifier| {
                    ServerName::split_opaque_identifier_and_server_name(identifier)
                });
                if suffix.is_none() {
                    return Err(RoomEventValidationError::EventIdentifierMissingDomain {
                        room_version,
                        event_identifier: self.0.clone(),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousRoom {
    pub event_id: EventIdentifier,
    pub room_id: RoomIdentifier,
}

impl PreviousRoom {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        self.room_id.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageEvent {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventHash {
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReference(pub EventIdentifier, pub EventHash);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventUnsignedData {
    pub age: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateEventTypeAndKey {
    pub event_type: String,
    pub state_key: String,
}

pub type RoomStateSnapshot = BTreeMap<StateEventTypeAndKey, EventIdentifier>;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RoomEventValidationError {
    #[error("state event `{event_type}` must include a state_key")]
    MissingStateKeyForStateEvent { event_type: String },
    #[error("m.room.create state_key must be empty, got `{received_state_key}`")]
    InvalidRoomCreateStateKey { received_state_key: String },
    #[error("room version `{room_version}` requires `creator` in m.room.create content")]
    CreatorRequired { room_version: RoomVersion },
    #[error("room version `{room_version}` does not support `creator` in m.room.create content")]
    CreatorNotSupported { room_version: RoomVersion },
    #[error(
        "room version `{room_version}` has invalid `additional_creators` user id `{user_identifier}` in m.room.create content"
    )]
    InvalidAdditionalCreatorUserIdentifier {
        room_version: RoomVersion,
        user_identifier: String,
    },
    #[error(
        "event identifier `{event_identifier}` in room version `{room_version}` must include a valid server-name domain"
    )]
    EventIdentifierMissingDomain {
        room_version: RoomVersion,
        event_identifier: String,
    },
    #[error(
        "room version 1 allows at most `{max_allowed}` auth events, received `{actual}` auth events"
    )]
    TooManyAuthEvents { max_allowed: usize, actual: usize },
    #[error(
        "room version 1 allows at most `{max_allowed}` previous events, received `{actual}` previous events"
    )]
    TooManyPrevEvents { max_allowed: usize, actual: usize },
    #[error("room version 1 requires depth <= `{max_allowed}` (2^63 - 1), received `{actual}`")]
    DepthExceedsAllowedMaximum { max_allowed: u64, actual: u64 },
    #[error("event type cannot be empty")]
    EmptyEventType,
    #[error("missing or empty sha256 hash in `{location}`")]
    MissingEventHash { location: &'static str },
    #[error("missing or empty sha256 hash in `{reference_list_name}` at index `{reference_index}`")]
    MissingEventHashInReference {
        reference_list_name: &'static str,
        reference_index: usize,
    },
    #[error("event signatures cannot be empty")]
    MissingSignatures,
    #[error("room version 1 create event cannot have prev_events, received `{actual}`")]
    CreateEventCannotHavePreviousEvents { actual: usize },
    #[error("invalid sender user id `{sender}`")]
    InvalidSenderIdentifier { sender: String },
    #[error("invalid room id `{room_identifier}` for m.room.create in room version 1")]
    InvalidRoomIdentifierForCreateEvent { room_identifier: String },
    #[error(
        "room version 1 create event requires sender and room domains to match, got sender `{sender_domain}` and room `{room_domain}`"
    )]
    CreateEventRoomAndSenderDomainMismatch {
        sender_domain: String,
        room_domain: String,
    },
    #[error("invalid m.room.create content: {reason}")]
    InvalidCreateEventContent { reason: String },
    #[error(
        "create event content must target room version `{expected_room_version}`, received `{room_version}`"
    )]
    CreateEventMustTargetRoomVersion {
        expected_room_version: RoomVersion,
        room_version: RoomVersion,
    },
    #[error("room version 1 alias events require state_key")]
    AliasEventRequiresStateKey,
    #[error(
        "room version 1 alias event state_key `{state_key}` must match sender domain `{sender_domain}`"
    )]
    AliasEventStateKeyMustMatchSenderDomain {
        state_key: String,
        sender_domain: String,
    },
    #[error(
        "state resolution input is missing event `{event_identifier}` referenced by state/auth edges"
    )]
    MissingRoomV2ResolutionEvent { event_identifier: String },
    #[error(transparent)]
    InvalidRoomIdentifier(#[from] crate::services::rooms::entities::RoomIdentifierValidationError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOriginKind {
    CreateRoom,
    CreatorJoin,
    DefaultPowerLevels,
    CanonicalAlias,
    PresetState,
    InitialState,
    Name,
    Topic,
    Invite,
    MessageSend,
    StateSet,
    MembershipChange,
    FederationReceive,
    Redaction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventIntent {
    pub room_id: String,
    pub sender: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub origin_kind: EventOriginKind,
    pub transaction_id: Option<String>,
    pub client_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomEventIntentPlan {
    pub room_id: String,
    pub room_version: RoomVersion,
    pub intents: Vec<EventIntent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventDraft {
    pub room_id: String,
    pub room_version: RoomVersion,
    pub sender: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub prev_events: Vec<String>,
    pub auth_events: Vec<String>,
    pub depth: u64,
    pub origin_server_ts: u64,
    pub redacts: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedEvent {
    pub event_id: String,
    pub room_id: String,
    pub room_version: RoomVersion,
    pub sender: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub prev_events: Vec<String>,
    pub auth_events: Vec<String>,
    pub depth: u64,
    pub origin_server_ts: u64,
    pub hashes: HashMap<String, String>,
    pub signatures: HashMap<String, HashMap<String, String>>,
    pub rejected: bool,
    pub soft_failed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventGraphEdge {
    pub room_id: String,
    pub event_id: String,
    pub linked_event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentStateUpdate {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipState {
    Join,
    Invite,
    Leave,
    Ban,
    Knock,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipProjectionUpdate {
    pub room_id: String,
    pub user_id: String,
    pub membership: MembershipState,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineProjectionUpdate {
    pub room_id: String,
    pub event_id: String,
    pub stream_position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStreamUpdate {
    pub room_id: String,
    pub event_id: String,
    pub stream_position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxTaskType {
    NotifyLocalUsers,
    WakeSyncWaiters,
    IndexEventForSearch,
    FederateEvent,
    FederateInvite,
    PushNotification,
    AppserviceTransaction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxTask {
    pub room_id: String,
    pub event_id: String,
    pub task_type: OutboxTaskType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    pub room_id: String,
    pub sender_user_id: String,
    pub transaction_id: String,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventWriteContract {
    pub events_to_insert: Vec<PersistedEvent>,
    pub prev_edges_to_insert: Vec<EventGraphEdge>,
    pub auth_edges_to_insert: Vec<EventGraphEdge>,
    pub forward_extremity_updates: Vec<String>,
    pub current_state_updates: Vec<CurrentStateUpdate>,
    pub membership_projection_updates: Vec<MembershipProjectionUpdate>,
    pub timeline_projection_updates: Vec<TimelineProjectionUpdate>,
    pub sync_stream_updates: Vec<SyncStreamUpdate>,
    pub outbox_tasks: Vec<OutboxTask>,
    pub idempotency_records: Vec<IdempotencyRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventBatchWriteContract {
    pub room_id: String,
    pub room_version: RoomVersion,
    pub event_write_contracts: Vec<EventWriteContract>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomVersionDefinition {
    pub version_id: RoomVersion,
    pub state_resolution_version: u8,
    pub event_identifier_format: crate::services::rooms::entities::EventIdentifierFormat,
    pub room_identifier_format: crate::services::rooms::entities::RoomIdentifierFormat,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RoomVersionRegistry;

impl RoomVersionRegistry {
    pub const fn default_room_version(self) -> RoomVersion {
        RoomVersion::V1
    }

    pub fn resolve(self, version: RoomVersion) -> Option<RoomVersionDefinition> {
        match version {
            RoomVersion::V1 => Some(RoomVersionDefinition {
                version_id: RoomVersion::V1,
                state_resolution_version: 1,
                event_identifier_format:
                    crate::services::rooms::entities::EventIdentifierFormat::ServerAssignedWithDomain,
                room_identifier_format:
                    crate::services::rooms::entities::RoomIdentifierFormat::LocalPartWithDomain,
            }),
            RoomVersion::V2 => Some(RoomVersionDefinition {
                version_id: RoomVersion::V2,
                // v2 reuses v1 behavior and only overrides state resolution.
                state_resolution_version: 2,
                event_identifier_format:
                    crate::services::rooms::entities::EventIdentifierFormat::ServerAssignedWithDomain,
                room_identifier_format:
                    crate::services::rooms::entities::RoomIdentifierFormat::LocalPartWithDomain,
            }),
            RoomVersion::V3
            | RoomVersion::V4
            | RoomVersion::V5
            | RoomVersion::V6
            | RoomVersion::V7
            | RoomVersion::V8
            | RoomVersion::V9
            | RoomVersion::V10
            | RoomVersion::V11
            | RoomVersion::V12 => None,
        }
    }
}
