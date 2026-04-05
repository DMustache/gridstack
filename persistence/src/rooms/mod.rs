mod models;
pub use models::{
    NewRoom, NewRoomEvent, NewRoomMembership, NewRoomState, Room, RoomEvent, RoomMembership,
    RoomState,
};

use ::models::error::internal_error::DatabaseError;
use chrono::Utc;
use deadpool_diesel::{
    Runtime,
    postgres::{Manager, Pool},
};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl,
    SelectableHelper,
    dsl::{exists, max},
    insert_into, select,
    upsert::excluded,
};

use crate::schema::{room_events, room_memberships, room_state, rooms};

use std::sync::LazyLock;

static CONNECTION_STRING: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set at runtime"));

pub struct Rooms {
    pool: Pool,
}

impl Rooms {
    pub fn try_new() -> Result<Self, DatabaseError> {
        let manager = Manager::new(&*CONNECTION_STRING, Runtime::Tokio1);
        let pool = Pool::builder(manager).build()?;

        Ok(Self { pool })
    }

    pub async fn create_room(&self, room: NewRoom) -> Result<Room, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let room = insert_into(rooms::table)
                    .values(room)
                    .returning(Room::as_select())
                    .get_result::<Room>(connection)?;

                Ok::<Room, DatabaseError>(room)
            })
            .await??)
    }

    pub async fn get_room(&self, room_id: String) -> Result<Option<Room>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let room = rooms::table
                    .filter(rooms::room_id.eq(room_id))
                    .select(Room::as_select())
                    .first::<Room>(connection)
                    .optional()?;

                Ok::<Option<Room>, DatabaseError>(room)
            })
            .await??)
    }

    pub async fn upsert_membership(
        &self,
        membership: NewRoomMembership,
    ) -> Result<RoomMembership, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let membership = insert_into(room_memberships::table)
                    .values(membership)
                    .on_conflict((room_memberships::room_id, room_memberships::user_id))
                    .do_update()
                    .set((
                        room_memberships::membership.eq(excluded(room_memberships::membership)),
                        room_memberships::membership_event_id
                            .eq(excluded(room_memberships::membership_event_id)),
                        room_memberships::updated_at.eq(Utc::now()),
                    ))
                    .returning(RoomMembership::as_select())
                    .get_result::<RoomMembership>(connection)?;

                Ok::<RoomMembership, DatabaseError>(membership)
            })
            .await??)
    }

    pub async fn get_membership(
        &self,
        room_id: String,
        user_id: String,
    ) -> Result<Option<RoomMembership>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let membership = room_memberships::table
                    .filter(room_memberships::room_id.eq(room_id))
                    .filter(room_memberships::user_id.eq(user_id))
                    .select(RoomMembership::as_select())
                    .first::<RoomMembership>(connection)
                    .optional()?;

                Ok::<Option<RoomMembership>, DatabaseError>(membership)
            })
            .await??)
    }

    pub async fn append_event(&self, event: NewRoomEvent) -> Result<RoomEvent, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let event = insert_into(room_events::table)
                    .values(event)
                    .returning(RoomEvent::as_select())
                    .get_result::<RoomEvent>(connection)?;

                Ok::<RoomEvent, DatabaseError>(event)
            })
            .await??)
    }

    pub async fn get_event(&self, event_id: String) -> Result<Option<RoomEvent>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let event = room_events::table
                    .filter(room_events::event_id.eq(event_id))
                    .select(RoomEvent::as_select())
                    .first::<RoomEvent>(connection)
                    .optional()?;

                Ok::<Option<RoomEvent>, DatabaseError>(event)
            })
            .await??)
    }

    pub async fn get_event_by_transaction_id(
        &self,
        room_id: String,
        sender_user_id: String,
        transaction_id: String,
    ) -> Result<Option<RoomEvent>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let event = room_events::table
                    .filter(room_events::room_id.eq(room_id))
                    .filter(room_events::sender_user_id.eq(sender_user_id))
                    .filter(room_events::transaction_id.eq(transaction_id))
                    .select(RoomEvent::as_select())
                    .first::<RoomEvent>(connection)
                    .optional()?;

                Ok::<Option<RoomEvent>, DatabaseError>(event)
            })
            .await??)
    }

    pub async fn list_events(
        &self,
        room_id: String,
        from_stream_ordering: Option<i64>,
        limit: i64,
    ) -> Result<Vec<RoomEvent>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let mut query = room_events::table
                    .filter(room_events::room_id.eq(room_id))
                    .into_boxed();

                if let Some(from_stream_ordering) = from_stream_ordering {
                    query = query.filter(room_events::stream_ordering.gt(from_stream_ordering));
                }

                let events = query
                    .order_by(room_events::stream_ordering.asc())
                    .limit(limit)
                    .select(RoomEvent::as_select())
                    .load::<RoomEvent>(connection)?;

                Ok::<Vec<RoomEvent>, DatabaseError>(events)
            })
            .await??)
    }

    pub async fn list_events_paginated(
        &self,
        room_id: String,
        from_stream_ordering: Option<i64>,
        to_stream_ordering: Option<i64>,
        direction: PaginationDirection,
        limit: i64,
    ) -> Result<Vec<RoomEvent>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let mut query = room_events::table
                    .filter(room_events::room_id.eq(room_id))
                    .into_boxed();

                match direction {
                    PaginationDirection::Forward => {
                        if let Some(from_stream_ordering) = from_stream_ordering {
                            query =
                                query.filter(room_events::stream_ordering.gt(from_stream_ordering));
                        }

                        if let Some(to_stream_ordering) = to_stream_ordering {
                            query =
                                query.filter(room_events::stream_ordering.lt(to_stream_ordering));
                        }

                        let events = query
                            .order_by(room_events::stream_ordering.asc())
                            .limit(limit)
                            .select(RoomEvent::as_select())
                            .load::<RoomEvent>(connection)?;

                        Ok::<Vec<RoomEvent>, DatabaseError>(events)
                    }
                    PaginationDirection::Backward => {
                        if let Some(from_stream_ordering) = from_stream_ordering {
                            query =
                                query.filter(room_events::stream_ordering.lt(from_stream_ordering));
                        }

                        if let Some(to_stream_ordering) = to_stream_ordering {
                            query =
                                query.filter(room_events::stream_ordering.gt(to_stream_ordering));
                        }

                        let events = query
                            .order_by(room_events::stream_ordering.desc())
                            .limit(limit)
                            .select(RoomEvent::as_select())
                            .load::<RoomEvent>(connection)?;

                        Ok::<Vec<RoomEvent>, DatabaseError>(events)
                    }
                }
            })
            .await??)
    }

    pub async fn get_latest_event_depth(
        &self,
        room_id: String,
    ) -> Result<Option<i64>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let depth = room_events::table
                    .filter(room_events::room_id.eq(room_id))
                    .select(max(room_events::depth))
                    .first::<Option<i64>>(connection)?;

                Ok::<Option<i64>, DatabaseError>(depth)
            })
            .await??)
    }

    pub async fn get_latest_stream_ordering_for_room(
        &self,
        room_id: String,
    ) -> Result<Option<i64>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let ordering = room_events::table
                    .filter(room_events::room_id.eq(room_id))
                    .select(max(room_events::stream_ordering))
                    .first::<Option<i64>>(connection)?;

                Ok::<Option<i64>, DatabaseError>(ordering)
            })
            .await??)
    }

    pub async fn upsert_state(&self, state: NewRoomState) -> Result<RoomState, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let state = insert_into(room_state::table)
                    .values(state)
                    .on_conflict((
                        room_state::room_id,
                        room_state::event_type,
                        room_state::state_key,
                    ))
                    .do_update()
                    .set((
                        room_state::event_id.eq(excluded(room_state::event_id)),
                        room_state::updated_at.eq(Utc::now()),
                    ))
                    .returning(RoomState::as_select())
                    .get_result::<RoomState>(connection)?;

                Ok::<RoomState, DatabaseError>(state)
            })
            .await??)
    }

    pub async fn get_state_event(
        &self,
        room_id: String,
        event_type: String,
        state_key: String,
    ) -> Result<Option<RoomEvent>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let event = room_state::table
                    .inner_join(
                        room_events::table.on(room_state::event_id.eq(room_events::event_id)),
                    )
                    .filter(room_state::room_id.eq(room_id))
                    .filter(room_state::event_type.eq(event_type))
                    .filter(room_state::state_key.eq(state_key))
                    .select(RoomEvent::as_select())
                    .first::<RoomEvent>(connection)
                    .optional()?;

                Ok::<Option<RoomEvent>, DatabaseError>(event)
            })
            .await??)
    }

    pub async fn list_current_state(
        &self,
        room_id: String,
    ) -> Result<Vec<RoomEvent>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let events = room_state::table
                    .inner_join(
                        room_events::table.on(room_state::event_id.eq(room_events::event_id)),
                    )
                    .filter(room_state::room_id.eq(room_id))
                    .select(RoomEvent::as_select())
                    .load::<RoomEvent>(connection)?;

                Ok::<Vec<RoomEvent>, DatabaseError>(events)
            })
            .await??)
    }

    pub async fn room_exists(&self, room_id: String) -> Result<bool, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let exists = select(exists(rooms::table.filter(rooms::room_id.eq(room_id))))
                    .get_result::<bool>(connection)?;

                Ok::<bool, DatabaseError>(exists)
            })
            .await??)
    }

    pub async fn list_joined_rooms(
        &self,
        user_id: String,
    ) -> Result<Vec<RoomMembership>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let rooms = room_memberships::table
                    .filter(room_memberships::user_id.eq(user_id))
                    .filter(room_memberships::membership.eq("join"))
                    .select(RoomMembership::as_select())
                    .load::<RoomMembership>(connection)?;

                Ok::<Vec<RoomMembership>, DatabaseError>(rooms)
            })
            .await??)
    }

    pub async fn count_members_by_membership(
        &self,
        room_id: String,
        membership: String,
    ) -> Result<i64, DatabaseError> {
        use diesel::dsl::count_star;

        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let count = room_memberships::table
                    .filter(room_memberships::room_id.eq(room_id))
                    .filter(room_memberships::membership.eq(membership))
                    .select(count_star())
                    .first::<i64>(connection)?;

                Ok::<i64, DatabaseError>(count)
            })
            .await??)
    }

    pub async fn list_hero_user_ids(
        &self,
        room_id: String,
        excluding_user_id: String,
        limit: i64,
    ) -> Result<Vec<String>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let heroes = room_memberships::table
                    .filter(room_memberships::room_id.eq(room_id))
                    .filter(room_memberships::user_id.ne(excluding_user_id))
                    .filter(
                        room_memberships::membership
                            .eq("join")
                            .or(room_memberships::membership.eq("invite")),
                    )
                    .order_by(room_memberships::created_at.asc())
                    .limit(limit)
                    .select(room_memberships::user_id)
                    .load::<String>(connection)?;

                Ok::<Vec<String>, DatabaseError>(heroes)
            })
            .await??)
    }

    pub async fn get_latest_stream_ordering(&self) -> Result<Option<i64>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let ordering = room_events::table
                    .select(max(room_events::stream_ordering))
                    .first::<Option<i64>>(connection)?;

                Ok::<Option<i64>, DatabaseError>(ordering)
            })
            .await??)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PaginationDirection {
    Forward,
    Backward,
}
