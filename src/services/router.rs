use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::any};
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest,
    DefaultOnResponse, TraceLayer,
};
use tracing::{Level, info};

use crate::services::{
    authorization, identity, rooms, shared::MatrixErrorResponse, state::ApplicationState,
    synchronization,
};

pub fn build_router(application_state: ApplicationState) -> Router {
    Router::new()
        .merge(authorization::routes::routes(&application_state))
        .merge(identity::routes::routes(&application_state))
        .merge(rooms::routes::routes(&application_state))
        .merge(synchronization::routes::routes(&application_state))
        .layer(http_trace_layer())
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

fn http_trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    DefaultOnFailure,
> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::WARN))
}
