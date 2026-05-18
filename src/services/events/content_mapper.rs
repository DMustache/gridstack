use serde::Serialize;
use serde_json::Value;

use crate::services::events::entities::{
    MatrixEventContent, MembershipContent, RoomCreateContent, RoomMemberContent,
    RoomPowerLevelsContent, UserPowerLevel,
};

#[derive(Clone, Debug, Serialize)]
struct MatrixPduPredecessorContent {
    event_id: String,
    room_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomCreateContent {
    creator: String,
    room_version: String,
    #[serde(rename = "m.federate")]
    federate: bool,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    room_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    predecessor: Option<MatrixPduPredecessorContent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    additional_creators: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixPduMembership {
    Join,
    Invite,
    Leave,
    Ban,
    Knock,
}

impl From<MembershipContent> for MatrixPduMembership {
    fn from(value: MembershipContent) -> Self {
        match value {
            MembershipContent::Join => Self::Join,
            MembershipContent::Invite => Self::Invite,
            MembershipContent::Leave => Self::Leave,
            MembershipContent::Ban => Self::Ban,
            MembershipContent::Knock => Self::Knock,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomMemberContent {
    membership: MatrixPduMembership,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_direct: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomPowerLevelUser {
    user_id: String,
    level: i64,
}

impl From<UserPowerLevel> for MatrixPduRoomPowerLevelUser {
    fn from(value: UserPowerLevel) -> Self {
        Self {
            user_id: value.user_id,
            level: value.level,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomPowerLevelNotifications {
    room: i64,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomPowerLevelsContent {
    ban: i64,
    events_default: i64,
    invite: i64,
    kick: i64,
    redact: i64,
    state_default: i64,
    users_default: i64,
    users: Vec<MatrixPduRoomPowerLevelUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notifications: Option<MatrixPduRoomPowerLevelNotifications>,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomJoinRulesContent {
    join_rule: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomHistoryVisibilityContent {
    history_visibility: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomGuestAccessContent {
    guest_access: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomCanonicalAliasContent {
    alias: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomNameContent {
    name: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomTopicContent {
    topic: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomThirdPartyInviteContent {
    display_name: String,
    key_validity_url: String,
    public_key: String,
    medium: String,
    id_server: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomMessageContent {
    body: String,
    msgtype: String,
}

#[derive(Clone, Debug, Serialize)]
struct MatrixPduRoomRedactionContent {
    redacts: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum MatrixPduContent {
    RoomCreate(MatrixPduRoomCreateContent),
    RoomMember(MatrixPduRoomMemberContent),
    RoomPowerLevels(MatrixPduRoomPowerLevelsContent),
    RoomJoinRules(MatrixPduRoomJoinRulesContent),
    RoomHistoryVisibility(MatrixPduRoomHistoryVisibilityContent),
    RoomGuestAccess(MatrixPduRoomGuestAccessContent),
    RoomCanonicalAlias(MatrixPduRoomCanonicalAliasContent),
    RoomName(MatrixPduRoomNameContent),
    RoomTopic(MatrixPduRoomTopicContent),
    RoomThirdPartyInvite(MatrixPduRoomThirdPartyInviteContent),
    RoomMessage(MatrixPduRoomMessageContent),
    RoomRedaction(MatrixPduRoomRedactionContent),
}

impl From<RoomCreateContent> for MatrixPduRoomCreateContent {
    fn from(value: RoomCreateContent) -> Self {
        let predecessor = match (value.predecessor_event_id, value.predecessor_room_id) {
            (Some(event_id), Some(room_id)) => {
                Some(MatrixPduPredecessorContent { event_id, room_id })
            }
            _ => None,
        };
        Self {
            creator: value.creator,
            room_version: value.room_version,
            federate: value.federate,
            room_type: value.room_type,
            predecessor,
            additional_creators: value.additional_creators,
        }
    }
}

impl From<RoomMemberContent> for MatrixPduRoomMemberContent {
    fn from(value: RoomMemberContent) -> Self {
        Self {
            membership: value.membership.into(),
            is_direct: value.is_direct,
        }
    }
}

impl From<RoomPowerLevelsContent> for MatrixPduRoomPowerLevelsContent {
    fn from(value: RoomPowerLevelsContent) -> Self {
        let notifications = value
            .notifications_room
            .map(|room| MatrixPduRoomPowerLevelNotifications { room });
        Self {
            ban: value.ban,
            events_default: value.events_default,
            invite: value.invite,
            kick: value.kick,
            redact: value.redact,
            state_default: value.state_default,
            users_default: value.users_default,
            users: value.users.into_iter().map(Into::into).collect(),
            notifications,
        }
    }
}

impl From<MatrixEventContent> for MatrixPduContent {
    fn from(value: MatrixEventContent) -> Self {
        match value {
            MatrixEventContent::RoomCreate(content) => Self::RoomCreate(content.into()),
            MatrixEventContent::RoomMember(content) => Self::RoomMember(content.into()),
            MatrixEventContent::RoomPowerLevels(content) => Self::RoomPowerLevels(content.into()),
            MatrixEventContent::RoomJoinRules { join_rule } => {
                Self::RoomJoinRules(MatrixPduRoomJoinRulesContent { join_rule })
            }
            MatrixEventContent::RoomHistoryVisibility { history_visibility } => {
                Self::RoomHistoryVisibility(MatrixPduRoomHistoryVisibilityContent {
                    history_visibility,
                })
            }
            MatrixEventContent::RoomGuestAccess { guest_access } => {
                Self::RoomGuestAccess(MatrixPduRoomGuestAccessContent { guest_access })
            }
            MatrixEventContent::RoomCanonicalAlias { alias } => {
                Self::RoomCanonicalAlias(MatrixPduRoomCanonicalAliasContent { alias })
            }
            MatrixEventContent::RoomName { name } => {
                Self::RoomName(MatrixPduRoomNameContent { name })
            }
            MatrixEventContent::RoomTopic { topic } => {
                Self::RoomTopic(MatrixPduRoomTopicContent { topic })
            }
            MatrixEventContent::RoomThirdPartyInvite {
                display_name,
                key_validity_url,
                public_key,
                medium,
                id_server,
            } => Self::RoomThirdPartyInvite(MatrixPduRoomThirdPartyInviteContent {
                display_name,
                key_validity_url,
                public_key,
                medium,
                id_server,
            }),
            MatrixEventContent::RoomMessage { body, msgtype } => {
                Self::RoomMessage(MatrixPduRoomMessageContent { body, msgtype })
            }
            MatrixEventContent::RoomRedaction { redacts, reason } => {
                Self::RoomRedaction(MatrixPduRoomRedactionContent { redacts, reason })
            }
        }
    }
}

pub fn matrix_event_content_to_json(content: &MatrixEventContent) -> Value {
    let protocol_content: MatrixPduContent = content.clone().into();
    serde_json::to_value(protocol_content).unwrap_or_default()
}
