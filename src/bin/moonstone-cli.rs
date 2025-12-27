use clap::{Parser, Subcommand};
use moonstone::challenge::run_challenge;
use moonstone::config::Config;
use moonstone::ipc::{is_daemon_running, IpcClient, Message};
use moonstone::schedule::Schedule;

#[derive(Parser)]
#[command(name = "moonstone")]
#[command(about = "Hardcore macOS focus blocker", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current status
    Status,

    /// Emergency disable (requires typing challenge)
    EmergencyDisable,

    /// Show current configuration
    Config,

    /// Check if currently in a block period
    IsBlocked,

    /// Show time until next allowed period
    TimeLeft,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cmd_status(),
        Commands::EmergencyDisable => cmd_emergency_disable(),
        Commands::Config => cmd_config(),
        Commands::IsBlocked => cmd_is_blocked(),
        Commands::TimeLeft => cmd_time_left(),
    }
}

fn cmd_status() {
    println!("=== Moonstone Status ===\n");

    // Check daemon
    let daemon_running = is_daemon_running();
    println!(
        "Daemon:    {}",
        if daemon_running { "RUNNING" } else { "STOPPED" }
    );

    // Load config and check schedule
    match Config::load() {
        Ok(config) => {
            let schedule = Schedule::new(&config.schedule);
            let is_blocked = schedule.is_blocked();

            println!(
                "Blocking:  {}",
                if is_blocked { "ACTIVE" } else { "INACTIVE" }
            );

            if is_blocked {
                if let Some(secs) = schedule.seconds_until_unblock() {
                    let hours = secs / 3600;
                    let mins = (secs % 3600) / 60;
                    println!("Time left: {}h {}m", hours, mins);
                }
            }

            println!("\nAllowed apps: {}", config.apps.allowed.len());
            println!("Allowed sites: {}", config.websites.allowed.len());
            println!("Tamper response: {:?}", config.hardcore.on_tamper);
        }
        Err(e) => {
            println!("Config:    ERROR - {}", e);
        }
    }
}

fn cmd_emergency_disable() {
    println!("=== Moonstone Emergency Disable ===\n");

    if !is_daemon_running() {
        println!("Daemon is not running. Nothing to disable.");
        return;
    }

    // Load config to get challenge duration
    let config = Config::load().unwrap_or_default();
    let duration = config.hardcore.emergency_disable_challenge;

    println!("This will disable Moonstone until the next block period.");
    println!("You must complete a {}-second typing challenge.\n", duration);

    // Run the challenge
    if run_challenge(duration) {
        // Challenge passed, send disable message to daemon
        let client = IpcClient::new();
        match client.send(Message::EmergencyDisable) {
            Ok(()) => {
                println!("Moonstone has been disabled.");
                println!("Blocking will resume at the start of the next block period.");
            }
            Err(e) => {
                println!("Failed to communicate with daemon: {}", e);
            }
        }
    } else {
        println!("\nChallenge failed. Moonstone remains active.");
    }
}

fn cmd_config() {
    match Config::load() {
        Ok(config) => {
            println!("=== Moonstone Configuration ===\n");
            println!("Config file: {:?}\n", Config::config_path());

            println!("[Schedule]");
            for (i, block) in config.schedule.blocks.iter().enumerate() {
                println!("  Block {}: {} - {}", i + 1, block.start, block.end);
            }

            println!("\n[Apps] (mode: {:?})", config.apps.mode);
            for app in &config.apps.allowed {
                println!("  - {}", app);
            }

            println!("\n[Websites] (mode: {:?})", config.websites.mode);
            for site in &config.websites.allowed {
                println!("  - {}", site);
            }

            println!("\n[Hardcore]");
            println!("  on_tamper: {:?}", config.hardcore.on_tamper);
            println!(
                "  emergency_challenge: {}s",
                config.hardcore.emergency_disable_challenge
            );
            println!("  lock_config: {}", config.hardcore.lock_config);
            println!("  kill_behavior: {:?}", config.hardcore.kill_behavior);
        }
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            eprintln!("\nUsing default configuration.");
            std::process::exit(1);
        }
    }
}

fn cmd_is_blocked() {
    let config = Config::load().unwrap_or_default();
    let schedule = Schedule::new(&config.schedule);

    if schedule.is_blocked() {
        println!("BLOCKED");
        std::process::exit(0);
    } else {
        println!("ALLOWED");
        std::process::exit(1);
    }
}

fn cmd_time_left() {
    let config = Config::load().unwrap_or_default();
    let schedule = Schedule::new(&config.schedule);

    if !schedule.is_blocked() {
        println!("Not currently blocked");
        return;
    }

    match schedule.seconds_until_unblock() {
        Some(secs) => {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            let secs_remaining = secs % 60;

            if hours > 0 {
                println!("{}h {}m {}s", hours, mins, secs_remaining);
            } else if mins > 0 {
                println!("{}m {}s", mins, secs_remaining);
            } else {
                println!("{}s", secs_remaining);
            }
        }
        None => {
            println!("Unknown");
        }
    }
}
