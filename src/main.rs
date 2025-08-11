mod config;
mod error;
mod routes;
mod state;
mod telemetry;

use config::AppConfig;
use error::InitializationError;
use state::AppState;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file first, so RUST_LOG is available for tracing
    if let Err(err) = dotenvy::dotenv() {
        // Don't fail if .env file doesn't exist, just log it
        eprintln!("No .env file found or error loading it: {}", err);
    }

    telemetry::init_tracing();

    if let Err(error) = run_app().await {
        tracing::error!(?error, "Initialization failed");
        std::process::exit(1);
    }
}

async fn run_app() -> Result<(), InitializationError> {
    let cfg = AppConfig::load()?;
    let state = AppState::new();
    let app = routes::app_router(state);

    let addr: SocketAddr = cfg.bind_addr()?;
    tracing::warn!(%addr, "starting HTTP server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    tracing::warn!(%local_addr, "listening");

    let app_state = AppState::new();
    let app = routes::app_router(app_state);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    // Wait for CTRL+C
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        term.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
