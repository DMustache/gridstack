use std::sync::RwLock;

use chrono::Utc;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    dsl::{exists, select},
    insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
};
use uuid::Uuid;

use crate::{
    infrastructure::{schema, user_identifier::UserIdentifier},
    services::{
        authorization::{
            entities::{AccessSession, AccessToken, UserAccount},
            persistence::users::{CreateUserModel, UserModel},
        },
        errors::DomainError,
        repositories::{SessionRepository, UserRepository},
    },
};
mod users;

#[derive(Clone)]
pub struct AuthorizationPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl AuthorizationPersistence {
    pub fn new(database_url: &str) -> Result<Self, DomainError> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder()
            .build(manager)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(Self { connection_pool })
    }
}

impl UserRepository for AuthorizationPersistence {
    fn create_user(&self, user_account: UserAccount) -> Result<(), DomainError> {
        use schema::users;

        let now = Utc::now().naive_utc();
        let create_model = CreateUserModel {
            user_id: user_account.user_identifier.as_str().to_owned(),
            account_id: Uuid::new_v4(),
            password_hash: Some(user_account.password_hash),
            is_guest: user_account.is_guest,
            created_at: now,
            updated_at: now,
        };

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        insert_into(users::table)
            .values(create_model)
            .execute(&mut connection)
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

    fn is_user_password_matches(&self, user_identifier: &UserIdentifier, password: &str) -> bool {
        use schema::users;

        let Ok(mut connection) = self.connection_pool.get() else {
            return false;
        };

        select(exists(
            users::table
                .filter(users::columns::user_id.eq(user_identifier.as_str()))
                .filter(users::columns::password_hash.eq(password)),
        ))
        .get_result::<bool>(&mut connection)
        .unwrap_or(false)
    }
}

#[derive(Default)]
pub struct InMemorySessionRepository {
    sessions_by_token: RwLock<std::collections::HashMap<String, AccessSession>>,
}

impl SessionRepository for InMemorySessionRepository {
    fn create_session(&self, access_session: AccessSession) -> Result<(), DomainError> {
        self.sessions_by_token
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?
            .insert(
                access_session.access_token.as_str().to_owned(),
                access_session,
            );
        Ok(())
    }

    fn find_session_by_access_token(&self, access_token: &AccessToken) -> Option<AccessSession> {
        let sessions_by_token = self.sessions_by_token.read().ok()?;
        sessions_by_token.get(access_token.as_str()).cloned()
    }

    fn delete_session_by_access_token(&self, access_token: AccessToken) -> Result<(), DomainError> {
        self.sessions_by_token
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?
            .remove(access_token.as_str());
        Ok(())
    }
}
