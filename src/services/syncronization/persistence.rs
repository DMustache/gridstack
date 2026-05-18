use diesel::{
    OptionalExtension, QueryableByName, RunQueryDsl,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
    sql_query,
    sql_types::{BigInt, Jsonb, Text},
};
use serde_json::Value;

use crate::services::errors::DomainError;

pub trait FilterRepository: Send + Sync {
    fn create_filter(
        &self,
        user_identifier: &str,
        filter_payload: Value,
    ) -> Result<String, DomainError>;
    fn fetch_filter(
        &self,
        user_identifier: &str,
        filter_identifier: &str,
    ) -> Result<Option<Value>, DomainError>;
}

#[derive(Clone)]
pub struct SyncronizationPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl SyncronizationPersistence {
    #[must_use]
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder().build_unchecked(manager);
        Self { connection_pool }
    }
}

#[derive(QueryableByName)]
struct CreatedFilterRecord {
    #[diesel(sql_type = BigInt)]
    id: i64,
}

#[derive(QueryableByName)]
struct StoredFilterRecord {
    #[diesel(sql_type = Jsonb)]
    filter_json: Value,
}

impl FilterRepository for SyncronizationPersistence {
    fn create_filter(
        &self,
        user_identifier: &str,
        filter_payload: Value,
    ) -> Result<String, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let created = sql_query(
            "INSERT INTO public.user_filters (user_id, filter_json) VALUES ($1, $2) RETURNING id",
        )
        .bind::<Text, _>(user_identifier)
        .bind::<Jsonb, _>(filter_payload)
        .get_result::<CreatedFilterRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(created.id.to_string())
    }

    fn fetch_filter(
        &self,
        user_identifier: &str,
        filter_identifier: &str,
    ) -> Result<Option<Value>, DomainError> {
        let filter_identifier_number = match filter_identifier.parse::<i64>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let stored = sql_query(
            "SELECT filter_json FROM public.user_filters WHERE user_id = $1 AND id = $2",
        )
        .bind::<Text, _>(user_identifier)
        .bind::<BigInt, _>(filter_identifier_number)
        .get_result::<StoredFilterRecord>(&mut connection)
        .optional()
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(stored.map(|record| record.filter_json))
    }
}
