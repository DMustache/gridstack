use crate::infrastructure::json_web_token::JsonWebTokenError;

pub trait IdGenerator: Send + Sync {
    fn next_access_token(&self, user_identifier: &str) -> String;
    fn next_room_identifier(&self, home_server_name: &str) -> String;
    fn next_event_identifier(&self, home_server_name: &str) -> String;
}

pub trait Id: Send + Sync {
    fn new(&self) -> Self;
}

pub trait JsonWebTokenAdapter: Send + Sync {
    fn encode<T: Clock>(
        &self,
        expires_at: T,
        encoding_key: &str,
    ) -> Result<String, JsonWebTokenError>;

    fn is_token_valid(token: &str, secret: &str) -> bool;
}

pub trait Clock: Send + Sync {
    fn now_unix_milliseconds(&self) -> i64;

    fn as_seconds(&self) -> i64;
}
