use crate::services::rooms::entities::{
    SupportsAdditionalRoomCreators, SupportsKnockRestrictedJoinRule, SupportsKnocking,
    SupportsRestrictedJoinRules, versions::RoomVersionMarker,
};

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version12;

impl RoomVersionMarker for Version12 {
    const VERSION: RoomVersion = RoomVersion::V12;
}

impl SupportsKnocking for Version12 {}

impl SupportsRestrictedJoinRules for Version12 {}

impl SupportsKnockRestrictedJoinRule for Version12 {}

impl SupportsAdditionalRoomCreators for Version12 {}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V12,
        description: "Room IDs are hash-derived, room creators are formalized with infinite power, and state resolution is updated.",
        event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
        room_identifier_format: RoomIdentifierFormat::HashBasedRoomId,
        creator_power_levels: CreatorPowerLevels::InfiniteForRoomCreators,
        supports_knocking: true,
        supports_restricted_join_rules: true,
        supports_knock_restricted_join_rule: true,
        supports_additional_room_creators: true,
        power_levels_must_be_integer_values: true,
    }
}
