use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    infrastructure::server_name::ServerName,
    services::{
        authorization::entities::AuthorizedUserIdentifier,
        events::entities::event_kinds::EventDefinitionKey,
        rooms::{
            entities::creation_plan::RoomCreationPlanBuilder,
            entities::versions::{
                RoomForVersion, RoomVersion, RoomVersionBusinessRules, RoomVersionMarker,
                VersionedRoom,
            },
            handlers::create_room::CreateRoomView,
            service::create_room::ValidatedCreateRoomRequest,
        },
    },
};

pub mod creation_plan;
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
    pub preset: String,

    pub room_alias_name: Option<String>,
    pub room_id: String,
    pub room_version: String,
    pub topic: Option<String>,
    pub visibility: String,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RoomIdentifier {
    localpart: String,
    server_name: Option<ServerName>,
    full_identifier: Arc<String>,
}

impl RoomIdentifier {
    pub fn generate(home_server_name: &ServerName) -> Self {
        let localpart = uuid::Uuid::new_v4().simple().to_string();
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
        info: ValidatedCreateRoomRequest,
        home_server_name: &ServerName,
    ) -> Result<CreateRoomContract, RoomCreateContractError> {
        let ValidatedCreateRoomRequest {
            creation_content,
            initial_state,
            invite,
            invite_3pid,
            is_direct,
            name,
            power_level_content_override,
            preset,
            room_alias_name,
            room_version,
            topic,
            visibility,
        } = info;
        if !room_version.is_supported() {
            return Err(RoomCreateContractError::UnsupportedRoomVersion {
                room_version: room_version.to_string(),
            });
        }

        let room_id = RoomIdentifier::generate(home_server_name);
        let room = Self::new(&room_id, room_version)?;
        let invitee_user_ids = invite
            .iter()
            .map(|invitee| invitee.as_str().to_owned())
            .collect::<Vec<_>>();

        let initial_state_payload = RoomCreationPlanBuilder::new(
            creator_user_id,
            room_id.as_str(),
            home_server_name.as_str(),
            room_version,
        )
        .with_room_create_event_content(creation_content.to_event_content_map())
        .with_preset(&preset)
        .with_invitees(&invitee_user_ids, &invite_3pid, Some(is_direct))
        .with_alias(room_alias_name.as_deref())
        .with_profile(name.as_deref(), topic.as_deref())
        .with_initial_state(Some(initial_state))
        .with_power_level_override(Some(power_level_content_override))
        .build()?;

        let payload = CreateRoomPersistencePayload {
            creator_user_id: creator_user_id.as_user_identifier().as_str().to_owned(),
            initial_state: initial_state_payload,
            is_direct: Some(is_direct),
            name,
            preset: preset.to_string(),
            room_alias_name,
            room_id: room_id.as_str().to_string(),
            room_version: room_version.to_string(),
            topic,
            visibility: visibility.to_string(),
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
    #[error("invalid third-party invite identifier")]
    InvalidInviteThirdPartyIdentifier,
    #[error("creation_content must be a JSON object")]
    InvalidCreationContent,
    #[error("power_level_content_override must be a JSON object")]
    InvalidPowerLevelContentOverride,
    #[error("invalid m.room.create content implied by createRoom: {reason}")]
    InvalidRoomCreateContent { reason: String },
    #[error("invalid room state event type")]
    InvalidRoomStateEventType,
    #[error("room creation plan must include at least create/member/power events")]
    EmptyRoomCreationEventPlan,
    #[error("invalid room creation event order: expected `{expected}`, received `{received}`")]
    InvalidRoomCreationEventOrder {
        expected: &'static str,
        received: String,
    },
    #[error("room domain `{room_domain}` does not match creator domain `{creator_domain}`")]
    RoomDomainAndCreatorDomainMismatch {
        room_domain: String,
        creator_domain: String,
    },
    #[error("event type `{event_type}` in initial_state must have object content")]
    InvalidRoomStateEventContentType { event_type: String },
    #[error("initial_state must not contain `m.room.create` events")]
    InitialStateContainsRoomCreateEvent,
    #[error(transparent)]
    InvalidRoomIdentifier(#[from] RoomIdentifierValidationError),
}

pub trait SupportsKnocking: RoomVersionMarker {}
pub trait SupportsRestrictedJoinRules: RoomVersionMarker {}
pub trait SupportsKnockRestrictedJoinRule: RoomVersionMarker {}
pub trait SupportsAdditionalRoomCreators: RoomVersionMarker {}

trait RoomVersionRulesContract {
    fn rules() -> RoomVersionRules;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        infrastructure::server_name::ServerName,
        services::{
            authorization::entities::AuthorizedUserIdentifier,
            rooms::{
                entities::{Room, RoomCreateContractError, RoomStateEventInfo},
                handlers::create_room::{
                    RoomPowerLevelsContentOverride, RoomPreset, RoomVisibility,
                },
                service::create_room::{CreationContent, ValidatedCreateRoomRequest},
            },
        },
    };

    use super::RoomVersion;

    fn creator_user_id() -> AuthorizedUserIdentifier {
        serde_json::from_value(json!("@alice:example.org")).expect("valid creator user id")
    }

    fn home_server_name() -> ServerName {
        ServerName::try_new("example.org").expect("valid homeserver name")
    }

    fn base_create_room_info() -> ValidatedCreateRoomRequest {
        ValidatedCreateRoomRequest {
            creation_content: CreationContent::default(),
            initial_state: Vec::new(),
            invite: Vec::new(),
            invite_3pid: Vec::new(),
            is_direct: false,
            name: None,
            power_level_content_override: RoomPowerLevelsContentOverride::default(),
            preset: RoomPreset::PrivateChat,
            room_alias_name: None,
            room_version: RoomVersion::V1,
            topic: None,
            visibility: RoomVisibility::Private,
        }
    }

    #[test]
    fn supported_versions_follow_manual_allowlist() {
        let expected_supported_versions = RoomVersion::SUPPORTED.to_vec();
        let actual_supported_versions = RoomVersion::supported_versions().collect::<Vec<_>>();

        assert_eq!(actual_supported_versions, expected_supported_versions);
        assert_eq!(actual_supported_versions, vec![RoomVersion::V1]);
    }

    #[test]
    fn create_room_contract_rejects_non_v1_room_version() {
        let mut create_room_info = base_create_room_info();
        create_room_info.room_version = RoomVersion::V12;

        let error =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect_err("non-v1 room creation must be rejected");
        assert!(matches!(
            error,
            RoomCreateContractError::UnsupportedRoomVersion { .. }
        ));
    }

    #[test]
    fn create_room_contract_allows_custom_initial_state_event_type() {
        let mut create_room_info = base_create_room_info();
        create_room_info.initial_state = vec![RoomStateEventInfo {
            content: json!({ "enabled": true }),
            state_key: None,
            event_type: "com.example.custom".to_owned(),
        }];

        let contract =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect("custom state event type with object content should be accepted");
        assert!(
            contract
                .persistence_payload
                .initial_state
                .iter()
                .any(|event| event.event_type == "com.example.custom")
        );
    }

    #[test]
    fn create_room_contract_rejects_non_object_initial_state_content() {
        let mut create_room_info = base_create_room_info();
        create_room_info.initial_state = vec![RoomStateEventInfo {
            content: json!("invalid"),
            state_key: None,
            event_type: "m.room.topic".to_owned(),
        }];

        let error =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect_err("non-object initial_state content must be rejected");
        assert!(matches!(
            error,
            RoomCreateContractError::InvalidRoomStateEventContentType { .. }
        ));
    }

    #[test]
    fn create_room_contract_rejects_m_room_create_in_initial_state() {
        let mut create_room_info = base_create_room_info();
        create_room_info.initial_state = vec![RoomStateEventInfo {
            content: json!({}),
            state_key: Some(String::new()),
            event_type: "m.room.create".to_owned(),
        }];

        let error =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect_err("m.room.create in initial_state must be rejected");
        assert!(matches!(
            error,
            RoomCreateContractError::InitialStateContainsRoomCreateEvent
        ));
    }

    #[test]
    fn create_room_contract_overrides_client_creator_and_room_version() {
        let mut create_room_info = base_create_room_info();
        create_room_info.creation_content.extra_content = json!({
            "creator": "not-a-valid-user-id",
            "room_version": "12"
        })
        .as_object()
        .cloned()
        .expect("object");

        Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
            .expect("server-owned creator and room_version keys must be overridden");
    }

    #[test]
    fn create_room_contract_allows_additional_creators_in_v1_when_prevalidated() {
        let mut create_room_info = base_create_room_info();
        create_room_info.creation_content.additional_creators = vec!["@bob:example.org".to_owned()];

        Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
            .expect("prevalidated additional_creators should pass for v1 contract build");
    }

    #[test]
    fn create_room_contract_collapses_duplicate_state_keys_with_last_write_wins() {
        let mut create_room_info = base_create_room_info();
        create_room_info.initial_state = vec![
            RoomStateEventInfo {
                content: json!({ "guest_access": "can_join" }),
                state_key: Some(String::new()),
                event_type: "m.room.guest_access".to_owned(),
            },
            RoomStateEventInfo {
                content: json!({ "history_visibility": "invited" }),
                state_key: Some(String::new()),
                event_type: "m.room.history_visibility".to_owned(),
            },
        ];

        let contract =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect("duplicate preset and initial_state keys must be collapsed");
        let duplicate_guest_access_events = contract
            .persistence_payload
            .initial_state
            .iter()
            .filter(|event| event.event_type == "m.room.guest_access" && event.state_key.is_empty())
            .count();
        let duplicate_history_visibility_events = contract
            .persistence_payload
            .initial_state
            .iter()
            .filter(|event| {
                event.event_type == "m.room.history_visibility" && event.state_key.is_empty()
            })
            .count();

        assert_eq!(duplicate_guest_access_events, 1);
        assert_eq!(duplicate_history_visibility_events, 1);
    }
}
