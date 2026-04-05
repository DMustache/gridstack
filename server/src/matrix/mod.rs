use crate::ServerState;
use axum::Router;

pub(crate) mod client;

pub(crate) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .nest("/client", client::router(state.clone()))
        .with_state(state.clone())
}
