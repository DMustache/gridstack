use std::sync::Arc;

use crate::{
    infrastructure::server_name::ServerName,
    services::{
        authorization::entities::AuthorizedUserIdentifier,
        rooms::{
            entities::{CreatedRoom, Room, RoomCreateContractError},
            errors::RoomsApplicationError,
            handlers::create_room::CreateRoomInfo,
            persistence::RoomRepository,
        },
    },
};

pub struct RoomsService {
    room_repository: Arc<dyn RoomRepository>,

    server_name: Arc<ServerName>,
}

impl RoomsService {
    pub fn new(room_repository: Arc<dyn RoomRepository>, server_name: &ServerName) -> Self {
        Self {
            room_repository,
            server_name: Arc::new(server_name.clone()),
        }
    }

    pub fn create_room(
        &self,
        creator_user_id: &AuthorizedUserIdentifier,
        info: CreateRoomInfo,
    ) -> Result<CreatedRoom, RoomsApplicationError> {
        let contract = Room::try_create_contract(creator_user_id, info, &self.server_name)
            .map_err(|error| map_contract_error(&error))?;

        if let Some(room_alias_name) = contract.persistence_payload.room_alias_name.as_deref()
            && !self
                .room_repository
                .reserve_room_alias(room_alias_name)
                .map_err(|_| RoomsApplicationError::Internal)?
        {
            return Err(RoomsApplicationError::RoomInUse);
        }

        self.room_repository
            .save_room(&contract.persistence_payload)
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("room_aliases") && message.contains("duplicate") {
                    return RoomsApplicationError::RoomInUse;
                }
                RoomsApplicationError::Internal
            })?;

        Ok(CreatedRoom {
            room_id: contract.persistence_payload.room_id,
        })
    }
}

const fn map_contract_error(error: &RoomCreateContractError) -> RoomsApplicationError {
    match error {
        RoomCreateContractError::UnsupportedRoomVersion { .. } => {
            RoomsApplicationError::UnsupportedRoomVersion
        }
        RoomCreateContractError::InvalidRoomStateEventType
        | RoomCreateContractError::UnsupportedStateEventForRoomVersion { .. } => {
            RoomsApplicationError::InvalidRoomState
        }
        RoomCreateContractError::InvalidHomeServerName { .. }
        | RoomCreateContractError::InvalidGeneratedRoomIdentifier { .. }
        | RoomCreateContractError::InvalidRoomIdentifier(_)
        | RoomCreateContractError::InvalidRoomAlias
        | RoomCreateContractError::InvalidInviteUserIdentifier => {
            RoomsApplicationError::InvalidParameter
        }
    }
}
