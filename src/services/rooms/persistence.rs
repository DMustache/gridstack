use std::{collections::HashMap, sync::RwLock};

use crate::services::{
    authorization::entities::UserId, errors::DomainError, messaging_events::entities::RoomEvent,
    repositories::RoomRepository, rooms::entities::ChatRoom,
};

mod room_memberships;
mod rooms;

#[derive(Default)]
pub struct InMemoryRoomRepository {
    rooms_by_identifier: RwLock<HashMap<String, ChatRoom>>,
    room_identifier_by_alias: RwLock<HashMap<String, String>>,
}

impl RoomRepository for InMemoryRoomRepository {
    fn create_room(&self, chat_room: ChatRoom) -> Result<(), DomainError> {
        let mut rooms_by_identifier = self
            .rooms_by_identifier
            .write()
            .map_err(|_| DomainError::InvalidRequest("room storage lock failure".to_owned()))?;
        let mut room_identifier_by_alias = self
            .room_identifier_by_alias
            .write()
            .map_err(|_| DomainError::InvalidRequest("room alias lock failure".to_owned()))?;
        if rooms_by_identifier.contains_key(&chat_room.room_identifier) {
            return Err(DomainError::AlreadyExists);
        }
        if let Some(room_alias) = &chat_room.room_alias {
            if room_identifier_by_alias.contains_key(room_alias) {
                return Err(DomainError::AlreadyExists);
            }
            room_identifier_by_alias.insert(room_alias.clone(), chat_room.room_identifier.clone());
        }
        rooms_by_identifier.insert(chat_room.room_identifier.clone(), chat_room);
        Ok(())
    }

    fn find_room_by_identifier_or_alias(&self, room_identifier_or_alias: &str) -> Option<ChatRoom> {
        let room_identifier = if room_identifier_or_alias.starts_with('#') {
            let room_identifier_by_alias = self.room_identifier_by_alias.read().ok()?;
            room_identifier_by_alias
                .get(room_identifier_or_alias)?
                .clone()
        } else {
            room_identifier_or_alias.to_owned()
        };
        let rooms_by_identifier = self.rooms_by_identifier.read().ok()?;
        rooms_by_identifier.get(&room_identifier).cloned()
    }

    fn list_public_rooms(&self) -> Vec<ChatRoom> {
        self.rooms_by_identifier
            .read()
            .map(|rooms_by_identifier| {
                rooms_by_identifier
                    .values()
                    .filter(|chat_room| chat_room.is_public)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn add_member(
        &self,
        room_identifier: &str,
        user_identifier: &UserId,
    ) -> Result<(), DomainError> {
        let mut rooms_by_identifier = self
            .rooms_by_identifier
            .write()
            .map_err(|_| DomainError::InvalidRequest("room storage lock failure".to_owned()))?;
        let room = rooms_by_identifier
            .get_mut(room_identifier)
            .ok_or(DomainError::NotFound)?;
        if !room
            .members
            .iter()
            .any(|member| member == user_identifier.as_str())
        {
            room.members.push(user_identifier.as_str().to_owned());
        }
        Ok(())
    }

    fn add_event(&self, room_identifier: &str, room_event: RoomEvent) -> Result<(), DomainError> {
        let mut rooms_by_identifier = self
            .rooms_by_identifier
            .write()
            .map_err(|_| DomainError::InvalidRequest("room storage lock failure".to_owned()))?;
        let room = rooms_by_identifier
            .get_mut(room_identifier)
            .ok_or(DomainError::NotFound)?;
        room.events.push(room_event);
        Ok(())
    }

    fn list_events(&self, room_identifier: &str) -> Result<Vec<RoomEvent>, DomainError> {
        let rooms_by_identifier = self
            .rooms_by_identifier
            .read()
            .map_err(|_| DomainError::InvalidRequest("room storage lock failure".to_owned()))?;
        let room = rooms_by_identifier
            .get(room_identifier)
            .ok_or(DomainError::NotFound)?;
        Ok(room.events.clone())
    }
}
