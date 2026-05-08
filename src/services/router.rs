use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::any};

use crate::services::{
    administration, authorization, devices_user_data, discovery, encryption, media,
    messaging_events, notifications, rooms, shared::MatrixErrorResponse, state::ApplicationState,
};

pub fn build_router(application_state: ApplicationState) -> Router {
    Router::new()
        .merge(authorization::routes::routes(application_state.clone()))
        .merge(rooms::routes::routes())
        .merge(messaging_events::routes::routes())
        .merge(encryption::routes::routes())
        .merge(media::routes::routes())
        .merge(notifications::routes::routes())
        .merge(devices_user_data::routes::routes())
        .merge(discovery::routes::routes())
        .merge(administration::routes::routes())
        .with_state(application_state.clone())
        // .layer(middleware::from_extractor_with_state::<
        //     layers::RateLimitLayer,
        //     ApplicationState,
        // >(application_state.clone()))
        // .layer(middleware::from_extractor_with_state::<
        //     layers::AuthorizationLayer,
        //     ApplicationState,
        // >(application_state.clone()))
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
