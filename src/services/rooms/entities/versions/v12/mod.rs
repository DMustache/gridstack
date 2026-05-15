use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

pub fn rules() -> RoomVersionRules {
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
