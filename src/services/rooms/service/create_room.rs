use axum_extra::routing::TypedPath;
use uuid::Uuid;

use crate::infrastructure::server_name::ServerName;
use crate::infrastructure::user_identifier::UserIdentifier;
use crate::services::authorization::entities::{AuthorizedUserIdentifier, ExistingUserIdentifier};
use crate::services::authorization::service::AuthorizationService;
use crate::services::events::entities::{
    EventBatchWriteContract, EventIntent, EventOriginKind, MatrixEventContent, MembershipContent,
    RoomCreateContent, RoomEventFlow, RoomMemberContent, RoomPowerLevelsContent, StateEventKind,
    SupportedRoomVersion, UserPowerLevel,
};
use crate::services::events::service::EventsService;
use crate::services::identity::endpoints::IdentityLegacyV1PublicKeyIsValidPath;
use crate::services::rooms::entities::{
    AliasIntent, CreateRoomCommand, CreatedRoom, DirectoryVisibilityIntent, RoomCreationFlow,
    RoomFactoryEvent, RoomIdentifier, RoomPowerLevelsOverrideDto, RoomPreset, RoomShellIntent,
    RoomValidationError, RoomVisibility, ValidatedCreateRoomInput, ValidatedCreationContent,
};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomCreationRepository;

pub struct CreateRoomUseCase<'a, Repository>
where
    Repository: RoomCreationRepository + ?Sized,
{
    room_repository: &'a Repository,
    events_service: &'a EventsService,
    authorization_service: &'a AuthorizationService,
    server_name: &'a ServerName,
}

impl<'a, Repository> CreateRoomUseCase<'a, Repository>
where
    Repository: RoomCreationRepository + ?Sized,
{
    pub fn new(
        room_repository: &'a Repository,
        events_service: &'a EventsService,
        authorization_service: &'a AuthorizationService,
        server_name: &'a ServerName,
    ) -> Self {
        Self {
            room_repository,
            events_service,
            authorization_service,
            server_name,
        }
    }

    pub fn execute(
        &self,
        creator_user_id: &AuthorizedUserIdentifier,
        command: CreateRoomCommand,
    ) -> Result<CreatedRoom, RoomsApplicationError> {
        let resolved_invite = self.resolve_invited_users(command.invite.clone())?;
        let validated_input: ValidatedCreateRoomInput = CreateRoomValidationInput {
            creator: creator_user_id.as_existing_user_identifier().clone(),
            command,
            resolved_invite,
            default_room_version: self.events_service.default_room_version(),
        }
        .try_into()?;

        if !self
            .events_service
            .room_version_is_supported(validated_input.room_version)
        {
            return Err(RoomsApplicationError::UnsupportedRoomVersion);
        }

        let room_creation_flow: RoomCreationFlow =
            RoomCreationFactory::new(&validated_input, self.server_name)
                .add_event(RoomFactoryEvent::RequiredCreateEvent)
                .add_event(RoomFactoryEvent::CreatorJoinEvent)
                .add_event(RoomFactoryEvent::DefaultPowerLevelsEvent)
                .add_event(RoomFactoryEvent::CanonicalAliasEventIfNeeded)
                .add_event(RoomFactoryEvent::PresetEvents)
                .add_event(RoomFactoryEvent::InitialStateEvents)
                .add_event(RoomFactoryEvent::NameAndTopicEvents)
                .add_event(RoomFactoryEvent::InviteEvents)
                .build_event_flow();

        let event_batch_write_contract: EventBatchWriteContract =
            self.events_service.compile_room_event_flow(RoomEventFlow {
                room_id: room_creation_flow.room_shell_intent.room_id.clone(),
                room_version: room_creation_flow.room_shell_intent.room_version.clone(),
                intents: room_creation_flow.ordered_event_intents.clone(),
            })?;

        self.room_repository
            .create_room_with_initial_events(&room_creation_flow, &event_batch_write_contract)
            .map_err(|error| {
                super::map_room_persistence_error(
                    error,
                    room_creation_flow.room_shell_intent.room_id.clone(),
                )
            })?;

        Ok(CreatedRoom {
            room_id: room_creation_flow.room_shell_intent.room_id,
        })
    }

    fn resolve_invited_users(
        &self,
        invited_user_ids: Vec<UserIdentifier>,
    ) -> Result<Vec<ExistingUserIdentifier>, RoomsApplicationError> {
        let mut invite = Vec::with_capacity(invited_user_ids.len());

        for invited_user_id in invited_user_ids {
            let existing_user = self
                .authorization_service
                .require_existing_user_identifier(invited_user_id.clone())
                .map_err(|_| {
                    RoomsApplicationError::from(RoomValidationError::InvalidInviteUserIdentifier {
                        user_id: invited_user_id.to_string(),
                    })
                })?;
            invite.push(existing_user);
        }

        Ok(invite)
    }
}

