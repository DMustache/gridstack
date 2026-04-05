use chrono::Utc;
use models::error::AppResult;
use persistence::rooms::{
    NewRoom, NewRoomEvent, NewRoomMembership, NewRoomState, Room, RoomEvent, RoomMembership,
};
use ruma::exports::serde_json::Value;
use ruma::{EventId, RoomId};
use uuid::Uuid;

use crate::state::ServerState;

pub(crate) const ROOM_MEMBERSHIP_JOIN: &str = "join";

#[derive(Debug, Clone)]
pub(crate) struct CreateRoomCommand {
    pub creator_user_id: String,
    pub room_id: Option<String>,
    pub room_version: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AppendRoomEventCommand {
    pub room_id: String,
    pub sender_user_id: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: Value,
    pub unsigned: Option<Value>,
    pub transaction_id: Option<String>,
}

pub(crate) async fn create_room(
    state: &ServerState,
    command: CreateRoomCommand,
) -> AppResult<Room> {
    let room = state
        .rooms
        .create_room(NewRoom {
            room_id: command.room_id.unwrap_or_else(|| generate_room_id(state)),
            creator_user_id: command.creator_user_id.clone(),
            room_version: command.room_version.unwrap_or_else(|| String::from("11")),
        })
        .await?;

    state
        .rooms
        .upsert_membership(NewRoomMembership {
            room_id: room.room_id.clone(),
            user_id: command.creator_user_id,
            membership: ROOM_MEMBERSHIP_JOIN.to_owned(),
            membership_event_id: None,
        })
        .await?;

    Ok(room)
}

pub(crate) async fn append_room_event(
    state: &ServerState,
    command: AppendRoomEventCommand,
) -> AppResult<RoomEvent> {
    let depth = state
        .rooms
        .get_latest_event_depth(command.room_id.clone())
        .await?
        .unwrap_or_default()
        + 1;

    let event = state
        .rooms
        .append_event(NewRoomEvent {
            event_id: generate_event_id(state),
            room_id: command.room_id.clone(),
            sender_user_id: command.sender_user_id.clone(),
            event_type: command.event_type.clone(),
            state_key: command.state_key.clone(),
            content: command.content.clone(),
            unsigned: command.unsigned,
            origin_server_ts: Utc::now(),
            depth,
            transaction_id: command.transaction_id,
        })
        .await?;

    if let Some(state_key) = command.state_key {
        state
            .rooms
            .upsert_state(NewRoomState {
                room_id: command.room_id,
                event_type: command.event_type.clone(),
                state_key: state_key.clone(),
                event_id: event.event_id.clone(),
            })
            .await?;

        if command.event_type == "m.room.member" {
            if let Some(membership) = command.content.get("membership").and_then(Value::as_str) {
                state
                    .rooms
                    .upsert_membership(NewRoomMembership {
                        room_id: event.room_id.clone(),
                        user_id: state_key,
                        membership: String::from(membership),
                        membership_event_id: Some(event.event_id.clone()),
                    })
                    .await?;
            }
        }
    }

    Ok(event)
}

pub(crate) async fn get_room_timeline(
    state: &ServerState,
    room_id: String,
    from_stream_ordering: Option<i64>,
    limit: i64,
) -> AppResult<Vec<RoomEvent>> {
    Ok(state
        .rooms
        .list_events(room_id, from_stream_ordering, limit)
        .await?)
}

pub(crate) async fn get_current_state(
    state: &ServerState,
    room_id: String,
) -> AppResult<Vec<RoomEvent>> {
    Ok(state.rooms.list_current_state(room_id).await?)
}

pub(crate) async fn get_membership(
    state: &ServerState,
    room_id: String,
    user_id: String,
) -> AppResult<Option<RoomMembership>> {
    Ok(state.rooms.get_membership(room_id, user_id).await?)
}

pub(crate) async fn get_event_by_transaction_id(
    state: &ServerState,
    room_id: String,
    sender_user_id: String,
    transaction_id: String,
) -> AppResult<Option<RoomEvent>> {
    Ok(state
        .rooms
        .get_event_by_transaction_id(room_id, sender_user_id, transaction_id)
        .await?)
}

pub(crate) async fn get_state_event(
    state: &ServerState,
    room_id: String,
    event_type: String,
    state_key: String,
) -> AppResult<Option<RoomEvent>> {
    Ok(state
        .rooms
        .get_state_event(room_id, event_type, state_key)
        .await?)
}

pub(crate) async fn list_joined_rooms(
    state: &ServerState,
    user_id: String,
) -> AppResult<Vec<RoomMembership>> {
    Ok(state.rooms.list_joined_rooms(user_id).await?)
}

fn generate_room_id(state: &ServerState) -> String {
    let room_id = format!("!{}:{}", Uuid::new_v4().simple(), state.server_name);
    RoomId::parse(&room_id)
        .map(|room_id| room_id.to_string())
        .unwrap_or(room_id)
}

fn generate_event_id(state: &ServerState) -> String {
    let event_id = format!("${}:{}", Uuid::new_v4().simple(), state.server_name);
    EventId::parse(&event_id)
        .map(|event_id| event_id.to_string())
        .unwrap_or(event_id)
}
