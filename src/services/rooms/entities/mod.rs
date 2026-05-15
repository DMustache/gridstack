use std::{marker::PhantomData, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    infrastructure::{server_name::ServerName, user_identifier::UserIdentifier},
    services::{
        authorization::entities::AuthorizedUserIdentifier,
        events::entities::event_kinds::EventDefinitionKey,
        rooms::{
            entities::versions::RoomVersion,
            handlers::create_room::{CreateRoomInfo, CreateRoomView, RoomPreset, RoomVisibility},
        },
    },
};

pub mod versions;

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

#[derive(Clone, Debug)]
pub struct CreateRoomStateEventPayload {
    pub content: serde_json::Value,
    pub event_type: String,
    pub ordering: i32,
    pub state_key: String,
}

#[derive(Clone, Debug)]
pub struct CreateRoomPersistencePayload {
    pub creator_user_id: String,
    pub initial_state: Vec<CreateRoomStateEventPayload>,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub preset: Option<String>,
    pub room_alias_name: Option<String>,
    pub room_id: String,
    pub room_version: String,
    pub topic: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreateRoomContract {
    pub persistence_payload: CreateRoomPersistencePayload,
    pub room: Room,
}

impl From<CreatedRoom> for CreateRoomView {
    fn from(value: CreatedRoom) -> Self {
        Self {
            room_id: value.room_id,
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
#[allow(
    clippy::struct_excessive_bools,
    reason = "room version capability matrix"
)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoomBusinessMethod {
    CreateEventContentValidation,
    PersistentDataUnitValidation,
    StateResolutionV2,
    KnockingMembership,
    RestrictedJoinRules,
    KnockRestrictedJoinRule,
    AdditionalRoomCreators,
}

impl RoomBusinessMethod {
    pub const ALL: [Self; 7] = [
        Self::CreateEventContentValidation,
        Self::PersistentDataUnitValidation,
        Self::StateResolutionV2,
        Self::KnockingMembership,
        Self::RestrictedJoinRules,
        Self::KnockRestrictedJoinRule,
        Self::AdditionalRoomCreators,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateEventContentValidationRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentDataUnitValidationRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateResolutionV2Rule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnockingMembershipRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestrictedJoinRulesRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnockRestrictedJoinRuleRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdditionalRoomCreatorsRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomBusinessMethodImplementation {
    CreateEventContentValidation(CreateEventContentValidationRule),
    PersistentDataUnitValidation(PersistentDataUnitValidationRule),
    StateResolutionV2(StateResolutionV2Rule),
    KnockingMembership(KnockingMembershipRule),
    RestrictedJoinRules(RestrictedJoinRulesRule),
    KnockRestrictedJoinRule(KnockRestrictedJoinRuleRule),
    AdditionalRoomCreators(AdditionalRoomCreatorsRule),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomBusinessMethodAvailability {
    pub method: RoomBusinessMethod,
    pub implementation: Option<RoomBusinessMethodImplementation>,
    pub execution_provider: Option<RoomMethodExecutionProvider>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomMethodExecutionProvider {
    SharedRoomCreateContentValidator,
    EventsV1PersistentDataUnitValidator,
    EventsV2PersistentDataUnitValidator,
    EventsV2StateResolution,
    RoomVersionCapabilityFlag,
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
        const EVENTS_V1: &[EventDefinitionKey] = &[
            EventDefinitionKey::RoomCreate,
            EventDefinitionKey::RoomAliases,
            EventDefinitionKey::RoomMessage,
            EventDefinitionKey::GenericStateEvent,
        ];
        const EVENTS_V2_PLUS: &[EventDefinitionKey] = &[
            EventDefinitionKey::RoomCreate,
            EventDefinitionKey::RoomAliases,
            EventDefinitionKey::RoomMessage,
            EventDefinitionKey::GenericStateEvent,
        ];

        match self.room_version {
            RoomVersion::V1 => EVENTS_V1,
            _ => EVENTS_V2_PLUS,
        }
    }

    pub fn iter_method_availability(self) -> impl Iterator<Item = RoomBusinessMethodAvailability> {
        RoomBusinessMethod::ALL
            .into_iter()
            .map(move |method| RoomBusinessMethodAvailability {
                method,
                implementation: self.implementation_for(method),
                execution_provider: self.execution_provider_for(method),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RoomIdentifier {
    localpart: String,
    server_name: Option<ServerName>,
    full_identifier: Arc<String>,
}

impl RoomIdentifier {
    /// # Panics
    ///
    /// Panics if a UUID-generated localpart or the provided validated `home_server_name`
    /// cannot form a valid room identifier.
    pub fn generate(home_server_name: &ServerName) -> Self {
        let localpart = Uuid::new_v4().simple().to_string();
        Self::from_localpart_and_server(&localpart, home_server_name)
            .expect("generated room identifier must be valid")
    }

    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        let (localpart, server_name) =
            ServerName::split_localpart_and_server_name(candidate.strip_prefix('!')?)?;
        Self::from_localpart_and_server(localpart, &ServerName::try_new(server_name)?)
    }

    pub fn parse_domain_less(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        let localpart = candidate.strip_prefix('!')?;
        Self::from_localpart(localpart)
    }

    pub fn validate(&self) -> Result<(), RoomIdentifierValidationError> {
        if self.server_name.is_none() {
            return Err(RoomIdentifierValidationError::InvalidStructure {
                room_id: self.as_str().to_owned(),
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
                if self.validate().is_ok() || self.server_name.is_none() {
                    return Ok(());
                }

                Err(RoomIdentifierValidationError::InvalidStructure {
                    room_id: self.as_str().to_owned(),
                    expected: "room IDs must use `!localpart:server.name` or hash-only `!localpart` form depending on version rules",
                })
            }
        }
    }

    pub fn as_str(&self) -> &str {
        &self.full_identifier
    }

    pub fn server_name(&self) -> Option<&str> {
        self.server_name.as_ref().map(ServerName::as_str)
    }

    pub fn localpart(&self) -> &str {
        &self.localpart
    }

    pub const fn server_name_value(&self) -> Option<&ServerName> {
        self.server_name.as_ref()
    }

    pub fn from_localpart_and_server(localpart: &str, server_name: &ServerName) -> Option<Self> {
        if !is_valid_room_localpart(localpart) {
            return None;
        }

        let full_identifier = format!("!{}:{}", localpart, server_name.as_str());
        Some(Self {
            localpart: localpart.to_owned(),
            server_name: Some(server_name.clone()),
            full_identifier: Arc::new(full_identifier),
        })
    }

    pub fn from_localpart(localpart: &str) -> Option<Self> {
        if !is_valid_room_localpart(localpart) {
            return None;
        }

        let full_identifier = format!("!{localpart}");
        Some(Self {
            localpart: localpart.to_owned(),
            server_name: None,
            full_identifier: Arc::new(full_identifier),
        })
    }
}

fn is_valid_room_localpart(localpart: &str) -> bool {
    !localpart.is_empty() && !localpart.contains(':')
}

impl Serialize for RoomIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RoomIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value.clone())
            .or_else(|| Self::parse_domain_less(value.clone()))
            .ok_or_else(|| serde::de::Error::custom("invalid room identifier"))
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
        room_id: &RoomIdentifier,
        room_version: RoomVersion,
    ) -> Result<Self, RoomIdentifierValidationError> {
        room_id.validate_for_room_version(room_version.as_number())?;
        Ok(Self {
            room_id: room_id.clone(),
            room_version,
        })
    }

    pub fn rules(&self) -> RoomVersionRules {
        self.room_version.rules()
    }

    pub fn try_create_contract(
        creator_user_id: &AuthorizedUserIdentifier,
        info: CreateRoomInfo,
        home_server_name: &ServerName,
    ) -> Result<CreateRoomContract, RoomCreateContractError> {
        let room_version = match info.room_version.as_deref() {
            Some(version) => RoomVersion::from_str(version.trim()).map_err(|_| {
                RoomCreateContractError::UnsupportedRoomVersion {
                    room_version: version.to_owned(),
                }
            })?,
            None => RoomVersion::default(),
        };

        let room_id = RoomIdentifier::generate(home_server_name);
        let room = Self::new(&room_id, room_version)?;

        if let Some(room_alias_name) = info.room_alias_name.as_deref()
            && room_alias_name.trim().is_empty()
        {
            return Err(RoomCreateContractError::InvalidRoomAlias);
        }

        if let Some(invitees) = info.invite.as_ref()
            && invitees
                .iter()
                .any(|invitee| UserIdentifier::try_from(invitee.clone()).is_err())
        {
            return Err(RoomCreateContractError::InvalidInviteUserIdentifier);
        }

        let supported_events = room.supported_events();
        let mut initial_state_payload = Vec::new();
        if let Some(initial_state) = info.initial_state {
            for (index, event) in initial_state.into_iter().enumerate() {
                if event.event_type.trim().is_empty() {
                    return Err(RoomCreateContractError::InvalidRoomStateEventType);
                }
                let event_key = map_event_type_to_definition_key(&event.event_type);
                if !supported_events.contains(&event_key) {
                    return Err(
                        RoomCreateContractError::UnsupportedStateEventForRoomVersion {
                            event_type: event.event_type,
                            room_version,
                        },
                    );
                }

                initial_state_payload.push(CreateRoomStateEventPayload {
                    content: event.content,
                    event_type: event.event_type,
                    ordering: i32::try_from(index).unwrap_or(i32::MAX),
                    state_key: event.state_key.unwrap_or_default(),
                });
            }
        }

        let payload = CreateRoomPersistencePayload {
            creator_user_id: creator_user_id.as_user_identifier().as_str().to_owned(),
            initial_state: initial_state_payload,
            is_direct: info.is_direct,
            name: info.name,
            preset: info.preset.map(|preset| match preset {
                RoomPreset::PrivateChat => "private_chat".to_owned(),
                RoomPreset::PublicChat => "public_chat".to_owned(),
                RoomPreset::TrustedPrivateChat => "trusted_private_chat".to_owned(),
            }),
            room_alias_name: info.room_alias_name,
            room_id: room_id.as_str().to_string(),
            room_version: room_version.to_string(),
            topic: info.topic,
            visibility: info.visibility.map(|visibility| match visibility {
                RoomVisibility::Public => "public".to_owned(),
                RoomVisibility::Private => "private".to_owned(),
            }),
        };

        Ok(CreateRoomContract {
            persistence_payload: payload,
            room,
        })
    }

    pub const fn business_rules(&self) -> RoomVersionBusinessRules {
        RoomVersionBusinessRules::new(self.room_version)
    }

    pub const fn supported_events(&self) -> &'static [EventDefinitionKey] {
        self.business_rules().supported_events()
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

#[derive(Debug, Error)]
pub enum RoomCreateContractError {
    #[error("invalid home server name `{home_server_name}`")]
    InvalidHomeServerName { home_server_name: String },
    #[error("unsupported room version `{room_version}`")]
    UnsupportedRoomVersion { room_version: String },
    #[error("invalid generated room identifier `{room_id}`")]
    InvalidGeneratedRoomIdentifier { room_id: String },
    #[error("invalid room alias")]
    InvalidRoomAlias,
    #[error("invalid invite user identifier")]
    InvalidInviteUserIdentifier,
    #[error("invalid room state event type")]
    InvalidRoomStateEventType,
    #[error("event type `{event_type}` is not supported in room version `{room_version}`")]
    UnsupportedStateEventForRoomVersion {
        event_type: String,
        room_version: RoomVersion,
    },
    #[error(transparent)]
    InvalidRoomIdentifier(#[from] RoomIdentifierValidationError),
}

fn map_event_type_to_definition_key(event_type: &str) -> EventDefinitionKey {
    match event_type {
        "m.room.create" => EventDefinitionKey::RoomCreate,
        "m.room.aliases" => EventDefinitionKey::RoomAliases,
        "m.room.message" => EventDefinitionKey::RoomMessage,
        _ => EventDefinitionKey::GenericStateEvent,
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
    const fn new(room_id: RoomIdentifier) -> Self {
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

trait RoomVersionRulesContract {
    fn rules() -> RoomVersionRules;
}
