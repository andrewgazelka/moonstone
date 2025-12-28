//! Focus Agent Daemon

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("focus-agent starting");

    // TODO: Connect to MDM server and receive policies
    // TODO: Start enforcement loop

    tracing::info!("focus-agent running (no policy loaded)");

    Ok(())
}
