use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthorizationDomainError {
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("user not found")]
    UserNotFound,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid username")]
    InvalidUsername,
    #[error("invalid token")]
    InvalidToken,
    #[error("registration disabled")]
    RegistrationDisabled,
    #[error("repository error: {0}")]
    Repository(String),
}

#[derive(Debug, Error)]
pub enum AuthorizationApplicationError {
    #[error("username is already used")]
    UserInUse,
    #[error("desired username is in exclusive namespace")]
    Exclusive,
    #[error("invalid username")]
    InvalidUsername,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("missing or invalid access token")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("unrecognized")]
    Unrecognized,
    #[error("Legacy authentication is in use on this homeserver.")]
    OAuthAuthorizationUnsupported,
    #[error("OAuth 2.0 authentication is in use on this homeserver.")]
    LegacyAuthorizationUnsupported,
    #[error("unexpected error: {0}")]
    Internal(String),
}

impl From<AuthorizationDomainError> for AuthorizationApplicationError {
    fn from(value: AuthorizationDomainError) -> Self {
        match value {
            AuthorizationDomainError::UserAlreadyExists => Self::UserInUse,
            AuthorizationDomainError::UserNotFound => Self::InvalidCredentials,
            AuthorizationDomainError::InvalidCredentials => Self::InvalidCredentials,
            AuthorizationDomainError::InvalidUsername => Self::InvalidUsername,
            AuthorizationDomainError::InvalidToken => Self::Unauthorized,
            AuthorizationDomainError::RegistrationDisabled => Self::Forbidden,
            AuthorizationDomainError::Repository(message) => Self::Internal(message),
        }
    }
}
