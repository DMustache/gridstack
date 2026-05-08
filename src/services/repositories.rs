use super::{
    authorization::entities::{AccessSession, AccessToken, UserAccount, UserId},
    errors::DomainError,
    messaging_events::entities::RoomEvent,
    rooms::entities::ChatRoom,
};

pub trait UserRepository: Send + Sync {
    fn create_user(&self, user_account: UserAccount) -> Result<(), DomainError>;
    fn find_user_by_identifier(&self, user_identifier: &UserId) -> Option<UserAccount>;
    fn user_exists(&self, user_identifier: &UserId) -> bool;
}

pub trait SessionRepository: Send + Sync {
    fn create_session(&self, access_session: AccessSession) -> Result<(), DomainError>;
    fn find_session_by_access_token(&self, access_token: &AccessToken) -> Option<AccessSession>;
}

pub trait RoomRepository: Send + Sync {
    fn create_room(&self, chat_room: ChatRoom) -> Result<(), DomainError>;
    fn find_room_by_identifier_or_alias(&self, room_identifier_or_alias: &str) -> Option<ChatRoom>;
    fn list_public_rooms(&self) -> Vec<ChatRoom>;
    fn add_member(
        &self,
        room_identifier: &str,
        user_identifier: &UserId,
    ) -> Result<(), DomainError>;
    fn add_event(&self, room_identifier: &str, room_event: RoomEvent) -> Result<(), DomainError>;
    fn list_events(&self, room_identifier: &str) -> Result<Vec<RoomEvent>, DomainError>;
}
