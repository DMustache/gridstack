use serde::Serialize;

use crate::services::authorization::layers::AuthenticatedUser;

#[derive(Serialize)]
pub struct GetViewModel {
    device_id: String,
    user_id: String,
    is_guest: bool,
}

impl From<AuthenticatedUser> for GetViewModel {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            device_id: user.device_id,
            user_id: user.user_id,
            is_guest: user.is_guest,
        }
    }
}
