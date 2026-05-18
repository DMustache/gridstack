use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::{
        authorization::{entities::ExistingUserIdentifier, service::AuthorizationService},
        events::entities::PreviousRoom,
        rooms::{
            entities::{RoomStateEventInfo, versions::RoomVersion},
            errors::RoomsApplicationError,
            handlers::create_room::{
                CreateRoomInfo, CreationContentInfo, InviteThirdPartyIdentifierInfo,
                RoomPowerLevelsContentOverride, RoomPreset, RoomVisibility,
            },
        },
    },
};

#[derive(Clone, Debug)]
pub struct CreationContent {
    pub additional_creators: Vec<String>,
    pub federate: bool,
    pub predecessor: Option<PreviousRoom>,
    pub room_type: Option<String>,
    pub extra_content: serde_json::Map<String, serde_json::Value>,
}

impl Default for CreationContent {
    fn default() -> Self {
        Self {
            additional_creators: Vec::new(),
            federate: true,
            predecessor: None,
            room_type: None,
            extra_content: serde_json::Map::new(),
        }
    }
}

impl CreationContent {
    fn try_from_creation_content_info(
        creation_content: CreationContentInfo,
    ) -> Result<Self, RoomsApplicationError> {
        let additional_creators = creation_content.additional_creators.unwrap_or_default();
        if additional_creators
            .iter()
            .any(|user_id| UserIdentifier::try_from(user_id.clone()).is_err())
        {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        let federate = creation_content.federate.unwrap_or(true);
        let predecessor = creation_content.predecessor;
        if let Some(predecessor) = predecessor.as_ref()
            && predecessor.validate().is_err()
        {
            return Err(RoomsApplicationError::InvalidParameter);
        }
        if let Some(room_type) = creation_content.room_type.as_deref()
            && room_type.trim().is_empty()
        {
            return Err(RoomsApplicationError::InvalidParameter);
        }
        let room_type = creation_content.room_type;
        let _ = creation_content.creator;
        let _ = creation_content.room_version;

        Ok(Self {
            additional_creators,
            federate,
            predecessor,
            room_type,
            extra_content: creation_content.extra_content,
        })
    }

    pub fn to_event_content_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut event_content = self.extra_content.clone();
        if !self.additional_creators.is_empty() {
            event_content.insert(
                "additional_creators".to_owned(),
                serde_json::Value::Array(
                    self.additional_creators
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        event_content.insert(
            "m.federate".to_owned(),
            serde_json::Value::Bool(self.federate),
        );
        if let Some(predecessor) = self.predecessor.as_ref() {
            if let Ok(predecessor_value) = serde_json::to_value(predecessor) {
                event_content.insert("predecessor".to_owned(), predecessor_value);
            }
        }
        if let Some(room_type) = self.room_type.as_ref() {
            event_content.insert(
                "type".to_owned(),
                serde_json::Value::String(room_type.clone()),
            );
        }

        event_content
    }
}

#[derive(Clone, Debug)]
pub struct ValidatedCreateRoomRequest {
    pub creation_content: CreationContent,
    pub initial_state: Vec<RoomStateEventInfo>,
    pub invite: Vec<ExistingUserIdentifier>,
    pub invite_3pid: Vec<InviteThirdPartyIdentifierInfo>,
    pub is_direct: bool,
    pub name: Option<String>,
    pub power_level_content_override: RoomPowerLevelsContentOverride,
    pub preset: RoomPreset,
    pub room_alias_name: Option<String>,
    pub room_version: RoomVersion,
    pub topic: Option<String>,
    pub visibility: RoomVisibility,
}

impl ValidatedCreateRoomRequest {
    pub fn try_from_request(
        value: CreateRoomInfo,
        authorization_service: &AuthorizationService,
    ) -> Result<Self, RoomsApplicationError> {
        let creation_content = CreationContent::try_from_creation_content_info(
            value.creation_content.unwrap_or_default(),
        )?;

        let mut invite = Vec::new();
        for invitee in value.invite.as_deref().unwrap_or(&[]) {
            let invitee_identifier = UserIdentifier::try_from(invitee.to_owned())
                .map_err(|_| RoomsApplicationError::InvalidParameter)?;
            let existing_user_identifier = authorization_service
                .require_existing_user_identifier(invitee_identifier)
                .map_err(|_| RoomsApplicationError::InvalidParameter)?;
            invite.push(existing_user_identifier);
        }

        if value.invite_3pid.as_ref().is_some_and(|invitees| {
            invitees
                .iter()
                .any(InviteThirdPartyIdentifierInfo::has_empty_required_field)
        }) {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        if let Some(room_alias_name) = value.room_alias_name.as_deref()
            && (room_alias_name.trim().is_empty()
                || room_alias_name.contains(':')
                || room_alias_name.contains('\0'))
        {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        let room_version = match value.room_version.as_deref() {
            Some("1") | None => RoomVersion::V1,
            Some(_) => return Err(RoomsApplicationError::UnsupportedRoomVersion),
        };

        Ok(Self {
            creation_content,
            initial_state: value.initial_state.unwrap_or_default(),
            invite,
            invite_3pid: value.invite_3pid.unwrap_or_default(),
            is_direct: value.is_direct.unwrap_or(false),
            name: value.name,
            power_level_content_override: value.power_level_content_override.unwrap_or_default(),
            preset: value.preset,
            room_alias_name: value.room_alias_name.map(|alias| alias.trim().to_owned()),
            room_version,
            topic: value.topic,
            visibility: value.visibility,
        })
    }
}

impl InviteThirdPartyIdentifierInfo {
    fn has_empty_required_field(&self) -> bool {
        self.id_server.trim().is_empty()
            || self.id_access_token.trim().is_empty()
            || self.medium.trim().is_empty()
            || self.address.trim().is_empty()
    }
}
