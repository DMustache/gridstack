pub(crate) mod invite;
pub(crate) mod join;
pub(crate) mod leave;
pub(crate) mod messages;
pub(crate) mod send;
pub(crate) mod state;

use axum::Router;

use crate::state::ServerState;

pub(super) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .nest("/{roomId}/invite", invite::router(state.clone()))
        .nest("/{roomId}/join", join::router(state.clone()))
        .nest("/{roomId}/leave", leave::router(state.clone()))
        .nest("/{roomId}/messages", messages::router(state.clone()))
        .nest("/{roomId}/send", send::router(state.clone()))
        .nest("/{roomId}/state", state::router(state.clone()))
        .with_state(state)
}
