use crate::services::errors::ApplicationError;

pub fn get_media_config() -> Result<(), ApplicationError> {
    Err(ApplicationError::NotImplemented(
        "Not implemented".to_owned(),
    ))
}
