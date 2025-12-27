use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Config file not found at {0}")]
    NotFound(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub schedule: ScheduleConfig,
    pub apps: AppsConfig,
    pub websites: WebsitesConfig,
    pub hardcore: HardcoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub blocks: Vec<BlockPeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPeriod {
    pub start: String, // "HH:MM" format
    pub end: String,   // "HH:MM" format
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppsConfig {
    pub mode: BlockMode,
    pub allowed: Vec<String>, // Bundle IDs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsitesConfig {
    pub mode: BlockMode,
    pub allowed: Vec<String>, // Domains
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlockMode {
    Allowlist,
    Blocklist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardcoreConfig {
    #[serde(default = "default_on_tamper")]
    pub on_tamper: TamperResponse,
    #[serde(default = "default_challenge_duration")]
    pub emergency_disable_challenge: u32, // seconds
    #[serde(default = "default_lock_config")]
    pub lock_config: bool,
    #[serde(default = "default_kill_behavior")]
    pub kill_behavior: KillBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TamperResponse {
    #[default]
    Sleep,
    Shutdown,
    Lock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum KillBehavior {
    #[default]
    Instant,
    Notify,
}

fn default_on_tamper() -> TamperResponse {
    TamperResponse::Sleep
}

fn default_challenge_duration() -> u32 {
    300 // 5 minutes
}

fn default_lock_config() -> bool {
    true
}

fn default_kill_behavior() -> KillBehavior {
    KillBehavior::Instant
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path();
        if !config_path.exists() {
            return Err(ConfigError::NotFound(config_path));
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/etc"))
            .join("moonstone")
            .join("config.toml")
    }

    pub fn is_app_allowed(&self, bundle_id: &str) -> bool {
        // System apps that should always be allowed
        const SYSTEM_ESSENTIALS: &[&str] = &[
            "com.apple.loginwindow",
            "com.apple.SecurityAgent",
            "com.apple.dock",
            "com.apple.WindowManager",
            "com.apple.notificationcenterui",
            "com.apple.Spotlight",
        ];

        if SYSTEM_ESSENTIALS.contains(&bundle_id) {
            return true;
        }

        match self.apps.mode {
            BlockMode::Allowlist => self.apps.allowed.iter().any(|a| a == bundle_id),
            BlockMode::Blocklist => !self.apps.allowed.iter().any(|a| a == bundle_id),
        }
    }

    pub fn is_website_allowed(&self, domain: &str) -> bool {
        match self.websites.mode {
            BlockMode::Allowlist => self.websites.allowed.iter().any(|d| {
                domain == *d || domain.ends_with(&format!(".{}", d))
            }),
            BlockMode::Blocklist => !self.websites.allowed.iter().any(|d| {
                domain == *d || domain.ends_with(&format!(".{}", d))
            }),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schedule: ScheduleConfig {
                blocks: vec![
                    BlockPeriod {
                        start: "04:00".to_string(),
                        end: "17:00".to_string(),
                    },
                    BlockPeriod {
                        start: "17:10".to_string(),
                        end: "03:59".to_string(),
                    },
                ],
            },
            apps: AppsConfig {
                mode: BlockMode::Allowlist,
                allowed: vec![
                    "com.apple.facetime".to_string(),
                    "com.mitchellh.ghostty".to_string(),
                    "net.sourceforge.skim-app.skim".to_string(),
                    "com.beeper.beeper-desktop".to_string(),
                    "com.apple.Music".to_string(),
                    "dev.orbstack.OrbStack".to_string(),
                    "com.flexibits.fantastical2.mac".to_string(),
                    "com.apple.Terminal".to_string(),
                    "com.apple.finder".to_string(),
                    "com.apple.systempreferences".to_string(),
                ],
            },
            websites: WebsitesConfig {
                mode: BlockMode::Allowlist,
                allowed: vec![
                    "github.com".to_string(),
                    "docs.rs".to_string(),
                    "crates.io".to_string(),
                    "localhost".to_string(),
                ],
            },
            hardcore: HardcoreConfig {
                on_tamper: TamperResponse::Sleep,
                emergency_disable_challenge: 300,
                lock_config: true,
                kill_behavior: KillBehavior::Instant,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.is_app_allowed("com.apple.facetime"));
        assert!(!config.is_app_allowed("com.twitter.twitter"));
        assert!(config.is_website_allowed("github.com"));
        assert!(!config.is_website_allowed("twitter.com"));
    }
}
