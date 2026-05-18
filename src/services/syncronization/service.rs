use std::sync::Arc;

use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::{
        authorization::{
            entities::AuthorizedUserIdentifier,
            persistence::access_session_storage_unit::AccessSessionStorageUnit,
            service::AuthorizationService,
        },
        syncronization::{
            errors::SyncronizationApplicationError,
            handlers::DefineFilterView,
            persistence::FilterRepository,
        },
    },
};

pub struct SyncronizationService {
    filter_repository: Arc<dyn FilterRepository>,
    authorization_service: Arc<AuthorizationService>,
}

impl SyncronizationService {
    pub fn new(
        filter_repository: Arc<dyn FilterRepository>,
        authorization_service: Arc<AuthorizationService>,
    ) -> Self {
        Self {
            filter_repository,
            authorization_service,
        }
    }

    pub fn define_filter(
        &self,
        access_session: &AccessSessionStorageUnit,
        user_id: &str,
        filter_payload: serde_json::Value,
    ) -> Result<DefineFilterView, SyncronizationApplicationError> {
        let authorized_user_identifier =
            self.require_authorized_user_for_request(access_session, user_id)?;

        let filter_id = self
            .filter_repository
            .create_filter(authorized_user_identifier.as_str(), filter_payload)
            .map_err(|_| SyncronizationApplicationError::Internal)?;

        Ok(DefineFilterView { filter_id })
    }

    pub fn get_filter(
        &self,
        access_session: &AccessSessionStorageUnit,
        user_id: &str,
        filter_id: &str,
    ) -> Result<serde_json::Value, SyncronizationApplicationError> {
        let authorized_user_identifier =
            self.require_authorized_user_for_request(access_session, user_id)?;

        self.filter_repository
            .fetch_filter(authorized_user_identifier.as_str(), filter_id)
            .map_err(|error| match error {
                crate::services::errors::DomainError::InvalidRequest(_reason) => {
                    SyncronizationApplicationError::InvalidParameter
                }
                _ => SyncronizationApplicationError::Internal,
            })?
            .ok_or(SyncronizationApplicationError::NotFound)
    }

    fn require_authorized_user_for_request(
        &self,
        access_session: &AccessSessionStorageUnit,
        user_id: &str,
    ) -> Result<AuthorizedUserIdentifier, SyncronizationApplicationError> {
        let requested_user_identifier = UserIdentifier::try_from(user_id.to_owned())
            .map_err(|_| SyncronizationApplicationError::InvalidParameter)?;

        self.authorization_service
            .require_authorized_user(access_session, &requested_user_identifier)
            .map_err(|error| match error {
                crate::services::authorization::errors::AuthorizationApplicationError::Unauthorized => {
                    SyncronizationApplicationError::Unauthorized
                }
                crate::services::authorization::errors::AuthorizationApplicationError::Forbidden => {
                    SyncronizationApplicationError::Forbidden
                }
                _ => SyncronizationApplicationError::Internal,
            })
    }
}
