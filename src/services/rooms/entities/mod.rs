use std::{str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    infrastructure::{server_name::ServerName, user_identifier::UserIdentifier},
    services::{
        authorization::entities::AuthorizedUserIdentifier,
        events::entities::event_kinds::{
            EventDefinitionKey, state_events::create::RoomCreateContent,
        },
        rooms::{
            entities::versions::{
                RoomForVersion, RoomVersion, RoomVersionBusinessRules, RoomVersionMarker,
                VersionedRoom,
            },
            handlers::create_room::{
                CreateRoomInfo, CreateRoomView, InviteThirdPartyIdentifierInfo, RoomPreset,
            },
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

impl RoomVersion {
    pub const SUPPORTED: [Self; 1] = [Self::V1];

    pub fn is_supported(self) -> bool {
        Self::SUPPORTED.contains(&self)
    }

    pub fn supported_versions() -> impl Iterator<Item = Self> {
        Self::SUPPORTED.into_iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RoomIdentifier {
    localpart: String,
    server_name: Option<ServerName>,
    full_identifier: Arc<String>,
}

impl RoomIdentifier {
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
        let CreateRoomInfo {
            creation_content,
            initial_state,
            invite,
            invite_3pid,
            is_direct,
            name,
            power_level_content_override,
            preset,
            room_alias_name,
            room_version: requested_room_version,
            topic,
            visibility,
        } = info;

        let room_version = match requested_room_version.as_deref() {
            Some(version) => RoomVersion::from_str(version.trim()).map_err(|_| {
                RoomCreateContractError::UnsupportedRoomVersion {
                    room_version: version.to_owned(),
                }
            })?,
            None => RoomVersion::default(),
        };
        if !room_version.is_supported() {
            return Err(RoomCreateContractError::UnsupportedRoomVersion {
                room_version: room_version.to_string(),
            });
        }
        let room_create_event_content = build_room_create_event_content(
            creator_user_id,
            room_version,
            &preset,
            &invite,
            creation_content,
        )?;

        if let Some(power_level_content_override) = power_level_content_override.as_ref()
            && !power_level_content_override.is_object()
        {
            return Err(RoomCreateContractError::InvalidPowerLevelContentOverride);
        }

        let room_id = RoomIdentifier::generate(home_server_name);
        let room = Self::new(&room_id, room_version)?;

        if let Some(room_alias_name) = room_alias_name.as_deref()
            && (room_alias_name.trim().is_empty()
                || room_alias_name.contains(':')
                || room_alias_name.contains('\0'))
        {
            return Err(RoomCreateContractError::InvalidRoomAlias);
        }

        if let Some(invitees) = invite.as_ref()
            && invitees
                .iter()
                .any(|invitee| UserIdentifier::try_from(invitee.clone()).is_err())
        {
            return Err(RoomCreateContractError::InvalidInviteUserIdentifier);
        }

        if let Some(invitees) = invite_3pid.as_ref()
            && invitees
                .iter()
                .any(InviteThirdPartyIdentifierInfo::has_empty_required_field)
        {
            return Err(RoomCreateContractError::InvalidInviteThirdPartyIdentifier);
        }

        let initial_state_payload = build_ordered_room_state_events(RoomCreationEventPlanInput {
            creator_user_id: creator_user_id.as_user_identifier().as_str(),
            room_id: room_id.as_str(),
            home_server_name: home_server_name.as_str(),
            room_create_event_content,
            power_level_content_override,
            room_alias_name: room_alias_name.as_deref(),
            preset: &preset,
            initial_state,
            name: name.as_deref(),
            topic: topic.as_deref(),
            invitees: invite.as_deref().unwrap_or(&[]),
            third_party_invitees: invite_3pid.as_deref().unwrap_or(&[]),
            is_direct,
        })?;

        let payload = CreateRoomPersistencePayload {
            creator_user_id: creator_user_id.as_user_identifier().as_str().to_owned(),
            initial_state: initial_state_payload,
            is_direct,
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

fn validate_creation_content(
    creator_user_id: &AuthorizedUserIdentifier,
    room_version: RoomVersion,
    creation_content: Option<Value>,
) -> Result<Value, RoomCreateContractError> {
    let mut creation_content = match creation_content {
        Some(content) => content,
        None => Value::Object(Map::new()),
    };
    let creation_content_object = creation_content
        .as_object_mut()
        .ok_or(RoomCreateContractError::InvalidCreationContent)?;

    // `creator` and `room_version` are server-owned keys in createRoom.
    creation_content_object.insert(
        "creator".to_owned(),
        Value::String(creator_user_id.as_user_identifier().as_str().to_owned()),
    );
    creation_content_object.insert(
        "room_version".to_owned(),
        Value::String(room_version.to_string()),
    );

    let room_create_content = serde_json::from_value::<RoomCreateContent>(creation_content.clone())
        .map_err(|error| RoomCreateContractError::InvalidRoomCreateContent {
            reason: error.to_string(),
        })?;
    room_create_content.validate().map_err(|error| {
        RoomCreateContractError::InvalidRoomCreateContent {
            reason: error.to_string(),
        }
    })?;

    Ok(creation_content)
}

fn build_room_create_event_content(
    creator_user_id: &AuthorizedUserIdentifier,
    room_version: RoomVersion,
    preset: &RoomPreset,
    invitees: &Option<Vec<String>>,
    creation_content: Option<Value>,
) -> Result<Value, RoomCreateContractError> {
    let mut room_create_event_content =
        validate_creation_content(creator_user_id, room_version, creation_content)?;
    if matches!(preset, RoomPreset::TrustedPrivateChat)
        && let Some(invitees) = invitees.as_ref()
        && !invitees.is_empty()
    {
        let room_create_content = room_create_event_content
            .as_object_mut()
            .ok_or(RoomCreateContractError::InvalidCreationContent)?;
        let invitee_values = invitees
            .iter()
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>();
        let additional_creators = room_create_content
            .entry("additional_creators")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(additional_creators) = additional_creators.as_array_mut() {
            for invitee in invitee_values {
                if !additional_creators.contains(&invitee) {
                    additional_creators.push(invitee);
                }
            }
        }
    }

    Ok(room_create_event_content)
}

fn validate_initial_state_event(
    event: &crate::services::rooms::handlers::create_room::RoomStateEventInfo,
) -> Result<(), RoomCreateContractError> {
    if event.event_type.trim().is_empty() {
        return Err(RoomCreateContractError::InvalidRoomStateEventType);
    }
    if event.event_type == "m.room.create" {
        return Err(RoomCreateContractError::InitialStateContainsRoomCreateEvent);
    }
    if !event.content.is_object() {
        return Err(RoomCreateContractError::InvalidRoomStateEventContentType {
            event_type: event.event_type.clone(),
        });
    }

    Ok(())
}

struct RoomCreationEventPlanInput<'a> {
    creator_user_id: &'a str,
    room_id: &'a str,
    home_server_name: &'a str,
    room_create_event_content: Value,
    power_level_content_override: Option<Value>,
    room_alias_name: Option<&'a str>,
    preset: &'a RoomPreset,
    initial_state: Option<Vec<crate::services::rooms::handlers::create_room::RoomStateEventInfo>>,
    name: Option<&'a str>,
    topic: Option<&'a str>,
    invitees: &'a [String],
    third_party_invitees: &'a [InviteThirdPartyIdentifierInfo],
    is_direct: Option<bool>,
}

fn build_ordered_room_state_events(
    input: RoomCreationEventPlanInput<'_>,
) -> Result<Vec<CreateRoomStateEventPayload>, RoomCreateContractError> {
    let mut planned_events = Vec::<CreateRoomStateEventPayload>::new();

    // 1. m.room.create
    push_room_state_event(
        &mut planned_events,
        "m.room.create",
        "",
        input.room_create_event_content,
    );
    // 2. creator join membership
    push_room_state_event(
        &mut planned_events,
        "m.room.member",
        input.creator_user_id,
        json!({
            "membership": "join"
        }),
    );
    // 3. default power levels (with override)
    let power_levels_content = build_default_power_levels_content(
        input.creator_user_id,
        input.preset,
        input.invitees,
        input.power_level_content_override,
    )?;
    push_room_state_event(
        &mut planned_events,
        "m.room.power_levels",
        "",
        power_levels_content,
    );
    // 4. canonical alias if provided
    if let Some(alias_localpart) = input.room_alias_name {
        push_room_state_event(
            &mut planned_events,
            "m.room.canonical_alias",
            "",
            json!({
                "alias": format!("#{alias_localpart}:{}", input.home_server_name),
            }),
        );
    }
    // 5. preset defaults
    apply_preset_events(&mut planned_events, input.preset);
    // 6. initial_state
    if let Some(initial_state) = input.initial_state {
        for initial_state_event in initial_state {
            validate_initial_state_event(&initial_state_event)?;
            push_room_state_event(
                &mut planned_events,
                &initial_state_event.event_type,
                &initial_state_event.state_key.unwrap_or_default(),
                initial_state_event.content,
            );
        }
    }
    // 7. name/topic overrides
    if let Some(name) = input.name {
        push_room_state_event(
            &mut planned_events,
            "m.room.name",
            "",
            json!({
                "name": name,
            }),
        );
    }
    if let Some(topic) = input.topic {
        push_room_state_event(
            &mut planned_events,
            "m.room.topic",
            "",
            json!({
                "topic": topic,
            }),
        );
    }
    // 8. invite and third-party invite events
    for invitee in input.invitees {
        push_room_state_event(
            &mut planned_events,
            "m.room.member",
            invitee,
            json!({
                "membership": "invite",
                "is_direct": input.is_direct.unwrap_or(false),
            }),
        );
    }
    for third_party_invitee in input.third_party_invitees {
        let token = format!("invite_{}", Uuid::new_v4().simple());
        push_room_state_event(
            &mut planned_events,
            "m.room.third_party_invite",
            &token,
            json!({
                "display_name": third_party_invitee.address,
                "key_validity_url": format!("https://{}/_matrix/identity/api/v1/pubkey/isvalid", third_party_invitee.id_server),
                "public_key": "",
                "medium": third_party_invitee.medium,
                "id_server": third_party_invitee.id_server,
            }),
        );
    }

    planned_events = collapse_duplicate_state_events(planned_events);
    for (index, event) in planned_events.iter_mut().enumerate() {
        event.ordering = i32::try_from(index).unwrap_or(i32::MAX);
    }
    validate_room_creation_event_order(&planned_events, input.room_id, input.creator_user_id)?;

    Ok(planned_events)
}

fn collapse_duplicate_state_events(
    events: Vec<CreateRoomStateEventPayload>,
) -> Vec<CreateRoomStateEventPayload> {
    use std::collections::HashMap;

    let mut latest_index_by_state_tuple = HashMap::new();
    for (index, event) in events.iter().enumerate() {
        latest_index_by_state_tuple
            .insert((event.event_type.clone(), event.state_key.clone()), index);
    }

    let mut collapsed_events = Vec::with_capacity(latest_index_by_state_tuple.len());
    for (index, event) in events.into_iter().enumerate() {
        let event_state_tuple = (event.event_type.clone(), event.state_key.clone());
        if latest_index_by_state_tuple.get(&event_state_tuple).copied() == Some(index) {
            collapsed_events.push(event);
        }
    }

    collapsed_events
}

fn push_room_state_event(
    planned_events: &mut Vec<CreateRoomStateEventPayload>,
    event_type: &str,
    state_key: &str,
    content: Value,
) {
    planned_events.push(CreateRoomStateEventPayload {
        content,
        event_type: event_type.to_owned(),
        ordering: 0,
        state_key: state_key.to_owned(),
    });
}

fn apply_preset_events(planned_events: &mut Vec<CreateRoomStateEventPayload>, preset: &RoomPreset) {
    let (join_rule, guest_access) = match preset {
        RoomPreset::PrivateChat | RoomPreset::TrustedPrivateChat => ("invite", "can_join"),
        RoomPreset::PublicChat => ("public", "forbidden"),
    };
    push_room_state_event(
        planned_events,
        "m.room.join_rules",
        "",
        json!({ "join_rule": join_rule }),
    );
    push_room_state_event(
        planned_events,
        "m.room.history_visibility",
        "",
        json!({ "history_visibility": "shared" }),
    );
    push_room_state_event(
        planned_events,
        "m.room.guest_access",
        "",
        json!({ "guest_access": guest_access }),
    );
}

fn build_default_power_levels_content(
    creator_user_id: &str,
    preset: &RoomPreset,
    invitees: &[String],
    power_level_content_override: Option<Value>,
) -> Result<Value, RoomCreateContractError> {
    let mut users = Map::new();
    users.insert(creator_user_id.to_owned(), json!(100));
    if matches!(preset, RoomPreset::TrustedPrivateChat) {
        for invitee in invitees {
            users.insert(invitee.clone(), json!(100));
        }
    }

    let mut power_levels = Map::new();
    power_levels.insert("ban".to_owned(), json!(50));
    power_levels.insert("events".to_owned(), Value::Object(Map::new()));
    power_levels.insert("events_default".to_owned(), json!(0));
    power_levels.insert("invite".to_owned(), json!(0));
    power_levels.insert("kick".to_owned(), json!(50));
    power_levels.insert("redact".to_owned(), json!(50));
    power_levels.insert("state_default".to_owned(), json!(50));
    power_levels.insert("users".to_owned(), Value::Object(users));
    power_levels.insert("users_default".to_owned(), json!(0));

    if let Some(override_content) = power_level_content_override {
        let override_content = override_content
            .as_object()
            .ok_or(RoomCreateContractError::InvalidPowerLevelContentOverride)?;
        for (key, value) in override_content {
            power_levels.insert(key.clone(), value.clone());
        }
    }

    Ok(Value::Object(power_levels))
}

fn validate_room_creation_event_order(
    planned_events: &[CreateRoomStateEventPayload],
    room_id: &str,
    creator_user_id: &str,
) -> Result<(), RoomCreateContractError> {
    let Some(first_event) = planned_events.first() else {
        return Err(RoomCreateContractError::EmptyRoomCreationEventPlan);
    };
    if first_event.event_type != "m.room.create" {
        return Err(RoomCreateContractError::InvalidRoomCreationEventOrder {
            expected: "m.room.create",
            received: first_event.event_type.clone(),
        });
    }

    let Some(second_event) = planned_events.get(1) else {
        return Err(RoomCreateContractError::InvalidRoomCreationEventOrder {
            expected: "m.room.member",
            received: "missing".to_owned(),
        });
    };
    if second_event.event_type != "m.room.member" || second_event.state_key != creator_user_id {
        return Err(RoomCreateContractError::InvalidRoomCreationEventOrder {
            expected: "creator m.room.member",
            received: format!("{}:{}", second_event.event_type, second_event.state_key),
        });
    }

    let Some(third_event) = planned_events.get(2) else {
        return Err(RoomCreateContractError::InvalidRoomCreationEventOrder {
            expected: "m.room.power_levels",
            received: "missing".to_owned(),
        });
    };
    if third_event.event_type != "m.room.power_levels" {
        return Err(RoomCreateContractError::InvalidRoomCreationEventOrder {
            expected: "m.room.power_levels",
            received: third_event.event_type.clone(),
        });
    }

    let room_identifier = RoomIdentifier::parse(room_id).ok_or_else(|| {
        RoomCreateContractError::InvalidGeneratedRoomIdentifier {
            room_id: room_id.to_owned(),
        }
    })?;
    let room_domain = room_identifier.server_name().ok_or_else(|| {
        RoomCreateContractError::InvalidGeneratedRoomIdentifier {
            room_id: room_id.to_owned(),
        }
    })?;
    let creator_domain = creator_user_id
        .split_once(':')
        .map(|(_, domain)| domain)
        .ok_or(RoomCreateContractError::InvalidInviteUserIdentifier)?;
    if room_domain != creator_domain {
        return Err(
            RoomCreateContractError::RoomDomainAndCreatorDomainMismatch {
                room_domain: room_domain.to_owned(),
                creator_domain: creator_domain.to_owned(),
            },
        );
    }

    Ok(())
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

impl InviteThirdPartyIdentifierInfo {
    fn has_empty_required_field(&self) -> bool {
        self.id_server.trim().is_empty()
            || self.id_access_token.trim().is_empty()
            || self.medium.trim().is_empty()
            || self.address.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        infrastructure::server_name::ServerName,
        services::{
            authorization::entities::AuthorizedUserIdentifier,
            rooms::{
                entities::{Room, RoomCreateContractError},
                handlers::create_room::{
                    CreateRoomInfo, RoomPreset, RoomStateEventInfo, RoomVisibility,
                },
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

    fn base_create_room_info() -> CreateRoomInfo {
        CreateRoomInfo {
            creation_content: None,
            initial_state: None,
            invite: None,
            invite_3pid: None,
            is_direct: None,
            name: None,
            power_level_content_override: None,
            preset: RoomPreset::PrivateChat,
            room_alias_name: None,
            room_version: Some("1".to_owned()),
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
        create_room_info.room_version = Some("12".to_owned());

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
        create_room_info.initial_state = Some(vec![RoomStateEventInfo {
            content: json!({ "enabled": true }),
            state_key: None,
            event_type: "com.example.custom".to_owned(),
        }]);

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
        create_room_info.initial_state = Some(vec![RoomStateEventInfo {
            content: json!("invalid"),
            state_key: None,
            event_type: "m.room.topic".to_owned(),
        }]);

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
        create_room_info.initial_state = Some(vec![RoomStateEventInfo {
            content: json!({}),
            state_key: Some(String::new()),
            event_type: "m.room.create".to_owned(),
        }]);

        let error =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect_err("m.room.create in initial_state must be rejected");
        assert!(matches!(
            error,
            RoomCreateContractError::InitialStateContainsRoomCreateEvent
        ));
    }

    #[test]
    fn create_room_contract_rejects_non_object_creation_content() {
        let mut create_room_info = base_create_room_info();
        create_room_info.creation_content = Some(json!("invalid"));

        let error =
            Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
                .expect_err("creation_content must be an object");
        assert!(matches!(
            error,
            RoomCreateContractError::InvalidCreationContent
        ));
    }

    #[test]
    fn create_room_contract_overrides_client_creator_and_room_version() {
        let mut create_room_info = base_create_room_info();
        create_room_info.creation_content = Some(json!({
            "creator": "not-a-valid-user-id",
            "room_version": "12"
        }));

        Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
            .expect("server-owned creator and room_version keys must be overridden");
    }

    #[test]
    fn create_room_contract_does_not_validate_additional_creators_in_v1() {
        let mut create_room_info = base_create_room_info();
        create_room_info.creation_content = Some(json!({
            "additional_creators": ["not-a-valid-user-id"]
        }));

        Room::try_create_contract(&creator_user_id(), create_room_info, &home_server_name())
            .expect("additional_creators must not be validated for v1 rooms");
    }

    #[test]
    fn create_room_contract_collapses_duplicate_state_keys_with_last_write_wins() {
        let mut create_room_info = base_create_room_info();
        create_room_info.initial_state = Some(vec![
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
        ]);

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
