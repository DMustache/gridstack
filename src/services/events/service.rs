use std::collections::HashMap;

use thiserror::Error;
use uuid::Uuid;

use crate::infrastructure::server_name::ServerName;
use crate::services::events::entities::{
    CurrentStateUpdate, EventBatchWriteContract, EventDraft, EventGraphEdge, EventIntent,
    EventWriteContract, MembershipProjectionUpdate, MembershipState, OutboxTask, OutboxTaskType,
    PersistedEvent, RoomEventIntentPlan, RoomVersionRegistry, SyncStreamUpdate,
    TimelineProjectionUpdate,
};

#[derive(Clone)]
pub struct EventsService {
    room_version_registry: RoomVersionRegistry,
    server_name: ServerName,
}

impl EventsService {
    pub fn new(server_name: &ServerName) -> Self {
        Self {
            room_version_registry: RoomVersionRegistry,
            server_name: server_name.clone(),
        }
    }

    pub fn compile_event_batch(
        &self,
        room_event_intent_plan: RoomEventIntentPlan,
    ) -> Result<EventBatchWriteContract, EventCompilationError> {
        if room_event_intent_plan.intents.is_empty() {
            return Err(EventCompilationError::EmptyIntentPlan);
        }
        let room_version_definition = self
            .room_version_registry
            .resolve(room_event_intent_plan.room_version)
            .ok_or(EventCompilationError::UnsupportedRoomVersion {
                room_version: room_event_intent_plan.room_version.to_string(),
            })?;

        let mut event_write_contracts = Vec::with_capacity(room_event_intent_plan.intents.len());
        let mut accepted_event_ids =
            Vec::<String>::with_capacity(room_event_intent_plan.intents.len());
        let mut current_state = HashMap::<(String, String), String>::with_capacity(
            room_event_intent_plan.intents.len(),
        );
        let mut depth_by_event_id =
            HashMap::<String, u64>::with_capacity(room_event_intent_plan.intents.len());
        let mut stream_position: i64 = 0;

        for (event_index, event_intent) in room_event_intent_plan.intents.iter().enumerate() {
            self.validate_event_intent(event_intent)?;

            let prev_events = accepted_event_ids
                .last()
                .map(|event_id| vec![event_id.clone()])
                .unwrap_or_default();
            let auth_events = self.select_auth_events(event_intent, &current_state);
            let depth = prev_events
                .first()
                .and_then(|event_id| depth_by_event_id.get(event_id).copied())
                .unwrap_or(0)
                + 1;
            let event_draft = EventDraft {
                room_id: event_intent.room_id.clone(),
                room_version: room_event_intent_plan.room_version,
                sender: event_intent.sender.clone(),
                event_type: event_intent.event_type.clone(),
                state_key: event_intent.state_key.clone(),
                content: event_intent.content.clone(),
                unsigned: event_intent.unsigned.clone(),
                prev_events: prev_events.clone(),
                auth_events: auth_events.clone(),
                depth,
                origin_server_ts: chrono::Utc::now().timestamp_millis() as u64,
                redacts: None,
            };

            self.authorize_event(
                event_index,
                &event_draft,
                &room_event_intent_plan.intents,
                &current_state,
            )?;

            let event_id = format!("${}:{}", Uuid::new_v4().simple(), self.server_name.as_str());
            accepted_event_ids.push(event_id.clone());
            depth_by_event_id.insert(event_id.clone(), depth);
            stream_position += 1;

            let persisted_event = PersistedEvent {
                event_id: event_id.clone(),
                room_id: event_draft.room_id.clone(),
                room_version: event_draft.room_version,
                sender: event_draft.sender.clone(),
                event_type: event_draft.event_type.clone(),
                state_key: event_draft.state_key.clone(),
                content: event_draft.content.clone(),
                unsigned: event_draft.unsigned.clone(),
                prev_events: prev_events.clone(),
                auth_events: auth_events.clone(),
                depth,
                origin_server_ts: event_draft.origin_server_ts,
                hashes: HashMap::from([("sha256".to_owned(), Uuid::new_v4().simple().to_string())]),
                signatures: HashMap::from([(
                    self.server_name.as_str().to_owned(),
                    HashMap::from([(
                        "ed25519:auto".to_owned(),
                        Uuid::new_v4().simple().to_string(),
                    )]),
                )]),
                rejected: false,
                soft_failed: false,
            };
            let mut prev_edges_to_insert = Vec::with_capacity(prev_events.len());
            for prev_event_id in prev_events {
                prev_edges_to_insert.push(EventGraphEdge {
                    room_id: persisted_event.room_id.clone(),
                    event_id: persisted_event.event_id.clone(),
                    linked_event_id: prev_event_id,
                });
            }
            let mut auth_edges_to_insert = Vec::with_capacity(auth_events.len());
            for auth_event_id in auth_events {
                auth_edges_to_insert.push(EventGraphEdge {
                    room_id: persisted_event.room_id.clone(),
                    event_id: persisted_event.event_id.clone(),
                    linked_event_id: auth_event_id,
                });
            }

            let mut current_state_updates = Vec::new();
            let mut membership_projection_updates = Vec::new();
            let mut timeline_projection_updates = Vec::new();
            if let Some(state_key) = persisted_event.state_key.clone() {
                current_state_updates.push(CurrentStateUpdate {
                    room_id: persisted_event.room_id.clone(),
                    event_type: persisted_event.event_type.clone(),
                    state_key: state_key.clone(),
                    event_id: persisted_event.event_id.clone(),
                });
                current_state.insert(
                    (persisted_event.event_type.clone(), state_key.clone()),
                    persisted_event.event_id.clone(),
                );
                if persisted_event.event_type == "m.room.member"
                    && let Some(membership_state) =
                        membership_state_from_content(&persisted_event.content)
                {
                    membership_projection_updates.push(MembershipProjectionUpdate {
                        room_id: persisted_event.room_id.clone(),
                        user_id: state_key,
                        membership: membership_state,
                        event_id: persisted_event.event_id.clone(),
                    });
                }
            } else {
                timeline_projection_updates.push(TimelineProjectionUpdate {
                    room_id: persisted_event.room_id.clone(),
                    event_id: persisted_event.event_id.clone(),
                    stream_position,
                });
            }

            event_write_contracts.push(EventWriteContract {
                events_to_insert: vec![persisted_event.clone()],
                prev_edges_to_insert,
                auth_edges_to_insert,
                forward_extremity_updates: vec![persisted_event.event_id.clone()],
                current_state_updates,
                membership_projection_updates,
                timeline_projection_updates,
                sync_stream_updates: vec![SyncStreamUpdate {
                    room_id: persisted_event.room_id.clone(),
                    event_id: persisted_event.event_id.clone(),
                    stream_position,
                }],
                outbox_tasks: vec![
                    OutboxTask {
                        room_id: persisted_event.room_id.clone(),
                        event_id: persisted_event.event_id.clone(),
                        task_type: OutboxTaskType::NotifyLocalUsers,
                    },
                    OutboxTask {
                        room_id: persisted_event.room_id.clone(),
                        event_id: persisted_event.event_id.clone(),
                        task_type: OutboxTaskType::WakeSyncWaiters,
                    },
                ],
                idempotency_records: Vec::new(),
            });
        }

        let _ = room_version_definition;
        Ok(EventBatchWriteContract {
            room_id: room_event_intent_plan.room_id,
            room_version: room_event_intent_plan.room_version,
            event_write_contracts,
        })
    }

