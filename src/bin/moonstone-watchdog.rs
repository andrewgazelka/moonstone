use moonstone::config::{Config, TamperResponse};
use moonstone::ipc::IpcClient;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("moonstone=info".parse().unwrap()),
        )
        .init();

    info!("Moonstone watchdog starting...");

    // Load configuration for tamper response setting
    let config = Config::load().unwrap_or_default();
    let tamper_response = config.hardcore.on_tamper.clone();

    let running = Arc::new(AtomicBool::new(true));

    // Setup signal handlers
    let running_clone = running.clone();
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
        }

        running_clone.store(false, Ordering::Relaxed);
    });

    // Wait a moment for daemon to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Start heartbeat monitoring
    let mut client = IpcClient::new();

    info!("Moonstone watchdog running - monitoring daemon");

    // Run heartbeat loop
    let err = client.run_heartbeat_loop().await;

    // If we get here, the daemon stopped responding
    if running.load(Ordering::Relaxed) {
        // We didn't receive a shutdown signal, so this is a tamper
        error!("Heartbeat loop exited with error: {}", err);
        trigger_tamper_response(&tamper_response);
    } else {
        info!("Watchdog shutting down normally");
    }
}

/// Trigger the configured tamper response
fn trigger_tamper_response(response: &TamperResponse) {
    match response {
        TamperResponse::Sleep => {
            error!("TAMPER DETECTED! Daemon not responding. Sleeping system...");
            // Give a brief moment for the log to flush
            std::thread::sleep(std::time::Duration::from_millis(100));
            let result = Command::new("pmset").args(["sleepnow"]).spawn();
            if let Err(e) = result {
                error!("Failed to sleep: {}", e);
                // Fallback to lock
                let _ = Command::new("pmset").args(["displaysleepnow"]).spawn();
            }
        }
        TamperResponse::Shutdown => {
            error!("TAMPER DETECTED! Daemon not responding. Shutting down...");
            std::thread::sleep(std::time::Duration::from_millis(100));
            let result = Command::new("shutdown").args(["-h", "now"]).spawn();
            if let Err(e) = result {
                error!("Failed to shutdown: {}", e);
                // Fallback to sleep
                let _ = Command::new("pmset").args(["sleepnow"]).spawn();
            }
        }
        TamperResponse::Lock => {
            error!("TAMPER DETECTED! Daemon not responding. Locking screen...");
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = Command::new("pmset").args(["displaysleepnow"]).spawn();
        }
    }
}
