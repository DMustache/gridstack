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
            handlers::create_room::{CreateRoomInfo, RoomPreset, RoomVisibility},
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
        room_id: &str,
        creator_user_id: &UserIdentifier,
        request: &CreateRoomInfo,
    ) -> Result<(), DomainError>;
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

    fn save_room(
        &self,
        room_id: &str,
        creator_user_id: &UserIdentifier,
        request: &CreateRoomInfo,
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
                    room_id: room_id.to_owned(),
                    room_version: request.room_version.clone(),
                    creator_user_id: creator_user_id.as_str().to_owned(),
                    is_direct: request.is_direct,
                    name: request.name.clone(),
                    topic: request.topic.clone(),
                    visibility: request
                        .visibility
                        .as_ref()
                        .map(|visibility| match visibility {
                            RoomVisibility::Public => "public".to_owned(),
                            RoomVisibility::Private => "private".to_owned(),
                        }),
                    preset: request.preset.as_ref().map(|preset| match preset {
                        RoomPreset::PrivateChat => "private_chat".to_owned(),
                        RoomPreset::PublicChat => "public_chat".to_owned(),
                        RoomPreset::TrustedPrivateChat => "trusted_private_chat".to_owned(),
                    }),
                    created_at: now,
                    updated_at: now,
                };

                insert_into(rooms::table)
                    .values(create_room)
                    .execute(connection)?;

                if let Some(alias_localpart) = request.room_alias_name.as_ref() {
                    insert_into(room_aliases::table)
                        .values(CreateRoomAliasModel {
                            alias_localpart: alias_localpart.clone(),
                            room_id: room_id.to_owned(),
                            created_at: now,
                        })
                        .execute(connection)?;
                }

                if let Some(initial_state) = request.initial_state.as_ref() {
                    let events = initial_state
                        .iter()
                        .enumerate()
                        .map(|(index, event)| CreateRoomStateEventModel {
                            id: Uuid::new_v4(),
                            room_id: room_id.to_owned(),
                            event_type: event.event_type.clone(),
                            state_key: event.state_key.clone().unwrap_or_default(),
                            content: event.content.clone(),
                            ordering: i32::try_from(index).unwrap_or(i32::MAX),
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
