use crate::accessibility::{get_frontmost_app, get_pids_for_bundle};
use crate::config::Config;
use crate::schedule::Schedule;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, info, warn};

pub struct Enforcer {
    config: Config,
    schedule: Schedule,
    recently_killed: HashSet<String>,
}

impl Enforcer {
    pub fn new(config: Config) -> Self {
        let schedule = Schedule::new(&config.schedule);
        Self {
            config,
            schedule,
            recently_killed: HashSet::new(),
        }
    }

    /// Run the enforcement loop - polls every 100ms
    pub async fn run(&mut self) {
        let mut ticker = interval(Duration::from_millis(100));

        loop {
            ticker.tick().await;

            if !self.schedule.is_blocked() {
                // Not in a block period, clear kill history
                self.recently_killed.clear();
                continue;
            }

            self.enforce_once();
        }
    }

    /// Single enforcement check - kills frontmost app if blocked
    pub fn enforce_once(&mut self) {
        if !self.schedule.is_blocked() {
            self.recently_killed.clear();
            return;
        }

        let Some(frontmost) = get_frontmost_app() else {
            debug!("Could not get frontmost app");
            return;
        };

        if self.config.is_app_allowed(&frontmost.bundle_id) {
            return;
        }

        // App is not allowed - kill it
        info!(
            "Killing blocked app: {} ({})",
            frontmost.name, frontmost.bundle_id
        );

        // Kill the frontmost process
        self.kill_process(frontmost.pid);

        // Also kill any other instances of the same app
        let all_pids = get_pids_for_bundle(&frontmost.bundle_id);
        for pid in all_pids {
            if pid != frontmost.pid {
                self.kill_process(pid);
            }
        }

        self.recently_killed.insert(frontmost.bundle_id);
    }

    /// Kill all currently running blocked apps (called on startup)
    pub fn kill_all_blocked(&self) {
        use crate::accessibility::get_all_running_apps;

        if !self.schedule.is_blocked() {
            return;
        }

        info!("Scanning for blocked apps to kill on startup...");

        let apps = get_all_running_apps();
        for app in apps {
            if !self.config.is_app_allowed(&app.bundle_id) && !app.bundle_id.is_empty() {
                info!("Killing blocked app on startup: {} ({})", app.name, app.bundle_id);
                self.kill_process(app.pid);
            }
        }
    }

    fn kill_process(&self, pid: i32) {
        let result = kill(Pid::from_raw(pid), Signal::SIGKILL);
        match result {
            Ok(()) => info!("SIGKILL sent to PID {}", pid),
            Err(e) => warn!("Failed to kill PID {}: {}", pid, e),
        }
    }

    /// Check if currently in a block period
    pub fn is_blocked(&self) -> bool {
        self.schedule.is_blocked()
    }

    /// Reload configuration
    pub fn reload_config(&mut self, config: Config) {
        self.schedule = Schedule::new(&config.schedule);
        self.config = config;
        info!("Configuration reloaded");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforcer_creation() {
        let config = Config::default();
        let enforcer = Enforcer::new(config);
        // Just verify it doesn't panic
        let _ = enforcer.is_blocked();
    }
}
