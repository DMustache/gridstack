use crate::schema::registration_sessions;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use serde_json::Value;
use uuid::Uuid;

static EXPIRATION_DURATION: std::time::Duration = std::time::Duration::from_secs(60 * 10);

pub struct NewRegistrationSession {
    pub session_id: String,
    pub completed_stages: Value,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = registration_sessions)]
pub struct RegistrationSessionInsertModel {
    pub session_id: String,
    pub account_id: Uuid,
    pub completed_stages: Value,
    pub expires_at: DateTime<Utc>,
}

impl RegistrationSessionInsertModel {
    pub fn new_with_account_id(
        account_id: Uuid,
        new_registration_session: NewRegistrationSession,
    ) -> Self {
        Self {
            session_id: new_registration_session.session_id,
            account_id,
            completed_stages: new_registration_session.completed_stages,
            expires_at: Utc::now() + EXPIRATION_DURATION,
        }
    }
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = registration_sessions)]
pub struct RegistrationSession {
    pub session_id: String,
    pub account_id: Uuid,
    pub completed_stages: Value,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
