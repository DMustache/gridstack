use crate::services::rooms::entities::{
    SupportsKnocking, SupportsRestrictedJoinRules, versions::RoomVersionMarker,
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version8;

impl RoomVersionMarker for Version8 {
    const VERSION: RoomVersion = RoomVersion::V8;
}

impl SupportsKnocking for Version8 {}

impl SupportsRestrictedJoinRules for Version8 {}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V8,
        description: "Adds restricted join rules based on membership in another room.",
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
