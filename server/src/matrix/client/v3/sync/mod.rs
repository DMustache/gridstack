use std::collections::BTreeMap;

use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    middleware::from_fn_with_state,
    routing::get,
};
use models::error::AppResult;
use persistence::rooms::RoomEvent;
use ruma::exports::serde_json::{Map, Value, json};

use crate::{
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        messages,
    },
    state::ServerState,
};

mod data_transfer_objects;
use data_transfer_objects::{
    BodyView, EventsSection, JoinedRoomView, QueryParameters, RoomSummaryView, RoomsView,
    TimelineView, UnreadNotificationsView,
};

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", get(get_sync))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/v3/sync",
    tag = "matrix-client",
    security(("access_token" = [])),
    params(
        ("filter" = Option<String>, Query, description = "Filter id or inline filter JSON"),
        ("full_state" = Option<bool>, Query, description = "Return full room state"),
        ("set_presence" = Option<String>, Query, description = "Presence override"),
        ("since" = Option<String>, Query, description = "Sync continuation token"),
        ("timeout" = Option<u64>, Query, description = "Long-poll timeout in milliseconds"),
        ("use_state_after" = Option<bool>, Query, description = "Return state_after instead of state")
    ),
    responses(
        (status = 200, description = "Initial sync or delta", body = crate::docs::SyncResponseDoc),
        (status = 403, description = "Authentication required", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn get_sync(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Query(query): Query<QueryParameters>,
) -> AppResult<Json<BodyView>> {
    let since = parse_since_token(query.since.as_deref());
    let full_state = query.full_state.unwrap_or(false) || since.is_none();
    let use_state_after = query.use_state_after.unwrap_or(false);
    let _filter = query.filter;
    let _set_presence = query.set_presence;
    let _timeout = query.timeout;

    let joined_rooms = state.rooms.list_joined_rooms(user.user_id.clone()).await?;
    let mut join = BTreeMap::new();

    for membership in joined_rooms {
        let timeline_events =
            messages::get_room_timeline(&state, membership.room_id.clone(), since, 20).await?;
        let state_events = if full_state {
            messages::get_current_state(&state, membership.room_id.clone()).await?
        } else if since.is_some() {
            filter_state_events(
                messages::get_current_state(&state, membership.room_id.clone()).await?,
                since,
            )
        } else {
            Vec::new()
        };

        let joined_member_count = state
            .rooms
            .count_members_by_membership(membership.room_id.clone(), String::from("join"))
            .await?;
        let invited_member_count = state
            .rooms
            .count_members_by_membership(membership.room_id.clone(), String::from("invite"))
            .await?;
        let heroes = state
            .rooms
            .list_hero_user_ids(membership.room_id.clone(), user.user_id.clone(), 5)
            .await?;

        let state_section = EventsSection {
            events: state_events
                .iter()
                .map(|event| room_event_to_sync_event(event, &membership.membership))
                .collect(),
        };
        let timeline_section = TimelineView {
            prev_batch: timeline_events
                .first()
                .map(|event| format!("s{}", event.stream_ordering.saturating_sub(1))),
            limited: false,
            events: timeline_events
                .iter()
                .map(|event| room_event_to_sync_event(event, &membership.membership))
                .collect(),
        };

        join.insert(
            membership.room_id.clone(),
            JoinedRoomView {
                timeline: timeline_section,
                state: (!use_state_after).then_some(state_section.clone()),
                state_after: use_state_after.then_some(state_section),
                account_data: EventsSection::default(),
                ephemeral: EventsSection::default(),
                summary: RoomSummaryView {
                    heroes,
                    joined_member_count,
                    invited_member_count,
                },
                unread_notifications: UnreadNotificationsView {
                    highlight_count: 0,
                    notification_count: 0,
                },
            },
        );
    }

    let next_batch = format!(
        "s{}",
        state
            .rooms
            .get_latest_stream_ordering()
            .await?
            .unwrap_or_default()
    );

    Ok(Json(BodyView {
        next_batch,
        account_data: EventsSection::default(),
        presence: EventsSection::default(),
        rooms: RoomsView {
            join,
            ..RoomsView::default()
        },
        to_device: EventsSection::default(),
        device_lists: BTreeMap::new(),
        device_one_time_keys_count: BTreeMap::new(),
    }))
}

fn parse_since_token(token: Option<&str>) -> Option<i64> {
    let token = token?;
    token.strip_prefix('s')?.parse().ok()
}

fn filter_state_events(events: Vec<RoomEvent>, since: Option<i64>) -> Vec<RoomEvent> {
    let Some(since) = since else {
        return events;
    };

    events
        .into_iter()
        .filter(|event| event.stream_ordering > since)
        .collect()
}

fn room_event_to_sync_event(event: &RoomEvent, membership: &str) -> Value {
    let age = (chrono::Utc::now() - event.origin_server_ts)
        .num_milliseconds()
        .max(0);

    let mut unsigned = event.unsigned.clone().unwrap_or_else(|| json!({}));
    if let Some(unsigned_object) = unsigned.as_object_mut() {
        unsigned_object.insert(String::from("age"), json!(age));
        unsigned_object.insert(String::from("membership"), json!(membership));
    }

    let mut object = Map::new();
    object.insert(String::from("content"), event.content.clone());
    object.insert(String::from("event_id"), json!(event.event_id));
    object.insert(
        String::from("origin_server_ts"),
        json!(event.origin_server_ts.timestamp_millis()),
    );
    object.insert(String::from("sender"), json!(event.sender_user_id));
    object.insert(String::from("type"), json!(event.event_type));
    object.insert(String::from("unsigned"), unsigned);

    if let Some(state_key) = &event.state_key {
        object.insert(String::from("state_key"), json!(state_key));
    }

    Value::Object(object)
}
