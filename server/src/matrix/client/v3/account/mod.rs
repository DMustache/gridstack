use axum::Router;

use crate::state::ServerState;

pub(crate) mod whoami;

pub(crate) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .nest("/whoami", whoami::router(state.clone()))
        .with_state(state)
}
