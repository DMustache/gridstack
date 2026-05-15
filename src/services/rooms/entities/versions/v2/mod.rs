use serde::{Deserialize, Serialize};

use crate::services::{
    events::entities::RoomEventValidationError,
    rooms::entities::versions::v1::RoomV1PersistentDataUnit,
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V2,
        description: "Implements State Resolution Version 2.",
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomV2PersistentDataUnit {
    pub inner: RoomV1PersistentDataUnit,
}

impl RoomV2PersistentDataUnit {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        const ROOM_V2_MAX_AUTH_EVENTS: usize = 10;
        const ROOM_V2_MAX_PREV_EVENTS: usize = 20;
        const ROOM_V2_MAX_DEPTH: u64 = i64::MAX as u64;

        self.inner.event_id.validate_for_version(RoomVersion::V2)?;
        self.inner
            .room_id
            .validate_for_room_version(RoomVersion::V2.as_number())?;

        if self.inner.auth_events.len() > ROOM_V2_MAX_AUTH_EVENTS {
            return Err(RoomEventValidationError::TooManyAuthEvents {
                max_allowed: ROOM_V2_MAX_AUTH_EVENTS,
                actual: self.inner.auth_events.len(),
            });
        }

        if self.inner.prev_events.len() > ROOM_V2_MAX_PREV_EVENTS {
            return Err(RoomEventValidationError::TooManyPrevEvents {
                max_allowed: ROOM_V2_MAX_PREV_EVENTS,
                actual: self.inner.prev_events.len(),
            });
        }

        if self.inner.depth > ROOM_V2_MAX_DEPTH {
            return Err(RoomEventValidationError::DepthExceedsAllowedMaximum {
                max_allowed: ROOM_V2_MAX_DEPTH,
                actual: self.inner.depth,
            });
        }

        if self.inner.event_type.trim().is_empty() {
            return Err(RoomEventValidationError::EmptyEventType);
        }

        if self.inner.hashes.sha256.trim().is_empty() {
            return Err(RoomEventValidationError::MissingEventHash { location: "hashes" });
        }

        if self.inner.signatures.is_empty() {
            return Err(RoomEventValidationError::MissingSignatures);
        }

        RoomV1PersistentDataUnit::validate_event_references(
            "auth_events",
            &self.inner.auth_events,
        )?;
        RoomV1PersistentDataUnit::validate_event_references(
            "prev_events",
            &self.inner.prev_events,
        )?;

        if self.inner.event_type == "m.room.create" {
            self.inner.validate_create_event_rules(RoomVersion::V2)?;
        }

        if self.inner.event_type == "m.room.aliases" {
            self.inner.validate_room_v1_alias_event_rules()?;
        }

        Ok(())
    }
}
