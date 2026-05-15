use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
};

use super::{
    authorization::entities::{AccessToken, UserAccount},
    errors::DomainError,
};

pub trait UserRepository: Send + Sync {
    fn create_user(&self, user_account: UserAccount) -> Result<(), DomainError>;
    fn find_user_by_identifier(&self, user_identifier: &UserIdentifier) -> Option<UserAccount>;
    fn user_exists(&self, user_identifier: &UserIdentifier) -> bool;
}

pub trait SessionRepository: Send + Sync {
    fn create_session(&self, access_session: AccessSessionStorageUnit) -> Result<(), DomainError>;
    fn find_session_by_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Option<AccessSessionStorageUnit>;
    fn delete_session_by_access_token(&self, access_token: &AccessToken)
    -> Result<(), DomainError>;
    fn flush(&self) -> Result<(), DomainError> {
        Ok(())
    }
}
