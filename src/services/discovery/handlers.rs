use axum::{Json, extract::State, response::IntoResponse};

use crate::services::{discovery::service, state::ApplicationState};

pub async fn get_well_known_client(
    State(application_state): State<ApplicationState>,
) -> impl IntoResponse {
    Json(service::get_well_known_client(&application_state))
}

pub async fn get_well_known_policy_server() -> impl IntoResponse {
    Json(service::get_well_known_policy_server())
}

pub async fn get_well_known_support(
    State(application_state): State<ApplicationState>,
) -> impl IntoResponse {
    Json(service::get_well_known_support(&application_state))
}
