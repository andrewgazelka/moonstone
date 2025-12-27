use moonstone::config::{Config, TamperResponse};
use moonstone::enforcer::Enforcer;
use moonstone::ipc::{IpcServer, Message};
use moonstone::network::NetworkBlocker;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("moonstone=info".parse().unwrap()),
        )
        .init();

    info!("Moonstone daemon starting...");

    // Load configuration
    let config = match Config::load() {
        Ok(c) => {
            info!("Configuration loaded from {:?}", Config::config_path());
            c
        }
        Err(e) => {
            warn!("Failed to load config: {}. Using defaults.", e);
            Config::default()
        }
    };

    // Start IPC server
    let ipc_server = match IpcServer::new() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to start IPC server: {}", e);
            std::process::exit(1);
        }
    };

    let mut ipc_rx = ipc_server.run();
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

    // Initialize network blocker
    let mut network_blocker = NetworkBlocker::new();

    // Resolve allowed domains from config (allowlist mode: block everything except these)
    let allowed_domains = config.websites.allowed.clone();
    if let Err(e) = network_blocker.resolve_allowed_domains(&allowed_domains) {
        warn!("Failed to resolve allowed domains: {}", e);
    }

    // Initialize enforcer
    let mut enforcer = Enforcer::new(config.clone());

    // Kill all blocked apps on startup
    enforcer.kill_all_blocked();

    // Track emergency disable state
    let emergency_disabled = Arc::new(AtomicBool::new(false));

    info!("Moonstone daemon running");

    // Main loop
    loop {
        tokio::select! {
            // Handle IPC messages
            Some((msg, _stream)) = ipc_rx.recv() => {
                match msg {
                    Message::Heartbeat => {
                        // Watchdog is alive, nothing to do
                    }
                    Message::Status => {
                        info!("Status requested");
                    }
                    Message::Shutdown => {
                        info!("Shutdown requested");
                        running.store(false, Ordering::Relaxed);
                    }
                    Message::EmergencyDisable => {
                        info!("Emergency disable activated");
                        emergency_disabled.store(true, Ordering::Relaxed);
                        // Disable network blocking
                        let _ = network_blocker.disable_blocking();
                    }
                }
            }

            // Run enforcer tick (100ms intervals handled internally)
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                if emergency_disabled.load(Ordering::Relaxed) {
                    // Check if we should re-enable (new block period started)
                    if !enforcer.is_blocked() {
                        // We're in an allowed period, reset emergency disable
                        // It will re-activate when next block period starts
                    }
                    continue;
                }

                // Run app enforcement
                enforcer.enforce_once();

                // Check if we should enable/disable network blocking
                if enforcer.is_blocked() {
                    if !network_blocker.is_active() {
                        if let Err(e) = network_blocker.enable_blocking() {
                            warn!("Failed to enable network blocking: {}", e);
                        }
                    }
                } else if network_blocker.is_active() {
                    if let Err(e) = network_blocker.disable_blocking() {
                        warn!("Failed to disable network blocking: {}", e);
                    }
                    // Reset emergency disable when leaving block period
                    emergency_disabled.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    // Cleanup
    info!("Moonstone daemon shutting down");
    let _ = network_blocker.disable_blocking();
    ipc_server.stop();
}

/// Trigger tamper response
#[allow(dead_code)]
fn trigger_tamper_response(response: &TamperResponse) {
    match response {
        TamperResponse::Sleep => {
            error!("Tamper detected! Sleeping system...");
            let _ = Command::new("pmset").args(["sleepnow"]).spawn();
        }
        TamperResponse::Shutdown => {
            error!("Tamper detected! Shutting down...");
            let _ = Command::new("shutdown").args(["-h", "now"]).spawn();
        }
        TamperResponse::Lock => {
            error!("Tamper detected! Locking screen...");
            let _ = Command::new("pmset").args(["displaysleepnow"]).spawn();
        }
    }
}

