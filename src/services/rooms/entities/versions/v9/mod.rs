use crate::services::rooms::entities::{
    SupportsKnocking, SupportsRestrictedJoinRules, versions::RoomVersionMarker,
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version9;

impl RoomVersionMarker for Version9 {
    const VERSION: RoomVersion = RoomVersion::V9;
}

impl SupportsKnocking for Version9 {}

impl SupportsRestrictedJoinRules for Version9 {}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V9,
        description: "Builds on v8 and fixes membership redaction edge cases.",
        event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
        room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
        creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
        supports_knocking: true,
        supports_restricted_join_rules: true,
        supports_knock_restricted_join_rule: false,
        supports_additional_room_creators: false,
        power_levels_must_be_integer_values: false,
    }
}
