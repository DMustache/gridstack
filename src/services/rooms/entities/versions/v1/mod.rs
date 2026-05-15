use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::{
        events::entities::{
            EventHash, EventIdentifier, EventReference, EventUnsignedData,
            RoomEventValidationError, event_kinds::state_events::create::RoomCreateContent,
        },
        rooms::entities::{RoomIdentifier, RoomVersionRulesContract},
    },
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

pub type RoomVersionRulesV1 = RoomVersionRules;

impl RoomVersionRulesContract for RoomVersionRulesV1 {
    fn rules() -> RoomVersionRules {
        Self {
            version: RoomVersion::V1,
            description: "Initial room version.",
            event_identifier_format: EventIdentifierFormat::ServerAssignedWithDomain,
            room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
            creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
            supports_knocking: false,
            supports_restricted_join_rules: false,
            supports_knock_restricted_join_rule: false,
            supports_additional_room_creators: false,
            power_levels_must_be_integer_values: false,
        }
    }
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

        Self::validate_event_references("auth_events", &self.auth_events)?;
        Self::validate_event_references("prev_events", &self.prev_events)?;

        if self.event_type == "m.room.create" {
            self.validate_create_event_rules(RoomVersion::V1)?;
        }

        if self.event_type == "m.room.aliases" {
            self.validate_room_v1_alias_event_rules()?;
        }

        Ok(())
    }

    pub(super) fn validate_event_references(
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

    pub(super) fn validate_create_event_rules(
        &self,
        expected_room_version: RoomVersion,
    ) -> Result<(), RoomEventValidationError> {
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

        if create_content.room_version != expected_room_version {
            return Err(RoomEventValidationError::CreateEventMustTargetRoomVersion {
                expected_room_version,
                room_version: create_content.room_version,
            });
        }
        create_content.validate()?;

        Ok(())
    }

    pub(super) fn validate_room_v1_alias_event_rules(
        &self,
    ) -> Result<(), RoomEventValidationError> {
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
