//! App enforcement via SIGKILL.

use color_eyre::eyre::WrapErr as _;
use std::collections::HashSet;

use crate::accessibility;
use crate::policy::{AppPolicy, FocusPolicy};

/// App enforcer that kills disallowed apps.
pub struct AppEnforcer {
    /// Recently killed bundle IDs (to avoid spam).
    recently_killed: HashSet<String>,
}

impl AppEnforcer {
    /// Create a new app enforcer.
    pub fn new() -> Self {
        Self {
            recently_killed: HashSet::new(),
        }
    }

    /// Enforce the policy by killing disallowed apps.
    pub fn enforce(&mut self, policy: &FocusPolicy) -> color_eyre::eyre::Result<()> {
        // Check if schedule is active
        if !policy.schedule.is_active() {
            self.recently_killed.clear();
            return Ok(());
        }

        // Get frontmost app
        let frontmost = accessibility::get_frontmost_app()?;

        if let Some((bundle_id, _name)) = frontmost {
            if !policy.apps.is_allowed(&bundle_id) {
                self.kill_app(&bundle_id)?;
            }
        }

        Ok(())
    }

    /// Kill all processes with the given bundle ID.
    fn kill_app(&mut self, bundle_id: &str) -> color_eyre::eyre::Result<()> {
        if self.recently_killed.contains(bundle_id) {
            return Ok(());
        }

        let pids = accessibility::get_pids_for_bundle(bundle_id)?;

        for pid in pids {
            tracing::info!(bundle_id = %bundle_id, pid = pid, "killing blocked app");

            // SIGKILL
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }

        self.recently_killed.insert(bundle_id.to_string());

        // Clear after a short delay to allow re-killing if reopened
        // In production, this would be timer-based
        if self.recently_killed.len() > 10 {
            self.recently_killed.clear();
        }

        Ok(())
    }
}

impl Default for AppEnforcer {
    fn default() -> Self {
        Self::new()
    }
}
