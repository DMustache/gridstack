use std::sync::Arc;

use crate::{
    application::ports::{SessionRepository, SynchronizationApi},
    domain::{
        error::ClientError,
        synchronization::{DefineFilterInfo, DefineFilterView, SyncQuery, SyncResponseView},
    },
};

pub struct SynchronizationClientService {
    server_url: String,
    synchronization_api: Arc<dyn SynchronizationApi>,
    session_repository: Arc<dyn SessionRepository>,
}

impl SynchronizationClientService {
    pub fn new(
        server_url: impl Into<String>,
        synchronization_api: Arc<dyn SynchronizationApi>,
        session_repository: Arc<dyn SessionRepository>,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            synchronization_api,
            session_repository,
        }
    }

    pub async fn define_filter(
        &self,
        request: &DefineFilterInfo,
    ) -> Result<DefineFilterView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;
        self.synchronization_api
            .define_filter(&session.access_token, &session.user_id, request)
            .await
    }

    pub async fn get_filter(&self, filter_id: &str) -> Result<serde_json::Value, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;
        self.synchronization_api
            .get_filter(&session.access_token, &session.user_id, filter_id)
            .await
    }

    pub async fn sync(&self, query: &SyncQuery) -> Result<SyncResponseView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;
        self.synchronization_api
            .sync(&session.access_token, query)
            .await
    }
}
