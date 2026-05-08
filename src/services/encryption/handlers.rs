use axum::{Json, response::IntoResponse};

use serde_json::json;

use crate::services::{encryption::service};

pub async fn upload_keys() -> impl IntoResponse {
    match service::upload_keys() {
        Ok(()) => Json(json!({})).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}
