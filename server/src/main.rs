use axum::serve;
use persistence::migrations;
use server::{ServerState, router};
use std::error::Error as StdError;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    migrations()?;

    let state = ServerState::try_new()?;

    let listener = TcpListener::bind(&state.server_address).await?;
    let router = router(state);

    serve(listener, router).await?;
    Ok(())
}
