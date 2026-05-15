use chrono::Utc;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
};
use uuid::Uuid;

use crate::{
    infrastructure::{schema, user_identifier::UserIdentifier},
    services::{
        errors::DomainError,
        rooms::{
            entities::CreateRoomPersistencePayload,
            persistence::models::{
                CreateRoomAliasModel, CreateRoomModel, CreateRoomStateEventModel,
            },
        },
    },
};

mod models;

pub trait RoomRepository: Send + Sync {
    fn reserve_room_alias(&self, room_alias_name: &str) -> Result<bool, DomainError>;
    fn save_room(&self, payload: &CreateRoomPersistencePayload) -> Result<(), DomainError>;
}

#[derive(Clone)]
pub struct RoomPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl RoomPersistence {
    pub fn new(database_url: &str) -> Result<Self, DomainError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder()
            .build(manager)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(Self { connection_pool })
    }
}

impl RoomRepository for RoomPersistence {
    fn reserve_room_alias(&self, room_alias_name: &str) -> Result<bool, DomainError> {
        use schema::room_aliases;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let existing = room_aliases::table
            .select(room_aliases::alias_localpart)
            .filter(room_aliases::alias_localpart.eq(room_alias_name))
            .first::<String>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(existing.is_none())
    }

    fn save_room(&self, payload: &CreateRoomPersistencePayload) -> Result<(), DomainError> {
        use schema::{room_aliases, room_state_events, rooms};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        connection
            .transaction(|connection| {
                let now = Utc::now().naive_utc();
                let create_room = CreateRoomModel {
                    room_id: payload.room_id.clone(),
                    room_version: Some(payload.room_version.clone()),
                    creator_user_id: payload.creator_user_id.clone(),
                    is_direct: payload.is_direct,
                    name: payload.name.clone(),
                    topic: payload.topic.clone(),
                    visibility: payload.visibility.clone(),
                    preset: payload.preset.clone(),
                    created_at: now,
                    updated_at: now,
                };

                insert_into(rooms::table)
                    .values(create_room)
                    .execute(connection)?;

                if let Some(alias_localpart) = payload.room_alias_name.as_ref() {
                    insert_into(room_aliases::table)
                        .values(CreateRoomAliasModel {
                            alias_localpart: alias_localpart.clone(),
                            room_id: payload.room_id.clone(),
                            created_at: now,
                        })
                        .execute(connection)?;
                }

                if !payload.initial_state.is_empty() {
                    let events = payload
                        .initial_state
                        .iter()
                        .map(|event| CreateRoomStateEventModel {
                            id: Uuid::new_v4(),
                            room_id: payload.room_id.clone(),
                            event_type: event.event_type.clone(),
                            state_key: event.state_key.clone(),
                            content: event.content.clone(),
                            ordering: event.ordering,
                            created_at: now,
                        })
                        .collect::<Vec<_>>();

                    if !events.is_empty() {
                        insert_into(room_state_events::table)
                            .values(events)
                            .execute(connection)?;
                    }
                }

                Ok::<(), diesel::result::Error>(())
            })
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(())
    }
}
