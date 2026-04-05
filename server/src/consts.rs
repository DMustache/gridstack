pub const ACCESS_TOKEN_EXPIRY_SECONDS: u64 = 60 * 60 * 3;
pub const REFRESH_TOKEN_EXPIRY_SECONDS: u64 = 60 * 60 * 24 * 3;

pub const SUPPORTED_AUTHORIZATION_TYPES: &[&str] = &["m.login.registration_token"];
pub const PASSWORD_LOGIN_TYPE: &str = "m.login.password";
pub const USER_IDENTIFIER_TYPE: &str = "m.id.user";
