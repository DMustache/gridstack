use chrono::Utc;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
};
use uuid::Uuid;

use crate::{
    infrastructure::schema,
    services::{
        errors::DomainError,
        events::entities::EventBatchWriteContract,
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
    fn save_room(
        &self,
        payload: &CreateRoomPersistencePayload,
        event_batch_write_contract: &EventBatchWriteContract,
    ) -> Result<(), DomainError>;
}

#[derive(Clone)]
pub struct RoomPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl RoomPersistence {
    #[must_use]
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder().build_unchecked(manager);

        Self { connection_pool }
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

    fn save_room(
        &self,
        payload: &CreateRoomPersistencePayload,
        event_batch_write_contract: &EventBatchWriteContract,
    ) -> Result<(), DomainError> {
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

                let state_events = event_batch_write_contract
                    .event_write_contracts
                    .iter()
                    .flat_map(|contract| contract.events_to_insert.iter())
                    .filter_map(|event| {
                        event
                            .state_key
                            .as_ref()
                            .map(|state_key| CreateRoomStateEventModel {
                                id: Uuid::new_v4(),
                                room_id: event.room_id.clone(),
                                event_type: event.event_type.clone(),
                                state_key: state_key.clone(),
                                content: event.content.clone(),
                                ordering: 0,
                                created_at: now,
                            })
                    })
                    .enumerate()
                    .map(|(index, mut event)| {
                        event.ordering = i32::try_from(index).unwrap_or(i32::MAX);
                        event
                    })
                    .collect::<Vec<_>>();

                if !state_events.is_empty() {
                    insert_into(room_state_events::table)
                        .values(state_events)
                        .execute(connection)?;
                }

                Ok::<(), diesel::result::Error>(())
            })
            .map_err(|error| match error {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => DomainError::AlreadyExists,
                other => DomainError::InvalidRequest(other.to_string()),
            })?;

        Ok(())
    }
}
