use crate::services::events::content_mapper::matrix_event_content_to_json;
use crate::services::events::entities::{
    CurrentStateUpdate, EventGraphEdge, EventIntent, EventJsonRow, EventWriteContract,
    ForwardExtremityUpdate, IdempotencyRecord, MatrixEventContent, MembershipProjectionUpdate,
    MembershipState, OutboxTask, OutboxTaskType, PersistedEvent, RoomSummaryProjectionKind,
    RoomSummaryProjectionUpdate, SyncStreamRow, TimelineProjectionUpdate,
};

use super::CompilationState;

pub(super) struct EventWriteContractBuilder<'a> {
    event_intent: &'a EventIntent,
    persisted_event: &'a PersistedEvent,
    prev_events: Vec<String>,
    auth_events: Vec<String>,
    compilation_state: &'a mut CompilationState,
}

impl<'a> EventWriteContractBuilder<'a> {
    pub(super) fn new(
        event_intent: &'a EventIntent,
        persisted_event: &'a PersistedEvent,
        prev_events: Vec<String>,
        auth_events: Vec<String>,
        compilation_state: &'a mut CompilationState,
    ) -> Self {
        Self {
            event_intent,
            persisted_event,
            prev_events,
            auth_events,
            compilation_state,
        }
    }

    pub(super) fn build(self) -> EventWriteContract {
        let prev_edge_rows = build_edge_rows(self.persisted_event, self.prev_events);
        let auth_edge_rows = build_edge_rows(self.persisted_event, self.auth_events);

        let mut current_state_updates = Vec::new();
        let mut membership_projection_updates = Vec::new();
        let mut timeline_projection_updates = Vec::new();
        let mut room_summary_projection_updates = Vec::new();

        if let Some(state_key) = self.persisted_event.state_key.as_ref() {
            current_state_updates.push(CurrentStateUpdate {
                room_id: self.persisted_event.room_id.clone(),
                event_type: self.persisted_event.event_type.clone(),
                state_key: state_key.clone(),
                event_id: self.persisted_event.event_id.clone(),
            });
            self.compilation_state.set_current_state_event_id(
                self.persisted_event.event_type.clone(),
                state_key.clone(),
                self.persisted_event.event_id.clone(),
            );

            if self.persisted_event.event_type == "m.room.member"
                && let Some(membership_state) =
                    membership_state_from_content(&self.persisted_event.content)
            {
                membership_projection_updates.push(MembershipProjectionUpdate {
                    room_id: self.persisted_event.room_id.clone(),
                    user_id: state_key.clone(),
                    membership: membership_state,
                    event_id: self.persisted_event.event_id.clone(),
                });
            }

            if let Some(summary_kind) =
                summary_projection_kind_from_event_type(&self.persisted_event.event_type)
            {
                room_summary_projection_updates.push(RoomSummaryProjectionUpdate {
                    room_id: self.persisted_event.room_id.clone(),
                    event_id: self.persisted_event.event_id.clone(),
                    projection_kind: summary_kind,
                });
            }
        } else {
            self.compilation_state.stream_position += 1;
            timeline_projection_updates.push(TimelineProjectionUpdate {
                room_id: self.persisted_event.room_id.clone(),
                event_id: self.persisted_event.event_id.clone(),
                stream_position: self.compilation_state.stream_position,
            });
        }

        self.compilation_state
            .accepted_event_ids
            .push(self.persisted_event.event_id.clone());
        self.compilation_state.set_event_depth(
            self.persisted_event.event_id.clone(),
            self.persisted_event.depth,
        );

        if self.persisted_event.state_key.is_some() {
            self.compilation_state.stream_position += 1;
        }

        let mut idempotency_records = Vec::new();
        if let Some(transaction_id) = self.event_intent.transaction_id.as_ref() {
            idempotency_records.push(IdempotencyRecord {
                room_id: self.persisted_event.room_id.clone(),
                sender_user_id: self.persisted_event.sender.clone(),
                transaction_id: transaction_id.clone(),
                event_id: self.persisted_event.event_id.clone(),
            });
        }

        EventWriteContract {
            event_rows: vec![self.persisted_event.clone()],
            event_json_rows: vec![EventJsonRow {
                event_id: self.persisted_event.event_id.clone(),
                canonical_json: matrix_event_content_to_json(&self.persisted_event.content),
            }],
            prev_edge_rows,
            auth_edge_rows,
            forward_extremity_updates: vec![ForwardExtremityUpdate {
                room_id: self.persisted_event.room_id.clone(),
                event_id: self.persisted_event.event_id.clone(),
            }],
            current_state_updates,
            membership_projection_updates,
            timeline_projection_updates,
            room_summary_projection_updates,
            sync_stream_rows: vec![SyncStreamRow {
                room_id: self.persisted_event.room_id.clone(),
                event_id: self.persisted_event.event_id.clone(),
                stream_position: self.compilation_state.stream_position,
            }],
            outbox_tasks: vec![
                OutboxTask {
                    room_id: self.persisted_event.room_id.clone(),
                    event_id: self.persisted_event.event_id.clone(),
                    task_type: OutboxTaskType::NotifyLocalUsers,
                },
                OutboxTask {
                    room_id: self.persisted_event.room_id.clone(),
                    event_id: self.persisted_event.event_id.clone(),
                    task_type: OutboxTaskType::WakeSyncWaiters,
                },
            ],
            idempotency_records,
        }
    }
}

fn build_edge_rows(
    persisted_event: &PersistedEvent,
    linked_event_ids: Vec<String>,
) -> Vec<EventGraphEdge> {
    let mut rows = Vec::with_capacity(linked_event_ids.len());
    for linked_event_id in linked_event_ids {
        rows.push(EventGraphEdge {
            room_id: persisted_event.room_id.clone(),
            event_id: persisted_event.event_id.clone(),
            linked_event_id,
        });
    }
    rows
}

fn summary_projection_kind_from_event_type(event_type: &str) -> Option<RoomSummaryProjectionKind> {
    match event_type {
        "m.room.name" => Some(RoomSummaryProjectionKind::RoomName),
        "m.room.topic" => Some(RoomSummaryProjectionKind::RoomTopic),
        "m.room.canonical_alias" => Some(RoomSummaryProjectionKind::CanonicalAlias),
        "m.room.join_rules" => Some(RoomSummaryProjectionKind::JoinRule),
        "m.room.history_visibility" => Some(RoomSummaryProjectionKind::HistoryVisibility),
        "m.room.guest_access" => Some(RoomSummaryProjectionKind::GuestAccess),
        "m.room.power_levels" => Some(RoomSummaryProjectionKind::PowerLevels),
        _ => None,
    }
}

fn membership_state_from_content(content: &MatrixEventContent) -> Option<MembershipState> {
    let MatrixEventContent::RoomMember(room_member_content) = content else {
        return None;
    };
    let membership = room_member_content.membership.as_str();
    match membership {
        "join" => Some(MembershipState::Join),
        "invite" => Some(MembershipState::Invite),
        "leave" => Some(MembershipState::Leave),
        "ban" => Some(MembershipState::Ban),
        "knock" => Some(MembershipState::Knock),
        _ => None,
    }
}