pub struct CreateRoomValidationInput {
    pub creator: ExistingUserIdentifier,
    pub command: CreateRoomCommand,
    pub resolved_invite: Vec<ExistingUserIdentifier>,
    pub default_room_version: SupportedRoomVersion,
}

impl TryFrom<CreateRoomValidationInput> for ValidatedCreateRoomInput {
    type Error = RoomValidationError;

    fn try_from(input: CreateRoomValidationInput) -> Result<Self, Self::Error> {
        let visibility = input.command.visibility.unwrap_or(RoomVisibility::Private);
        let preset = input
            .command
            .preset
            .unwrap_or_else(|| RoomPreset::from(visibility));

        let room_version = input
            .command
            .room_version
            .as_deref()
            .map(|room_version| {
                room_version.parse::<SupportedRoomVersion>().map_err(|_| {
                    RoomValidationError::UnsupportedRoomVersion {
                        room_version: room_version.to_owned(),
                    }
                })
            })
            .transpose()?
            .unwrap_or(input.default_room_version);

        Ok(Self {
            creator: input.creator,
            creation_content: input.command.creation_content,
            initial_state: input.command.initial_state,
            invite: input.resolved_invite,
            invite_3pid: input.command.invite_3pid,
            is_direct: input.command.is_direct,
            name: input.command.name,
            topic: input.command.topic,
            power_level_content_override: input
                .command
                .power_level_content_override
                .unwrap_or_default(),
            preset,
            room_alias_name: input.command.room_alias_name,
            room_version,
            visibility,
        })
    }
}

pub struct RoomCreationFactory<'a> {
    room_id: RoomIdentifier,
    server_name: &'a ServerName,
    validated_input: &'a ValidatedCreateRoomInput,
    planned_events: Vec<RoomFactoryEvent>,
    ordered_event_intents: Vec<EventIntent>,
}

impl<'a> RoomCreationFactory<'a> {
    pub fn new(validated_input: &'a ValidatedCreateRoomInput, server_name: &'a ServerName) -> Self {
        Self {
            room_id: RoomIdentifier::generate(server_name),
            server_name,
            validated_input,
            planned_events: Vec::new(),
            ordered_event_intents: Vec::new(),
        }
    }

    pub fn add_event(mut self, event: RoomFactoryEvent) -> Self {
        self.planned_events.push(event);
        self
    }

