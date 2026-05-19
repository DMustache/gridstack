use std::sync::Arc;

use crate::{
    application::ports::{RoomRepository, RoomsApi, SessionRepository},
    domain::{
        error::ClientError,
        rooms::{CreateRoomInfo, CreateRoomView, RoomListItem},
    },
};

pub struct RoomsClientService {
    server_url: String,
    rooms_api: Arc<dyn RoomsApi>,
    session_repository: Arc<dyn SessionRepository>,
    room_repository: Arc<dyn RoomRepository>,
}

impl RoomsClientService {
    pub fn new(
        server_url: impl Into<String>,
        rooms_api: Arc<dyn RoomsApi>,
        session_repository: Arc<dyn SessionRepository>,
        room_repository: Arc<dyn RoomRepository>,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            rooms_api,
            session_repository,
            room_repository,
        }
    }

    pub async fn create_room(&self, request: &CreateRoomInfo) -> Result<CreateRoomView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        let view = self.rooms_api.create_room(&session.access_token, request).await?;

        let room_item = RoomListItem {
            server_url: self.server_url.clone(),
            user_id: session.user_id,
            room_id: view.room_id.clone(),
            name: request.name.clone(),
            topic: request.topic.clone(),
        };
        self.room_repository.add_room(&room_item).await?;

        Ok(view)
    }

    pub async fn list_cached_rooms(&self) -> Result<Vec<RoomListItem>, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        self.room_repository
            .list_rooms_by_server_user(&self.server_url, &session.user_id)
            .await
    }
}
