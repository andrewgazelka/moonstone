//! Focus policy types.

use chrono::Datelike as _;

/// Focus policy received from MDM server.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FocusPolicy {
    /// Schedule defining when blocking is active.
    pub schedule: Schedule,
    /// App blocking configuration.
    pub apps: AppPolicy,
    /// Website blocking configuration.
    pub websites: WebsitePolicy,
}

/// Time-based schedule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    /// Time periods when blocking is active.
    pub periods: Vec<TimePeriod>,
}

/// A time period.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimePeriod {
    /// Start time (HH:MM format).
    pub start: String,
    /// End time (HH:MM format).
    pub end: String,
    /// Days of week (0 = Sunday, 6 = Saturday). Empty = all days.
    #[serde(default)]
    pub days: Vec<u8>,
}

/// App blocking policy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode")]
pub enum AppPolicy {
    /// Block all apps except those in the list.
    #[serde(rename = "allowlist")]
    Allowlist { apps: Vec<String> },
    /// Allow all apps except those in the list.
    #[serde(rename = "blocklist")]
    Blocklist { apps: Vec<String> },
}

/// Website blocking policy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode")]
pub enum WebsitePolicy {
    /// Block all websites except those in the list.
    #[serde(rename = "allowlist")]
    Allowlist { domains: Vec<String> },
    /// Allow all websites except those in the list.
    #[serde(rename = "blocklist")]
    Blocklist { domains: Vec<String> },
}

impl Schedule {
    /// Check if the current time is within any active period.
    pub fn is_active(&self) -> bool {
        let now = chrono::Local::now();
        let current_time = now.format("%H:%M").to_string();
        let current_day = now.weekday().num_days_from_sunday() as u8;

        for period in &self.periods {
            // Check day constraint
            if !period.days.is_empty() && !period.days.contains(&current_day) {
                continue;
            }

            // Check time constraint (handles midnight crossing)
            if period.start <= period.end {
                // Normal period (e.g., 09:00 to 17:00)
                if current_time >= period.start && current_time <= period.end {
                    return true;
                }
            } else {
                // Midnight-crossing period (e.g., 22:00 to 06:00)
                if current_time >= period.start || current_time <= period.end {
                    return true;
                }
            }
        }

        false
    }
}

impl AppPolicy {
    /// Check if an app (by bundle ID) is allowed.
    pub fn is_allowed(&self, bundle_id: &str) -> bool {
        // System essentials are always allowed
        const SYSTEM_ESSENTIALS: &[&str] = &[
            "com.apple.dock",
            "com.apple.finder",
            "com.apple.loginwindow",
            "com.apple.SecurityAgent",
            "com.apple.WindowManager",
            "com.apple.systemuiserver",
            "com.apple.controlcenter",
            "com.apple.notificationcenterui",
        ];

        if SYSTEM_ESSENTIALS.iter().any(|&s| bundle_id == s) {
            return true;
        }

        match self {
            Self::Allowlist { apps } => apps.iter().any(|a| a == bundle_id),
            Self::Blocklist { apps } => !apps.iter().any(|a| a == bundle_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_allowlist() {
        let policy = AppPolicy::Allowlist {
            apps: vec!["com.apple.Terminal".into()],
        };

        assert!(policy.is_allowed("com.apple.Terminal"));
        assert!(!policy.is_allowed("com.apple.Safari"));
        // System essentials always allowed
        assert!(policy.is_allowed("com.apple.finder"));
    }

    #[test]
    fn test_app_blocklist() {
        let policy = AppPolicy::Blocklist {
            apps: vec!["com.twitter.twitter".into()],
        };

        assert!(!policy.is_allowed("com.twitter.twitter"));
        assert!(policy.is_allowed("com.apple.Terminal"));
    }
}
