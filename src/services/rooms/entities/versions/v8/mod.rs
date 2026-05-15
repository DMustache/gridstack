use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

pub fn rules() -> RoomVersionRules {
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
