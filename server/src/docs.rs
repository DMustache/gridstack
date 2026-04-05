use std::collections::BTreeMap;

use axum::{Json, Router, routing::get};
use ruma::exports::serde_json::Value;
use serde::{Deserialize, Serialize};
use utoipa::{
    Modify, OpenApi, ToSchema,
    openapi::{
        Components,
        security::{Http, HttpAuthScheme, SecurityScheme},
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::matrix::client::versions::get_versions,
        crate::matrix::client::register::post_register,
        crate::matrix::client::v3::login::get_login,
        crate::matrix::client::v3::login::post_login,
        crate::matrix::client::v3::logout::post_logout,
        crate::matrix::client::v3::logout::all::post_logout_all,
        crate::matrix::client::v3::account::whoami::get_whoami,
        crate::matrix::client::v3::sync::get_sync,
        crate::matrix::client::v3::create_room::post_create_room,
        crate::matrix::client::v3::joined_rooms::get_joined_rooms,
        crate::matrix::client::v3::join::post_join,
        crate::matrix::client::v3::rooms::invite::post_invite,
        crate::matrix::client::v3::rooms::join::post_join_room,
        crate::matrix::client::v3::rooms::leave::post_leave_room,
        crate::matrix::client::v3::rooms::messages::get_messages,
        crate::matrix::client::v3::rooms::send::put_send_event,
        crate::matrix::client::v3::rooms::state::get_room_state,
        crate::matrix::client::v3::rooms::state::get_room_state_event_with_empty_state_key,
        crate::matrix::client::v3::rooms::state::get_room_state_event
    ),
    components(
        schemas(
            MatrixErrorResponse,
            RateLimitErrorResponse,
            EmptyResponse,
            VersionsResponseDoc,
            RegistrationRequestDoc,
            RegistrationResponseDoc,
            RegistrationUiaaResponseDoc,
            AuthFlowDoc,
            GetLoginResponseDoc,
            LoginFlowDoc,
            LoginRequestDoc,
            LoginIdentifierDoc,
            LoginResponseDoc,
            WellKnownDoc,
            HomeserverInfoDoc,
            WhoAmIResponseDoc,
            SyncResponseDoc,
            CreateRoomRequestDoc,
            CreateRoomResponseDoc,
            CreateRoomStateEventDoc,
            Invite3pidDoc,
            JoinedRoomsResponseDoc,
            JoinRequestDoc,
            JoinResponseDoc,
            InviteRequestDoc,
            LeaveRequestDoc,
            RoomMessagesResponseDoc,
            SendEventResponseDoc,
            ClientEventDoc
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "matrix-client", description = "Implemented Matrix client-server endpoints")
    )
)]
pub struct ApiDoc;

pub(crate) fn router() -> Router<crate::ServerState> {
    Router::new().route("/docs/openapi.json", get(get_openapi))
}

async fn get_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "access_token",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatrixErrorResponse {
    pub errcode: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitErrorResponse {
    pub errcode: String,
    pub error: String,
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmptyResponse {}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionsResponseDoc {
    pub unstable_features: BTreeMap<String, bool>,
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistrationRequestDoc {
    pub auth: Option<BTreeMap<String, Value>>,
    pub device_id: Option<String>,
    pub inhibit_login: Option<bool>,
    pub initial_device_display_name: Option<String>,
    pub password: Option<String>,
    pub refresh_token: Option<bool>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistrationResponseDoc {
    pub access_token: Option<String>,
    pub device_id: Option<String>,
    pub expires_in_ms: Option<i32>,
    pub home_server: Option<String>,
    pub refresh_token: Option<String>,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistrationUiaaResponseDoc {
    pub completed: Vec<String>,
    pub flows: Vec<AuthFlowDoc>,
    pub params: BTreeMap<String, Value>,
    pub session: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthFlowDoc {
    pub stages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetLoginResponseDoc {
    pub flows: Vec<LoginFlowDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginFlowDoc {
    #[serde(rename = "type")]
    pub login_type: String,
    pub get_login_token: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequestDoc {
    pub address: Option<String>,
    pub device_id: Option<String>,
    pub identifier: Option<LoginIdentifierDoc>,
    pub initial_device_display_name: Option<String>,
    pub medium: Option<String>,
    pub password: Option<String>,
    pub refresh_token: Option<bool>,
    pub token: Option<String>,
    #[serde(rename = "type")]
    pub login_type: String,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginIdentifierDoc {
    #[serde(rename = "type")]
    pub identifier_type: String,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponseDoc {
    pub access_token: String,
    pub device_id: String,
    pub expires_in_ms: Option<u64>,
    pub home_server: Option<String>,
    pub refresh_token: Option<String>,
    pub user_id: String,
    pub well_known: Option<WellKnownDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WellKnownDoc {
    #[serde(rename = "m.homeserver")]
    pub homeserver: HomeserverInfoDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HomeserverInfoDoc {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WhoAmIResponseDoc {
    pub device_id: String,
    pub user_id: String,
    pub is_guest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncResponseDoc {
    pub next_batch: String,
    pub account_data: Option<BTreeMap<String, Value>>,
    pub presence: Option<BTreeMap<String, Value>>,
    pub rooms: Option<BTreeMap<String, Value>>,
    pub to_device: Option<BTreeMap<String, Value>>,
    pub device_lists: Option<BTreeMap<String, Value>>,
    pub device_one_time_keys_count: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoomRequestDoc {
    pub creation_content: Option<BTreeMap<String, Value>>,
    pub initial_state: Option<Vec<CreateRoomStateEventDoc>>,
    pub invite: Option<Vec<String>>,
    pub invite_3pid: Option<Vec<Invite3pidDoc>>,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub power_level_content_override: Option<BTreeMap<String, Value>>,
    pub preset: Option<String>,
    pub room_alias_name: Option<String>,
    pub room_version: Option<String>,
    pub topic: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoomStateEventDoc {
    #[serde(rename = "type")]
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Invite3pidDoc {
    pub address: String,
    pub id_access_token: String,
    pub id_server: String,
    pub medium: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoomResponseDoc {
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinedRoomsResponseDoc {
    pub joined_rooms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinRequestDoc {
    pub reason: Option<String>,
    pub third_party_signed: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinResponseDoc {
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InviteRequestDoc {
    pub reason: Option<String>,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaveRequestDoc {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoomMessagesResponseDoc {
    pub chunk: Vec<ClientEventDoc>,
    pub start: String,
    pub end: Option<String>,
    pub state: Vec<ClientEventDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SendEventResponseDoc {
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientEventDoc {
    pub content: BTreeMap<String, Value>,
    pub event_id: String,
    pub origin_server_ts: i64,
    pub room_id: String,
    pub sender: String,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub unsigned: Option<BTreeMap<String, Value>>,
}
