use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::services::{
    events::entities::event_kinds::EventDefinitionKey,
    rooms::entities::{
        AdditionalRoomCreatorsRule, CreateEventContentValidationRule, KnockRestrictedJoinRuleRule,
        KnockingMembershipRule, PersistentDataUnitValidationRule, RestrictedJoinRulesRule,
        RoomBusinessMethod, RoomBusinessMethodAvailability, RoomBusinessMethodImplementation,
        RoomIdentifier, RoomMethodExecutionProvider, RoomVersionRules, RoomVersionRulesContract,
        StateResolutionV2Rule, SupportsAdditionalRoomCreators, SupportsKnockRestrictedJoinRule,
        SupportsKnocking, SupportsRestrictedJoinRules,
        versions::{
            self,
            v1::{RoomVersionRulesV1, Version1},
            v2::Version2,
            v3::Version3,
            v4::Version4,
            v5::Version5,
            v6::Version6,
            v7::Version7,
            v8::Version8,
            v9::Version9,
            v10::Version10,
            v11::Version11,
            v12::Version12,
        },
    },
};

pub mod v1;
pub mod v10;
pub mod v11;
pub mod v12;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;
pub mod v8;
pub mod v9;

pub trait RoomVersionMarker {
    const VERSION: RoomVersion;
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
    /// TODO: should be default then implements
    #[serde(rename = "12")]
    #[strum(serialize = "12")]
    V12 = 12,
}

impl RoomVersion {
    pub const ALL: [Self; 12] = [
        Self::V1,
        Self::V2,
        Self::V3,
        Self::V4,
        Self::V5,
        Self::V6,
        Self::V7,
        Self::V8,
        Self::V9,
        Self::V10,
        Self::V11,
        Self::V12,
    ];

    pub const fn as_number(self) -> u8 {
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
            Self::V1 => RoomVersionRulesV1::rules(),
            Self::V2 => versions::v2::rules(),
            Self::V3 => versions::v3::rules(),
            Self::V4 => versions::v4::rules(),
            Self::V5 => versions::v5::rules(),
            Self::V6 => versions::v6::rules(),
            Self::V7 => versions::v7::rules(),
            Self::V8 => versions::v8::rules(),
            Self::V9 => versions::v9::rules(),
            Self::V10 => versions::v10::rules(),
            Self::V11 => versions::v11::rules(),
            Self::V12 => versions::v12::rules(),
        }
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
    pub const fn version(&self) -> RoomVersion {
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

    pub const fn business_rules(&self) -> RoomVersionBusinessRules {
        RoomVersionBusinessRules::new(self.version())
    }

    pub const fn supported_events(&self) -> &'static [EventDefinitionKey] {
        self.business_rules().supported_events()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomForVersion<V: RoomVersionMarker> {
    room_id: RoomIdentifier,
    marker: PhantomData<V>,
}

impl<V: RoomVersionMarker> RoomForVersion<V> {
    pub const fn new(room_id: RoomIdentifier) -> Self {
        Self {
            room_id,
            marker: PhantomData,
        }
    }

    pub const fn room_id(&self) -> &RoomIdentifier {
        &self.room_id
    }

    pub const fn version(&self) -> RoomVersion {
        V::VERSION
    }

    pub fn rules(&self) -> RoomVersionRules {
        V::VERSION.rules()
    }
}

impl<V: SupportsKnocking> RoomForVersion<V> {
    pub const fn knocking_capability(&self) -> &'static str {
        "knock membership and knock join rules are valid in this room version"
    }
}

impl<V: SupportsRestrictedJoinRules> RoomForVersion<V> {
    pub const fn restricted_join_rule_capability(&self) -> &'static str {
        "restricted join rules are valid in this room version"
    }
}

impl<V: SupportsKnockRestrictedJoinRule> RoomForVersion<V> {
    pub const fn knock_restricted_join_rule_capability(&self) -> &'static str {
        "knock_restricted join rule is valid in this room version"
    }
}

impl<V: SupportsAdditionalRoomCreators> RoomForVersion<V> {
    pub const fn additional_room_creators_capability(&self) -> &'static str {
        "additional room creators are valid in this room version"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomVersionBusinessRules {
    pub room_version: RoomVersion,
}

impl RoomVersionBusinessRules {
    pub const fn new(room_version: RoomVersion) -> Self {
        Self { room_version }
    }

