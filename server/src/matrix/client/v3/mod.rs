pub(crate) mod account;
pub(crate) mod create_room;
pub(crate) mod join;
pub(crate) mod joined_rooms;
pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod rooms;
pub(crate) mod sync;

use crate::ServerState;
use axum::Router;

pub(crate) fn router(state: ServerState) -> Router<ServerState> {
    Router::new()
        .nest("/login", login::router(state.clone()))
        .nest("/logout", logout::router(state.clone()))
        .nest("/account", account::router(state.clone()))
        .nest("/sync", sync::router(state.clone()))
        .nest("/createRoom", create_room::router(state.clone()))
        .nest("/join", join::router(state.clone()))
        .nest("/joined_rooms", joined_rooms::router(state.clone()))
        .nest("/rooms", rooms::router(state.clone()))
        .with_state(state)
}
