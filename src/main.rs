use axum::serve;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

use crate::{
    infrastructure::configuration::ApplicationConfiguration,
    services::{router::build_router, state::ApplicationState},
};

pub mod infrastructure;
pub mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_logging();

    let application_configuration = ApplicationConfiguration::load_from_path("configuration.toml")?;
    info!("application configuration loaded");

    let socket_address = application_configuration.server.socket_address()?;
    let application_state = ApplicationState::new(&application_configuration);
    let router = build_router(application_state.clone());

    let tcp_listener = TcpListener::bind(socket_address).await?;
    info!(address = %socket_address, "server started");
    if let Err(error) = serve(tcp_listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!(error = %error, "server failed");
        return Err(error.into());
    }
    application_state.flush_runtime_state();

    Ok(())
}

fn initialize_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));
    fmt()
        .with_env_filter(env_filter)
        .with_file(true)
        .with_line_number(true)
        .init();
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_ok() {
        info!("shutdown signal received");
    }
}
