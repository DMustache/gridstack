use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::any};

use crate::services::{authorization, shared::MatrixErrorResponse, state::ApplicationState};

pub fn build_router(application_state: ApplicationState) -> Router {
    Router::new()
        .merge(authorization::routes::routes(&application_state))
        .with_state(application_state.clone())
        .fallback(any(matrix_fallback))
        .with_state(application_state)
}

async fn matrix_fallback() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(MatrixErrorResponse {
            errcode: "M_NOT_FOUND".to_owned(),
            error: "Endpoint is not part of this server API".to_owned(),
        }),
    )
        .into_response()
}
