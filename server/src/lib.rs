use axum::Router;

pub mod consts;
pub mod docs;
pub mod matrix;
pub mod services;
pub mod state;

pub use state::ServerState;

pub fn router(state: ServerState) -> Router {
    Router::new()
        .nest("/_matrix", matrix::router(state.clone()))
        .merge(docs::router())
        .with_state(state)
}
