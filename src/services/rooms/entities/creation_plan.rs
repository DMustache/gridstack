use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::services::rooms::entities::RoomStateEventInfo;
use crate::services::{
    authorization::entities::AuthorizedUserIdentifier,
    rooms::{
        entities::{CreateRoomStateEventPayload, RoomCreateContractError, RoomIdentifier},
        handlers::create_room::{
            InviteThirdPartyIdentifierInfo, RoomPowerLevelsContentOverride, RoomPreset,
        },
    },
};

use super::versions::RoomVersion;

pub struct RoomCreationPlanBuilder<'a> {
    creator_user_id_str: &'a str,
    room_id: &'a str,
    home_server_name: &'a str,
    room_version: RoomVersion,
    preset: &'a RoomPreset,
    invitees: &'a [String],
    third_party_invitees: &'a [InviteThirdPartyIdentifierInfo],
    is_direct: Option<bool>,
    name: Option<&'a str>,
    topic: Option<&'a str>,
    room_alias_name: Option<&'a str>,
    initial_state: Option<Vec<RoomStateEventInfo>>,
    power_level_content_override: Option<RoomPowerLevelsContentOverride>,
    room_create_event_content: Option<Map<String, Value>>,
    planned_events: Vec<CreateRoomStateEventPayload>,
}

impl<'a> RoomCreationPlanBuilder<'a> {
    pub fn new(
        creator_user_id: &'a AuthorizedUserIdentifier,
        room_id: &'a str,
        home_server_name: &'a str,
        room_version: RoomVersion,
    ) -> Self {
        Self {
            creator_user_id_str: creator_user_id.as_user_identifier().as_str(),
            room_id,
            home_server_name,
            room_version,
            preset: &RoomPreset::PrivateChat,
            invitees: &[],
            third_party_invitees: &[],
            is_direct: None,
            name: None,
            topic: None,
            room_alias_name: None,
            initial_state: None,
            power_level_content_override: None,
            room_create_event_content: None,
            planned_events: Vec::new(),
        }
    }

    pub fn with_room_create_event_content(
        mut self,
        mut creation_content: Map<String, Value>,
    ) -> Self {
        creation_content.insert(
            "creator".to_owned(),
            Value::String(self.creator_user_id_str.to_owned()),
        );
        creation_content.insert(
            "room_version".to_owned(),
            Value::String(self.room_version.to_string()),
        );

        self.room_create_event_content = Some(creation_content);
        self
    }

    pub fn with_preset(mut self, preset: &'a RoomPreset) -> Self {
        self.preset = preset;
        self
    }

    pub fn with_invitees(
        mut self,
        invitees: &'a [String],
        third_party_invitees: &'a [InviteThirdPartyIdentifierInfo],
        is_direct: Option<bool>,
    ) -> Self {
        self.invitees = invitees;
        self.third_party_invitees = third_party_invitees;
        self.is_direct = is_direct;
        self
    }

    pub fn with_alias(mut self, room_alias_name: Option<&'a str>) -> Self {
        self.room_alias_name = room_alias_name;
        self
    }

    pub fn with_profile(mut self, name: Option<&'a str>, topic: Option<&'a str>) -> Self {
        self.name = name;
        self.topic = topic;
        self
    }

    pub fn with_initial_state(mut self, initial_state: Option<Vec<RoomStateEventInfo>>) -> Self {
        self.initial_state = initial_state;
        self
    }

    pub fn with_power_level_override(
        mut self,
        override_content: Option<RoomPowerLevelsContentOverride>,
    ) -> Self {
        self.power_level_content_override = override_content;
        self
    }

    pub fn build(mut self) -> Result<Vec<CreateRoomStateEventPayload>, RoomCreateContractError> {
        self.apply_trusted_private_chat_additional_creators()?;
        self.push_room_create_event()?;
        self.push_creator_join_event();
        self.push_default_power_levels_event()?;
        self.push_optional_alias_event();
        self.push_preset_events();
        self.push_initial_state_events()?;
        self.push_name_topic_events();
        self.push_invite_events();
        self.push_third_party_invite_events();
        self.finalize_plan()
    }

    fn apply_trusted_private_chat_additional_creators(
        &mut self,
    ) -> Result<(), RoomCreateContractError> {
        if !matches!(self.preset, RoomPreset::TrustedPrivateChat) || self.invitees.is_empty() {
            return Ok(());
        }

        let room_create_content = self
            .room_create_event_content
            .as_mut()
            .ok_or(RoomCreateContractError::InvalidCreationContent)?;

        let additional_creators = room_create_content
            .entry("additional_creators")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(additional_creators) = additional_creators.as_array_mut() {
            for invitee in self.invitees {
                let invitee_value = Value::String(invitee.clone());
                if !additional_creators.contains(&invitee_value) {
                    additional_creators.push(invitee_value);
                }
            }
        }

        Ok(())
    }

    fn push_room_create_event(&mut self) -> Result<(), RoomCreateContractError> {
        let room_create_event_content = self
            .room_create_event_content
            .take()
            .ok_or(RoomCreateContractError::InvalidCreationContent)?;
        self.push_state_event(
            "m.room.create",
            "",
            Value::Object(room_create_event_content),
        );
        Ok(())
    }

    fn push_creator_join_event(&mut self) {
        self.push_state_event(
            "m.room.member",
            self.creator_user_id_str,
            json!({ "membership": "join" }),
        );
    }

