use axum::{Json, response::IntoResponse};

use serde_json::json;

use crate::services::{media::service};

pub async fn get_media_config() -> impl IntoResponse {
    match service::get_media_config() {
        Ok(()) => Json(json!({})).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}
