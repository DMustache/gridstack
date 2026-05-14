use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    infrastructure::server_name,
    infrastructure::user_identifier::UserIdentifier,
    services::rooms::entities::{RoomIdentifier, RoomVersion},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomEventKind {
    State,
    Message,
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateEvent {
    RoomCreate(RoomCreateEvent),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateEvent {
    pub state_key: String,
    pub content: RoomCreateContent,
}

impl RoomCreateEvent {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        if !self.state_key.is_empty() {
            return Err(RoomEventValidationError::InvalidRoomCreateStateKey {
                received_state_key: self.state_key.clone(),
            });
        }

        self.content.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateContent {
    #[serde(default)]
    pub additional_creators: Option<Vec<UserIdentifier>>,
    pub creator: Option<UserIdentifier>,
    #[serde(rename = "m.federate", default = "default_true")]
    pub federate: bool,
    pub predecessor: Option<PreviousRoom>,
    #[serde(default)]
    pub room_version: RoomVersion,
    #[serde(rename = "type")]
    pub room_type: Option<String>,
}

impl RoomCreateContent {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        if self.room_version.supports_additional_creators() {
            if let Some(additional_creators) = self.additional_creators.as_ref()
                && additional_creators.is_empty()
            {
                return Err(RoomEventValidationError::AdditionalCreatorsCannotBeEmpty {
                    room_version: self.room_version,
                });
            }
        } else if self.additional_creators.is_some() {
            return Err(RoomEventValidationError::AdditionalCreatorsNotSupported {
                room_version: self.room_version,
            });
        }

        if self.room_version.requires_creator_field() && self.creator.is_none() {
            return Err(RoomEventValidationError::CreatorRequired {
                room_version: self.room_version,
            });
        }

        if !self.room_version.requires_creator_field() && self.creator.is_some() {
            return Err(RoomEventValidationError::CreatorNotSupported {
                room_version: self.room_version,
            });
        }

        if let Some(predecessor) = self.predecessor.as_ref() {
            predecessor.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
                    server_name::split_opaque_identifier_and_server_name(identifier)
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomV1PersistentDataUnit {
    pub auth_events: Vec<EventReference>,
    pub content: Value,
    pub depth: u64,
    pub event_id: EventIdentifier,
    pub hashes: EventHash,
    pub origin_server_ts: u64,
    pub prev_events: Vec<EventReference>,
    pub redacts: Option<EventIdentifier>,
    pub room_id: RoomIdentifier,
    pub sender: UserIdentifier,
    pub signatures: BTreeMap<String, BTreeMap<String, String>>,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub unsigned: Option<EventUnsignedData>,
}

impl RoomV1PersistentDataUnit {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        const ROOM_V1_MAX_AUTH_EVENTS: usize = 10;
        const ROOM_V1_MAX_PREV_EVENTS: usize = 20;
        const ROOM_V1_MAX_DEPTH: u64 = i64::MAX as u64;

        self.event_id.validate_for_version(RoomVersion::V1)?;
        self.room_id
            .validate_for_room_version(RoomVersion::V1.as_number())?;

        if self.auth_events.len() > ROOM_V1_MAX_AUTH_EVENTS {
            return Err(RoomEventValidationError::TooManyAuthEvents {
                max_allowed: ROOM_V1_MAX_AUTH_EVENTS,
                actual: self.auth_events.len(),
            });
        }

        if self.prev_events.len() > ROOM_V1_MAX_PREV_EVENTS {
            return Err(RoomEventValidationError::TooManyPrevEvents {
                max_allowed: ROOM_V1_MAX_PREV_EVENTS,
                actual: self.prev_events.len(),
            });
        }

        if self.depth > ROOM_V1_MAX_DEPTH {
            return Err(RoomEventValidationError::DepthExceedsAllowedMaximum {
                max_allowed: ROOM_V1_MAX_DEPTH,
                actual: self.depth,
            });
        }

        if self.event_type.trim().is_empty() {
            return Err(RoomEventValidationError::EmptyEventType);
        }

        if self.hashes.sha256.trim().is_empty() {
            return Err(RoomEventValidationError::MissingEventHash { location: "hashes" });
        }

        if self.signatures.is_empty() {
            return Err(RoomEventValidationError::MissingSignatures);
        }

        self.validate_event_references("auth_events", &self.auth_events)?;
        self.validate_event_references("prev_events", &self.prev_events)?;

        if self.event_type == "m.room.create" {
            self.validate_room_v1_create_event_rules()?;
        }

        if self.event_type == "m.room.aliases" {
            self.validate_room_v1_alias_event_rules()?;
        }

        Ok(())
    }

    fn validate_event_references(
        &self,
        reference_list_name: &'static str,
        references: &[EventReference],
    ) -> Result<(), RoomEventValidationError> {
        for (reference_index, reference) in references.iter().enumerate() {
            reference.0.validate_for_version(RoomVersion::V1)?;
            if reference.1.sha256.trim().is_empty() {
                return Err(RoomEventValidationError::MissingEventHashInReference {
                    reference_list_name,
                    reference_index,
                });
            }
        }

        Ok(())
    }

    fn validate_room_v1_create_event_rules(&self) -> Result<(), RoomEventValidationError> {
        if !self.prev_events.is_empty() {
            return Err(
                RoomEventValidationError::CreateEventCannotHavePreviousEvents {
                    actual: self.prev_events.len(),
                },
            );
        }

        let sender_server_name = self.sender.server_name().ok_or_else(|| {
            RoomEventValidationError::InvalidSenderIdentifier {
                sender: self.sender.as_str().to_owned(),
            }
        })?;
        let room_server_name = self.room_id.server_name().ok_or_else(|| {
            RoomEventValidationError::InvalidRoomIdentifierForCreateEvent {
                room_identifier: self.room_id.as_str().to_owned(),
            }
        })?;

        if sender_server_name != room_server_name {
            return Err(
                RoomEventValidationError::CreateEventRoomAndSenderDomainMismatch {
                    sender_domain: sender_server_name.to_owned(),
                    room_domain: room_server_name.to_owned(),
                },
            );
        }

        let create_content: RoomCreateContent = serde_json::from_value(self.content.clone())
            .map_err(
                |error| RoomEventValidationError::InvalidCreateEventContent {
                    reason: error.to_string(),
                },
            )?;

        if create_content.room_version != RoomVersion::V1 {
            return Err(
                RoomEventValidationError::CreateEventMustTargetRoomVersionOne {
                    room_version: create_content.room_version,
                },
            );
        }
        create_content.validate()?;

        Ok(())
    }

    fn validate_room_v1_alias_event_rules(&self) -> Result<(), RoomEventValidationError> {
        let state_key = self
            .state_key
            .as_ref()
            .ok_or(RoomEventValidationError::AliasEventRequiresStateKey)?;
        let sender_server_name = self.sender.server_name().ok_or_else(|| {
            RoomEventValidationError::InvalidSenderIdentifier {
                sender: self.sender.as_str().to_owned(),
            }
        })?;

        if state_key != sender_server_name {
            return Err(
                RoomEventValidationError::AliasEventStateKeyMustMatchSenderDomain {
                    state_key: state_key.to_owned(),
                    sender_domain: sender_server_name.to_owned(),
                },
            );
        }

        Ok(())
    }
}

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
        "room version `{room_version}` does not support `additional_creators` in m.room.create content"
    )]
    AdditionalCreatorsNotSupported { room_version: RoomVersion },
    #[error("room version `{room_version}` requires non-empty `additional_creators` when provided")]
    AdditionalCreatorsCannotBeEmpty { room_version: RoomVersion },
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
        "room version 1 create event content must target room version `1`, received `{room_version}`"
    )]
    CreateEventMustTargetRoomVersionOne { room_version: RoomVersion },
    #[error("room version 1 alias events require state_key")]
    AliasEventRequiresStateKey,
    #[error(
        "room version 1 alias event state_key `{state_key}` must match sender domain `{sender_domain}`"
    )]
    AliasEventStateKeyMustMatchSenderDomain {
        state_key: String,
        sender_domain: String,
    },
    #[error(transparent)]
    InvalidRoomIdentifier(#[from] crate::services::rooms::entities::RoomIdentifierValidationError),
}

fn default_true() -> bool {
    true
}
