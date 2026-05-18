use chrono::Utc;
use diesel::{
    BoolExpressionMethods, Connection, ExpressionMethods, JoinOnDsl, NullableExpressionMethods,
    OptionalExtension, QueryDsl, RunQueryDsl, insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
};
use uuid::Uuid;

use crate::{
    infrastructure::schema,
    services::events::{
        content_mapper::matrix_event_content_to_json,
        entities::{EventBatchWriteContract, EventWriteContract},
    },
    services::{
        errors::DomainError,
        events::entities::MatrixEventContent,
        rooms::{
            entities::RoomCreationFlow,
            persistence::models::{
                CreateRoomAliasModel, CreateRoomCurrentStateModel, CreateRoomEventAuthEdgeModel,
                CreateRoomEventModel, CreateRoomEventPrevEdgeModel,
                CreateRoomForwardExtremityModel, CreateRoomIdempotencyRecordModel,
                CreateRoomMembershipProjectionModel, CreateRoomModel, CreateRoomOutboxTaskModel,
                CreateRoomStateEventModel, CreateRoomSyncStreamModel,
                CreateRoomTimelineProjectionModel,
            },
        },
    },
};

pub mod models;

#[derive(Clone, Debug)]
pub struct RoomJoinContext {
    pub room_version: Option<String>,
    pub visibility: Option<String>,
    pub membership_state: Option<String>,
}

pub trait RoomRepository: Send + Sync {
    fn create_room_with_initial_events(
        &self,
        room_creation_flow: &RoomCreationFlow,
        event_batch_write_contract: &EventBatchWriteContract,
    ) -> Result<(), DomainError>;

