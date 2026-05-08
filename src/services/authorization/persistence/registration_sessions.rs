use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde_json::Value;
use uuid::Uuid;

use crate::infrastructure::schema::registration_sessions;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = registration_sessions)]
#[diesel(primary_key(session_id))]
pub(super) struct RegistrationSessionModel {
    pub session_id: String,
    pub account_id: Uuid,
    pub completed_stages: Value,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = registration_sessions)]
pub(super) struct CreateRegistrationSessionModel {
    pub session_id: String,
    pub account_id: Uuid,
    pub completed_stages: Value,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
