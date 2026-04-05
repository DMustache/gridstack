use axum::{
    Extension, Json, Router, extract::State, middleware::from_fn_with_state, routing::post,
};
use models::error::{AppError, AppResult};
use ruma::{
    api::client::error::ErrorKind,
    exports::serde_json::{Map, Value, json},
};

use crate::{
    services::{
        authorization::layers::{AuthenticatedUserExtension, require_authenticated_user},
        messages::{self, AppendRoomEventCommand, CreateRoomCommand},
    },
    state::ServerState,
};

mod data_transfer_objects;
use data_transfer_objects::{BodyInfo, BodyView, StateEvent};

const DEFAULT_ROOM_VERSION: &str = "12";
const PRIVATE_CHAT_PRESET: &str = "private_chat";
const PUBLIC_CHAT_PRESET: &str = "public_chat";
const TRUSTED_PRIVATE_CHAT_PRESET: &str = "trusted_private_chat";

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .route("/", post(post_create_room))
        .route_layer(from_fn_with_state(
            state.clone(),
            require_authenticated_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/_matrix/client/v3/createRoom",
    tag = "matrix-client",
    security(("access_token" = [])),
    request_body = crate::docs::CreateRoomRequestDoc,
    responses(
        (status = 200, description = "Created room", body = crate::docs::CreateRoomResponseDoc),
        (status = 400, description = "Invalid create room request", body = crate::docs::MatrixErrorResponse),
        (status = 403, description = "Create room forbidden", body = crate::docs::MatrixErrorResponse)
    )
)]
pub(crate) async fn post_create_room(
    State(state): State<ServerState>,
    Extension(user): AuthenticatedUserExtension,
    Json(body): Json<BodyInfo>,
) -> AppResult<Json<BodyView>> {
    ensure_supported_request(&body, &state, &user.user_id).await?;

    let room_version = resolve_room_version(body.room_version.as_deref())?;
    let room = messages::create_room(
        &state,
        CreateRoomCommand {
            creator_user_id: user.user_id.clone(),
            room_id: None,
            room_version: Some(room_version.clone()),
        },
    )
    .await?;

    append_implied_events(&state, &user.user_id, &room.room_id, &room_version, &body).await?;

    Ok(Json(BodyView {
        room_id: room.room_id,
    }))
}

async fn ensure_supported_request(
    body: &BodyInfo,
    state: &ServerState,
    creator_user_id: &str,
) -> AppResult<()> {
    if body
        .invite_3pid
        .as_ref()
        .is_some_and(|invites| !invites.is_empty())
    {
        return Err(AppError::bad_request(
            ErrorKind::Unknown,
            "invite_3pid is not supported yet",
        ));
    }

    if let Some(invites) = &body.invite {
        for invitee in invites {
            if invitee == creator_user_id {
                continue;
            }

            if !state.users.exists(invitee.clone()).await? {
                return Err(AppError::forbidden(format!(
                    "Inviting unknown user is not supported: {invitee}"
                )));
            }
        }
    }

    Ok(())
}

fn resolve_room_version(room_version: Option<&str>) -> AppResult<String> {
    let room_version = room_version.unwrap_or(DEFAULT_ROOM_VERSION);

    if (1..=12).any(|version| room_version == version.to_string()) {
        return Ok(room_version.to_owned());
    }

    Err(AppError::bad_request(
        ErrorKind::UnsupportedRoomVersion,
        "Unsupported room version",
    ))
}

async fn append_implied_events(
    state: &ServerState,
    creator_user_id: &str,
    room_id: &str,
    room_version: &str,
    body: &BodyInfo,
) -> AppResult<()> {
    append_state_event(
        state,
        room_id,
        creator_user_id,
        "m.room.create",
        String::new(),
        build_create_content(creator_user_id, room_version, body),
    )
    .await?;

    append_state_event(
        state,
        room_id,
        creator_user_id,
        "m.room.member",
        creator_user_id.to_owned(),
        json!({ "membership": "join" }),
    )
    .await?;

    append_state_event(
        state,
        room_id,
        creator_user_id,
        "m.room.power_levels",
        String::new(),
        build_power_levels_content(creator_user_id, room_version, body),
    )
    .await?;

    if let Some(room_alias_name) = &body.room_alias_name {
        append_state_event(
            state,
            room_id,
            creator_user_id,
            "m.room.canonical_alias",
            String::new(),
            json!({
                "alias": format!("#{room_alias_name}:{}", state.server_name),
            }),
        )
        .await?;
    }

    for preset_event in preset_state_events(body.preset.as_deref(), body.visibility.as_deref()) {
        append_state_event(
            state,
            room_id,
            creator_user_id,
            &preset_event.event_type,
            preset_event.state_key.unwrap_or_default(),
            preset_event.content,
        )
        .await?;
    }

    if let Some(initial_state) = &body.initial_state {
        for state_event in initial_state {
            append_state_event(
                state,
                room_id,
                creator_user_id,
                &state_event.event_type,
                state_event.state_key.clone().unwrap_or_default(),
                state_event.content.clone(),
            )
            .await?;
        }
    }

    if let Some(name) = &body.name {
        append_state_event(
            state,
            room_id,
            creator_user_id,
            "m.room.name",
            String::new(),
            json!({ "name": name }),
        )
        .await?;
    }

    if let Some(topic) = &body.topic {
        append_state_event(
            state,
            room_id,
            creator_user_id,
            "m.room.topic",
            String::new(),
            json!({ "topic": topic }),
        )
        .await?;
    }

    if let Some(invites) = &body.invite {
        for invitee in invites {
            append_state_event(
                state,
                room_id,
                creator_user_id,
                "m.room.member",
                invitee.clone(),
                build_invite_content(body.is_direct.unwrap_or(false)),
            )
            .await?;
        }
    }

    Ok(())
}