    fn fetch_join_context(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> Result<Option<RoomJoinContext>, DomainError>;

    fn append_membership_event(
        &self,
        event_write_contract: &EventWriteContract,
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
    fn create_room_with_initial_events(
        &self,
        room_creation_flow: &RoomCreationFlow,
        event_batch_write_contract: &EventBatchWriteContract,
    ) -> Result<(), DomainError> {
        use schema::{
            room_aliases, room_current_state, room_event_auth_edges, room_event_prev_edges,
            room_events, room_forward_extremities, room_idempotency_records,
            room_membership_projection, room_outbox_tasks, room_state_events, room_sync_stream,
            room_timeline_projection, rooms,
        };

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        connection
            .transaction(|connection| {
                let now = Utc::now().naive_utc();
                let room_shell = &room_creation_flow.room_shell_intent;

                insert_into(rooms::table)
                    .values(CreateRoomModel {
                        room_id: room_shell.room_id.clone(),
                        room_version: Some(room_shell.room_version.to_string()),
                        creator_user_id: room_shell.creator_user_id.clone(),
                        is_direct: Some(room_shell.is_direct),
                        name: room_shell.name.clone(),
                        topic: room_shell.topic.clone(),
                        visibility: room_shell.visibility.as_str().to_owned(),
                        preset: room_shell.preset.as_str().to_owned(),
                        created_at: now,
                        updated_at: now,
                    })
                    .execute(connection)?;

                if let Some(alias_intent) = room_creation_flow.alias_intent.as_ref() {
                    let alias_rows = vec![CreateRoomAliasModel {
                        alias_localpart: alias_intent.alias_localpart.clone(),
                        room_id: alias_intent.room_id.clone(),
                        created_at: now,
                    }];
                    insert_into(room_aliases::table)
                        .values(alias_rows)
                        .execute(connection)?;
                }

                let mut room_events_rows = Vec::new();
                let mut prev_edge_rows = Vec::new();
                let mut auth_edge_rows = Vec::new();
                let mut forward_extremity_rows = Vec::new();
                let mut current_state_rows = Vec::new();
                let mut membership_rows = Vec::new();
                let mut timeline_rows = Vec::new();
                let mut sync_rows = Vec::new();
                let mut outbox_rows = Vec::new();
                let mut idempotency_rows = Vec::new();

                for event_write_contract in &event_batch_write_contract.event_write_contracts {
                    for event_row in &event_write_contract.event_rows {
                        room_events_rows.push(CreateRoomEventModel {
                            event_id: event_row.event_id.clone(),
                            room_id: event_row.room_id.clone(),
                            room_version: event_row.room_version.to_string(),
                            sender_user_id: event_row.sender.clone(),
                            event_type: event_row.event_type.clone(),
                            state_key: event_row.state_key.clone(),
                            depth: i64::try_from(event_row.depth).unwrap_or(i64::MAX),
                            origin_server_ts: i64::try_from(event_row.origin_server_ts)
                                .unwrap_or(i64::MAX),
                            redacts: None,
                            rejected: event_row.rejected,
                            soft_failed: event_row.soft_failed,
                            membership: extract_membership(&event_row.content),
                            join_rule: extract_join_rule(&event_row.content),
                            history_visibility: extract_history_visibility(&event_row.content),
                            guest_access: extract_guest_access(&event_row.content),
                            canonical_alias: extract_canonical_alias(&event_row.content),
                            room_name: extract_room_name(&event_row.content),
                            room_topic: extract_room_topic(&event_row.content),
                            is_direct: extract_is_direct(&event_row.content),
                            content_json: matrix_event_content_to_json(&event_row.content),
                            unsigned_json: event_row.unsigned.clone(),
                            hashes_json: serde_json::to_value(&event_row.hashes)
                                .unwrap_or_default(),
                            signatures_json: serde_json::to_value(&event_row.signatures)
                                .unwrap_or_default(),
                            created_at: now,
                        });
                    }

                    for edge in &event_write_contract.prev_edge_rows {
                        prev_edge_rows.push(CreateRoomEventPrevEdgeModel {
                            room_id: edge.room_id.clone(),
                            event_id: edge.event_id.clone(),
                            prev_event_id: edge.linked_event_id.clone(),
                            created_at: now,
                        });
                    }
                    for edge in &event_write_contract.auth_edge_rows {
                        auth_edge_rows.push(CreateRoomEventAuthEdgeModel {
                            room_id: edge.room_id.clone(),
                            event_id: edge.event_id.clone(),
                            auth_event_id: edge.linked_event_id.clone(),
                            created_at: now,
                        });
                    }
                    for forward in &event_write_contract.forward_extremity_updates {
                        forward_extremity_rows.push(CreateRoomForwardExtremityModel {
                            room_id: forward.room_id.clone(),
                            event_id: forward.event_id.clone(),
                            created_at: now,
                        });
                    }
                    for current in &event_write_contract.current_state_updates {
                        current_state_rows.push(CreateRoomCurrentStateModel {
                            room_id: current.room_id.clone(),
                            event_type: current.event_type.clone(),
                            state_key: current.state_key.clone(),
                            event_id: current.event_id.clone(),
                            updated_at: now,
                        });
                    }
                    for membership in &event_write_contract.membership_projection_updates {
                        membership_rows.push(CreateRoomMembershipProjectionModel {
                            room_id: membership.room_id.clone(),
                            user_id: membership.user_id.clone(),
                            membership: format!("{:?}", membership.membership).to_lowercase(),
                            event_id: membership.event_id.clone(),
                            updated_at: now,
                        });
                    }
                    for timeline in &event_write_contract.timeline_projection_updates {
                        timeline_rows.push(CreateRoomTimelineProjectionModel {
                            room_id: timeline.room_id.clone(),
                            stream_position: timeline.stream_position,
                            event_id: timeline.event_id.clone(),
                        });
                    }
                    for sync_row in &event_write_contract.sync_stream_rows {
                        sync_rows.push(CreateRoomSyncStreamModel {
                            room_id: sync_row.room_id.clone(),
                            event_id: sync_row.event_id.clone(),
                            created_at: now,
                        });
                    }
                    for outbox_task in &event_write_contract.outbox_tasks {
                        outbox_rows.push(CreateRoomOutboxTaskModel {
                            id: Uuid::new_v4(),
                            room_id: outbox_task.room_id.clone(),
                            event_id: outbox_task.event_id.clone(),
                            task_type: format!("{:?}", outbox_task.task_type).to_lowercase(),
                            status: "pending".to_owned(),
                            payload_json: None,
                            created_at: now,
                        });
                    }
                    for idempotency in &event_write_contract.idempotency_records {
                        idempotency_rows.push(CreateRoomIdempotencyRecordModel {
                            room_id: idempotency.room_id.clone(),
                            sender_user_id: idempotency.sender_user_id.clone(),
                            transaction_id: idempotency.transaction_id.clone(),
                            event_id: idempotency.event_id.clone(),
                            created_at: now,
                        });
                    }
                }

                if !room_events_rows.is_empty() {
                    insert_into(room_events::table)
                        .values(room_events_rows)
                        .execute(connection)?;
                }
                if !prev_edge_rows.is_empty() {
                    insert_into(room_event_prev_edges::table)
                        .values(prev_edge_rows)
                        .execute(connection)?;
                }
                if !auth_edge_rows.is_empty() {
                    insert_into(room_event_auth_edges::table)
                        .values(auth_edge_rows)
                        .execute(connection)?;
                }
                if !forward_extremity_rows.is_empty() {
                    insert_into(room_forward_extremities::table)
                        .values(forward_extremity_rows)
                        .execute(connection)?;
                }
                if !current_state_rows.is_empty() {
                    insert_into(room_current_state::table)
                        .values(current_state_rows)
                        .on_conflict((
                            room_current_state::room_id,
                            room_current_state::event_type,
                            room_current_state::state_key,
                        ))
                        .do_update()
                        .set((
                            room_current_state::event_id
                                .eq(diesel::upsert::excluded(room_current_state::event_id)),
                            room_current_state::updated_at
                                .eq(diesel::upsert::excluded(room_current_state::updated_at)),
                        ))
                        .execute(connection)?;
                }
                if !membership_rows.is_empty() {
                    insert_into(room_membership_projection::table)
                        .values(membership_rows)
                        .on_conflict((
                            room_membership_projection::room_id,
                            room_membership_projection::user_id,
                        ))
                        .do_update()
                        .set((
                            room_membership_projection::membership.eq(diesel::upsert::excluded(
                                room_membership_projection::membership,
                            )),
                            room_membership_projection::event_id.eq(diesel::upsert::excluded(
                                room_membership_projection::event_id,
                            )),
                            room_membership_projection::updated_at.eq(diesel::upsert::excluded(
                                room_membership_projection::updated_at,
                            )),
                        ))
                        .execute(connection)?;
                }
                if !timeline_rows.is_empty() {
                    insert_into(room_timeline_projection::table)
                        .values(timeline_rows)
                        .execute(connection)?;
                }
                if !sync_rows.is_empty() {
                    insert_into(room_sync_stream::table)
                        .values(sync_rows)
                        .execute(connection)?;
                }
                if !outbox_rows.is_empty() {
                    insert_into(room_outbox_tasks::table)
                        .values(outbox_rows)
                        .execute(connection)?;
                }
                if !idempotency_rows.is_empty() {
                    insert_into(room_idempotency_records::table)
                        .values(idempotency_rows)
                        .on_conflict_do_nothing()
                        .execute(connection)?;
                }

                let mut latest_state_event_by_key =
                    Vec::<((String, String, String), (serde_json::Value, usize))>::new();
                let mut state_event_sequence = 0usize;

                for event_write_contract in &event_batch_write_contract.event_write_contracts {
                    for current_state_update in &event_write_contract.current_state_updates {
                        state_event_sequence = state_event_sequence.saturating_add(1);
                        if let Some(event_row) = event_write_contract
                            .event_rows
                            .iter()
                            .find(|event| event.event_id == current_state_update.event_id)
                        {
                            let key = (
                                current_state_update.room_id.clone(),
                                current_state_update.event_type.clone(),
                                current_state_update.state_key.clone(),
                            );
                            if let Some((_, value)) = latest_state_event_by_key
                                .iter_mut()
                                .find(|(existing_key, _)| *existing_key == key)
                            {
                                *value = (
                                    matrix_event_content_to_json(&event_row.content),
                                    state_event_sequence,
                                );
                            } else {
                                latest_state_event_by_key.push((
                                    key,
                                    (
                                        matrix_event_content_to_json(&event_row.content),
                                        state_event_sequence,
                                    ),
                                ));
                            }
                        }
                    }
                }

                let mut state_events = latest_state_event_by_key
                    .into_iter()
                    .map(|((room_id, event_type, state_key), (content, sequence))| {
                        (
                            sequence,
                            CreateRoomStateEventModel {
                                id: Uuid::new_v4(),
                                room_id,
                                event_type,
                                state_key,
                                content,
                                ordering: 0,
                                created_at: now,
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                state_events.sort_by_key(|(sequence, _)| *sequence);

                let mut ordered_state_events = Vec::with_capacity(state_events.len());
                for (index, (_, mut state_event)) in state_events.into_iter().enumerate() {
                    state_event.ordering = i32::try_from(index).unwrap_or(i32::MAX);
                    ordered_state_events.push(state_event);
                }

                if !ordered_state_events.is_empty() {
                    insert_into(room_state_events::table)
                        .values(ordered_state_events)
                        .execute(connection)?;
                }

                Ok::<(), diesel::result::Error>(())
            })
            .map_err(|error| match error {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    database_error_info,
                ) => {
                    let detail = database_error_info.details().unwrap_or_default().to_owned();
                    DomainError::InvalidRequest(format!("unique violation: {detail}"))
                }
                other => DomainError::InvalidRequest(other.to_string()),
            })?;

        Ok(())
    }

    fn fetch_join_context(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> Result<Option<RoomJoinContext>, DomainError> {
        use schema::{room_membership_projection, rooms};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = rooms::table
            .left_outer_join(
                room_membership_projection::table.on(room_membership_projection::room_id
                    .eq(rooms::room_id)
                    .and(room_membership_projection::user_id.eq(user_id))),
            )
            .filter(rooms::room_id.eq(room_id))
            .select((
                rooms::room_version,
                rooms::visibility,
                room_membership_projection::membership.nullable(),
            ))
            .first::<(Option<String>, Option<String>, Option<String>)>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.map(
            |(room_version, visibility, membership_state)| RoomJoinContext {
                room_version,
                visibility,
                membership_state,
            },
        ))
    }

    fn append_membership_event(
        &self,
        event_write_contract: &EventWriteContract,
    ) -> Result<(), DomainError> {
        let room_id = event_write_contract
            .event_rows
            .first()
            .map(|value| value.room_id.clone())
            .ok_or_else(|| DomainError::InvalidRequest("missing event row".to_owned()))?;

        let event_batch_write_contract = EventBatchWriteContract {
            room_id,
            room_version: event_write_contract
                .event_rows
                .first()
                .map(|value| value.room_version)
                .ok_or_else(|| DomainError::InvalidRequest("missing room version".to_owned()))?,
            event_write_contracts: vec![event_write_contract.clone()],
        };

        persist_event_batch(&self.connection_pool, &event_batch_write_contract)
    }
}

fn persist_event_batch(
    connection_pool: &r2d2::Pool<ConnectionManager<PgConnection>>,
    event_batch_write_contract: &EventBatchWriteContract,
) -> Result<(), DomainError> {
    use schema::{
        room_current_state, room_event_auth_edges, room_event_prev_edges, room_events,
        room_forward_extremities, room_idempotency_records, room_membership_projection,
        room_outbox_tasks, room_state_events, room_sync_stream, room_timeline_projection,
    };

    let mut connection = connection_pool
        .get()
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

    connection
        .transaction(|connection| {
            let now = Utc::now().naive_utc();
            let mut room_events_rows = Vec::new();
            let mut prev_edge_rows = Vec::new();
            let mut auth_edge_rows = Vec::new();
            let mut forward_extremity_rows = Vec::new();
            let mut current_state_rows = Vec::new();
            let mut membership_rows = Vec::new();
            let mut timeline_rows = Vec::new();
            let mut sync_rows = Vec::new();
            let mut outbox_rows = Vec::new();
            let mut idempotency_rows = Vec::new();

            for event_write_contract in &event_batch_write_contract.event_write_contracts {
                for event_row in &event_write_contract.event_rows {
                    room_events_rows.push(CreateRoomEventModel {
                        event_id: event_row.event_id.clone(),
                        room_id: event_row.room_id.clone(),
                        room_version: event_row.room_version.to_string(),
                        sender_user_id: event_row.sender.clone(),
                        event_type: event_row.event_type.clone(),
                        state_key: event_row.state_key.clone(),
                        depth: i64::try_from(event_row.depth).unwrap_or(i64::MAX),
                        origin_server_ts: i64::try_from(event_row.origin_server_ts)
                            .unwrap_or(i64::MAX),
                        redacts: None,
                        rejected: event_row.rejected,
                        soft_failed: event_row.soft_failed,
                        membership: extract_membership(&event_row.content),
                        join_rule: extract_join_rule(&event_row.content),
                        history_visibility: extract_history_visibility(&event_row.content),
                        guest_access: extract_guest_access(&event_row.content),
                        canonical_alias: extract_canonical_alias(&event_row.content),
                        room_name: extract_room_name(&event_row.content),
                        room_topic: extract_room_topic(&event_row.content),
                        is_direct: extract_is_direct(&event_row.content),
                        content_json: matrix_event_content_to_json(&event_row.content),
                        unsigned_json: event_row.unsigned.clone(),
                        hashes_json: serde_json::to_value(&event_row.hashes).unwrap_or_default(),
                        signatures_json: serde_json::to_value(&event_row.signatures)
                            .unwrap_or_default(),
                        created_at: now,
                    });
                }

                for edge in &event_write_contract.prev_edge_rows {
                    prev_edge_rows.push(CreateRoomEventPrevEdgeModel {
                        room_id: edge.room_id.clone(),
                        event_id: edge.event_id.clone(),
                        prev_event_id: edge.linked_event_id.clone(),
                        created_at: now,
                    });
                }
                for edge in &event_write_contract.auth_edge_rows {
                    auth_edge_rows.push(CreateRoomEventAuthEdgeModel {
                        room_id: edge.room_id.clone(),
                        event_id: edge.event_id.clone(),
                        auth_event_id: edge.linked_event_id.clone(),
                        created_at: now,
                    });
                }
                for forward in &event_write_contract.forward_extremity_updates {
                    forward_extremity_rows.push(CreateRoomForwardExtremityModel {
                        room_id: forward.room_id.clone(),
                        event_id: forward.event_id.clone(),
                        created_at: now,
                    });
                }
                for current in &event_write_contract.current_state_updates {
                    current_state_rows.push(CreateRoomCurrentStateModel {
                        room_id: current.room_id.clone(),
                        event_type: current.event_type.clone(),
                        state_key: current.state_key.clone(),
                        event_id: current.event_id.clone(),
                        updated_at: now,
                    });
                }
                for membership in &event_write_contract.membership_projection_updates {
                    membership_rows.push(CreateRoomMembershipProjectionModel {
                        room_id: membership.room_id.clone(),
                        user_id: membership.user_id.clone(),
                        membership: format!("{:?}", membership.membership).to_lowercase(),
                        event_id: membership.event_id.clone(),
                        updated_at: now,
                    });
                }
                for timeline in &event_write_contract.timeline_projection_updates {
                    timeline_rows.push(CreateRoomTimelineProjectionModel {
                        room_id: timeline.room_id.clone(),
                        stream_position: timeline.stream_position,
                        event_id: timeline.event_id.clone(),
                    });
                }
                for sync_row in &event_write_contract.sync_stream_rows {
                    sync_rows.push(CreateRoomSyncStreamModel {
                        room_id: sync_row.room_id.clone(),
                        event_id: sync_row.event_id.clone(),
                        created_at: now,
                    });
                }
                for outbox_task in &event_write_contract.outbox_tasks {
                    outbox_rows.push(CreateRoomOutboxTaskModel {
                        id: Uuid::new_v4(),
                        room_id: outbox_task.room_id.clone(),
                        event_id: outbox_task.event_id.clone(),
                        task_type: format!("{:?}", outbox_task.task_type).to_lowercase(),
                        status: "pending".to_owned(),
                        payload_json: None,
                        created_at: now,
                    });
                }
                for idempotency in &event_write_contract.idempotency_records {
                    idempotency_rows.push(CreateRoomIdempotencyRecordModel {
                        room_id: idempotency.room_id.clone(),
                        sender_user_id: idempotency.sender_user_id.clone(),
                        transaction_id: idempotency.transaction_id.clone(),
                        event_id: idempotency.event_id.clone(),
                        created_at: now,
                    });
                }
            }

            if !room_events_rows.is_empty() {
                insert_into(room_events::table)
                    .values(room_events_rows)
                    .execute(connection)?;
            }
            if !prev_edge_rows.is_empty() {
                insert_into(room_event_prev_edges::table)
                    .values(prev_edge_rows)
                    .execute(connection)?;
            }
            if !auth_edge_rows.is_empty() {
                insert_into(room_event_auth_edges::table)
                    .values(auth_edge_rows)
                    .execute(connection)?;
            }
            if !forward_extremity_rows.is_empty() {
                insert_into(room_forward_extremities::table)
                    .values(forward_extremity_rows)
                    .execute(connection)?;
            }
            if !current_state_rows.is_empty() {
                insert_into(room_current_state::table)
                    .values(current_state_rows)
                    .on_conflict((
                        room_current_state::room_id,
                        room_current_state::event_type,
                        room_current_state::state_key,
                    ))
                    .do_update()
                    .set((
                        room_current_state::event_id
                            .eq(diesel::upsert::excluded(room_current_state::event_id)),
                        room_current_state::updated_at
                            .eq(diesel::upsert::excluded(room_current_state::updated_at)),
                    ))
                    .execute(connection)?;
            }
            if !membership_rows.is_empty() {
                insert_into(room_membership_projection::table)
                    .values(membership_rows)
                    .on_conflict((
                        room_membership_projection::room_id,
                        room_membership_projection::user_id,
                    ))
                    .do_update()
                    .set((
                        room_membership_projection::membership.eq(diesel::upsert::excluded(
                            room_membership_projection::membership,
                        )),
                        room_membership_projection::event_id.eq(diesel::upsert::excluded(
                            room_membership_projection::event_id,
                        )),
                        room_membership_projection::updated_at.eq(diesel::upsert::excluded(
                            room_membership_projection::updated_at,
                        )),
                    ))
                    .execute(connection)?;
            }
            if !timeline_rows.is_empty() {
                insert_into(room_timeline_projection::table)
                    .values(timeline_rows)
                    .execute(connection)?;
            }
            if !sync_rows.is_empty() {
                insert_into(room_sync_stream::table)
                    .values(sync_rows)
                    .execute(connection)?;
            }
            if !outbox_rows.is_empty() {
                insert_into(room_outbox_tasks::table)
                    .values(outbox_rows)
                    .execute(connection)?;
            }
            if !idempotency_rows.is_empty() {
                insert_into(room_idempotency_records::table)
                    .values(idempotency_rows)
                    .on_conflict_do_nothing()
                    .execute(connection)?;
            }

            let mut latest_state_event_by_key =
                Vec::<((String, String, String), (serde_json::Value, usize))>::new();
            let mut state_event_sequence = 0usize;

            for event_write_contract in &event_batch_write_contract.event_write_contracts {
                for current_state_update in &event_write_contract.current_state_updates {
                    state_event_sequence = state_event_sequence.saturating_add(1);
                    if let Some(event_row) = event_write_contract
                        .event_rows
                        .iter()
                        .find(|event| event.event_id == current_state_update.event_id)
                    {
                        let key = (
                            current_state_update.room_id.clone(),
                            current_state_update.event_type.clone(),
                            current_state_update.state_key.clone(),
                        );
                        if let Some((_, value)) = latest_state_event_by_key
                            .iter_mut()
                            .find(|(existing_key, _)| *existing_key == key)
                        {
                            *value = (
                                matrix_event_content_to_json(&event_row.content),
                                state_event_sequence,
                            );
                        } else {
                            latest_state_event_by_key.push((
                                key,
                                (
                                    matrix_event_content_to_json(&event_row.content),
                                    state_event_sequence,
                                ),
                            ));
                        }
                    }
                }
            }

            let mut state_events = latest_state_event_by_key
                .into_iter()
                .map(|((room_id, event_type, state_key), (content, sequence))| {
                    (
                        sequence,
                        CreateRoomStateEventModel {
                            id: Uuid::new_v4(),
                            room_id,
                            event_type,
                            state_key,
                            content,
                            ordering: 0,
                            created_at: now,
                        },
                    )
                })
                .collect::<Vec<_>>();
            state_events.sort_by_key(|(sequence, _)| *sequence);

            let mut ordered_state_events = Vec::with_capacity(state_events.len());
            for (index, (_, mut state_event)) in state_events.into_iter().enumerate() {
                state_event.ordering = i32::try_from(index).unwrap_or(i32::MAX);
                ordered_state_events.push(state_event);
            }

            if !ordered_state_events.is_empty() {
                insert_into(room_state_events::table)
                    .values(ordered_state_events)
                    .execute(connection)?;
            }

            Ok::<(), diesel::result::Error>(())
        })
        .map_err(|error| match error {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                database_error_info,
            ) => {
                let detail = database_error_info.details().unwrap_or_default().to_owned();
                DomainError::InvalidRequest(format!("unique violation: {detail}"))
            }
            other => DomainError::InvalidRequest(other.to_string()),
        })?;

    Ok(())
}

fn extract_membership(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomMember(value) => Some(value.membership.as_str().to_owned()),
        _ => None,
    }
}

fn extract_join_rule(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomJoinRules { join_rule } => Some(join_rule.clone()),
        _ => None,
    }
}

fn extract_history_visibility(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomHistoryVisibility { history_visibility } => {
            Some(history_visibility.clone())
        }
        _ => None,
    }
}

fn extract_guest_access(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomGuestAccess { guest_access } => Some(guest_access.clone()),
        _ => None,
    }
}

fn extract_canonical_alias(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomCanonicalAlias { alias } => Some(alias.clone()),
        _ => None,
    }
}

fn extract_room_name(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomName { name } => Some(name.clone()),
        _ => None,
    }
}

fn extract_room_topic(content: &MatrixEventContent) -> Option<String> {
    match content {
        MatrixEventContent::RoomTopic { topic } => Some(topic.clone()),
        _ => None,
    }
}

fn extract_is_direct(content: &MatrixEventContent) -> Option<bool> {
    match content {
        MatrixEventContent::RoomMember(value) => value.is_direct,
        _ => None,
    }
}
