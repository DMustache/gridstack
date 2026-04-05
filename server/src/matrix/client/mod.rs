use axum::Router;

use crate::ServerState;

pub(crate) mod register;
pub(crate) mod v3;
pub(crate) mod versions;

pub(crate) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .nest("/register", register::router(state.clone()))
        .nest("/v3", v3::router(state.clone()))
        .nest("/versions", versions::router(state.clone()))
        .with_state(state)
}