    fn validate_event_intent(
        &self,
        event_intent: &EventIntent,
    ) -> Result<(), EventCompilationError> {
        if event_intent.event_type.trim().is_empty() {
            return Err(EventCompilationError::InvalidEventType);
        }
        if !event_intent.content.is_object() {
            return Err(EventCompilationError::InvalidEventContentShape {
                event_type: event_intent.event_type.clone(),
            });
        }
        if event_intent.event_type.starts_with("m.room.")
            && event_intent.state_key.is_none()
            && event_intent.event_type != "m.room.message"
        {
            return Err(EventCompilationError::MissingStateKeyForStateEvent {
                event_type: event_intent.event_type.clone(),
            });
        }

        Ok(())
    }

    fn select_auth_events(
        &self,
        event_intent: &EventIntent,
        current_state: &HashMap<(String, String), String>,
    ) -> Vec<String> {
        let mut auth_events = Vec::new();
        if let Some(create_event_id) = current_state
            .get(&("m.room.create".to_owned(), String::new()))
            .cloned()
        {
            auth_events.push(create_event_id);
        }
        if let Some(power_levels_event_id) = current_state
            .get(&("m.room.power_levels".to_owned(), String::new()))
            .cloned()
        {
            auth_events.push(power_levels_event_id);
        }
        if let Some(sender_membership_event_id) = current_state
            .get(&("m.room.member".to_owned(), event_intent.sender.clone()))
            .cloned()
        {
            auth_events.push(sender_membership_event_id);
        }

        auth_events
    }

