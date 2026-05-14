use std::sync::Arc;

use uuid::Uuid;

use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::rooms::{
        entities::CreatedRoom, errors::RoomsApplicationError,
        handlers::create_room::CreateRoomInfo, persistence::RoomRepository,
    },
};

pub struct RoomsService {
    room_repository: Arc<dyn RoomRepository>,
    home_server_name: String,
}

impl RoomsService {
    pub fn new(room_repository: Arc<dyn RoomRepository>, home_server_name: String) -> Self {
        Self {
            room_repository,
            home_server_name,
        }
    }

    pub fn create_room(
        &self,
        creator_user_id: &UserIdentifier,
        info: CreateRoomInfo,
    ) -> Result<CreatedRoom, RoomsApplicationError> {
        self.validate_create_room_request(&info)?;

        if let Some(room_alias_name) = info.room_alias_name.as_deref()
            && !self
                .room_repository
                .reserve_room_alias(room_alias_name)
                .map_err(|_| RoomsApplicationError::Internal)?
        {
            return Err(RoomsApplicationError::RoomInUse);
        }

        let room_id = format!("!{}:{}", Uuid::new_v4().simple(), self.home_server_name);
        self.room_repository
            .save_room(&room_id, creator_user_id, &info)
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("room_aliases") && message.contains("duplicate") {
                    return RoomsApplicationError::RoomInUse;
                }
                RoomsApplicationError::Internal
            })?;

        Ok(CreatedRoom { room_id })
    }

    fn validate_create_room_request(
        &self,
        info: &CreateRoomInfo,
    ) -> Result<(), RoomsApplicationError> {
        if let Some(room_alias_name) = info.room_alias_name.as_deref()
            && room_alias_name.trim().is_empty()
        {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        if let Some(room_version) = info.room_version.as_deref()
            && (room_version.trim().is_empty()
                || !room_version.chars().all(|char| char.is_ascii_digit()))
        {
            return Err(RoomsApplicationError::UnsupportedRoomVersion);
        }

        if let Some(invitees) = info.invite.as_ref()
            && invitees.iter().any(|invitee| !invitee.starts_with('@'))
        {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        if let Some(initial_state) = info.initial_state.as_ref()
            && initial_state
                .iter()
                .any(|state| state.event_type.trim().is_empty())
        {
            return Err(RoomsApplicationError::InvalidRoomState);
        }

        Ok(())
    }
}
