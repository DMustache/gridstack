use axum::{Json, response::IntoResponse};

use serde_json::json;

use crate::services::{notifications::service};

pub async fn get_notifications() -> impl IntoResponse {
    match service::get_notifications() {
        Ok(()) => Json(json!({})).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}
