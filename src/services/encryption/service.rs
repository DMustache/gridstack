use crate::services::errors::ApplicationError;

pub fn upload_keys() -> Result<(), ApplicationError> {
    Err(ApplicationError::NotImplemented(
        "Encryption endpoint is documented but not implemented".to_owned(),
    ))
}
