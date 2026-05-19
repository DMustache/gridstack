use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    mem,
    path::{Path, PathBuf},
    sync::RwLock,
};

use chrono::Utc;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    dsl::{exists, select},
    insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
};
use uuid::Uuid;

use crate::{
    infrastructure::{device_id::DeviceId, schema, user_identifier::UserIdentifier},
    services::{
        authorization::{
            entities::{
                AccessToken, AuthorizedUserIdentifier, ExistingUserIdentifier, SessionPrincipal,
                UserAccount,
            },
            persistence::{
                access_session_storage_unit::AccessSessionStorageUnit,
                users::{CreateUserModel, UserModel},
            },
        },
        errors::DomainError,
        repositories::{SessionRepository, UserRepository},
    },
};
mod users;

pub mod access_session_storage_unit;

#[derive(Clone)]
pub struct AuthorizationPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl AuthorizationPersistence {
    #[must_use]
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder().build_unchecked(manager);

        Self { connection_pool }
    }
}

impl UserRepository for AuthorizationPersistence {
    fn create_user(&self, user_account: UserAccount) -> Result<(), DomainError> {
        use schema::{accounts, users};

        let now = Utc::now().naive_utc();
        let account_identifier = Uuid::new_v4();
        let create_account = (accounts::id.eq(account_identifier),);
        let create_model = CreateUserModel {
            user_id: user_account.user_identifier.as_str().to_owned(),
            account_id: account_identifier,
            password_hash: Some(user_account.password_hash),
            is_guest: user_account.is_guest,
            created_at: now,
            updated_at: now,
        };

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        connection
            .transaction(|connection| {
                insert_into(accounts::table)
                    .values(create_account)
                    .execute(connection)?;

                insert_into(users::table)
                    .values(create_model)
                    .execute(connection)?;

                Ok::<(), diesel::result::Error>(())
            })
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(())
    }

    fn find_user_by_identifier(&self, user_identifier: &UserIdentifier) -> Option<UserAccount> {
        use schema::users;

        let mut connection = self.connection_pool.get().ok()?;
        users::table
            .select(users::all_columns)
            .filter(users::user_id.eq(user_identifier.as_str()))
            .first::<UserModel>(&mut connection)
            .optional()
            .ok()
            .flatten()
            .and_then(|model| model.try_into().ok())
    }

    fn user_exists(&self, user_identifier: &UserIdentifier) -> bool {
        use schema::users;

        let Ok(mut connection) = self.connection_pool.get() else {
            return false;
        };

        select(exists(
            users::table.filter(users::columns::user_id.eq(user_identifier.as_str())),
        ))
        .get_result::<bool>(&mut connection)
        .unwrap_or(false)
    }
}

#[derive(Default)]
pub struct InMemorySessionRepository {
    sessions_by_token: RwLock<HashMap<String, AccessSessionStorageUnit>>,
    storage_file_path: PathBuf,
    pending_records: RwLock<Vec<SessionStorageMutation>>,
}

impl InMemorySessionRepository {
    pub fn from_storage_path(storage_file_path: impl Into<PathBuf>) -> Self {
        let storage_file_path = storage_file_path.into();
        let sessions_by_token = RwLock::new(Self::load_sessions_from_file(&storage_file_path));
        Self {
            sessions_by_token,
            storage_file_path,
            pending_records: RwLock::new(Vec::new()),
        }
    }

