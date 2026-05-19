use async_trait::async_trait;

use crate::domain::{
    authorization::{
        AccountKind, CheckUsernameAvailableView, GetAuthMetadataView, GetLoginFlowsView,
        LoginUserInfo, LoginUserView, LogoutUserView, RegisterUserInfo, RegisterUserView,
        ServerCredentials, SessionRecord, UiaaResponseView, WhoAmIView,
    },
    rooms::{CreateRoomInfo, CreateRoomView, RoomListItem},
    error::ClientError,
};

pub enum RegisterUserResult {
    Registered(RegisterUserView),
    AuthenticationRequired(UiaaResponseView),
}

#[async_trait]
pub trait AuthorizationApi: Send + Sync {
    async fn get_auth_metadata(&self) -> Result<GetAuthMetadataView, ClientError>;
    async fn get_login_flows(&self) -> Result<GetLoginFlowsView, ClientError>;
    async fn check_username_available(
        &self,
        username: &str,
    ) -> Result<CheckUsernameAvailableView, ClientError>;
    async fn register_user(
        &self,
        kind: AccountKind,
        request: &RegisterUserInfo,
    ) -> Result<RegisterUserResult, ClientError>;
    async fn login_user(&self, request: &LoginUserInfo) -> Result<LoginUserView, ClientError>;
    async fn who_am_i(&self, access_token: &str) -> Result<WhoAmIView, ClientError>;
    async fn logout_user(&self, access_token: &str) -> Result<LogoutUserView, ClientError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn upsert_session(&self, session: &SessionRecord) -> Result<(), ClientError>;
    async fn get_session_by_server(
        &self,
        server_url: &str,
    ) -> Result<Option<SessionRecord>, ClientError>;
    async fn clear_session_by_server(&self, server_url: &str) -> Result<(), ClientError>;
}

#[async_trait]
pub trait RoomsApi: Send + Sync {
    async fn create_room(
        &self,
        access_token: &str,
        request: &CreateRoomInfo,
    ) -> Result<CreateRoomView, ClientError>;
}

#[async_trait]
pub trait RoomRepository: Send + Sync {
    async fn add_room(&self, room: &RoomListItem) -> Result<(), ClientError>;
    async fn list_rooms_by_server_user(
        &self,
        server_url: &str,
        user_id: &str,
    ) -> Result<Vec<RoomListItem>, ClientError>;
}

#[async_trait]
pub trait ServerProfileRepository: Send + Sync {
    async fn upsert_server_credentials(
        &self,
        credentials: &ServerCredentials,
    ) -> Result<(), ClientError>;

    async fn list_server_credentials(&self) -> Result<Vec<ServerCredentials>, ClientError>;

    async fn list_server_accounts(&self, server_url: &str) -> Result<Vec<ServerCredentials>, ClientError>;

    async fn get_server_credentials(
        &self,
        server_url: &str,
        username: &str,
    ) -> Result<Option<ServerCredentials>, ClientError>;
}
