use crate::services::errors::ApplicationError;

pub fn get_notifications() -> Result<(), ApplicationError> {
    Err(ApplicationError::NotImplemented(
        "Notifications endpoint is documented but not implemented".to_owned(),
    ))
}
