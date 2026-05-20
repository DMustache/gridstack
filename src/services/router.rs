use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::any};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::services::{
    authorization, identity, rooms, shared::MatrixErrorResponse, state::ApplicationState,
    syncronization,
};

pub fn build_router(application_state: ApplicationState) -> Router {
    Router::new()
        .merge(authorization::routes::routes(&application_state))
        .merge(identity::routes::routes(&application_state))
        .merge(rooms::routes::routes(&application_state))
        .merge(syncronization::routes::routes(&application_state))
        .layer(TraceLayer::new_for_http())
        .with_state(application_state.clone())
        .fallback(any(matrix_fallback))
        .with_state(application_state)
}

async fn matrix_fallback() -> impl IntoResponse {
    info!("request hit matrix fallback route");
    (
        StatusCode::NOT_FOUND,
        Json(MatrixErrorResponse {
            errcode: "M_NOT_FOUND".to_owned(),
            error: "Endpoint is not part of this server API".to_owned(),
        }),
    )
        .into_response()
}
