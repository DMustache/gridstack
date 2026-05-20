use std::sync::Arc;

use crate::{
    application::ports::{RoomRepository, RoomsApi, SessionRepository},
    domain::{
        error::ClientError,
        rooms::{
            CreateRoomInfo, CreateRoomView, GetRoomMessagesQuery, GetRoomMessagesView,
            JoinRoomInfo, JoinRoomView, JoinedRoomsView, LeaveRoomInfo, LeaveRoomView,
            RoomListItem, RoomStateEventView, SendRoomEventView,
        },
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

    pub async fn join_room_by_id(
        &self,
        room_id: &str,
        request: &JoinRoomInfo,
    ) -> Result<JoinRoomView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        let view = self
            .rooms_api
            .join_room_by_id(&session.access_token, room_id, request)
            .await?;

        let room_item = RoomListItem {
            server_url: self.server_url.clone(),
            user_id: session.user_id,
            room_id: view.room_id.clone(),
            name: None,
            topic: None,
        };
        self.room_repository.add_room(&room_item).await?;
        Ok(view)
    }

    pub async fn leave_room_by_id(
        &self,
        room_id: &str,
        request: &LeaveRoomInfo,
    ) -> Result<LeaveRoomView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        let view = self
            .rooms_api
            .leave_room_by_id(&session.access_token, room_id, request)
            .await?;

        self.room_repository
            .remove_room(&self.server_url, &session.user_id, room_id)
            .await?;
        Ok(view)
    }

    pub async fn get_room_state(
        &self,
        room_id: &str,
    ) -> Result<Vec<RoomStateEventView>, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        self.rooms_api
            .get_room_state(&session.access_token, room_id)
            .await
    }

    pub async fn get_room_messages(
        &self,
        room_id: &str,
        query: &GetRoomMessagesQuery,
    ) -> Result<GetRoomMessagesView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        self.rooms_api
            .get_room_messages(&session.access_token, room_id, query)
            .await
    }

    pub async fn get_joined_rooms(&self) -> Result<JoinedRoomsView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        self.rooms_api.get_joined_rooms(&session.access_token).await
    }

    pub async fn send_room_message_event(
        &self,
        room_id: &str,
        event_type: &str,
        transaction_id: &str,
        content: &serde_json::Value,
    ) -> Result<SendRoomEventView, ClientError> {
        let session = self
            .session_repository
            .get_session_by_server(&self.server_url)
            .await?
            .ok_or_else(|| ClientError::Matrix {
                status: 401,
                errcode: "M_MISSING_TOKEN".to_owned(),
                message: "No saved session token".to_owned(),
            })?;

        self.rooms_api
            .send_room_message_event(
                &session.access_token,
                room_id,
                event_type,
                transaction_id,
                content,
            )
            .await
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

    pub async fn upsert_cached_room(
        &self,
        room_id: &str,
        name: Option<String>,
        topic: Option<String>,
    ) -> Result<(), ClientError> {
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
            .add_room(&RoomListItem {
                server_url: self.server_url.clone(),
                user_id: session.user_id,
                room_id: room_id.to_owned(),
                name,
                topic,
            })
            .await
    }
}
