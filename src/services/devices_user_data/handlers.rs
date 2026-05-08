use axum::{Json, response::IntoResponse};

use serde_json::json;

use crate::services::{devices_user_data::service};

pub async fn get_devices() -> impl IntoResponse {
    match service::get_devices() {
        Ok(()) => Json(json!({})).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}