    pub fn build_event_flow(mut self) -> RoomCreationFlow {
        let room_version_rules =
            RoomVersionEventRules::from_supported_room_version(self.validated_input.room_version);

        for event in self.planned_events.clone() {
            self.apply_event(event, room_version_rules);
        }

        RoomCreationFlow {
            room_shell_intent: RoomShellIntent {
                room_id: self.room_id.as_str().to_owned(),
                room_version: self.validated_input.room_version,
                creator_user_id: self.validated_input.creator.as_str().to_owned(),
                is_direct: self.validated_input.is_direct,
                name: self
                    .validated_input
                    .name
                    .as_ref()
                    .map(|room_name| room_name.as_str().to_owned()),
                topic: self
                    .validated_input
                    .topic
                    .as_ref()
                    .map(|room_topic| room_topic.as_str().to_owned()),
                visibility: self.validated_input.visibility,
                preset: self.validated_input.preset,
            },
            alias_intent: self
                .validated_input
                .room_alias_name
                .as_ref()
                .map(|room_alias_name| AliasIntent {
                    alias_localpart: room_alias_name.as_str().to_owned(),
                    room_id: self.room_id.as_str().to_owned(),
                }),
            directory_visibility_intent: Some(DirectoryVisibilityIntent {
                room_id: self.room_id.as_str().to_owned(),
                visibility: self.validated_input.visibility,
            }),
            ordered_event_intents: self.ordered_event_intents,
        }
    }

    fn apply_event(&mut self, event: RoomFactoryEvent, room_version_rules: RoomVersionEventRules) {
        match event {
            RoomFactoryEvent::RequiredCreateEvent => {
                self.add_required_create_event(room_version_rules);
            }
            RoomFactoryEvent::CreatorJoinEvent => self.add_creator_join_event(),
            RoomFactoryEvent::DefaultPowerLevelsEvent => {
                self.add_default_power_levels_event(room_version_rules);
            }
            RoomFactoryEvent::CanonicalAliasEventIfNeeded => {
                self.add_canonical_alias_event_if_needed();
            }
            RoomFactoryEvent::PresetEvents => self.add_preset_events(room_version_rules),
            RoomFactoryEvent::InitialStateEvents => self.add_initial_state_events(),
            RoomFactoryEvent::NameAndTopicEvents => self.add_name_and_topic_events(),
            RoomFactoryEvent::InviteEvents => self.add_invite_events(),
        }
    }

    fn add_required_create_event(&mut self, room_version_rules: RoomVersionEventRules) {
        self.ordered_event_intents.push(EventIntent::state(
            self.room_id.as_str().to_owned(),
            self.validated_input.creator.as_str().to_owned(),
            StateEventKind::RoomCreate,
            String::new(),
            build_room_create_content(
                &self.validated_input.creation_content,
                self.validated_input.room_version,
                self.validated_input.creator.as_str(),
                room_version_rules,
            ),
            EventOriginKind::CreateRoom,
        ));
    }

    fn add_creator_join_event(&mut self) {
        self.ordered_event_intents.push(EventIntent::state(
            self.room_id.as_str().to_owned(),
            self.validated_input.creator.as_str().to_owned(),
            StateEventKind::RoomMember,
            self.validated_input.creator.as_str().to_owned(),
            MatrixEventContent::RoomMember(RoomMemberContent {
                membership: MembershipContent::Join,
                is_direct: None,
            }),
            EventOriginKind::CreatorJoin,
        ));
    }

    fn add_default_power_levels_event(&mut self, room_version_rules: RoomVersionEventRules) {
        let power_level_content = build_default_power_levels_content(
            self.validated_input.creator.as_str(),
            &self.validated_input.invite,
            self.validated_input.preset,
            &self.validated_input.power_level_content_override,
            room_version_rules,
        );

        self.ordered_event_intents.push(EventIntent::state(
            self.room_id.as_str().to_owned(),
            self.validated_input.creator.as_str().to_owned(),
            StateEventKind::RoomPowerLevels,
            String::new(),
            MatrixEventContent::RoomPowerLevels(power_level_content),
            EventOriginKind::DefaultPowerLevels,
        ));
    }

