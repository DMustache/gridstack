use chrono::NaiveDateTime;
use diesel::Insertable;
use serde_json::Value;
use uuid::Uuid;

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::rooms)]
pub struct CreateRoomModel {
    pub room_id: String,
    pub room_version: Option<String>,
    pub creator_user_id: String,
    pub is_direct: Option<bool>,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub visibility: String,
    pub preset: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_aliases)]
pub struct CreateRoomAliasModel {
    pub alias_localpart: String,
    pub room_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_state_events)]
pub struct CreateRoomStateEventModel {
    pub id: Uuid,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub content: Value,
    pub ordering: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_events)]
pub struct CreateRoomEventModel {
    pub event_id: String,
    pub room_id: String,
    pub room_version: String,
    pub sender_user_id: String,
    pub event_type: String,
    pub state_key: Option<String>,
    pub depth: i64,
    pub origin_server_ts: i64,
    pub redacts: Option<String>,
    pub rejected: bool,
    pub soft_failed: bool,
    pub membership: Option<String>,
    pub join_rule: Option<String>,
    pub history_visibility: Option<String>,
    pub guest_access: Option<String>,
    pub canonical_alias: Option<String>,
    pub room_name: Option<String>,
    pub room_topic: Option<String>,
    pub is_direct: Option<bool>,
    pub content_json: Value,
    pub unsigned_json: Option<Value>,
    pub hashes_json: Value,
    pub signatures_json: Value,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_event_prev_edges)]
pub struct CreateRoomEventPrevEdgeModel {
    pub room_id: String,
    pub event_id: String,
    pub prev_event_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_event_auth_edges)]
pub struct CreateRoomEventAuthEdgeModel {
    pub room_id: String,
    pub event_id: String,
    pub auth_event_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_forward_extremities)]
pub struct CreateRoomForwardExtremityModel {
    pub room_id: String,
    pub event_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_current_state)]
pub struct CreateRoomCurrentStateModel {
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub event_id: String,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_membership_projection)]
pub struct CreateRoomMembershipProjectionModel {
    pub room_id: String,
    pub user_id: String,
    pub membership: String,
    pub event_id: String,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_timeline_projection)]
pub struct CreateRoomTimelineProjectionModel {
    pub room_id: String,
    pub stream_position: i64,
    pub event_id: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_sync_stream)]
pub struct CreateRoomSyncStreamModel {
    pub room_id: String,
    pub event_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_outbox_tasks)]
pub struct CreateRoomOutboxTaskModel {
    pub id: Uuid,
    pub room_id: String,
    pub event_id: String,
    pub task_type: String,
    pub status: String,
    pub payload_json: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::infrastructure::schema::room_idempotency_records)]
pub struct CreateRoomIdempotencyRecordModel {
    pub room_id: String,
    pub sender_user_id: String,
    pub transaction_id: String,
    pub event_id: String,
    pub created_at: NaiveDateTime,
}
