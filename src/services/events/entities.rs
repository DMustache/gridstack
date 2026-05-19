use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display)]
pub enum SupportedRoomVersion {
    #[serde(rename = "1")]
    #[strum(serialize = "1")]
    V1,
    #[serde(rename = "2")]
    #[strum(serialize = "2")]
    V2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateResolutionAlgorithmKind {
    V1,
    V2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomVersionDefinition {
    pub version_id: SupportedRoomVersion,
    pub state_resolution_algorithm: StateResolutionAlgorithmKind,
    pub max_prev_events: usize,
    pub max_auth_events: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct RoomVersionRegistry {
    default_room_version: SupportedRoomVersion,
}

impl Default for RoomVersionRegistry {
    fn default() -> Self {
        Self {
            default_room_version: SupportedRoomVersion::V1,
        }
    }
}

impl RoomVersionRegistry {
    pub fn new(default_room_version: SupportedRoomVersion) -> Self {
        Self {
            default_room_version,
        }
    }

    pub const fn default_room_version(self) -> SupportedRoomVersion {
        self.default_room_version
    }

    pub fn resolve(self, version: SupportedRoomVersion) -> Option<RoomVersionDefinition> {
        match version {
            SupportedRoomVersion::V1 => Some(RoomVersionDefinition {
                version_id: SupportedRoomVersion::V1,
                state_resolution_algorithm: StateResolutionAlgorithmKind::V1,
                max_prev_events: 20,
                max_auth_events: 10,
            }),
            SupportedRoomVersion::V2 => Some(RoomVersionDefinition {
                version_id: SupportedRoomVersion::V2,
                // v2 inherits v1 and only overrides state resolution.
                state_resolution_algorithm: StateResolutionAlgorithmKind::V2,
                max_prev_events: 20,
                max_auth_events: 10,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOriginKind {
    CreateRoom,
    CreatorJoin,
    DefaultPowerLevels,
    CanonicalAlias,
    PresetState,
    InitialState,
    Name,
    Topic,
    Invite,
    ThirdPartyInvite,
    MessageSend,
    StateSet,
    MembershipChange,
    FederationReceive,
    Redaction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateEventKind {
    RoomCreate,
    RoomMember,
    RoomPowerLevels,
    RoomJoinRules,
    RoomHistoryVisibility,
    RoomGuestAccess,
    RoomCanonicalAlias,
    RoomName,
    RoomTopic,
    ThirdPartyInvite,
    Custom(String),
}

impl StateEventKind {
    pub fn as_event_type(&self) -> &str {
        match self {
            Self::RoomCreate => "m.room.create",
            Self::RoomMember => "m.room.member",
            Self::RoomPowerLevels => "m.room.power_levels",
            Self::RoomJoinRules => "m.room.join_rules",
            Self::RoomHistoryVisibility => "m.room.history_visibility",
            Self::RoomGuestAccess => "m.room.guest_access",
            Self::RoomCanonicalAlias => "m.room.canonical_alias",
            Self::RoomName => "m.room.name",
            Self::RoomTopic => "m.room.topic",
            Self::ThirdPartyInvite => "m.room.third_party_invite",
            Self::Custom(event_type) => event_type.as_str(),
        }
    }
}

impl AsRef<str> for StateEventKind {
    fn as_ref(&self) -> &str {
        self.as_event_type()
    }
}

impl std::fmt::Display for StateEventKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.as_event_type())
    }
}

impl std::str::FromStr for StateEventKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "m.room.create" => Self::RoomCreate,
            "m.room.member" => Self::RoomMember,
            "m.room.power_levels" => Self::RoomPowerLevels,
            "m.room.join_rules" => Self::RoomJoinRules,
            "m.room.history_visibility" => Self::RoomHistoryVisibility,
            "m.room.guest_access" => Self::RoomGuestAccess,
            "m.room.canonical_alias" => Self::RoomCanonicalAlias,
            "m.room.name" => Self::RoomName,
            "m.room.topic" => Self::RoomTopic,
            "m.room.third_party_invite" => Self::ThirdPartyInvite,
            _ => Self::Custom(value.to_owned()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageEventKind {
    RoomMessage,
    RoomRedaction,
    Custom(String),
}

impl MessageEventKind {
    pub fn as_event_type(&self) -> &str {
        match self {
            Self::RoomMessage => "m.room.message",
            Self::RoomRedaction => "m.room.redaction",
            Self::Custom(event_type) => event_type.as_str(),
        }
    }
}

impl AsRef<str> for MessageEventKind {
    fn as_ref(&self) -> &str {
        self.as_event_type()
    }
}

impl std::fmt::Display for MessageEventKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.as_event_type())
    }
}

impl std::str::FromStr for MessageEventKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "m.room.message" => Self::RoomMessage,
            "m.room.redaction" => Self::RoomRedaction,
            _ => Self::Custom(value.to_owned()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    State(StateEventKind),
    Message(MessageEventKind),
}

impl EventKind {
    pub fn as_event_type(&self) -> &str {
        match self {
            Self::State(kind) => kind.as_ref(),
            Self::Message(kind) => kind.as_ref(),
        }
    }

    pub const fn is_state(&self) -> bool {
        matches!(self, Self::State(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipContent {
    Join,
    Invite,
    Leave,
    Ban,
    Knock,
}

impl MembershipContent {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Join => "join",
            Self::Invite => "invite",
            Self::Leave => "leave",
            Self::Ban => "ban",
            Self::Knock => "knock",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPowerLevel {
    pub user_id: String,
    pub level: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateContent {
    pub creator: String,
    pub room_version: String,
    pub federate: bool,
    pub room_type: Option<String>,
    pub predecessor_event_id: Option<String>,
    pub predecessor_room_id: Option<String>,
    pub additional_creators: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomMemberContent {
    pub membership: MembershipContent,
    pub is_direct: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomPowerLevelsContent {
    pub ban: i64,
    pub events_default: i64,
    pub invite: i64,
    pub kick: i64,
    pub redact: i64,
    pub state_default: i64,
    pub users_default: i64,
    pub users: Vec<UserPowerLevel>,
    pub notifications_room: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatrixEventContent {
    RoomCreate(RoomCreateContent),
    RoomMember(RoomMemberContent),
    RoomPowerLevels(RoomPowerLevelsContent),
    RoomJoinRules {
        join_rule: String,
    },
    RoomHistoryVisibility {
        history_visibility: String,
    },
    RoomGuestAccess {
        guest_access: String,
    },
    RoomCanonicalAlias {
        alias: String,
    },
    RoomName {
        name: String,
    },
    RoomTopic {
        topic: String,
    },
    RoomThirdPartyInvite {
        display_name: String,
        key_validity_url: String,
        public_key: String,
        medium: String,
        id_server: String,
    },
    RoomMessage {
        body: String,
        msgtype: String,
    },
    RoomRedaction {
        redacts: String,
        reason: Option<String>,
    },
    CustomJson(Value),
}

impl MatrixEventContent {
    pub fn as_debug_json(&self) -> Value {
        match self {
            Self::RoomCreate(content) => {
                let mut value = json!({
                    "creator": content.creator,
                    "room_version": content.room_version,
                    "m.federate": content.federate,
                });
                if let Some(room_type) = content.room_type.as_ref() {
                    value["type"] = Value::String(room_type.clone());
                }
                if !content.additional_creators.is_empty() {
                    value["additional_creators"] = Value::Array(
                        content
                            .additional_creators
                            .iter()
                            .cloned()
                            .map(Value::String)
                            .collect(),
                    );
                }
                if let (Some(event_id), Some(room_id)) = (
                    content.predecessor_event_id.as_ref(),
                    content.predecessor_room_id.as_ref(),
                ) {
                    value["predecessor"] = json!({
                        "event_id": event_id,
                        "room_id": room_id,
                    });
                }
                value
            }
            Self::RoomMember(content) => {
                let mut value = json!({ "membership": content.membership.as_str() });
                if let Some(is_direct) = content.is_direct {
                    value["is_direct"] = Value::Bool(is_direct);
                }
                value
            }
            Self::RoomPowerLevels(content) => {
                let users = content
                    .users
                    .iter()
                    .map(|user| (user.user_id.clone(), Value::from(user.level)))
                    .collect::<serde_json::Map<String, Value>>();
                let mut value = json!({
                    "ban": content.ban,
                    "events": {},
                    "events_default": content.events_default,
                    "invite": content.invite,
                    "kick": content.kick,
                    "redact": content.redact,
                    "state_default": content.state_default,
                    "users": users,
                    "users_default": content.users_default,
                });
                if let Some(notifications_room) = content.notifications_room {
                    value["notifications"] = json!({ "room": notifications_room });
                }
                value
            }
            Self::RoomJoinRules { join_rule } => json!({ "join_rule": join_rule }),
            Self::RoomHistoryVisibility { history_visibility } => {
                json!({ "history_visibility": history_visibility })
            }
            Self::RoomGuestAccess { guest_access } => json!({ "guest_access": guest_access }),
            Self::RoomCanonicalAlias { alias } => json!({ "alias": alias }),
            Self::RoomName { name } => json!({ "name": name }),
            Self::RoomTopic { topic } => json!({ "topic": topic }),
            Self::RoomThirdPartyInvite {
                display_name,
                key_validity_url,
                public_key,
                medium,
                id_server,
            } => json!({
                "display_name": display_name,
                "key_validity_url": key_validity_url,
                "public_key": public_key,
                "medium": medium,
                "id_server": id_server,
            }),
            Self::RoomMessage { body, msgtype } => json!({ "body": body, "msgtype": msgtype }),
            Self::RoomRedaction { redacts, reason } => {
                let mut value = json!({ "redacts": redacts });
                if let Some(reason) = reason.as_ref() {
                    value["reason"] = Value::String(reason.clone());
                }
                value
            }
            Self::CustomJson(content) => content.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventIntent {
    pub room_id: String,
    pub sender: String,
    pub event_kind: EventKind,
    pub state_key: Option<String>,
    pub content: MatrixEventContent,
    pub unsigned: Option<Value>,
    pub origin: EventOriginKind,
    pub transaction_id: Option<String>,
}

impl EventIntent {
    pub fn state(
        room_id: String,
        sender: String,
        state_event_kind: StateEventKind,
        state_key: String,
        content: MatrixEventContent,
        origin: EventOriginKind,
    ) -> Self {
        Self {
            room_id,
            sender,
            event_kind: EventKind::State(state_event_kind),
            state_key: Some(state_key),
            content,
            unsigned: None,
            origin,
            transaction_id: None,
        }
    }

    pub fn message(
        room_id: String,
        sender: String,
        message_event_kind: MessageEventKind,
        content: MatrixEventContent,
        origin: EventOriginKind,
        transaction_id: Option<String>,
    ) -> Self {
        Self {
            room_id,
            sender,
            event_kind: EventKind::Message(message_event_kind),
            state_key: None,
            content,
            unsigned: None,
            origin,
            transaction_id,
        }
    }

    pub fn event_type(&self) -> &str {
        self.event_kind.as_event_type()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomEventFlow {
    pub room_id: String,
    pub room_version: SupportedRoomVersion,
    pub intents: Vec<EventIntent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventDraft {
    pub room_id: String,
    pub room_version: SupportedRoomVersion,
    pub sender: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: MatrixEventContent,
    pub unsigned: Option<Value>,
    pub prev_events: Vec<String>,
    pub auth_events: Vec<String>,
    pub depth: u64,
    pub origin_server_ts: u64,
    pub redacts: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedEvent {
    pub event_id: String,
    pub room_id: String,
    pub room_version: SupportedRoomVersion,
    pub sender: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: MatrixEventContent,
    pub unsigned: Option<Value>,
    pub prev_events: Vec<String>,
    pub auth_events: Vec<String>,
    pub depth: u64,
    pub origin_server_ts: u64,
    pub hashes: EventHashes,
    pub signatures: Vec<EventSignature>,
    pub rejected: bool,
    pub soft_failed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventJsonRow {
    pub event_id: String,
    pub canonical_json: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventHashes {
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSignature {
    pub server_name: String,
    pub key_id: String,
    pub signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventGraphEdge {
    pub room_id: String,
    pub event_id: String,
    pub linked_event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardExtremityUpdate {
    pub room_id: String,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentStateUpdate {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipState {
    Join,
    Invite,
    Leave,
    Ban,
    Knock,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipProjectionUpdate {
    pub room_id: String,
    pub user_id: String,
    pub membership: MembershipState,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineProjectionUpdate {
    pub room_id: String,
    pub event_id: String,
    pub stream_position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomSummaryProjectionKind {
    RoomName,
    RoomTopic,
    CanonicalAlias,
    JoinRule,
    HistoryVisibility,
    GuestAccess,
    PowerLevels,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomSummaryProjectionUpdate {
    pub room_id: String,
    pub event_id: String,
    pub projection_kind: RoomSummaryProjectionKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStreamRow {
    pub room_id: String,
    pub event_id: String,
    pub stream_position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxTaskType {
    NotifyLocalUsers,
    WakeSyncWaiters,
    FederateEvent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxTask {
    pub room_id: String,
    pub event_id: String,
    pub task_type: OutboxTaskType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    pub room_id: String,
    pub sender_user_id: String,
    pub transaction_id: String,
    pub event_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventWriteContract {
    pub event_rows: Vec<PersistedEvent>,
    pub event_json_rows: Vec<EventJsonRow>,
    pub prev_edge_rows: Vec<EventGraphEdge>,
    pub auth_edge_rows: Vec<EventGraphEdge>,
    pub forward_extremity_updates: Vec<ForwardExtremityUpdate>,
    pub current_state_updates: Vec<CurrentStateUpdate>,
    pub membership_projection_updates: Vec<MembershipProjectionUpdate>,
    pub timeline_projection_updates: Vec<TimelineProjectionUpdate>,
    pub room_summary_projection_updates: Vec<RoomSummaryProjectionUpdate>,
    pub sync_stream_rows: Vec<SyncStreamRow>,
    pub outbox_tasks: Vec<OutboxTask>,
    pub idempotency_records: Vec<IdempotencyRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventBatchWriteContract {
    pub room_id: String,
    pub room_version: SupportedRoomVersion,
    pub event_write_contracts: Vec<EventWriteContract>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventClass {
    State,
    Message,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKeyPolicy {
    Required,
    Forbidden,
    MustBeEmpty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventDefinition {
    pub event_class: EventClass,
    pub state_key_policy: StateKeyPolicy,
}

pub struct EventDefinitionRegistry;

impl EventDefinitionRegistry {
    pub fn for_event_kind(event_kind: &EventKind) -> EventDefinition {
        match event_kind {
            EventKind::State(StateEventKind::RoomCreate)
            | EventKind::State(StateEventKind::RoomPowerLevels)
            | EventKind::State(StateEventKind::RoomJoinRules)
            | EventKind::State(StateEventKind::RoomHistoryVisibility)
            | EventKind::State(StateEventKind::RoomGuestAccess)
            | EventKind::State(StateEventKind::RoomCanonicalAlias)
            | EventKind::State(StateEventKind::RoomName)
            | EventKind::State(StateEventKind::RoomTopic) => EventDefinition {
                event_class: EventClass::State,
                state_key_policy: StateKeyPolicy::MustBeEmpty,
            },
            EventKind::State(StateEventKind::RoomMember)
            | EventKind::State(StateEventKind::ThirdPartyInvite) => EventDefinition {
                event_class: EventClass::State,
                state_key_policy: StateKeyPolicy::Required,
            },
            EventKind::State(StateEventKind::Custom(_)) => EventDefinition {
                event_class: EventClass::State,
                state_key_policy: StateKeyPolicy::Required,
            },
            EventKind::Message(_) => EventDefinition {
                event_class: EventClass::Message,
                state_key_policy: StateKeyPolicy::Forbidden,
            },
        }
    }
}
