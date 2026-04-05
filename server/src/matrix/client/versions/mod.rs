use axum::{Extension, Json, Router, extract::State, middleware::from_fn_with_state, routing::get};

mod data_transfer_objects;
use crate::{
    services::authorization::layers::{
        OptionalAuthenticatedUserExtension, optionally_authenticate_user,
    },
    state::ServerState,
};

use self::data_transfer_objects::BodyView;

pub(super) fn router(state: ServerState) -> axum::Router<ServerState> {
    Router::new()
        .route("/", get(get_versions))
        .route_layer(from_fn_with_state(
            state.clone(),
            optionally_authenticate_user,
        ))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/_matrix/client/versions",
    tag = "matrix-client",
    responses(
        (status = 200, description = "Homeserver-supported Matrix spec versions", body = crate::docs::VersionsResponseDoc)
    )
)]
pub(crate) async fn get_versions(
    State(state): State<ServerState>,
    Extension(user): OptionalAuthenticatedUserExtension,
) -> Json<BodyView> {
    Json(BodyView::new(&state, user.is_some()))
}
