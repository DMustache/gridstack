use std::sync::Arc;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use client::{
    infrastructure::persistence::sqlite_session_repository::SqliteSessionRepository,
    ui::ClientDesktopApplication,
};

fn main() {
    let log_file_path = initialize_logging();
    info!("client startup initiated");
    info!(path = %log_file_path.display(), "file logging initialized");

    let database_path = resolve_database_path();

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            error!(%error, "failed to initialize async runtime");
            eprintln!("Failed to initialize async runtime: {error}");
            return;
        }
    };

    let session_repository = match runtime.block_on(SqliteSessionRepository::new(&database_path)) {
        Ok(repository) => Arc::new(repository),
        Err(error) => {
            error!(%error, "failed to initialize sqlite session repository");
            eprintln!("Failed to initialize SQLite session repository: {error}");
            return;
        }
    };

    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let repository = Arc::clone(&session_repository);
    if let Err(error) = eframe::run_native(
        "Gridstack Client",
        native_options,
        Box::new(move |_creation_context| {
            let app = ClientDesktopApplication::new(Arc::clone(&repository))
                .expect("failed to build desktop application");
            Ok(Box::new(app))
        }),
    ) {
        error!(%error, "desktop runtime failed");
        eprintln!("Failed to run desktop application: {error}");
    }
}

fn initialize_logging() -> PathBuf {
    static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

    let log_file_path = resolve_log_file_path();
    if let Some(parent_directory) = log_file_path.parent() {
        let _ = fs::create_dir_all(parent_directory);
    }

    let file_appender = tracing_appender::rolling::never(
        log_file_path
            .parent()
            .unwrap_or_else(|| Path::new(".")),
        log_file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("client.log"),
    );
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking_writer)
        .with_target(false)
        .with_ansi(false)
        .try_init();

    let _ = writeln_fallback(&log_file_path, "logger initialized");
    log_file_path
}

fn resolve_log_file_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return Path::new(&local_app_data)
                .join("GridstackClient")
                .join("logs")
                .join("client.log");
        }
    }

    if let Ok(current_directory) = std::env::current_dir() {
        return current_directory
            .join("generated")
            .join("logs")
            .join("client-app.log");
    }

    PathBuf::from("client.log")
}

fn resolve_database_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return Path::new(&local_app_data)
                .join("GridstackClient")
                .join("data")
                .join("client.db");
        }
    }

    if let Ok(current_directory) = std::env::current_dir() {
        return current_directory.join("client.db");
    }

    PathBuf::from("client.db")
}

fn writeln_fallback(path: &Path, message: &str) -> Result<(), std::io::Error> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{message}")?;
    Ok(())
}