    pub fn implementation_for(
        &self,
        method: RoomBusinessMethod,
    ) -> Option<RoomBusinessMethodImplementation> {
        match method {
            RoomBusinessMethod::CreateEventContentValidation => Some(
                RoomBusinessMethodImplementation::CreateEventContentValidation(
                    CreateEventContentValidationRule,
                ),
            ),
            RoomBusinessMethod::PersistentDataUnitValidation => Some(
                RoomBusinessMethodImplementation::PersistentDataUnitValidation(
                    PersistentDataUnitValidationRule,
                ),
            ),
            RoomBusinessMethod::StateResolutionV2 => (self.room_version >= RoomVersion::V2)
                .then_some(RoomBusinessMethodImplementation::StateResolutionV2(
                    StateResolutionV2Rule,
                )),
            RoomBusinessMethod::KnockingMembership => (self.room_version >= RoomVersion::V7)
                .then_some(RoomBusinessMethodImplementation::KnockingMembership(
                    KnockingMembershipRule,
                )),
            RoomBusinessMethod::RestrictedJoinRules => (self.room_version >= RoomVersion::V8)
                .then_some(RoomBusinessMethodImplementation::RestrictedJoinRules(
                    RestrictedJoinRulesRule,
                )),
            RoomBusinessMethod::KnockRestrictedJoinRule => (self.room_version >= RoomVersion::V10)
                .then_some(RoomBusinessMethodImplementation::KnockRestrictedJoinRule(
                    KnockRestrictedJoinRuleRule,
                )),
            RoomBusinessMethod::AdditionalRoomCreators => (self.room_version >= RoomVersion::V12)
                .then_some(RoomBusinessMethodImplementation::AdditionalRoomCreators(
                    AdditionalRoomCreatorsRule,
                )),
        }
    }

    pub fn execution_provider_for(
        &self,
        method: RoomBusinessMethod,
    ) -> Option<RoomMethodExecutionProvider> {
        match method {
            RoomBusinessMethod::CreateEventContentValidation => {
                Some(RoomMethodExecutionProvider::SharedRoomCreateContentValidator)
            }
            RoomBusinessMethod::PersistentDataUnitValidation => {
                Some(if self.room_version == RoomVersion::V1 {
                    RoomMethodExecutionProvider::EventsV1PersistentDataUnitValidator
                } else {
                    RoomMethodExecutionProvider::EventsV2PersistentDataUnitValidator
                })
            }
            RoomBusinessMethod::StateResolutionV2 => (self.room_version >= RoomVersion::V2)
                .then_some(RoomMethodExecutionProvider::EventsV2StateResolution),
            RoomBusinessMethod::KnockingMembership
            | RoomBusinessMethod::RestrictedJoinRules
            | RoomBusinessMethod::KnockRestrictedJoinRule
            | RoomBusinessMethod::AdditionalRoomCreators => self
                .implementation_for(method)
                .map(|_| RoomMethodExecutionProvider::RoomVersionCapabilityFlag),
        }
    }

    pub const fn supported_events(self) -> &'static [EventDefinitionKey] {
        const ROOM_V1_EVENTS: &[EventDefinitionKey] = &[
            EventDefinitionKey::RoomCreate,
            EventDefinitionKey::RoomAliases,
            EventDefinitionKey::RoomMessage,
        ];
        const ROOM_V2_EVENTS: &[EventDefinitionKey] = &[
            EventDefinitionKey::RoomCreate,
            EventDefinitionKey::RoomAliases,
            EventDefinitionKey::RoomMessage,
        ];

        match self.room_version {
            RoomVersion::V1 => ROOM_V1_EVENTS,
            _ => ROOM_V2_EVENTS,
        }
    }

    pub fn iter_method_availability(
        self,
    ) -> Option<impl Iterator<Item = RoomBusinessMethodAvailability>> {
        Some(RoomBusinessMethod::ALL.into_iter().map(move |method| {
            RoomBusinessMethodAvailability {
                method,
                implementation: self.implementation_for(method),
                execution_provider: self.execution_provider_for(method),
            }
        }))
    }
}
