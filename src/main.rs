use axum::serve;
use tokio::net::TcpListener;

pub mod infrastructure;
pub mod services;

use services::{configuration::load_configuration, router::build_router, state::ApplicationState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let application_configuration = load_configuration("configuration.toml")?;

    let socket_address = application_configuration.server.socket_address()?;
    let application_state = ApplicationState::new(&application_configuration);
    let router = build_router(application_state);

    let tcp_listener = TcpListener::bind(socket_address).await?;
    serve(tcp_listener, router).await?;

    Ok(())
}
