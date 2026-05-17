use crate::services::rooms::entities::{
    SupportsKnockRestrictedJoinRule, SupportsKnocking, SupportsRestrictedJoinRules,
    versions::RoomVersionMarker,
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version11;

impl RoomVersionMarker for Version11 {
    const VERSION: RoomVersion = RoomVersion::V11;
}

impl SupportsKnocking for Version11 {}

impl SupportsRestrictedJoinRules for Version11 {}

impl SupportsKnockRestrictedJoinRule for Version11 {}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V11,
        description: "Clarifies redaction algorithm behavior.",
        event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
        room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
        creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
        supports_knocking: true,
        supports_restricted_join_rules: true,
        supports_knock_restricted_join_rule: true,
        supports_additional_room_creators: false,
        power_levels_must_be_integer_values: true,
    }
}
