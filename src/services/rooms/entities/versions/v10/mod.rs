use crate::services::rooms::entities::{
    SupportsKnockRestrictedJoinRule, SupportsKnocking, SupportsRestrictedJoinRules,
    versions::RoomVersionMarker,
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version10;

impl RoomVersionMarker for Version10 {
    const VERSION: RoomVersion = RoomVersion::V10;
}

impl SupportsKnocking for Version10 {}

impl SupportsRestrictedJoinRules for Version10 {}

impl SupportsKnockRestrictedJoinRule for Version10 {}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V10,
        description: "Requires integer-only power levels and adds knock_restricted join rule.",
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
