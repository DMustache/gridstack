use chrono::Utc;
use diesel::{
    BoolExpressionMethods, Connection, ExpressionMethods, JoinOnDsl, NullableExpressionMethods,
    OptionalExtension, QueryDsl, RunQueryDsl,
    dsl::{exists, select},
    insert_into,
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
            entities::{
                RoomCreationFlow, RoomEventFilter, RoomMessagesPage, RoomStateEvent,
                RoomTimelineEvent,
            },
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

#[derive(Clone, Debug)]
pub struct JoinedRoomMemberProfile {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub trait RoomCreationRepository: Send + Sync {
    fn create_room_with_initial_events(
        &self,
        room_creation_flow: &RoomCreationFlow,
        event_batch_write_contract: &EventBatchWriteContract,
    ) -> Result<(), DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomCreationRepository for T {
    fn create_room_with_initial_events(
        &self,
        room_creation_flow: &RoomCreationFlow,
        event_batch_write_contract: &EventBatchWriteContract,
    ) -> Result<(), DomainError> {
        <T as RoomRepository>::create_room_with_initial_events(
            self,
            room_creation_flow,
            event_batch_write_contract,
        )
    }
}

pub trait RoomMembershipRepository: Send + Sync {
    fn fetch_join_context(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> Result<Option<RoomJoinContext>, DomainError>;

    fn append_room_event(
        &self,
        event_write_contract: &EventWriteContract,
    ) -> Result<(), DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomMembershipRepository for T {
    fn fetch_join_context(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> Result<Option<RoomJoinContext>, DomainError> {
        <T as RoomRepository>::fetch_join_context(self, room_id, user_id)
    }

    fn append_room_event(
        &self,
        event_write_contract: &EventWriteContract,
    ) -> Result<(), DomainError> {
        <T as RoomRepository>::append_room_event(self, event_write_contract)
    }
}

pub trait RoomStateWriteRepository: RoomMembershipRepository + Send + Sync {
    fn fetch_room_id_by_alias_localpart(
        &self,
        alias_localpart: &str,
    ) -> Result<Option<String>, DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomStateWriteRepository for T {
    fn fetch_room_id_by_alias_localpart(
        &self,
        alias_localpart: &str,
    ) -> Result<Option<String>, DomainError> {
        <T as RoomRepository>::fetch_room_id_by_alias_localpart(self, alias_localpart)
    }
}

pub trait RoomMessageWriteRepository: RoomMembershipRepository + Send + Sync {
    fn fetch_room_event_id_by_transaction_id(
        &self,
        room_id: &str,
        sender_user_id: &str,
        event_type: &str,
        transaction_id: &str,
    ) -> Result<Option<String>, DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomMessageWriteRepository for T {
    fn fetch_room_event_id_by_transaction_id(
        &self,
        room_id: &str,
        sender_user_id: &str,
        event_type: &str,
        transaction_id: &str,
    ) -> Result<Option<String>, DomainError> {
        <T as RoomRepository>::fetch_room_event_id_by_transaction_id(
            self,
            room_id,
            sender_user_id,
            event_type,
            transaction_id,
        )
    }
}

pub trait RoomReceiptRepository: RoomMembershipRepository + Send + Sync {
    fn append_room_receipt(
        &self,
        room_id: &str,
        user_id: &str,
        receipt_type: &str,
        event_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), DomainError>;

    fn append_room_fully_read_marker(
        &self,
        room_id: &str,
        user_id: &str,
        event_id: &str,
    ) -> Result<(), DomainError>;

    fn room_event_matches_thread(
        &self,
        room_id: &str,
        event_id: &str,
        thread_id: &str,
    ) -> Result<bool, DomainError>;

    fn fetch_room_timeline_event_by_id(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<Option<RoomTimelineEvent>, DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomReceiptRepository for T {
    fn append_room_receipt(
        &self,
        room_id: &str,
        user_id: &str,
        receipt_type: &str,
        event_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), DomainError> {
        <T as RoomRepository>::append_room_receipt(
            self,
            room_id,
            user_id,
            receipt_type,
            event_id,
            thread_id,
        )
    }

    fn append_room_fully_read_marker(
        &self,
        room_id: &str,
        user_id: &str,
        event_id: &str,
    ) -> Result<(), DomainError> {
        <T as RoomRepository>::append_room_fully_read_marker(self, room_id, user_id, event_id)
    }

    fn room_event_matches_thread(
        &self,
        room_id: &str,
        event_id: &str,
        thread_id: &str,
    ) -> Result<bool, DomainError> {
        <T as RoomRepository>::room_event_matches_thread(self, room_id, event_id, thread_id)
    }

    fn fetch_room_timeline_event_by_id(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<Option<RoomTimelineEvent>, DomainError> {
        <T as RoomRepository>::fetch_room_timeline_event_by_id(self, room_id, event_id)
    }
}

pub trait RoomTimelineQueryRepository: Send + Sync {
    fn fetch_room_timeline_event_by_id(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<Option<RoomTimelineEvent>, DomainError>;

    fn fetch_room_history_visibility_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;

    fn fetch_room_history_visibility_before_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;

    fn fetch_user_membership_at_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;

    fn fetch_user_membership_before_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;

    fn user_joined_since_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<bool, DomainError>;

    fn fetch_room_timeline_events(
        &self,
        room_id: &str,
        from_stream_position: Option<i64>,
        to_stream_position: Option<i64>,
        limit: usize,
        backward: bool,
        filter: Option<RoomEventFilter>,
    ) -> Result<RoomMessagesPage, DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomTimelineQueryRepository for T {
    fn fetch_room_timeline_event_by_id(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<Option<RoomTimelineEvent>, DomainError> {
        <T as RoomRepository>::fetch_room_timeline_event_by_id(self, room_id, event_id)
    }

    fn fetch_room_history_visibility_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        <T as RoomRepository>::fetch_room_history_visibility_at_stream_position(
            self,
            room_id,
            stream_position,
        )
    }

    fn fetch_room_history_visibility_before_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        <T as RoomRepository>::fetch_room_history_visibility_before_stream_position(
            self,
            room_id,
            stream_position,
        )
    }

    fn fetch_user_membership_at_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        <T as RoomRepository>::fetch_user_membership_at_stream_position(
            self,
            room_id,
            user_id,
            stream_position,
        )
    }

    fn fetch_user_membership_before_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        <T as RoomRepository>::fetch_user_membership_before_stream_position(
            self,
            room_id,
            user_id,
            stream_position,
        )
    }

    fn user_joined_since_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<bool, DomainError> {
        <T as RoomRepository>::user_joined_since_stream_position(
            self,
            room_id,
            user_id,
            stream_position,
        )
    }

    fn fetch_room_timeline_events(
        &self,
        room_id: &str,
        from_stream_position: Option<i64>,
        to_stream_position: Option<i64>,
        limit: usize,
        backward: bool,
        filter: Option<RoomEventFilter>,
    ) -> Result<RoomMessagesPage, DomainError> {
        <T as RoomRepository>::fetch_room_timeline_events(
            self,
            room_id,
            from_stream_position,
            to_stream_position,
            limit,
            backward,
            filter,
        )
    }
}

pub trait RoomStateQueryRepository: RoomMembershipRepository + Send + Sync {
    fn fetch_room_state_events(&self, room_id: &str) -> Result<Vec<RoomStateEvent>, DomainError>;

    fn fetch_room_member_state_events_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Vec<RoomStateEvent>, DomainError>;

    fn fetch_room_state_event_by_type_and_key(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
    ) -> Result<Option<RoomStateEvent>, DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomStateQueryRepository for T {
    fn fetch_room_state_events(&self, room_id: &str) -> Result<Vec<RoomStateEvent>, DomainError> {
        <T as RoomRepository>::fetch_room_state_events(self, room_id)
    }

    fn fetch_room_member_state_events_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Vec<RoomStateEvent>, DomainError> {
        <T as RoomRepository>::fetch_room_member_state_events_at_stream_position(
            self,
            room_id,
            stream_position,
        )
    }

    fn fetch_room_state_event_by_type_and_key(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
    ) -> Result<Option<RoomStateEvent>, DomainError> {
        <T as RoomRepository>::fetch_room_state_event_by_type_and_key(
            self, room_id, event_type, state_key,
        )
    }
}

pub trait RoomJoinedMembersRepository: RoomMembershipRepository + Send + Sync {
    fn fetch_joined_members_profiles(
        &self,
        room_id: &str,
    ) -> Result<Vec<JoinedRoomMemberProfile>, DomainError>;
}

impl<T: RoomRepository + ?Sized> RoomJoinedMembersRepository for T {
    fn fetch_joined_members_profiles(
        &self,
        room_id: &str,
    ) -> Result<Vec<JoinedRoomMemberProfile>, DomainError> {
        <T as RoomRepository>::fetch_joined_members_profiles(self, room_id)
    }
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
    fn fetch_joined_room_ids_for_user(&self, user_id: &str) -> Result<Vec<String>, DomainError>;
    fn fetch_joined_members_profiles(
        &self,
        room_id: &str,
    ) -> Result<Vec<JoinedRoomMemberProfile>, DomainError>;

    fn append_room_event(
        &self,
        event_write_contract: &EventWriteContract,
    ) -> Result<(), DomainError>;
    fn append_room_receipt(
        &self,
        room_id: &str,
        user_id: &str,
        receipt_type: &str,
        event_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), DomainError>;
    fn append_room_fully_read_marker(
        &self,
        room_id: &str,
        user_id: &str,
        event_id: &str,
    ) -> Result<(), DomainError>;
    fn room_event_matches_thread(
        &self,
        room_id: &str,
        event_id: &str,
        thread_id: &str,
    ) -> Result<bool, DomainError>;

    fn fetch_room_event_id_by_transaction_id(
        &self,
        room_id: &str,
        sender_user_id: &str,
        event_type: &str,
        transaction_id: &str,
    ) -> Result<Option<String>, DomainError>;

    fn fetch_room_state_events(&self, room_id: &str) -> Result<Vec<RoomStateEvent>, DomainError>;
    fn fetch_room_member_state_events_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Vec<RoomStateEvent>, DomainError>;
    fn fetch_room_history_visibility_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;
    fn fetch_room_history_visibility_before_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;
    fn fetch_user_membership_at_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;
    fn fetch_user_membership_before_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError>;
    fn user_joined_since_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<bool, DomainError>;

    fn fetch_room_timeline_event_by_id(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<Option<RoomTimelineEvent>, DomainError>;

    fn fetch_room_state_event_by_type_and_key(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
    ) -> Result<Option<RoomStateEvent>, DomainError>;

    fn fetch_room_id_by_alias_localpart(
        &self,
        alias_localpart: &str,
    ) -> Result<Option<String>, DomainError>;

    fn fetch_room_timeline_events(
        &self,
        room_id: &str,
        from_stream_position: Option<i64>,
        to_stream_position: Option<i64>,
        limit: usize,
        backward: bool,
        filter: Option<RoomEventFilter>,
    ) -> Result<RoomMessagesPage, DomainError>;
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
            room_event_relations, room_events, room_forward_extremities, room_idempotency_records,
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
                let mut event_relation_rows = Vec::new();
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
                        if let Some((relation_type, related_event_id)) =
                            extract_event_relation(&event_row.content)
                        {
                            event_relation_rows.push((
                                event_row.room_id.clone(),
                                event_row.event_id.clone(),
                                relation_type,
                                related_event_id,
                            ));
                        }
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
                if !event_relation_rows.is_empty() {
                    for (room_id, event_id, relation_type, related_event_id) in event_relation_rows
                    {
                        insert_into(room_event_relations::table)
                            .values((
                                room_event_relations::room_id.eq(room_id),
                                room_event_relations::event_id.eq(event_id),
                                room_event_relations::rel_type.eq(relation_type),
                                room_event_relations::related_event_id.eq(related_event_id),
                            ))
                            .on_conflict_do_nothing()
                            .execute(connection)?;
                    }
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

    fn fetch_joined_room_ids_for_user(&self, user_id: &str) -> Result<Vec<String>, DomainError> {
        use schema::room_membership_projection;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        room_membership_projection::table
            .filter(room_membership_projection::user_id.eq(user_id))
            .filter(room_membership_projection::membership.eq("join"))
            .select(room_membership_projection::room_id)
            .order(room_membership_projection::room_id.asc())
            .load::<String>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))
    }

    fn fetch_joined_members_profiles(
        &self,
        room_id: &str,
    ) -> Result<Vec<JoinedRoomMemberProfile>, DomainError> {
        use schema::{room_events, room_membership_projection};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let rows = room_membership_projection::table
            .inner_join(
                room_events::table
                    .on(room_events::event_id.eq(room_membership_projection::event_id)),
            )
            .filter(room_membership_projection::room_id.eq(room_id))
            .filter(room_membership_projection::membership.eq("join"))
            .order(room_membership_projection::user_id.asc())
            .select((
                room_membership_projection::user_id,
                room_events::content_json,
            ))
            .load::<(String, serde_json::Value)>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(user_id, content)| {
                let display_name = content
                    .get("displayname")
                    .or_else(|| content.get("display_name"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                let avatar_url = content
                    .get("avatar_url")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);

                JoinedRoomMemberProfile {
                    user_id,
                    display_name,
                    avatar_url,
                }
            })
            .collect())
    }

    fn append_room_event(
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

    fn append_room_receipt(
        &self,
        room_id: &str,
        user_id: &str,
        receipt_type: &str,
        event_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), DomainError> {
        use schema::{room_events, room_receipts};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let event_exists = select(exists(
            room_events::table
                .filter(room_events::room_id.eq(room_id))
                .filter(room_events::event_id.eq(event_id)),
        ))
        .get_result::<bool>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        if !event_exists {
            return Err(DomainError::NotFound);
        }

        insert_into(room_receipts::table)
            .values((
                room_receipts::room_id.eq(room_id),
                room_receipts::user_id.eq(user_id),
                room_receipts::receipt_type.eq(receipt_type),
                room_receipts::event_id.eq(event_id),
                room_receipts::thread_id.eq(thread_id.unwrap_or_default()),
                room_receipts::receipt_ts.eq(Utc::now().timestamp_millis()),
            ))
            .execute(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(())
    }

    fn append_room_fully_read_marker(
        &self,
        room_id: &str,
        user_id: &str,
        event_id: &str,
    ) -> Result<(), DomainError> {
        use schema::{room_events, room_read_markers};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let event_exists = select(exists(
            room_events::table
                .filter(room_events::room_id.eq(room_id))
                .filter(room_events::event_id.eq(event_id)),
        ))
        .get_result::<bool>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        if !event_exists {
            return Err(DomainError::NotFound);
        }

        insert_into(room_read_markers::table)
            .values((
                room_read_markers::room_id.eq(room_id),
                room_read_markers::user_id.eq(user_id),
                room_read_markers::fully_read_event_id.eq(event_id),
                room_read_markers::updated_at.eq(Utc::now()),
            ))
            .execute(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(())
    }

    fn room_event_matches_thread(
        &self,
        room_id: &str,
        event_id: &str,
        thread_id: &str,
    ) -> Result<bool, DomainError> {
        use schema::room_event_relations;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        if thread_id == "main" {
            let has_thread_relation = select(exists(
                room_event_relations::table
                    .filter(room_event_relations::room_id.eq(room_id))
                    .filter(room_event_relations::event_id.eq(event_id))
                    .filter(room_event_relations::rel_type.eq("m.thread")),
            ))
            .get_result::<bool>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
            return Ok(!has_thread_relation);
        }

        if event_id == thread_id {
            return Ok(true);
        }

        select(exists(
            room_event_relations::table
                .filter(room_event_relations::room_id.eq(room_id))
                .filter(room_event_relations::event_id.eq(event_id))
                .filter(room_event_relations::rel_type.eq("m.thread"))
                .filter(room_event_relations::related_event_id.eq(thread_id)),
        ))
        .get_result::<bool>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))
    }

    fn fetch_room_event_id_by_transaction_id(
        &self,
        room_id: &str,
        sender_user_id: &str,
        event_type: &str,
        transaction_id: &str,
    ) -> Result<Option<String>, DomainError> {
        use schema::{room_events, room_idempotency_records};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        room_idempotency_records::table
            .inner_join(
                room_events::table.on(room_events::event_id.eq(room_idempotency_records::event_id)),
            )
            .filter(room_idempotency_records::room_id.eq(room_id))
            .filter(room_idempotency_records::sender_user_id.eq(sender_user_id))
            .filter(room_events::event_type.eq(event_type))
            .filter(room_idempotency_records::transaction_id.eq(transaction_id))
            .select(room_idempotency_records::event_id)
            .first::<String>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))
    }

    fn fetch_room_state_events(&self, room_id: &str) -> Result<Vec<RoomStateEvent>, DomainError> {
        use schema::{room_current_state, room_events};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let rows = room_current_state::table
            .inner_join(
                room_events::table.on(room_events::event_id.eq(room_current_state::event_id)),
            )
            .filter(room_current_state::room_id.eq(room_id))
            .order((
                room_current_state::event_type.asc(),
                room_current_state::state_key.asc(),
            ))
            .select((
                room_events::content_json,
                room_events::event_id,
                room_events::origin_server_ts,
                room_events::room_id,
                room_events::sender_user_id,
                room_events::state_key.nullable(),
                room_events::event_type,
                room_events::unsigned_json,
            ))
            .load::<(
                serde_json::Value,
                String,
                i64,
                String,
                String,
                Option<String>,
                String,
                Option<serde_json::Value>,
            )>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    content,
                    event_id,
                    origin_server_ts,
                    room_id,
                    sender,
                    state_key,
                    event_type,
                    unsigned,
                )| {
                    RoomStateEvent {
                        content,
                        event_id,
                        origin_server_ts,
                        room_id,
                        sender,
                        state_key: state_key.unwrap_or_default(),
                        event_type,
                        unsigned,
                    }
                },
            )
            .collect())
    }

    fn fetch_room_member_state_events_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Vec<RoomStateEvent>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let rows = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_id))
            .filter(room_sync_stream::stream_position.le(stream_position))
            .filter(room_events::event_type.eq("m.room.member"))
            .filter(room_events::state_key.is_not_null())
            .distinct_on(room_events::state_key)
            .order((
                room_events::state_key.asc(),
                room_sync_stream::stream_position.desc(),
            ))
            .select((
                room_events::content_json,
                room_events::event_id,
                room_events::origin_server_ts,
                room_events::room_id,
                room_events::sender_user_id,
                room_events::state_key,
                room_events::event_type,
                room_events::unsigned_json,
            ))
            .load::<(
                serde_json::Value,
                String,
                i64,
                String,
                String,
                Option<String>,
                String,
                Option<serde_json::Value>,
            )>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    content,
                    event_id,
                    origin_server_ts,
                    room_id,
                    sender,
                    state_key,
                    event_type,
                    unsigned,
                )| RoomStateEvent {
                    content,
                    event_id,
                    origin_server_ts,
                    room_id,
                    sender,
                    state_key: state_key.unwrap_or_default(),
                    event_type,
                    unsigned,
                },
            )
            .collect())
    }

    fn fetch_room_history_visibility_at_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_id))
            .filter(room_sync_stream::stream_position.le(stream_position))
            .filter(room_events::event_type.eq("m.room.history_visibility"))
            .filter(room_events::state_key.eq(""))
            .order(room_sync_stream::stream_position.desc())
            .select(room_events::history_visibility)
            .first::<Option<String>>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.flatten())
    }

    fn fetch_room_history_visibility_before_stream_position(
        &self,
        room_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_id))
            .filter(room_sync_stream::stream_position.lt(stream_position))
            .filter(room_events::event_type.eq("m.room.history_visibility"))
            .filter(room_events::state_key.eq(""))
            .order(room_sync_stream::stream_position.desc())
            .select(room_events::history_visibility)
            .first::<Option<String>>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.flatten())
    }

    fn fetch_user_membership_at_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_id))
            .filter(room_sync_stream::stream_position.le(stream_position))
            .filter(room_events::event_type.eq("m.room.member"))
            .filter(room_events::state_key.eq(user_id))
            .order(room_sync_stream::stream_position.desc())
            .select(room_events::membership)
            .first::<Option<String>>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.flatten())
    }

    fn fetch_user_membership_before_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<Option<String>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_id))
            .filter(room_sync_stream::stream_position.lt(stream_position))
            .filter(room_events::event_type.eq("m.room.member"))
            .filter(room_events::state_key.eq(user_id))
            .order(room_sync_stream::stream_position.desc())
            .select(room_events::membership)
            .first::<Option<String>>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.flatten())
    }

    fn user_joined_since_stream_position(
        &self,
        room_id: &str,
        user_id: &str,
        stream_position: i64,
    ) -> Result<bool, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let found = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_id))
            .filter(room_sync_stream::stream_position.gt(stream_position))
            .filter(room_events::event_type.eq("m.room.member"))
            .filter(room_events::state_key.eq(user_id))
            .filter(room_events::membership.eq("join"))
            .select(room_sync_stream::stream_position)
            .first::<i64>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(found.is_some())
    }

    fn fetch_room_timeline_event_by_id(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<Option<RoomTimelineEvent>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = room_events::table
            .inner_join(
                room_sync_stream::table.on(room_sync_stream::event_id
                    .eq(room_events::event_id)
                    .and(room_sync_stream::room_id.eq(room_events::room_id))),
            )
            .filter(room_events::room_id.eq(room_id))
            .filter(room_events::event_id.eq(event_id))
            .select((
                room_events::content_json,
                room_events::event_id,
                room_events::origin_server_ts,
                room_events::room_id,
                room_events::sender_user_id,
                room_events::state_key,
                room_events::event_type,
                room_events::unsigned_json,
                room_sync_stream::stream_position,
            ))
            .first::<(
                serde_json::Value,
                String,
                i64,
                String,
                String,
                Option<String>,
                String,
                Option<serde_json::Value>,
                i64,
            )>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.map(
            |(
                content,
                event_id,
                origin_server_ts,
                room_id,
                sender,
                state_key,
                event_type,
                unsigned,
                stream_position,
            )| RoomTimelineEvent {
                content,
                event_id,
                origin_server_ts,
                room_id,
                sender,
                state_key,
                event_type,
                unsigned,
                stream_position,
            },
        ))
    }

    fn fetch_room_state_event_by_type_and_key(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
    ) -> Result<Option<RoomStateEvent>, DomainError> {
        use schema::{room_current_state, room_events};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let row = room_current_state::table
            .inner_join(
                room_events::table.on(room_events::event_id.eq(room_current_state::event_id)),
            )
            .filter(room_current_state::room_id.eq(room_id))
            .filter(room_current_state::event_type.eq(event_type))
            .filter(room_current_state::state_key.eq(state_key))
            .select((
                room_events::content_json,
                room_events::event_id,
                room_events::origin_server_ts,
                room_events::room_id,
                room_events::sender_user_id,
                room_events::state_key.nullable(),
                room_events::event_type,
                room_events::unsigned_json,
            ))
            .first::<(
                serde_json::Value,
                String,
                i64,
                String,
                String,
                Option<String>,
                String,
                Option<serde_json::Value>,
            )>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(row.map(
            |(
                content,
                event_id,
                origin_server_ts,
                room_id,
                sender,
                state_key,
                event_type,
                unsigned,
            )| {
                RoomStateEvent {
                    content,
                    event_id,
                    origin_server_ts,
                    room_id,
                    sender,
                    state_key: state_key.unwrap_or_default(),
                    event_type,
                    unsigned,
                }
            },
        ))
    }

    fn fetch_room_id_by_alias_localpart(
        &self,
        alias_localpart: &str,
    ) -> Result<Option<String>, DomainError> {
        use schema::room_aliases;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        room_aliases::table
            .filter(room_aliases::alias_localpart.eq(alias_localpart))
            .select(room_aliases::room_id)
            .first::<String>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))
    }

    fn fetch_room_timeline_events(
        &self,
        room_id: &str,
        from_stream_position: Option<i64>,
        to_stream_position: Option<i64>,
        limit: usize,
        backward: bool,
        filter: Option<RoomEventFilter>,
    ) -> Result<RoomMessagesPage, DomainError> {
        use schema::{room_events, room_timeline_projection};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let room_max_stream_position = room_timeline_projection::table
            .filter(room_timeline_projection::room_id.eq(room_id))
            .select(diesel::dsl::max(room_timeline_projection::stream_position))
            .first::<Option<i64>>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?
            .unwrap_or(0);

        let start_stream_position = from_stream_position.unwrap_or(if backward {
            room_max_stream_position
        } else {
            0
        });
        let query_limit = i64::try_from(limit).unwrap_or(i64::MAX);

        let mut query = room_timeline_projection::table
            .inner_join(
                room_events::table.on(room_events::event_id.eq(room_timeline_projection::event_id)),
            )
            .filter(room_timeline_projection::room_id.eq(room_id))
            .into_boxed();

        if backward {
            query =
                query.filter(room_timeline_projection::stream_position.le(start_stream_position));
            if let Some(to_stream_position) = to_stream_position {
                query =
                    query.filter(room_timeline_projection::stream_position.gt(to_stream_position));
            }
            query = query.order(room_timeline_projection::stream_position.desc());
        } else {
            query =
                query.filter(room_timeline_projection::stream_position.ge(start_stream_position));
            if let Some(to_stream_position) = to_stream_position {
                query =
                    query.filter(room_timeline_projection::stream_position.lt(to_stream_position));
            }
            query = query.order(room_timeline_projection::stream_position.asc());
        }

        if let Some(filter) = filter {
            if let Some(types) = filter.types
                && !types.is_empty()
            {
                query = query.filter(room_events::event_type.eq_any(types));
            }
            if let Some(not_types) = filter.not_types
                && !not_types.is_empty()
            {
                query = query.filter(room_events::event_type.ne_all(not_types));
            }
            if let Some(senders) = filter.senders
                && !senders.is_empty()
            {
                query = query.filter(room_events::sender_user_id.eq_any(senders));
            }
            if let Some(not_senders) = filter.not_senders
                && !not_senders.is_empty()
            {
                query = query.filter(room_events::sender_user_id.ne_all(not_senders));
            }
            if let Some(contains_url) = filter.contains_url {
                query = if contains_url {
                    query.filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                        "room_events.content_json ? 'url'",
                    ))
                } else {
                    query.filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                        "NOT (room_events.content_json ? 'url')",
                    ))
                };
            }
        }

        let rows = query
            .limit(query_limit)
            .select((
                room_events::content_json,
                room_events::event_id,
                room_events::origin_server_ts,
                room_events::room_id,
                room_events::sender_user_id,
                room_events::state_key,
                room_events::event_type,
                room_events::unsigned_json,
                room_timeline_projection::stream_position,
            ))
            .load::<(
                serde_json::Value,
                String,
                i64,
                String,
                String,
                Option<String>,
                String,
                Option<serde_json::Value>,
                i64,
            )>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let chunk = rows
            .into_iter()
            .map(
                |(
                    content,
                    event_id,
                    origin_server_ts,
                    room_id,
                    sender,
                    state_key,
                    event_type,
                    unsigned,
                    stream_position,
                )| RoomTimelineEvent {
                    content,
                    event_id,
                    origin_server_ts,
                    room_id,
                    sender,
                    state_key,
                    event_type,
                    unsigned,
                    stream_position,
                },
            )
            .collect::<Vec<_>>();

        let end = chunk.last().map(|value| {
            if backward {
                value.stream_position.saturating_sub(1).to_string()
            } else {
                value.stream_position.saturating_add(1).to_string()
            }
        });

        Ok(RoomMessagesPage {
            start: start_stream_position.to_string(),
            end,
            chunk,
            state: Vec::new(),
        })
    }
}

