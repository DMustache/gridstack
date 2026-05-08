use crate::services::errors::ApplicationError;

pub fn get_admin_whois() -> Result<(), ApplicationError> {
    Err(ApplicationError::NotImplemented(
        "Administration endpoint is documented but not implemented".to_owned(),
    ))
}
