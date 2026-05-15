use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

pub fn rules() -> RoomVersionRules {
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
