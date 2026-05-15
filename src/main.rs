use axum::serve;
use tokio::net::TcpListener;

use crate::{
    infrastructure::configuration::ApplicationConfiguration,
    services::{router::build_router, state::ApplicationState},
};

pub mod infrastructure;
pub mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let application_configuration = ApplicationConfiguration::load_from_path("configuration.toml")?;

    let socket_address = application_configuration.server.socket_address()?;
    let application_state = ApplicationState::new(&application_configuration);
    let router = build_router(application_state);

    let tcp_listener = TcpListener::bind(socket_address).await?;
    serve(tcp_listener, router).await?;

    Ok(())
}
