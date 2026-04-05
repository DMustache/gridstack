use models::error::{AppError, AppResult};

pub fn sanitize_option_string_or_error(
    option: Option<&str>,
    field_name: &'static str,
) -> Result<String, AppError> {
    Ok(option
        .filter(|string| !string.trim().is_empty())
        .ok_or_else(|| {
            AppError::missing_parameter(format!("{field_name} must not be empty").as_str())
        })?
        .to_string())
}

pub fn get_some_or_error<T>(option: Option<T>, field_name: &'static str) -> AppResult<T> {
    option.ok_or_else(|| {
        AppError::missing_parameter(format!("{field_name} must not be empty").as_str())
    })
}