    fn add_canonical_alias_event_if_needed(&mut self) {
        if let Some(alias_localpart) = self.validated_input.room_alias_name.as_ref() {
            self.ordered_event_intents.push(EventIntent::state(
                self.room_id.as_str().to_owned(),
                self.validated_input.creator.as_str().to_owned(),
                StateEventKind::RoomCanonicalAlias,
                String::new(),
                MatrixEventContent::RoomCanonicalAlias {
                    alias: format!(
                        "#{}:{}",
                        alias_localpart.as_str(),
                        self.server_name.as_str()
                    ),
                },
                EventOriginKind::CanonicalAlias,
            ));
        }
    }

    fn add_preset_events(&mut self, room_version_rules: RoomVersionEventRules) {
        let (join_rule, guest_access, history_visibility) =
            room_version_rules.preset_defaults(self.validated_input.preset);

        self.ordered_event_intents.push(EventIntent::state(
            self.room_id.as_str().to_owned(),
            self.validated_input.creator.as_str().to_owned(),
            StateEventKind::RoomJoinRules,
            String::new(),
            MatrixEventContent::RoomJoinRules {
                join_rule: join_rule.to_owned(),
            },
            EventOriginKind::PresetState,
        ));
        self.ordered_event_intents.push(EventIntent::state(
            self.room_id.as_str().to_owned(),
            self.validated_input.creator.as_str().to_owned(),
            StateEventKind::RoomHistoryVisibility,
            String::new(),
            MatrixEventContent::RoomHistoryVisibility {
                history_visibility: history_visibility.to_owned(),
            },
            EventOriginKind::PresetState,
        ));
        self.ordered_event_intents.push(EventIntent::state(
            self.room_id.as_str().to_owned(),
            self.validated_input.creator.as_str().to_owned(),
            StateEventKind::RoomGuestAccess,
            String::new(),
            MatrixEventContent::RoomGuestAccess {
                guest_access: guest_access.to_owned(),
            },
            EventOriginKind::PresetState,
        ));
    }

    fn add_initial_state_events(&mut self) {
        for initial_state_event in &self.validated_input.initial_state {
            self.ordered_event_intents.push(EventIntent::state(
                self.room_id.as_str().to_owned(),
                self.validated_input.creator.as_str().to_owned(),
                initial_state_event.event_type.clone(),
                initial_state_event.state_key.as_str().to_owned(),
                initial_state_event.content.clone(),
                EventOriginKind::InitialState,
            ));
        }
    }

    fn add_name_and_topic_events(&mut self) {
        if let Some(room_name) = self.validated_input.name.as_ref() {
            self.ordered_event_intents.push(EventIntent::state(
                self.room_id.as_str().to_owned(),
                self.validated_input.creator.as_str().to_owned(),
                StateEventKind::RoomName,
                String::new(),
                MatrixEventContent::RoomName {
                    name: room_name.as_str().to_owned(),
                },
                EventOriginKind::Name,
            ));
        }

        if let Some(room_topic) = self.validated_input.topic.as_ref() {
            self.ordered_event_intents.push(EventIntent::state(
                self.room_id.as_str().to_owned(),
                self.validated_input.creator.as_str().to_owned(),
                StateEventKind::RoomTopic,
                String::new(),
                MatrixEventContent::RoomTopic {
                    topic: room_topic.as_str().to_owned(),
                },
                EventOriginKind::Topic,
            ));
        }
    }