async fn append_state_event(
    state: &ServerState,
    room_id: &str,
    sender_user_id: &str,
    event_type: &str,
    state_key: String,
    content: Value,
) -> AppResult<()> {
    messages::append_room_event(
        state,
        AppendRoomEventCommand {
            room_id: room_id.to_owned(),
            sender_user_id: sender_user_id.to_owned(),
            event_type: event_type.to_owned(),
            state_key: Some(state_key),
            content,
            unsigned: None,
            transaction_id: None,
        },
    )
    .await?;

    Ok(())
}

fn build_create_content(creator_user_id: &str, room_version: &str, body: &BodyInfo) -> Value {
    let mut content = body.creation_content.clone().unwrap_or_else(|| json!({}));
    let object = ensure_object(&mut content);
    object.insert(String::from("creator"), json!(creator_user_id));
    object.insert(String::from("room_version"), json!(room_version));

    if matches!(body.preset.as_deref(), Some(TRUSTED_PRIVATE_CHAT_PRESET)) {
        let additional_creators = body
            .invite
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        if !additional_creators.is_empty() {
            object.insert(
                String::from("additional_creators"),
                Value::Array(additional_creators.into_iter().map(Value::String).collect()),
            );
        }
    }

    content
}

fn build_power_levels_content(creator_user_id: &str, room_version: &str, body: &BodyInfo) -> Value {
    let mut content = json!({
        "ban": 50,
        "events": {
            "m.room.name": 50,
            "m.room.power_levels": 100,
            "m.room.history_visibility": 100,
            "m.room.canonical_alias": 50,
            "m.room.avatar": 50,
            "m.room.tombstone": if is_room_version_12_or_higher(room_version) { 150 } else { 100 },
        },
        "events_default": 0,
        "invite": 0,
        "kick": 50,
        "notifications": {
            "room": 50
        },
        "redact": 50,
        "state_default": 50,
        "users_default": 0,
    });

    let object = ensure_object(&mut content);
    if !is_room_version_12_or_higher(room_version) {
        object.insert(String::from("users"), json!({ creator_user_id: 100 }));
    }

    if matches!(body.preset.as_deref(), Some(TRUSTED_PRIVATE_CHAT_PRESET)) {
        let users = object
            .entry(String::from("users"))
            .or_insert_with(|| json!({}));
        let users_object = ensure_object(users);
        for invitee in body.invite.clone().unwrap_or_default() {
            users_object.insert(invitee, json!(100));
        }
    }

    if let Some(override_content) = &body.power_level_content_override {
        merge_json(object, override_content);
    }

    content
}

fn preset_state_events(preset: Option<&str>, visibility: Option<&str>) -> Vec<StateEvent> {
    let preset = preset.unwrap_or(match visibility {
        Some("public") => PUBLIC_CHAT_PRESET,
        _ => PRIVATE_CHAT_PRESET,
    });

    match preset {
        PUBLIC_CHAT_PRESET => vec![
            state_event("m.room.join_rules", json!({ "join_rule": "public" })),
            state_event(
                "m.room.history_visibility",
                json!({ "history_visibility": "shared" }),
            ),
            state_event(
                "m.room.guest_access",
                json!({ "guest_access": "forbidden" }),
            ),
        ],
        TRUSTED_PRIVATE_CHAT_PRESET | PRIVATE_CHAT_PRESET => vec![
            state_event("m.room.join_rules", json!({ "join_rule": "invite" })),
            state_event(
                "m.room.history_visibility",
                json!({ "history_visibility": "shared" }),
            ),
            state_event("m.room.guest_access", json!({ "guest_access": "can_join" })),
        ],
        _ => vec![
            state_event("m.room.join_rules", json!({ "join_rule": "invite" })),
            state_event(
                "m.room.history_visibility",
                json!({ "history_visibility": "shared" }),
            ),
            state_event("m.room.guest_access", json!({ "guest_access": "can_join" })),
        ],
    }
}

fn build_invite_content(is_direct: bool) -> Value {
    if is_direct {
        json!({
            "membership": "invite",
            "is_direct": true,
        })
    } else {
        json!({ "membership": "invite" })
    }
}

fn state_event(event_type: &str, content: Value) -> StateEvent {
    StateEvent {
        event_type: event_type.to_owned(),
        state_key: Some(String::new()),
        content,
    }
}

fn is_room_version_12_or_higher(room_version: &str) -> bool {
    room_version
        .parse::<u16>()
        .map(|version| version >= 12)
        .unwrap_or(false)
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = json!({});
    }

    value
        .as_object_mut()
        .expect("value was converted to object")
}

fn merge_json(target: &mut Map<String, Value>, source: &Value) {
    let Some(source_object) = source.as_object() else {
        return;
    };

    for (key, value) in source_object {
        target.insert(key.clone(), value.clone());
    }
}
