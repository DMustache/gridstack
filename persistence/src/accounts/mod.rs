use ::models::error::internal_error::DatabaseError;
use chrono::Utc;
use deadpool_diesel::{
    Runtime,
    postgres::{Manager, Pool},
};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, PgConnection, QueryDsl, RunQueryDsl,
    SelectableHelper, delete, dsl::exists, insert_into, select, sql_query,
};
use uuid::Uuid;

use crate::{
    accounts::models::RegistrationSessionInsertModel,
    schema::{access_tokens, accounts, devices, registration_sessions},
};

mod models;
pub use models::{
    AccessToken, AuthenticatedSession, Device, NewAccessToken, NewDevice, NewRegistrationSession,
    RegistrationSession,
};

use std::sync::LazyLock;

static CONNECTION_STRING: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set at runtime"));

pub struct Accounts {
    pool: Pool,
}

impl Accounts {
    fn create_account(connection: &mut PgConnection) -> Result<Uuid, DatabaseError> {
        Ok(insert_into(accounts::table)
            .default_values()
            .returning(accounts::id)
            .get_result::<Uuid>(connection)?)
    }

    pub fn try_new() -> Result<Self, DatabaseError> {
        let manager = Manager::new(&*CONNECTION_STRING, Runtime::Tokio1);
        let pool = Pool::builder(manager).build()?;

        Ok(Self { pool })
    }

    pub async fn create_pending_registration_session(
        &self,
        registration_session: NewRegistrationSession,
    ) -> Result<(), DatabaseError> {
        self.pool
            .get()
            .await?
            .interact(move |connection| {
                connection.transaction::<(), DatabaseError, _>(|connection| {
                    let account_id = Self::create_account(connection)?;

                    insert_into(registration_sessions::table)
                        .values(RegistrationSessionInsertModel::new_with_account_id(
                            account_id,
                            registration_session,
                        ))
                        .execute(connection)?;

                    Ok::<(), DatabaseError>(())
                })
            })
            .await?
    }

    pub async fn get_account_by_registration_session(
        &self,
        session_id: String,
    ) -> Result<Uuid, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let account_id = registration_sessions::table
                    .filter(registration_sessions::session_id.eq(session_id))
                    .filter(registration_sessions::expires_at.gt(Utc::now()))
                    .select(registration_sessions::account_id)
                    .first::<Uuid>(connection)?;

                Ok::<Uuid, DatabaseError>(account_id)
            })
            .await??)
    }

    pub async fn is_device_id_exists(
        &self,
        user_id: String,
        device_id: String,
    ) -> Result<bool, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let exists = select(exists(
                    devices::table
                        .filter(devices::user_id.eq(user_id))
                        .filter(devices::device_id.eq(device_id)),
                ))
                .get_result::<bool>(connection)?;

                Ok::<bool, DatabaseError>(exists)
            })
            .await??)
    }

    pub async fn create_device(&self, new_device: NewDevice) -> Result<(), DatabaseError> {
        self.pool
            .get()
            .await?
            .interact(move |connection| {
                insert_into(devices::table)
                    .values(new_device)
                    .execute(connection)?;

                Ok::<(), DatabaseError>(())
            })
            .await?
    }

    pub async fn get_device(
        &self,
        user_id: String,
        device_id: String,
    ) -> Result<Option<Device>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let device = devices::table
                    .filter(devices::user_id.eq(user_id))
                    .filter(devices::device_id.eq(device_id))
                    .select(Device::as_select())
                    .first::<Device>(connection)
                    .optional()?;

                Ok::<Option<Device>, DatabaseError>(device)
            })
            .await??)
    }

    pub async fn create_access_token(
        &self,
        new_access_token: NewAccessToken,
    ) -> Result<(), DatabaseError> {
        self.pool
            .get()
            .await?
            .interact(move |connection| {
                insert_into(access_tokens::table)
                    .values(new_access_token)
                    .execute(connection)?;

                Ok::<(), DatabaseError>(())
            })
            .await?
    }

    pub async fn try_get_authenticated_session_by_access_token(
        &self,
        access_token: String,
    ) -> Result<Option<AuthenticatedSession>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let session = sql_query(
                    "
                    SELECT
                        t.user_id,
                        t.device_id,
                        d.display_name AS device_display_name,
                        t.access_token
                    FROM access_tokens t
                    INNER JOIN devices d
                        ON d.user_id = t.user_id
                       AND d.device_id = t.device_id
                    WHERE t.access_token = $1
                      AND t.access_token_expires_at > NOW()
                    LIMIT 1
                    ",
                )
                .bind::<diesel::sql_types::Text, _>(access_token)
                .get_result::<AuthenticatedSession>(connection)
                .optional()?;

                Ok::<Option<AuthenticatedSession>, DatabaseError>(session)
            })
            .await??)
    }

    pub async fn logout_by_access_token(&self, access_token: String) -> Result<(), DatabaseError> {
        self.pool
            .get()
            .await?
            .interact(move |connection| {
                let token = access_tokens::table
                    .filter(access_tokens::access_token.eq(access_token))
                    .select((access_tokens::user_id, access_tokens::device_id))
                    .first::<(String, String)>(connection)
                    .optional()?;

                let Some((user_id, device_id)) = token else {
                    return Ok(());
                };

                delete(
                    access_tokens::table
                        .filter(access_tokens::user_id.eq(&user_id))
                        .filter(access_tokens::device_id.eq(&device_id)),
                )
                .execute(connection)?;

                delete(
                    devices::table
                        .filter(devices::user_id.eq(user_id))
                        .filter(devices::device_id.eq(device_id)),
                )
                .execute(connection)?;

                Ok::<(), DatabaseError>(())
            })
            .await?
    }

    pub async fn logout_all_for_user(&self, user_id: String) -> Result<(), DatabaseError> {
        self.pool
            .get()
            .await?
            .interact(move |connection| {
                delete(devices::table.filter(devices::user_id.eq(user_id))).execute(connection)?;

                Ok::<(), DatabaseError>(())
            })
            .await?
    }
}
