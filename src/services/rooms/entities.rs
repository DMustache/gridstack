use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::infrastructure::{server_name::ServerName, user_identifier::UserIdentifier};
use crate::services::authorization::entities::ExistingUserIdentifier;
use crate::services::events::entities::{
    EventIntent, MatrixEventContent, MessageEventKind, StateEventKind, SupportedRoomVersion,
};

#[derive(Clone, Debug, Deserialize)]
pub struct CreateRoomRequestDto {
    pub creation_content: Option<CreateRoomCreationContentDto>,
    pub initial_state: Option<Vec<RawInitialStateEventDto>>,
    pub invite: Option<Vec<String>>,
    pub invite_3pid: Option<Vec<InviteThirdPartyDto>>,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub power_level_content_override: Option<RoomPowerLevelsOverrideDto>,
    pub preset: Option<RoomPreset>,
    pub room_alias_name: Option<String>,
    pub room_version: Option<String>,
    pub topic: Option<String>,
    pub visibility: Option<RoomVisibility>,
}

#[derive(Clone, Debug)]
pub struct CreateRoomCommand {
    pub creation_content: ValidatedCreationContent,
    pub initial_state: Vec<ValidatedInitialStateEventInput>,
    pub invite: Vec<UserIdentifier>,
    pub invite_3pid: Vec<ValidatedThirdPartyInvite>,
    pub is_direct: bool,
    pub name: Option<ValidatedRoomName>,
    pub power_level_content_override: Option<RoomPowerLevelsOverrideDto>,
    pub preset: Option<RoomPreset>,
    pub room_alias_name: Option<ValidatedRoomAliasLocalPart>,
    pub room_version: Option<String>,
    pub topic: Option<ValidatedRoomTopic>,
    pub visibility: Option<RoomVisibility>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct JoinRoomRequestDto {
    pub reason: Option<String>,
    pub third_party_signed: Option<ThirdPartySignedDto>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct LeaveRoomRequestDto {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ThirdPartySignedDto {
    pub sender: String,
    pub mxid: String,
    pub token: String,
    pub signatures: Value,
}

#[derive(Clone, Debug, Default)]
pub struct JoinRoomCommand {
    pub reason: Option<String>,
    pub third_party_signed: Option<ThirdPartySignedDto>,
}

impl From<JoinRoomRequestDto> for JoinRoomCommand {
    fn from(value: JoinRoomRequestDto) -> Self {
        Self {
            reason: value.reason,
            third_party_signed: value.third_party_signed,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LeaveRoomCommand {
    pub reason: Option<String>,
}

impl From<LeaveRoomRequestDto> for LeaveRoomCommand {
    fn from(value: LeaveRoomRequestDto) -> Self {
        Self {
            reason: value.reason,
        }
    }
}

impl TryFrom<CreateRoomRequestDto> for CreateRoomCommand {
    type Error = RoomValidationError;

    fn try_from(value: CreateRoomRequestDto) -> Result<Self, Self::Error> {
        let mut invite = Vec::new();
        for invited_user_id in value.invite.unwrap_or_default() {
            let user_identifier =
                UserIdentifier::try_from(invited_user_id.clone()).map_err(|_| {
                    RoomValidationError::InvalidInviteUserIdentifier {
                        user_id: invited_user_id,
                    }
                })?;
            invite.push(user_identifier);
        }

        let mut invite_3pid = Vec::new();
        for invite_3pid_value in value.invite_3pid.unwrap_or_default() {
            invite_3pid.push(validate_third_party_invite(invite_3pid_value)?);
        }

        let room_alias_name = match value.room_alias_name {
            Some(value) => Some(
                ValidatedRoomAliasLocalPart::parse(value)
                    .ok_or(RoomValidationError::InvalidRoomAlias)?,
            ),
            None => None,
        };

        let name = match value.name {
            Some(value) => {
                Some(ValidatedRoomName::parse(value).ok_or(RoomValidationError::InvalidRoomName)?)
            }
            None => None,
        };

        let topic = match value.topic {
            Some(value) => Some(
                ValidatedRoomTopic::parse(value).ok_or(RoomValidationError::InvalidRoomTopic)?,
            ),
            None => None,
        };

        Ok(Self {
            creation_content: validate_creation_content(
                value.creation_content.unwrap_or_default(),
            )?,
            initial_state: validate_initial_state(value.initial_state.unwrap_or_default())?,
            invite,
            invite_3pid,
            is_direct: value.is_direct.unwrap_or(false),
            name,
            power_level_content_override: value.power_level_content_override,
            preset: value.preset,
            room_alias_name,
            room_version: value.room_version,
            topic,
            visibility: value.visibility,
        })
    }
}

fn validate_creation_content(
    creation_content: CreateRoomCreationContentDto,
) -> Result<ValidatedCreationContent, RoomValidationError> {
    let mut additional_creators = Vec::new();
    for creator in creation_content.additional_creators.unwrap_or_default() {
        let creator = UserIdentifier::try_from(creator)
            .map_err(|_| RoomValidationError::InvalidCreationContent)?;
        additional_creators.push(creator);
    }

    if let Some(predecessor) = creation_content.predecessor.as_ref()
        && (predecessor.event_id.trim().is_empty()
            || RoomIdentifier::parse(predecessor.room_id.clone()).is_none())
    {
        return Err(RoomValidationError::InvalidPredecessor);
    }

    if creation_content
        .room_type
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(RoomValidationError::InvalidCreationContent);
    }

    Ok(ValidatedCreationContent {
        additional_creators,
        federate: creation_content.federate.unwrap_or(true),
        predecessor: creation_content.predecessor,
        room_type: creation_content.room_type,
    })
}

fn validate_initial_state(
    initial_state: Vec<RawInitialStateEventDto>,
) -> Result<Vec<ValidatedInitialStateEventInput>, RoomValidationError> {
    let mut validated_initial_state = Vec::with_capacity(initial_state.len());
    for raw_state_event in initial_state {
        let event_type = raw_state_event
            .event_type
            .trim()
            .parse::<StateEventKind>()
            .map_err(|_| RoomValidationError::InvalidInitialStateEventType)?;
        if matches!(event_type, StateEventKind::RoomCreate) {
            return Err(RoomValidationError::InitialStateContainsRoomCreate);
        }
        if !raw_state_event.content.is_object() {
            return Err(RoomValidationError::InvalidInitialStateEventContent {
                event_type: event_type.as_ref().to_owned(),
            });
        }

        let parsed_content =
            parse_room_state_event_content(event_type.clone(), raw_state_event.content).map_err(
                |_| RoomValidationError::InvalidInitialStateEventContent {
                    event_type: event_type.as_ref().to_owned(),
                },
            )?;

        validated_initial_state.push(ValidatedInitialStateEventInput {
            event_type,
            state_key: ValidatedStateKey::parse(raw_state_event.state_key),
            content: parsed_content,
        });
    }
    Ok(validated_initial_state)
}

fn validate_third_party_invite(
    invite: InviteThirdPartyDto,
) -> Result<ValidatedThirdPartyInvite, RoomValidationError> {
    if invite.id_server.trim().is_empty()
        || invite.id_access_token.trim().is_empty()
        || invite.medium.trim().is_empty()
        || invite.address.trim().is_empty()
    {
        return Err(RoomValidationError::InvalidThirdPartyInvite);
    }

    Ok(ValidatedThirdPartyInvite {
        id_server: invite.id_server,
        id_access_token: invite.id_access_token,
        medium: invite.medium,
        address: invite.address,
    })
}

pub(crate) fn parse_room_state_event_content(
    state_event_kind: StateEventKind,
    content: Value,
) -> Result<MatrixEventContent, ()> {
    Ok(match state_event_kind {
        StateEventKind::RoomCreate => {
            let parsed: RoomCreateStateContent = serde_json::from_value(content).map_err(|_| ())?;
            MatrixEventContent::RoomCreate(crate::services::events::entities::RoomCreateContent {
                creator: parsed.creator.unwrap_or_default(),
                room_version: parsed.room_version.unwrap_or_else(|| "1".to_owned()),
                federate: parsed.federate.unwrap_or(true),
                room_type: parsed.room_type,
                predecessor_event_id: parsed.predecessor.as_ref().map(|p| p.event_id.clone()),
                predecessor_room_id: parsed.predecessor.map(|p| p.room_id),
                additional_creators: parsed.additional_creators.unwrap_or_default(),
            })
        }
        StateEventKind::RoomMember => {
            let parsed: RoomMemberStateContent = serde_json::from_value(content).map_err(|_| ())?;
            MatrixEventContent::RoomMember(crate::services::events::entities::RoomMemberContent {
                membership: parsed.membership.try_into()?,
                is_direct: parsed.is_direct,
            })
        }
        StateEventKind::RoomPowerLevels => {
            let parsed: RoomPowerLevelsStateContent =
                serde_json::from_value(content).map_err(|_| ())?;
            MatrixEventContent::RoomPowerLevels(parsed.into_domain())
        }
        StateEventKind::RoomJoinRules => MatrixEventContent::RoomJoinRules {
            join_rule: serde_json::from_value::<RoomJoinRulesStateContent>(content)
                .map_err(|_| ())?
                .join_rule,
        },
        StateEventKind::RoomHistoryVisibility => MatrixEventContent::RoomHistoryVisibility {
            history_visibility: serde_json::from_value::<RoomHistoryVisibilityStateContent>(
                content,
            )
            .map_err(|_| ())?
            .history_visibility,
        },
        StateEventKind::RoomGuestAccess => MatrixEventContent::RoomGuestAccess {
            guest_access: serde_json::from_value::<RoomGuestAccessStateContent>(content)
                .map_err(|_| ())?
                .guest_access,
        },
        StateEventKind::RoomCanonicalAlias => MatrixEventContent::RoomCanonicalAlias {
            alias: serde_json::from_value::<RoomCanonicalAliasStateContent>(content)
                .map_err(|_| ())?
                .alias,
        },
        StateEventKind::RoomName => MatrixEventContent::RoomName {
            name: serde_json::from_value::<RoomNameStateContent>(content)
                .map_err(|_| ())?
                .name,
        },
        StateEventKind::RoomTopic => MatrixEventContent::RoomTopic {
            topic: serde_json::from_value::<RoomTopicStateContent>(content)
                .map_err(|_| ())?
                .topic,
        },
        StateEventKind::ThirdPartyInvite => {
            let parsed: RoomThirdPartyInviteStateContent =
                serde_json::from_value(content).map_err(|_| ())?;
            MatrixEventContent::RoomThirdPartyInvite {
                display_name: parsed.display_name,
                key_validity_url: parsed.key_validity_url,
                public_key: parsed.public_key,
                medium: parsed.medium,
                id_server: parsed.id_server,
            }
        }
        StateEventKind::Custom(_) => MatrixEventContent::CustomJson(content),
    })
}

pub(crate) fn parse_room_message_event_content(
    event_type: &str,
    content: Value,
) -> Result<(MessageEventKind, MatrixEventContent), ()> {
    let message_event_kind = event_type.parse::<MessageEventKind>()?;
    let message_content = match &message_event_kind {
        MessageEventKind::RoomMessage => {
            let parsed: RoomMessageEventContent =
                serde_json::from_value(content).map_err(|_| ())?;
            if parsed.body.trim().is_empty() || parsed.msgtype.trim().is_empty() {
                return Err(());
            }
            MatrixEventContent::RoomMessage {
                body: parsed.body,
                msgtype: parsed.msgtype,
            }
        }
        MessageEventKind::RoomRedaction => {
            let parsed: RoomRedactionEventContent =
                serde_json::from_value(content).map_err(|_| ())?;
            if parsed.redacts.trim().is_empty() {
                return Err(());
            }
            MatrixEventContent::RoomRedaction {
                redacts: parsed.redacts,
                reason: parsed.reason,
            }
        }
        MessageEventKind::Custom(_) => MatrixEventContent::CustomJson(content),
    };

    Ok((message_event_kind, message_content))
}

#[derive(Clone, Debug, Deserialize)]
struct RoomPredecessorStateContent {
    event_id: String,
    room_id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomCreateStateContent {
    creator: Option<String>,
    room_version: Option<String>,
    #[serde(rename = "m.federate")]
    federate: Option<bool>,
    #[serde(rename = "type")]
    room_type: Option<String>,
    predecessor: Option<RoomPredecessorStateContent>,
    additional_creators: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MembershipStateValue {
    Join,
    Invite,
    Leave,
    Ban,
    Knock,
}

impl TryFrom<MembershipStateValue> for crate::services::events::entities::MembershipContent {
    type Error = ();

    fn try_from(value: MembershipStateValue) -> Result<Self, Self::Error> {
        Ok(match value {
            MembershipStateValue::Join => Self::Join,
            MembershipStateValue::Invite => Self::Invite,
            MembershipStateValue::Leave => Self::Leave,
            MembershipStateValue::Ban => Self::Ban,
            MembershipStateValue::Knock => Self::Knock,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RoomMemberStateContent {
    membership: MembershipStateValue,
    is_direct: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomPowerLevelsNotificationsStateContent {
    room: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomPowerLevelsStateContent {
    ban: Option<i64>,
    events_default: Option<i64>,
    invite: Option<i64>,
    kick: Option<i64>,
    redact: Option<i64>,
    state_default: Option<i64>,
    users_default: Option<i64>,
    users: Option<BTreeMap<String, i64>>,
    notifications: Option<RoomPowerLevelsNotificationsStateContent>,
}

impl RoomPowerLevelsStateContent {
    fn into_domain(self) -> crate::services::events::entities::RoomPowerLevelsContent {
        crate::services::events::entities::RoomPowerLevelsContent {
            ban: self.ban.unwrap_or(50),
            events_default: self.events_default.unwrap_or(0),
            invite: self.invite.unwrap_or(0),
            kick: self.kick.unwrap_or(50),
            redact: self.redact.unwrap_or(50),
            state_default: self.state_default.unwrap_or(50),
            users_default: self.users_default.unwrap_or(0),
            users: self
                .users
                .unwrap_or_default()
                .into_iter()
                .map(
                    |(user_id, level)| crate::services::events::entities::UserPowerLevel {
                        user_id,
                        level,
                    },
                )
                .collect(),
            notifications_room: self.notifications.and_then(|value| value.room),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RoomJoinRulesStateContent {
    join_rule: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomHistoryVisibilityStateContent {
    history_visibility: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomGuestAccessStateContent {
    guest_access: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomCanonicalAliasStateContent {
    alias: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomNameStateContent {
    name: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomTopicStateContent {
    topic: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomThirdPartyInviteStateContent {
    display_name: String,
    key_validity_url: String,
    public_key: String,
    medium: String,
    id_server: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomMessageEventContent {
    body: String,
    msgtype: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RoomRedactionEventContent {
    redacts: String,
    reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct CreateRoomCreationContentDto {
    #[serde(default)]
    pub additional_creators: Option<Vec<String>>,
    #[serde(default, rename = "m.federate")]
    pub federate: Option<bool>,
    pub predecessor: Option<PreviousRoomDto>,
    #[serde(rename = "type")]
    pub room_type: Option<String>,
    pub creator: Option<String>,
    pub room_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawInitialStateEventDto {
    pub content: Value,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct InviteThirdPartyDto {
    pub id_server: String,
    pub id_access_token: String,
    pub medium: String,
    pub address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreviousRoomDto {
    pub event_id: String,
    pub room_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RoomPowerLevelNotifications {
    pub room: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RoomPowerLevelsOverrideDto {
    pub ban: Option<i64>,
    pub events: Option<BTreeMap<String, i64>>,
    pub events_default: Option<i64>,
    pub invite: Option<i64>,
    pub kick: Option<i64>,
    pub notifications: Option<RoomPowerLevelNotifications>,
    pub redact: Option<i64>,
    pub state_default: Option<i64>,
    pub users: Option<BTreeMap<String, i64>>,
    pub users_default: Option<i64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoomVisibility {
    Public,
    #[default]
    Private,
}

impl RoomVisibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomPreset {
    PrivateChat,
    PublicChat,
    TrustedPrivateChat,
}

impl RoomPreset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrivateChat => "private_chat",
            Self::PublicChat => "public_chat",
            Self::TrustedPrivateChat => "trusted_private_chat",
        }
    }
}

impl From<RoomVisibility> for RoomPreset {
    fn from(value: RoomVisibility) -> Self {
        match value {
            RoomVisibility::Public => Self::PublicChat,
            RoomVisibility::Private => Self::PrivateChat,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValidatedRoomAliasLocalPart(String);

impl ValidatedRoomAliasLocalPart {
    pub fn parse(value: String) -> Option<Self> {
        let candidate = value.trim();
        if candidate.is_empty()
            || candidate.contains(':')
            || candidate.contains('#')
            || candidate.contains('\0')
            || candidate.contains(' ')
        {
            return None;
        }
        Some(Self(candidate.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValidatedRoomName(String);

impl ValidatedRoomName {
    pub fn parse(value: String) -> Option<Self> {
        let candidate = value.trim();
        if candidate.is_empty() {
            return None;
        }
        Some(Self(candidate.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValidatedRoomTopic(String);

impl ValidatedRoomTopic {
    pub fn parse(value: String) -> Option<Self> {
        let candidate = value.trim();
        if candidate.is_empty() {
            return None;
        }
        Some(Self(candidate.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValidatedStateKey(String);

impl ValidatedStateKey {
    pub fn parse(value: Option<String>) -> Self {
        Self(value.unwrap_or_default())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct ValidatedInitialStateEventInput {
    pub event_type: StateEventKind,
    pub state_key: ValidatedStateKey,
    pub content: MatrixEventContent,
}

#[derive(Clone, Debug)]
pub struct ValidatedCreationContent {
    pub additional_creators: Vec<UserIdentifier>,
    pub federate: bool,
    pub predecessor: Option<PreviousRoomDto>,
    pub room_type: Option<String>,
}

impl Default for ValidatedCreationContent {
    fn default() -> Self {
        Self {
            additional_creators: Vec::new(),
            federate: true,
            predecessor: None,
            room_type: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidatedThirdPartyInvite {
    pub id_server: String,
    pub id_access_token: String,
    pub medium: String,
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct ValidatedCreateRoomInput {
    pub creator: ExistingUserIdentifier,
    pub creation_content: ValidatedCreationContent,
    pub initial_state: Vec<ValidatedInitialStateEventInput>,
    pub invite: Vec<ExistingUserIdentifier>,
    pub invite_3pid: Vec<ValidatedThirdPartyInvite>,
    pub is_direct: bool,
    pub name: Option<ValidatedRoomName>,
    pub topic: Option<ValidatedRoomTopic>,
    pub power_level_content_override: RoomPowerLevelsOverrideDto,
    pub preset: RoomPreset,
    pub room_alias_name: Option<ValidatedRoomAliasLocalPart>,
    pub room_version: SupportedRoomVersion,
    pub visibility: RoomVisibility,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RoomIdentifier {
    localpart: String,
    server_name: ServerName,
    full_identifier: Arc<String>,
}

impl RoomIdentifier {
    pub fn generate(server_name: &ServerName) -> Self {
        let localpart = uuid::Uuid::new_v4().simple().to_string();
        Self {
            full_identifier: Arc::new(format!("!{}:{}", localpart, server_name.as_str())),
            localpart,
            server_name: server_name.clone(),
        }
    }

    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        let room_identifier_without_prefix = candidate.strip_prefix('!')?;
        let (localpart, server_name) =
            ServerName::split_localpart_and_server_name(room_identifier_without_prefix)?;
        let server_name = ServerName::try_new(server_name)?;
        Some(Self {
            full_identifier: Arc::new(format!("!{}:{}", localpart, server_name.as_str())),
            localpart: localpart.to_owned(),
            server_name,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.full_identifier
    }

    pub fn server_name(&self) -> &ServerName {
        &self.server_name
    }
}

#[derive(Clone, Debug)]
pub struct RoomShellIntent {
    pub room_id: String,
    pub room_version: SupportedRoomVersion,
    pub creator_user_id: String,
    pub is_direct: bool,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub visibility: RoomVisibility,
    pub preset: RoomPreset,
}

#[derive(Clone, Debug)]
pub struct AliasIntent {
    pub alias_localpart: String,
    pub room_id: String,
}

#[derive(Clone, Debug)]
pub struct DirectoryVisibilityIntent {
    pub room_id: String,
    pub visibility: RoomVisibility,
}

#[derive(Clone, Debug)]
pub struct RoomCreationFlow {
    pub room_shell_intent: RoomShellIntent,
    pub alias_intent: Option<AliasIntent>,
    pub directory_visibility_intent: Option<DirectoryVisibilityIntent>,
    pub ordered_event_intents: Vec<EventIntent>,
}

#[derive(Clone, Copy, Debug)]
pub enum RoomFactoryEvent {
    RequiredCreateEvent,
    CreatorJoinEvent,
    DefaultPowerLevelsEvent,
    CanonicalAliasEventIfNeeded,
    PresetEvents,
    InitialStateEvents,
    NameAndTopicEvents,
    InviteEvents,
}

#[derive(Clone, Debug)]
pub struct CreatedRoom {
    pub room_id: String,
}

#[derive(Clone, Debug)]
pub struct JoinedRoom {
    pub room_id: String,
}

#[derive(Clone, Debug)]
pub struct JoinedRooms {
    pub room_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LeftRoom;

#[derive(Clone, Debug, Serialize)]
pub struct CreateRoomView {
    pub room_id: String,
}

impl From<CreatedRoom> for CreateRoomView {
    fn from(value: CreatedRoom) -> Self {
        Self {
            room_id: value.room_id,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct JoinRoomView {
    pub room_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct JoinedRoomsView {
    pub joined_rooms: Vec<String>,
}

impl From<JoinedRoom> for JoinRoomView {
    fn from(value: JoinedRoom) -> Self {
        Self {
            room_id: value.room_id,
        }
    }
}

impl From<JoinedRooms> for JoinedRoomsView {
    fn from(value: JoinedRooms) -> Self {
        Self {
            joined_rooms: value.room_ids,
        }
    }
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct LeaveRoomView {}

impl From<LeftRoom> for LeaveRoomView {
    fn from(_: LeftRoom) -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomStateEventView {
    pub content: Value,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    pub state_key: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomTimelineEventView {
    pub content: Value,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct RoomStateEvent {
    pub content: Value,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    pub state_key: String,
    pub event_type: String,
    pub unsigned: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct RoomTimelineEvent {
    pub content: Value,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    pub state_key: Option<String>,
    pub event_type: String,
    pub unsigned: Option<Value>,
    pub stream_position: i64,
}

#[derive(Clone, Debug)]
pub enum RoomMessageDirection {
    Backward,
    Forward,
}

#[derive(Clone, Debug)]
pub struct GetRoomMessagesCommand {
    pub from_token: Option<String>,
    pub to_token: Option<String>,
    pub direction: RoomMessageDirection,
    pub limit: usize,
    pub filter: Option<RoomEventFilter>,
}

#[derive(Clone, Debug)]
pub struct RoomMessagesPage {
    pub start: String,
    pub end: Option<String>,
    pub chunk: Vec<RoomTimelineEvent>,
    pub state: Vec<RoomTimelineEvent>,
}

#[derive(Clone, Debug, Default)]
pub struct RoomEventFilter {
    pub types: Option<Vec<String>>,
    pub not_types: Option<Vec<String>>,
    pub senders: Option<Vec<String>>,
    pub not_senders: Option<Vec<String>>,
    pub contains_url: Option<bool>,
}

impl From<RoomStateEvent> for RoomStateEventView {
    fn from(value: RoomStateEvent) -> Self {
        Self {
            content: value.content,
            event_id: value.event_id,
            origin_server_ts: value.origin_server_ts,
            room_id: value.room_id,
            sender: value.sender,
            state_key: value.state_key,
            event_type: value.event_type,
            unsigned: value.unsigned,
        }
    }
}

impl From<RoomTimelineEvent> for RoomTimelineEventView {
    fn from(value: RoomTimelineEvent) -> Self {
        Self {
            content: value.content,
            event_id: value.event_id,
            origin_server_ts: value.origin_server_ts,
            room_id: value.room_id,
            sender: value.sender,
            state_key: value.state_key,
            event_type: value.event_type,
            unsigned: value.unsigned,
        }
    }
}

#[derive(Debug, Error)]
pub enum RoomValidationError {
    #[error("invalid room alias localpart")]
    InvalidRoomAlias,
    #[error("invalid room name")]
    InvalidRoomName,
    #[error("invalid room topic")]
    InvalidRoomTopic,
    #[error("invalid initial state event type")]
    InvalidInitialStateEventType,
    #[error("invalid initial state event content for `{event_type}`")]
    InvalidInitialStateEventContent { event_type: String },
    #[error("initial_state must not include `m.room.create`")]
    InitialStateContainsRoomCreate,
    #[error("unsupported room version `{room_version}`")]
    UnsupportedRoomVersion { room_version: String },
    #[error("invalid invite user id `{user_id}`")]
    InvalidInviteUserIdentifier { user_id: String },
    #[error("invalid third-party invite object")]
    InvalidThirdPartyInvite,
    #[error("invalid creation content")]
    InvalidCreationContent,
    #[error("invalid predecessor in creation content")]
    InvalidPredecessor,
}
