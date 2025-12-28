//! Focus Server - MDM server with focus policy management.

use std::net::SocketAddr;

use axum::Router;
use color_eyre::eyre::WrapErr as _;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("focus-server starting");

    // Initialize storage
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:focus.db".to_string());

    let storage =
        mdm_storage::SqliteStorage::new(&database_url).wrap_err("failed to initialize storage")?;

    storage
        .run_migrations()
        .wrap_err("failed to run migrations")?;

    // Create MDM service
    let service = mdm_service::NanoMdm::new(storage.clone());

    // Build router
    let app = Router::new()
        .merge(mdm_http::mdm_router(service))
        .merge(mdm_http::api_router(storage.clone()))
        .merge(focus_server::api::focus_router(storage))
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!(addr = %addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .wrap_err("failed to bind")?;

    axum::serve(listener, app).await.wrap_err("server error")?;

    Ok(())
}
