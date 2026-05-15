use crate::services::rooms::entities::RoomVersionRulesContract;

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

pub type RoomVersionRulesV1 = RoomVersionRules;

impl RoomVersionRulesContract for RoomVersionRulesV1 {
    fn rules() -> RoomVersionRules {
        Self {
            version: RoomVersion::V1,
            description: "Initial room version.",
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
}
