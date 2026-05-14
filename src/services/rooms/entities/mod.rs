use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use thiserror::Error;

use crate::{infrastructure::server_name, services::rooms::handlers::create_room::CreateRoomView};

#[derive(Clone, Debug, Deserialize)]
pub struct RoomStateEventInfo {
    pub content: serde_json::Value,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Clone, Debug)]
pub struct CreatedRoom {
    pub room_id: String,
}

impl From<CreatedRoom> for CreateRoomView {
    fn from(value: CreatedRoom) -> Self {
        Self {
            room_id: value.room_id,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    EnumString,
    Display,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
)]
pub enum RoomVersion {
    #[default]
    #[serde(rename = "1")]
    #[strum(serialize = "1")]
    V1 = 1,
    #[serde(rename = "2")]
    #[strum(serialize = "2")]
    V2 = 2,
    #[serde(rename = "3")]
    #[strum(serialize = "3")]
    V3 = 3,
    #[serde(rename = "4")]
    #[strum(serialize = "4")]
    V4 = 4,
    #[serde(rename = "5")]
    #[strum(serialize = "5")]
    V5 = 5,
    #[serde(rename = "6")]
    #[strum(serialize = "6")]
    V6 = 6,
    #[serde(rename = "7")]
    #[strum(serialize = "7")]
    V7 = 7,
    #[serde(rename = "8")]
    #[strum(serialize = "8")]
    V8 = 8,
    #[serde(rename = "9")]
    #[strum(serialize = "9")]
    V9 = 9,
    #[serde(rename = "10")]
    #[strum(serialize = "10")]
    V10 = 10,
    #[serde(rename = "11")]
    #[strum(serialize = "11")]
    V11 = 11,
    #[serde(rename = "12")]
    #[strum(serialize = "12")]
    V12 = 12,
}

impl RoomVersion {
    pub fn as_number(self) -> u8 {
        self as u8
    }

    pub fn requires_creator_field(self) -> bool {
        self <= Self::V11
    }

    pub fn supports_additional_creators(self) -> bool {
        self >= Self::V12
    }

