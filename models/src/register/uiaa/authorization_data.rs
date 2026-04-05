use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuthorizationData {
    RegistrationToken(RegistrationTokenAuth),
}

mod registration_token_auth;
pub use registration_token_auth::RegistrationTokenAuth;

impl AuthorizationData {
    pub fn get_session_id(&self) -> Option<String> {
        match self {
            AuthorizationData::RegistrationToken(registration_token_auth) => {
                registration_token_auth.session.clone()
            }
        }
    }
}
