use crate::services::rooms::entities::versions::RoomVersionMarker;

use super::super::{
    CreatorPowerLevels, EventIdentifierFormat, RoomIdentifierFormat, RoomVersion, RoomVersionRules,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version6;

impl RoomVersionMarker for Version6 {
    const VERSION: RoomVersion = RoomVersion::V6;
}

pub const fn rules() -> RoomVersionRules {
    RoomVersionRules {
        version: RoomVersion::V6,
        description: "Alters authorization rules for events.",
        event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
        room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
        creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
        supports_knocking: false,
        supports_restricted_join_rules: false,
        supports_knock_restricted_join_rule: false,
        supports_additional_room_creators: false,
        power_levels_must_be_integer_values: false,
    }
}