    fn push_default_power_levels_event(&mut self) -> Result<(), RoomCreateContractError> {
        let mut users = Map::new();
        users.insert(self.creator_user_id_str.to_owned(), json!(100));
        if matches!(self.preset, RoomPreset::TrustedPrivateChat) {
            for invitee in self.invitees {
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

        if let Some(override_content) = self.power_level_content_override.as_ref() {
            if let Some(ban) = override_content.ban {
                power_levels.insert("ban".to_owned(), json!(ban));
            }
            if let Some(events) = override_content.events.as_ref() {
                power_levels.insert(
                    "events".to_owned(),
                    serde_json::to_value(events)
                        .map_err(|_| RoomCreateContractError::InvalidPowerLevelContentOverride)?,
                );
            }
            if let Some(events_default) = override_content.events_default {
                power_levels.insert("events_default".to_owned(), json!(events_default));
            }
            if let Some(invite) = override_content.invite {
                power_levels.insert("invite".to_owned(), json!(invite));
            }
            if let Some(kick) = override_content.kick {
                power_levels.insert("kick".to_owned(), json!(kick));
            }
            if let Some(notifications) = override_content.notifications.as_ref() {
                power_levels.insert(
                    "notifications".to_owned(),
                    serde_json::to_value(notifications)
                        .map_err(|_| RoomCreateContractError::InvalidPowerLevelContentOverride)?,
                );
            }
            if let Some(redact) = override_content.redact {
                power_levels.insert("redact".to_owned(), json!(redact));
            }
            if let Some(state_default) = override_content.state_default {
                power_levels.insert("state_default".to_owned(), json!(state_default));
            }
            if let Some(users) = override_content.users.as_ref() {
                power_levels.insert(
                    "users".to_owned(),
                    serde_json::to_value(users)
                        .map_err(|_| RoomCreateContractError::InvalidPowerLevelContentOverride)?,
                );
            }
            if let Some(users_default) = override_content.users_default {
                power_levels.insert("users_default".to_owned(), json!(users_default));
            }
        }

        self.push_state_event("m.room.power_levels", "", Value::Object(power_levels));
        Ok(())
    }

    fn push_optional_alias_event(&mut self) {
        if let Some(alias_localpart) = self.room_alias_name {
            self.push_state_event(
                "m.room.canonical_alias",
                "",
                json!({ "alias": format!("#{alias_localpart}:{}", self.home_server_name) }),
            );
        }
    }

    fn push_preset_events(&mut self) {
        let (join_rule, guest_access) = match self.preset {
            RoomPreset::PrivateChat | RoomPreset::TrustedPrivateChat => ("invite", "can_join"),
            RoomPreset::PublicChat => ("public", "forbidden"),
        };
        self.push_state_event("m.room.join_rules", "", json!({ "join_rule": join_rule }));
        self.push_state_event(
            "m.room.history_visibility",
            "",
            json!({ "history_visibility": "shared" }),
        );
        self.push_state_event(
            "m.room.guest_access",
            "",
            json!({ "guest_access": guest_access }),
        );
    }

    fn push_initial_state_events(&mut self) -> Result<(), RoomCreateContractError> {
        if let Some(initial_state) = self.initial_state.take() {
            for initial_state_event in initial_state {
                validate_initial_state_event(&initial_state_event)?;
                self.push_state_event(
                    &initial_state_event.event_type,
                    &initial_state_event.state_key.unwrap_or_default(),
                    initial_state_event.content,
                );
            }
        }

        Ok(())
    }

    fn push_name_topic_events(&mut self) {
        if let Some(name) = self.name {
            self.push_state_event("m.room.name", "", json!({ "name": name }));
        }
        if let Some(topic) = self.topic {
            self.push_state_event("m.room.topic", "", json!({ "topic": topic }));
        }
    }

    fn push_invite_events(&mut self) {
        for invitee in self.invitees {
            self.push_state_event(
                "m.room.member",
                invitee,
                json!({
                    "membership": "invite",
                    "is_direct": self.is_direct.unwrap_or(false),
                }),
            );
        }
    }

    fn push_third_party_invite_events(&mut self) {
        for third_party_invitee in self.third_party_invitees {
            let token = format!("invite_{}", Uuid::new_v4().simple());
            self.push_state_event(
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
    }

    fn finalize_plan(self) -> Result<Vec<CreateRoomStateEventPayload>, RoomCreateContractError> {
        let planned_events = collapse_duplicate_state_events(self.planned_events);
        validate_room_creation_event_order(
            &planned_events,
            self.room_id,
            self.creator_user_id_str,
            self.room_version,
        )?;

        Ok(planned_events
            .into_iter()
            .enumerate()
            .map(|(index, mut event)| {
                event.ordering = i32::try_from(index).unwrap_or(i32::MAX);
                event
            })
            .collect())
    }

    fn push_state_event(&mut self, event_type: &str, state_key: &str, content: Value) {
        self.planned_events.push(CreateRoomStateEventPayload {
            content,
            event_type: event_type.to_owned(),
            ordering: 0,
            state_key: state_key.to_owned(),
        });
    }
}

fn validate_initial_state_event(event: &RoomStateEventInfo) -> Result<(), RoomCreateContractError> {
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

fn validate_room_creation_event_order(
    planned_events: &[CreateRoomStateEventPayload],
    room_id: &str,
    creator_user_id: &str,
    room_version: RoomVersion,
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

    let _ = room_version;
    Ok(())
}
