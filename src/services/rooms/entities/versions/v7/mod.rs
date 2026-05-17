use crate::services::rooms::entities::{SupportsKnocking, versions::RoomVersionMarker};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version7;

impl RoomVersionMarker for Version7 {
    const VERSION: RoomVersion = RoomVersion::V7;
}

impl SupportsKnocking for Version7 {}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V7,
        description: "Introduces knocking membership and join behavior.",
        event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
        room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
        creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
        supports_knocking: true,
        supports_restricted_join_rules: false,
        supports_knock_restricted_join_rule: false,
        supports_additional_room_creators: false,
        power_levels_must_be_integer_values: false,
    }
}
