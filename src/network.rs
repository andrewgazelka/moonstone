use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, error, info, warn};

const PF_ANCHOR: &str = "com.moonstone";
const PF_RULES_PATH: &str = "/etc/pf.anchors/com.moonstone";

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("pfctl command failed: {0}")]
    PfctlError(String),
    #[error("DNS resolution failed for {0}")]
    DnsError(String),
}

pub struct NetworkBlocker {
    blocked_ips: HashSet<String>,
    rules_active: bool,
}

impl NetworkBlocker {
    pub fn new() -> Self {
        Self {
            blocked_ips: HashSet::new(),
            rules_active: false,
        }
    }

    /// Resolve domains to IP addresses
    pub fn resolve_domains(&mut self, blocked_domains: &[String]) -> Result<(), NetworkError> {
        self.blocked_ips.clear();

        for domain in blocked_domains {
            match (domain.as_str(), 443).to_socket_addrs() {
                Ok(addrs) => {
                    for addr in addrs {
                        let ip = addr.ip().to_string();
                        debug!("Resolved {} -> {}", domain, ip);
                        self.blocked_ips.insert(ip);
                    }
                }
                Err(e) => {
                    warn!("Failed to resolve {}: {}", domain, e);
                }
            }
        }

        info!("Resolved {} IPs to block", self.blocked_ips.len());
        Ok(())
    }

    /// Generate pf rules for blocked IPs
    fn generate_rules(&self) -> String {
        if self.blocked_ips.is_empty() {
            return String::new();
        }

        let ips: Vec<&str> = self.blocked_ips.iter().map(|s| s.as_str()).collect();
        let ip_list = ips.join(", ");

        format!(
            "# Moonstone blocking rules - DO NOT EDIT\n\
             # Generated automatically\n\
             block drop out quick proto {{ tcp, udp }} to {{ {} }}\n",
            ip_list
        )
    }

    /// Write and load pf rules
    pub fn enable_blocking(&mut self) -> Result<(), NetworkError> {
        if self.blocked_ips.is_empty() {
            info!("No IPs to block, skipping pf setup");
            return Ok(());
        }

        let rules = self.generate_rules();

        // Write rules to anchor file
        std::fs::write(PF_RULES_PATH, &rules)?;
        info!("Wrote pf rules to {}", PF_RULES_PATH);

        // Load the anchor
        let output = Command::new("pfctl")
            .args(["-a", PF_ANCHOR, "-f", PF_RULES_PATH])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("pfctl load failed: {}", stderr);
            return Err(NetworkError::PfctlError(stderr.to_string()));
        }

        // Enable pf if not already enabled
        let _ = Command::new("pfctl").args(["-e"]).output();

        self.rules_active = true;
        info!("Network blocking enabled");
        Ok(())
    }

    /// Remove pf rules
    pub fn disable_blocking(&mut self) -> Result<(), NetworkError> {
        if !self.rules_active {
            return Ok(());
        }

        // Flush the anchor
        let output = Command::new("pfctl")
            .args(["-a", PF_ANCHOR, "-F", "all"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("pfctl flush warning: {}", stderr);
        }

        // Remove the rules file
        let _ = std::fs::remove_file(PF_RULES_PATH);

        self.rules_active = false;
        info!("Network blocking disabled");
        Ok(())
    }

    /// Check if rules are currently active
    pub fn is_active(&self) -> bool {
        self.rules_active
    }
}

impl Default for NetworkBlocker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for NetworkBlocker {
    fn drop(&mut self) {
        if self.rules_active {
            let _ = self.disable_blocking();
        }
    }
}

/// Setup pf anchor in main pf.conf if not present
pub fn setup_pf_anchor() -> Result<(), NetworkError> {
    let pf_conf = std::fs::read_to_string("/etc/pf.conf").unwrap_or_default();

    if pf_conf.contains(PF_ANCHOR) {
        debug!("pf anchor already configured");
        return Ok(());
    }

    // Append anchor reference to pf.conf
    let new_conf = format!(
        "{}\n\
         # Moonstone network blocking\n\
         anchor \"{}\"\n\
         load anchor \"{}\" from \"{}\"\n",
        pf_conf, PF_ANCHOR, PF_ANCHOR, PF_RULES_PATH
    );

    std::fs::write("/etc/pf.conf", new_conf)?;
    info!("Added moonstone anchor to pf.conf");

    // Reload pf
    let _ = Command::new("pfctl").args(["-f", "/etc/pf.conf"]).output();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rules() {
        let mut blocker = NetworkBlocker::new();
        blocker.blocked_ips.insert("1.2.3.4".to_string());
        blocker.blocked_ips.insert("5.6.7.8".to_string());

        let rules = blocker.generate_rules();
        assert!(rules.contains("1.2.3.4"));
        assert!(rules.contains("5.6.7.8"));
        assert!(rules.contains("block drop"));
    }
}