    fn load_sessions_from_file(
        storage_file_path: &Path,
    ) -> HashMap<String, AccessSessionStorageUnit> {
        let Ok(file) = fs::File::open(storage_file_path) else {
            return HashMap::new();
        };

        let now_seconds = Utc::now().timestamp();
        let mut sessions_by_token = HashMap::new();
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let mut parts = line.split('\t');
            match parts.next() {
                Some("U") => {
                    let access_token = parts.next().and_then(AccessToken::parse);
                    let user_identifier = parts
                        .next()
                        .and_then(|value| UserIdentifier::try_from(value.to_owned()).ok())
                        .map(ExistingUserIdentifier::new)
                        .map(AuthorizedUserIdentifier::new);
                    let device_id = parts.next().and_then(DeviceId::parse);
                    let expires_at_seconds =
                        parts.next().and_then(|value| value.parse::<i64>().ok());

                    let (
                        Some(access_token),
                        Some(user_identifier),
                        Some(device_id),
                        Some(expires_at_seconds),
                    ) = (access_token, user_identifier, device_id, expires_at_seconds)
                    else {
                        continue;
                    };
                    if expires_at_seconds <= now_seconds {
                        continue;
                    }

                    let storage = AccessSessionStorageUnit::new(
                        access_token.clone(),
                        user_identifier,
                        device_id,
                        expires_at_seconds,
                        SessionPrincipal::User,
                    );
                    sessions_by_token.insert(access_token.into_inner(), storage);
                }
                Some("D") => {
                    if let Some(access_token) = parts.next() {
                        sessions_by_token.remove(access_token);
                    }
                }
                _ => continue,
            }
        }

        sessions_by_token
    }

    fn flush_pending_records_to_file(&self) -> Result<(), DomainError> {
        if let Some(parent) = self.storage_file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        }

        let mut pending_records_guard = self
            .pending_records
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?;
        if pending_records_guard.is_empty() {
            return Ok(());
        }
        let pending_records = mem::take(&mut *pending_records_guard);
        drop(pending_records_guard);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_file_path)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        for record in pending_records {
            match record {
                SessionStorageMutation::Upsert(session) => writeln!(
                    file,
                    "U\t{}\t{}\t{}\t{}",
                    session.access_token().as_str(),
                    session.user_identifier().as_str(),
                    session.device_id().clone().into_inner(),
                    session.expires_at_seconds()
                )
                .map_err(|error| DomainError::InvalidRequest(error.to_string()))?,
                SessionStorageMutation::Delete(access_token) => writeln!(file, "D\t{access_token}")
                    .map_err(|error| DomainError::InvalidRequest(error.to_string()))?,
            }
        }
        file.flush()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        Ok(())
    }
}

enum SessionStorageMutation {
    Upsert(AccessSessionStorageUnit),
    Delete(String),
}

impl InMemorySessionRepository {
    fn push_pending_record(&self, record: SessionStorageMutation) -> Result<(), DomainError> {
        let mut pending_records = self
            .pending_records
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?;
        pending_records.push(record);
        Ok(())
    }
}

impl SessionRepository for InMemorySessionRepository {
    fn create_session(&self, access_session: AccessSessionStorageUnit) -> Result<(), DomainError> {
        self.sessions_by_token
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?
            .insert(
                access_session.access_token().clone().into_inner(),
                access_session.clone(),
            );
        self.push_pending_record(SessionStorageMutation::Upsert(access_session))?;
        Ok(())
    }

    fn find_session_by_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Option<AccessSessionStorageUnit> {
        let now_seconds = Utc::now().timestamp();
        let mut sessions_by_token = self.sessions_by_token.write().ok()?;
        let session = sessions_by_token.get(access_token.as_str()).cloned()?;
        if session.expires_at_seconds() <= now_seconds {
            sessions_by_token.remove(access_token.as_str());
            let _ = self.push_pending_record(SessionStorageMutation::Delete(
                access_token.as_str().to_owned(),
            ));
            return None;
        }
        Some(session)
    }

    fn delete_session_by_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Result<(), DomainError> {
        self.sessions_by_token
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?
            .remove(access_token.as_str());
        self.push_pending_record(SessionStorageMutation::Delete(
            access_token.as_str().to_owned(),
        ))?;
        Ok(())
    }

    fn flush(&self) -> Result<(), DomainError> {
        self.flush_pending_records_to_file()
    }
}

impl Drop for InMemorySessionRepository {
    fn drop(&mut self) {
        let _ = self.flush_pending_records_to_file();
    }
}
