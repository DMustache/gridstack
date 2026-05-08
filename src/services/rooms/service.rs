use std::sync::Arc;

use serde_json::Value;

use crate::services::{
    authorization::entities::UserId, errors::ApplicationError, repositories::RoomRepository,
    rooms::entities::ChatRoom, traits::IdGenerator,
};

use super::view_models::{
    CreateRoomRequestViewModel, CreateRoomResponseViewModel, JoinRoomResponseViewModel,
    PublicRoomViewModel, PublicRoomsResponseViewModel,
};

pub struct RoomsService {
    room_repository: Arc<dyn RoomRepository>,
    id_generator: Arc<dyn IdGenerator>,
    home_server_name: String,
}

impl RoomsService {
    pub fn new(
        room_repository: Arc<dyn RoomRepository>,
        id_generator: Arc<dyn IdGenerator>,
        home_server_name: String,
    ) -> Self {
        Self {
            room_repository,
            id_generator,
            home_server_name,
        }
    }

    pub fn create_room(
        &self,
        creator_identifier: UserId,
        request: CreateRoomRequestViewModel,
    ) -> Result<CreateRoomResponseViewModel, ApplicationError> {
        let room_identifier = self
            .id_generator
            .next_room_identifier(&self.home_server_name);
        let room_alias = request.room_alias_name.map(|alias_name| {
            let sanitized_alias = alias_name.trim_start_matches('#');
            format!("#{}:{}", sanitized_alias, self.home_server_name)
        });
        let room_name = request.name.unwrap_or_else(|| "Room".to_owned());
        let is_public = matches!(request.visibility.as_deref(), Some("public"));

        self.room_repository.create_room(ChatRoom {
            room_identifier: room_identifier.clone(),
            room_alias,
            name: room_name,
            creator_identifier: creator_identifier.as_str().to_owned(),
            is_public,
            members: vec![creator_identifier.as_str().to_owned()],
            events: Vec::new(),
        })?;

        Ok(CreateRoomResponseViewModel { room_identifier })
    }

    pub fn join_room(
        &self,
        user_identifier: UserId,
        room_identifier_or_alias: &str,
    ) -> Result<JoinRoomResponseViewModel, ApplicationError> {
        let room = self
            .room_repository
            .find_room_by_identifier_or_alias(room_identifier_or_alias)
            .ok_or(ApplicationError::NotFound)?;

        self.room_repository
            .add_member(&room.room_identifier, &user_identifier)?;

        Ok(JoinRoomResponseViewModel {
            room_identifier: room.room_identifier,
        })
    }

    pub fn get_public_rooms(&self) -> PublicRoomsResponseViewModel {
        PublicRoomsResponseViewModel {
            chunk: self
                .room_repository
                .list_public_rooms()
                .into_iter()
                .map(|chat_room| PublicRoomViewModel {
                    room_identifier: chat_room.room_identifier,
                    name: chat_room.name,
                    joined_member_count: chat_room.members.len(),
                })
                .collect(),
        }
    }

    pub fn get_room_state(
        &self,
        room_identifier_or_alias: &str,
    ) -> Result<Vec<Value>, ApplicationError> {
        let room = self
            .room_repository
            .find_room_by_identifier_or_alias(room_identifier_or_alias)
            .ok_or(ApplicationError::NotFound)?;
        let events = self.room_repository.list_events(&room.room_identifier)?;
        Ok(events
            .into_iter()
            .map(|event| serde_json::to_value(event).unwrap_or(Value::Null))
            .collect())
    }
}
