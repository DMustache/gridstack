use std::{collections::HashMap, sync::RwLock};

use diesel::{
    ExpressionMethods, PgConnection, QueryDsl,
    dsl::{exists, select},
    r2d2::{self, ConnectionManager},
};

use crate::{
    infrastructure::schema,
    services::{
        authorization::entities::{AccessSession, AccessToken, UserAccount, UserId},
        errors::DomainError,
        repositories::{SessionRepository, UserRepository},
    },
};

mod registration_sessions;
mod users;

#[derive(Default)]
pub struct AuthorizationPersistence {
    connection: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl UserRepository for AuthorizationPersistence {
    fn create_user(&self, user_account: UserAccount) -> Result<(), DomainError> {
        let mut users_by_identifier = self
            .users_by_identifier
            .write()
            .map_err(|_| DomainError::InvalidRequest("user storage lock failure".to_owned()))?;
        if users_by_identifier.contains_key(user_account.user_identifier.as_str()) {
            return Err(DomainError::AlreadyExists);
        }
        users_by_identifier.insert(
            user_account.user_identifier.as_str().to_owned(),
            user_account,
        );
        Ok(())
    }

    fn find_user_by_identifier(&self, user_identifier: &UserId) -> Option<UserAccount> {
        let users_by_identifier = self.users_by_identifier.read().ok()?;
        users_by_identifier.get(user_identifier.as_str()).cloned()
    }

    fn user_exists(&self, user_identifier: &UserId) -> bool {
        use schema::users;

        select(exists(
            users::table.filter(users::columns::user_id.eq("Sean")),
        ))
        .get_result(self.connection);
    }
}

#[derive(Default)]
pub struct InMemorySessionRepository {
    sessions_by_token: RwLock<HashMap<String, AccessSession>>,
}

impl SessionRepository for InMemorySessionRepository {
    fn create_session(&self, access_session: AccessSession) -> Result<(), DomainError> {
        let mut sessions_by_token = self
            .sessions_by_token
            .write()
            .map_err(|_| DomainError::InvalidRequest("session storage lock failure".to_owned()))?;
        sessions_by_token.insert(
            access_session.access_token.as_str().to_owned(),
            access_session,
        );
        Ok(())
    }

    fn find_session_by_access_token(&self, access_token: &AccessToken) -> Option<AccessSession> {
        let sessions_by_token = self.sessions_by_token.read().ok()?;
        sessions_by_token.get(access_token.as_str()).cloned()
    }
}
