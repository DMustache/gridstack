use crate::services::errors::ApplicationError;

pub fn get_devices() -> Result<(), ApplicationError> {
    Err(ApplicationError::NotImplemented(
        "Devices and User Data endpoint is documented but not implemented".to_owned(),
    ))
}
