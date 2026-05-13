use crate::infrastructure::json_web_token::JsonWebTokenError;

pub trait IdGenerator: Send + Sync {
    fn next_access_token(&self, user_identifier: &str) -> String;
    fn next_room_identifier(&self, home_server_name: &str) -> String;
    fn next_event_identifier(&self, home_server_name: &str) -> String;
}

pub trait Id: Send + Sync {
    #[must_use]
    fn new_id() -> String;
}

pub trait JsonWebTokenAdapter: Send + Sync {
    fn encode(
        &self,
        expires_at: &dyn Clock,
        encoding_key: &str,
    ) -> Result<String, JsonWebTokenError>;

    fn is_token_valid(&self, token: &str, secret: &str) -> bool;
}

pub trait Clock: Send + Sync {
    fn as_seconds(&self) -> i64;

    fn now_unix_milliseconds(&self) -> i64;
}
