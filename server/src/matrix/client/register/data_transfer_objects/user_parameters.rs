use models::{
    DeviceId,
    error::{AppError, MatrixError},
    password::Password,
    username::Localpart,
};
use uuid::Uuid;

use crate::state::ServerState;

use super::BodyInfo;

const LOCALPART_LENGTH: usize = 12;

#[derive(Clone, Debug)]
pub struct UserParameters {
    pub username: Localpart,
    pub password: Password,
    pub device_id: DeviceId,
    pub initial_device_display_name: Option<String>,
    pub inhibit_login: bool,
    pub refresh_token: bool,
}

impl TryFrom<(BodyInfo, &ServerState)> for UserParameters {
    type Error = AppError;

    fn try_from((body_parameters, _state): (BodyInfo, &ServerState)) -> Result<Self, Self::Error> {
        let generated_localpart = Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(LOCALPART_LENGTH)
            .collect::<String>();

        Ok(Self {
            username: Localpart::try_new(body_parameters.username.unwrap_or(generated_localpart))?,
            password: Password::try_new(body_parameters.password.ok_or_else(|| {
                MatrixError::missing_parameter("Password field shouldn't be empty")
            })?)?,
            device_id: DeviceId::get_or_generate(body_parameters.device_id),
            initial_device_display_name: body_parameters.initial_device_display_name,
            inhibit_login: body_parameters.inhibit_login.unwrap_or_default(),
            refresh_token: body_parameters.refresh_token.unwrap_or_default(),
        })
    }
}
