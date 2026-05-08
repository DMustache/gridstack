use axum::{Json, response::IntoResponse};

use serde_json::json;

use crate::services::{administration::service};

pub async fn get_admin_whois() -> impl IntoResponse {
    match service::get_admin_whois() {
        Ok(()) => Json(json!({})).into_response(),
        Err(application_error) => application_error.into_response(),
    }
}
