use std::{collections::BTreeMap, sync::Arc};

use serde_json::{Map, Value, json};

use crate::services::{
    authorization::entities::UserId,
    errors::ApplicationError,
    messaging_events::entities::{EventId, RoomEvent},
    repositories::RoomRepository,
    traits::{Clock, IdGenerator},
};

use super::view_models::{
    RoomEventEnvelopeViewModel, RoomMessagesResponseViewModel, SendRoomMessageResponseViewModel,
    SyncResponseViewModel,
};

pub struct MessagingEventsService {
    room_repository: Arc<dyn RoomRepository>,
    id_generator: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
    home_server_name: String,
}

impl MessagingEventsService {
    pub fn new(
        room_repository: Arc<dyn RoomRepository>,
        id_generator: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
        home_server_name: String,
    ) -> Self {
        Self {
            room_repository,
            id_generator,
            clock,
            home_server_name,
        }
    }

    pub fn send_room_message(
        &self,
        sender_identifier: UserId,
        room_identifier_or_alias: &str,
        event_type: String,
        content: Map<String, Value>,
    ) -> Result<SendRoomMessageResponseViewModel, ApplicationError> {
        let room = self
            .room_repository
            .find_room_by_identifier_or_alias(room_identifier_or_alias)
            .ok_or(ApplicationError::NotFound)?;

        let event_identifier = self
            .id_generator
            .next_event_identifier(&self.home_server_name);
        let parsed_event_id = EventId::parse(event_identifier.clone())
            .ok_or_else(|| ApplicationError::invalid_input("invalid generated event id"))?;
        self.room_repository.add_event(
            &room.room_identifier,
            RoomEvent {
                event_identifier: parsed_event_id,
                event_type,
                sender_identifier: sender_identifier.into_inner(),
                content: Value::Object(content),
                timestamp_milliseconds: self.clock.now_unix_milliseconds(),
            },
        )?;

        Ok(SendRoomMessageResponseViewModel { event_identifier })
    }

    pub fn get_room_messages(
        &self,
        room_identifier_or_alias: &str,
    ) -> Result<RoomMessagesResponseViewModel, ApplicationError> {
        let room = self
            .room_repository
            .find_room_by_identifier_or_alias(room_identifier_or_alias)
            .ok_or(ApplicationError::NotFound)?;
        let chunk = self
            .room_repository
            .list_events(&room.room_identifier)?
            .into_iter()
            .map(RoomEventEnvelopeViewModel::from)
            .collect();

        Ok(RoomMessagesResponseViewModel { chunk })
    }

    pub fn get_events(&self) -> RoomMessagesResponseViewModel {
        let mut chunk = Vec::new();
        for room in self.room_repository.list_public_rooms() {
            for event in room.events {
                chunk.push(RoomEventEnvelopeViewModel::from(event));
            }
        }
        RoomMessagesResponseViewModel { chunk }
    }

    pub fn get_event_by_identifier(
        &self,
        event_identifier: &str,
    ) -> Result<RoomEventEnvelopeViewModel, ApplicationError> {
        for room in self.room_repository.list_public_rooms() {
            if let Some(event) = room
                .events
                .into_iter()
                .find(|event| event.event_identifier.as_str() == event_identifier)
            {
                return Ok(RoomEventEnvelopeViewModel::from(event));
            }
        }
        Err(ApplicationError::NotFound)
    }

    pub fn sync(&self) -> SyncResponseViewModel {
        let mut joined_rooms = Map::new();
        for room in self.room_repository.list_public_rooms() {
            let timeline_events: Vec<Value> = room
                .events
                .into_iter()
                .map(|room_event| {
                    json!({
                        "event_id": room_event.event_identifier.into_inner(),
                        "type": room_event.event_type,
                        "sender": room_event.sender_identifier,
                        "origin_server_ts": room_event.timestamp_milliseconds,
                        "content": room_event.content
                    })
                })
                .collect();

            joined_rooms.insert(
                room.room_identifier,
                json!({
                    "timeline": {
                        "events": timeline_events
                    }
                }),
            );
        }

        SyncResponseViewModel {
            next_batch: format!("batch_{}", self.clock.now_unix_milliseconds()),
            rooms: Value::Object(
                BTreeMap::from([("join".to_owned(), Value::Object(joined_rooms))])
                    .into_iter()
                    .collect(),
            ),
        }
    }
}