fn persist_event_batch(
    connection_pool: &r2d2::Pool<ConnectionManager<PgConnection>>,
    event_batch_write_contract: &EventBatchWriteContract,
) -> Result<(), DomainError> {
    use schema::{
        room_current_state, room_event_auth_edges, room_event_prev_edges, room_event_relations,
        room_events, room_forward_extremities, room_idempotency_records,
        room_membership_projection, room_outbox_tasks, room_state_events, room_sync_stream,
        room_timeline_projection,
    };

    let mut connection = connection_pool
        .get()
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

    connection
        .transaction(|connection| {
            let now = Utc::now().naive_utc();
            let mut room_events_rows = Vec::new();
            let mut event_relation_rows = Vec::new();
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
                    if let Some((relation_type, related_event_id)) =
                        extract_event_relation(&event_row.content)
                    {
                        event_relation_rows.push((
                            event_row.room_id.clone(),
                            event_row.event_id.clone(),
                            relation_type,
                            related_event_id,
                        ));
                    }
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
            if !event_relation_rows.is_empty() {
                for (room_id, event_id, relation_type, related_event_id) in event_relation_rows {
                    insert_into(room_event_relations::table)
                        .values((
                            room_event_relations::room_id.eq(room_id),
                            room_event_relations::event_id.eq(event_id),
                            room_event_relations::rel_type.eq(relation_type),
                            room_event_relations::related_event_id.eq(related_event_id),
                        ))
                        .on_conflict_do_nothing()
                        .execute(connection)?;
                }
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

fn extract_event_relation(content: &MatrixEventContent) -> Option<(String, String)> {
    match content {
        MatrixEventContent::RoomMessage {
            thread_root_event_id: Some(thread_root_event_id),
            ..
        } => Some(("m.thread".to_owned(), thread_root_event_id.clone())),
        MatrixEventContent::CustomJson(content) => {
            let relation_object = content.get("m.relates_to")?.as_object()?;
            let relation_type = relation_object.get("rel_type")?.as_str()?.trim();
            let related_event_id = relation_object.get("event_id")?.as_str()?.trim();
            if relation_type.is_empty() || related_event_id.is_empty() {
                return None;
            }
            Some((relation_type.to_owned(), related_event_id.to_owned()))
        }
        _ => None,
    }
}