    fn add_invite_events(&mut self) {
        for invitee in &self.validated_input.invite {
            self.ordered_event_intents.push(EventIntent::state(
                self.room_id.as_str().to_owned(),
                self.validated_input.creator.as_str().to_owned(),
                StateEventKind::RoomMember,
                invitee.as_str().to_owned(),
                MatrixEventContent::RoomMember(RoomMemberContent {
                    membership: MembershipContent::Invite,
                    is_direct: Some(self.validated_input.is_direct),
                }),
                EventOriginKind::Invite,
            ));
        }

        for third_party_invite in &self.validated_input.invite_3pid {
            let token = format!("invite_{}", Uuid::new_v4().simple());
            self.ordered_event_intents.push(EventIntent::state(
                self.room_id.as_str().to_owned(),
                self.validated_input.creator.as_str().to_owned(),
                StateEventKind::ThirdPartyInvite,
                token,
                MatrixEventContent::RoomThirdPartyInvite {
                    display_name: third_party_invite.address.clone(),
                    key_validity_url: format!(
                        "https://{}{}",
                        third_party_invite.id_server.as_str(),
                        IdentityLegacyV1PublicKeyIsValidPath.to_uri().to_string()
                    ),
                    public_key: String::new(),
                    medium: third_party_invite.medium.clone(),
                    id_server: third_party_invite.id_server.clone(),
                },
                EventOriginKind::ThirdPartyInvite,
            ));
        }
    }
}

#[derive(Clone, Copy)]
enum RoomVersionEventRules {
    Version1,
    Version2,
}

impl RoomVersionEventRules {
    fn from_supported_room_version(room_version: SupportedRoomVersion) -> Self {
        match room_version {
            SupportedRoomVersion::V1 => Self::Version1,
            SupportedRoomVersion::V2 => Self::Version2,
        }
    }

    fn supports_additional_creators(self) -> bool {
        match self {
            Self::Version1 | Self::Version2 => false,
        }
    }

    fn power_levels_must_be_integer_values(self) -> bool {
        match self {
            Self::Version1 | Self::Version2 => true,
        }
    }

    fn preset_defaults(self, preset: RoomPreset) -> (&'static str, &'static str, &'static str) {
        let _ = self;
        match preset {
            RoomPreset::PrivateChat | RoomPreset::TrustedPrivateChat => {
                ("invite", "can_join", "shared")
            }
            RoomPreset::PublicChat => ("public", "forbidden", "shared"),
        }
    }
}

fn build_room_create_content(
    creation_content: &ValidatedCreationContent,
    room_version: SupportedRoomVersion,
    creator_user_id: &str,
    room_version_rules: RoomVersionEventRules,
) -> MatrixEventContent {
    MatrixEventContent::RoomCreate(RoomCreateContent {
        creator: creator_user_id.to_owned(),
        room_version: room_version.to_string(),
        federate: creation_content.federate,
        room_type: creation_content.room_type.clone(),
        predecessor_event_id: creation_content
            .predecessor
            .as_ref()
            .map(|predecessor| predecessor.event_id.clone()),
        predecessor_room_id: creation_content
            .predecessor
            .as_ref()
            .map(|predecessor| predecessor.room_id.clone()),
        additional_creators: if room_version_rules.supports_additional_creators() {
            creation_content
                .additional_creators
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            Vec::new()
        },
    })
}

fn build_default_power_levels_content(
    creator_user_id: &str,
    invitees: &[ExistingUserIdentifier],
    preset: RoomPreset,
    override_content: &RoomPowerLevelsOverrideDto,
    room_version_rules: RoomVersionEventRules,
) -> RoomPowerLevelsContent {
    let mut users = vec![UserPowerLevel {
        user_id: creator_user_id.to_owned(),
        level: 100,
    }];

    if matches!(preset, RoomPreset::TrustedPrivateChat) {
        for invitee in invitees {
            users.push(UserPowerLevel {
                user_id: invitee.as_str().to_owned(),
                level: 100,
            });
        }
    }

    let _ = room_version_rules.power_levels_must_be_integer_values();
    RoomPowerLevelsContent {
        ban: override_content.ban.unwrap_or(50),
        events_default: override_content.events_default.unwrap_or(0),
        invite: override_content.invite.unwrap_or(0),
        kick: override_content.kick.unwrap_or(50),
        redact: override_content.redact.unwrap_or(50),
        state_default: override_content.state_default.unwrap_or(50),
        users_default: override_content.users_default.unwrap_or(0),
        users,
        notifications_room: override_content.notifications.as_ref().and_then(|n| n.room),
    }
}