    pub fn rules(self) -> RoomVersionRules {
        match self {
            Self::V1 => RoomVersionRules {
                version: self,
                description: "Initial room version.",
                event_identifier_format: EventIdentifierFormat::ServerAssignedWithDomain,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: false,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V2 => RoomVersionRules {
                version: self,
                description: "Implements State Resolution Version 2.",
                event_identifier_format: EventIdentifierFormat::ServerAssignedWithDomain,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: false,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V3 => RoomVersionRules {
                version: self,
                description: "Event IDs are reference-hash based and may include slash characters.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUnpaddedBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: false,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V4 => RoomVersionRules {
                version: self,
                description: "Builds on v3 with URL-safe base64 event identifiers.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: false,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V5 => RoomVersionRules {
                version: self,
                description: "Introduces enforcement of signing key validity periods.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: false,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V6 => RoomVersionRules {
                version: self,
                description: "Alters authorization rules for events.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: false,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V7 => RoomVersionRules {
                version: self,
                description: "Introduces knocking membership and join behavior.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: true,
                supports_restricted_join_rules: false,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V8 => RoomVersionRules {
                version: self,
                description: "Adds restricted join rules based on membership in another room.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: true,
                supports_restricted_join_rules: true,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V9 => RoomVersionRules {
                version: self,
                description: "Builds on v8 and fixes membership redaction edge cases.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: true,
                supports_restricted_join_rules: true,
                supports_knock_restricted_join_rule: false,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: false,
            },
            Self::V10 => RoomVersionRules {
                version: self,
                description: "Requires integer-only power levels and adds knock_restricted join rule.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: true,
                supports_restricted_join_rules: true,
                supports_knock_restricted_join_rule: true,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: true,
            },
            Self::V11 => RoomVersionRules {
                version: self,
                description: "Clarifies redaction algorithm behavior.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::LocalPartWithDomain,
                creator_power_levels: CreatorPowerLevels::DefaultLevel100ForCreator,
                supports_knocking: true,
                supports_restricted_join_rules: true,
                supports_knock_restricted_join_rule: true,
                supports_additional_room_creators: false,
                power_levels_must_be_integer_values: true,
            },
            Self::V12 => RoomVersionRules {
                version: self,
                description: "Room IDs are hash-derived, room creators are formalized with infinite power, and state resolution is updated.",
                event_identifier_format: EventIdentifierFormat::ReferenceHashUrlSafeBase64,
                room_identifier_format: RoomIdentifierFormat::HashBasedRoomId,
                creator_power_levels: CreatorPowerLevels::InfiniteForRoomCreators,
                supports_knocking: true,
                supports_restricted_join_rules: true,
                supports_knock_restricted_join_rule: true,
                supports_additional_room_creators: true,
                power_levels_must_be_integer_values: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventIdentifierFormat {
    ServerAssignedWithDomain,
    ReferenceHashUnpaddedBase64,
    ReferenceHashUrlSafeBase64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomIdentifierFormat {
    LocalPartWithDomain,
    HashBasedRoomId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatorPowerLevels {
    DefaultLevel100ForCreator,
    InfiniteForRoomCreators,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomVersionRules {
    pub version: RoomVersion,
    pub description: &'static str,
    pub event_identifier_format: EventIdentifierFormat,
    pub room_identifier_format: RoomIdentifierFormat,
    pub creator_power_levels: CreatorPowerLevels,
    pub supports_knocking: bool,
    pub supports_restricted_join_rules: bool,
    pub supports_knock_restricted_join_rule: bool,
    pub supports_additional_room_creators: bool,
    pub power_levels_must_be_integer_values: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomIdentifier(String);

impl RoomIdentifier {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        let (localpart, _) =
            server_name::split_localpart_and_server_name(candidate.strip_prefix('!')?)?;
        if localpart.is_empty() {
            return None;
        }

        Some(Self(candidate))
    }

    pub fn parse_domain_less(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        let localpart = candidate.strip_prefix('!')?;
        if localpart.is_empty() || localpart.contains(':') {
            return None;
        }

        Some(Self(candidate))
    }

    pub fn validate(&self) -> Result<(), RoomIdentifierValidationError> {
        if Self::parse(self.0.clone()).is_none() {
            return Err(RoomIdentifierValidationError::InvalidStructure {
                room_id: self.0.clone(),
                expected: "room IDs must use the `!localpart:server.name` format",
            });
        }

        Ok(())
    }

    pub fn validate_for_room_version(
        &self,
        room_version: u8,
    ) -> Result<(), RoomIdentifierValidationError> {
        match room_version {
            1 | 2 => self.validate(),
            _ => {
                if self.validate().is_ok() || Self::parse_domain_less(self.0.clone()).is_some() {
                    return Ok(());
                }

                Err(RoomIdentifierValidationError::InvalidStructure {
                    room_id: self.0.clone(),
                    expected: "room IDs must use `!localpart:server.name` or hash-only `!localpart` form depending on version rules",
                })
            }
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn server_name(&self) -> Option<&str> {
        let (_, server_name) = self.0.strip_prefix('!')?.split_once(':')?;
        server_name::is_valid_server_name(server_name).then_some(server_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RoomIdentifierValidationError {
    #[error("invalid room identifier `{room_id}`: {expected}")]
    InvalidStructure {
        room_id: String,
        expected: &'static str,
    },
}

pub enum RoomType {}

pub enum Spaces {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    pub room_id: RoomIdentifier,
    pub room_version: RoomVersion,
}

impl Room {
    pub fn new(
        room_id: RoomIdentifier,
        room_version: RoomVersion,
    ) -> Result<Self, RoomIdentifierValidationError> {
        room_id.validate_for_room_version(room_version.as_number())?;
        Ok(Self {
            room_id,
            room_version,
        })
    }

    pub fn rules(&self) -> RoomVersionRules {
        self.room_version.rules()
    }

    pub fn versioned(self) -> VersionedRoom {
        match self.room_version {
            RoomVersion::V1 => VersionedRoom::V1(RoomForVersion::new(self.room_id)),
            RoomVersion::V2 => VersionedRoom::V2(RoomForVersion::new(self.room_id)),
            RoomVersion::V3 => VersionedRoom::V3(RoomForVersion::new(self.room_id)),
            RoomVersion::V4 => VersionedRoom::V4(RoomForVersion::new(self.room_id)),
            RoomVersion::V5 => VersionedRoom::V5(RoomForVersion::new(self.room_id)),
            RoomVersion::V6 => VersionedRoom::V6(RoomForVersion::new(self.room_id)),
            RoomVersion::V7 => VersionedRoom::V7(RoomForVersion::new(self.room_id)),
            RoomVersion::V8 => VersionedRoom::V8(RoomForVersion::new(self.room_id)),
            RoomVersion::V9 => VersionedRoom::V9(RoomForVersion::new(self.room_id)),
            RoomVersion::V10 => VersionedRoom::V10(RoomForVersion::new(self.room_id)),
            RoomVersion::V11 => VersionedRoom::V11(RoomForVersion::new(self.room_id)),
            RoomVersion::V12 => VersionedRoom::V12(RoomForVersion::new(self.room_id)),
        }
    }
}

pub trait RoomVersionMarker {
    const VERSION: RoomVersion;
}

pub trait SupportsKnocking: RoomVersionMarker {}
pub trait SupportsRestrictedJoinRules: RoomVersionMarker {}
pub trait SupportsKnockRestrictedJoinRule: RoomVersionMarker {}
pub trait SupportsAdditionalRoomCreators: RoomVersionMarker {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version1;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version2;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version3;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version4;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version5;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version6;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version7;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version8;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version9;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version10;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version11;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version12;

impl RoomVersionMarker for Version1 {
    const VERSION: RoomVersion = RoomVersion::V1;
}
impl RoomVersionMarker for Version2 {
    const VERSION: RoomVersion = RoomVersion::V2;
}
impl RoomVersionMarker for Version3 {
    const VERSION: RoomVersion = RoomVersion::V3;
}
impl RoomVersionMarker for Version4 {
    const VERSION: RoomVersion = RoomVersion::V4;
}
impl RoomVersionMarker for Version5 {
    const VERSION: RoomVersion = RoomVersion::V5;
}
impl RoomVersionMarker for Version6 {
    const VERSION: RoomVersion = RoomVersion::V6;
}
impl RoomVersionMarker for Version7 {
    const VERSION: RoomVersion = RoomVersion::V7;
}
impl RoomVersionMarker for Version8 {
    const VERSION: RoomVersion = RoomVersion::V8;
}
impl RoomVersionMarker for Version9 {
    const VERSION: RoomVersion = RoomVersion::V9;
}
impl RoomVersionMarker for Version10 {
    const VERSION: RoomVersion = RoomVersion::V10;
}
impl RoomVersionMarker for Version11 {
    const VERSION: RoomVersion = RoomVersion::V11;
}
impl RoomVersionMarker for Version12 {
    const VERSION: RoomVersion = RoomVersion::V12;
}

impl SupportsKnocking for Version7 {}
impl SupportsKnocking for Version8 {}
impl SupportsKnocking for Version9 {}
impl SupportsKnocking for Version10 {}
impl SupportsKnocking for Version11 {}
impl SupportsKnocking for Version12 {}

impl SupportsRestrictedJoinRules for Version8 {}
impl SupportsRestrictedJoinRules for Version9 {}
impl SupportsRestrictedJoinRules for Version10 {}
impl SupportsRestrictedJoinRules for Version11 {}
impl SupportsRestrictedJoinRules for Version12 {}

impl SupportsKnockRestrictedJoinRule for Version10 {}
impl SupportsKnockRestrictedJoinRule for Version11 {}
impl SupportsKnockRestrictedJoinRule for Version12 {}

impl SupportsAdditionalRoomCreators for Version12 {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomForVersion<V: RoomVersionMarker> {
    room_id: RoomIdentifier,
    marker: PhantomData<V>,
}

impl<V: RoomVersionMarker> RoomForVersion<V> {
    fn new(room_id: RoomIdentifier) -> Self {
        Self {
            room_id,
            marker: PhantomData,
        }
    }

    pub fn room_id(&self) -> &RoomIdentifier {
        &self.room_id
    }

    pub fn version(&self) -> RoomVersion {
        V::VERSION
    }

    pub fn rules(&self) -> RoomVersionRules {
        V::VERSION.rules()
    }
}

impl<V: SupportsKnocking> RoomForVersion<V> {
    pub fn knocking_capability(&self) -> &'static str {
        "knock membership and knock join rules are valid in this room version"
    }
}

impl<V: SupportsRestrictedJoinRules> RoomForVersion<V> {
    pub fn restricted_join_rule_capability(&self) -> &'static str {
        "restricted join rules are valid in this room version"
    }
}

impl<V: SupportsKnockRestrictedJoinRule> RoomForVersion<V> {
    pub fn knock_restricted_join_rule_capability(&self) -> &'static str {
        "knock_restricted join rule is valid in this room version"
    }
}

impl<V: SupportsAdditionalRoomCreators> RoomForVersion<V> {
    pub fn additional_room_creators_capability(&self) -> &'static str {
        "additional room creators are valid in this room version"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionedRoom {
    V1(RoomForVersion<Version1>),
    V2(RoomForVersion<Version2>),
    V3(RoomForVersion<Version3>),
    V4(RoomForVersion<Version4>),
    V5(RoomForVersion<Version5>),
    V6(RoomForVersion<Version6>),
    V7(RoomForVersion<Version7>),
    V8(RoomForVersion<Version8>),
    V9(RoomForVersion<Version9>),
    V10(RoomForVersion<Version10>),
    V11(RoomForVersion<Version11>),
    V12(RoomForVersion<Version12>),
}

impl VersionedRoom {
    pub fn version(&self) -> RoomVersion {
        match self {
            Self::V1(_) => RoomVersion::V1,
            Self::V2(_) => RoomVersion::V2,
            Self::V3(_) => RoomVersion::V3,
            Self::V4(_) => RoomVersion::V4,
            Self::V5(_) => RoomVersion::V5,
            Self::V6(_) => RoomVersion::V6,
            Self::V7(_) => RoomVersion::V7,
            Self::V8(_) => RoomVersion::V8,
            Self::V9(_) => RoomVersion::V9,
            Self::V10(_) => RoomVersion::V10,
            Self::V11(_) => RoomVersion::V11,
            Self::V12(_) => RoomVersion::V12,
        }
    }

    pub fn rules(&self) -> RoomVersionRules {
        self.version().rules()
    }
}