    fn authorize_event(
        &self,
        event_index: usize,
        event_draft: &EventDraft,
        full_intent_sequence: &[EventIntent],
        current_state: &HashMap<(String, String), String>,
    ) -> Result<(), EventCompilationError> {
        if event_index == 0 {
            if event_draft.event_type != "m.room.create"
                || event_draft.state_key.as_deref() != Some("")
            {
                return Err(EventCompilationError::CreateEventMustBeFirst);
            }
            return Ok(());
        }

        if event_index == 1 {
            let first_intent_sender = full_intent_sequence
                .first()
                .map(|event_intent| event_intent.sender.as_str())
                .ok_or(EventCompilationError::EmptyIntentPlan)?;
            if event_draft.event_type != "m.room.member"
                || event_draft.state_key.as_deref() != Some(first_intent_sender)
            {
                return Err(EventCompilationError::CreatorJoinMustBeSecond);
            }
            return Ok(());
        }

        let sender_membership_event = current_state
            .get(&("m.room.member".to_owned(), event_draft.sender.clone()))
            .cloned();
        if sender_membership_event.is_none() {
            return Err(EventCompilationError::MissingSenderMembershipForAuth {
                sender: event_draft.sender.clone(),
                event_type: event_draft.event_type.clone(),
            });
        }

        Ok(())
    }
}

fn membership_state_from_content(content: &serde_json::Value) -> Option<MembershipState> {
    let membership = content.get("membership")?.as_str()?;
    match membership {
        "join" => Some(MembershipState::Join),
        "invite" => Some(MembershipState::Invite),
        "leave" => Some(MembershipState::Leave),
        "ban" => Some(MembershipState::Ban),
        "knock" => Some(MembershipState::Knock),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum EventCompilationError {
    #[error("event intent plan is empty")]
    EmptyIntentPlan,
    #[error("unsupported room version `{room_version}` for event compilation")]
    UnsupportedRoomVersion { room_version: String },
    #[error("event type cannot be empty")]
    InvalidEventType,
    #[error("event `{event_type}` must have object content")]
    InvalidEventContentShape { event_type: String },
    #[error("state event `{event_type}` must include state_key")]
    MissingStateKeyForStateEvent { event_type: String },
    #[error("m.room.create with empty state_key must be first event in a room")]
    CreateEventMustBeFirst,
    #[error("creator join membership must be second event in a room creation sequence")]
    CreatorJoinMustBeSecond,
    #[error("missing joined membership for sender `{sender}` when authorizing `{event_type}`")]
    MissingSenderMembershipForAuth { sender: String, event_type: String },
}
