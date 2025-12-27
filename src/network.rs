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
    allowed_ips: HashSet<String>,
    rules_active: bool,
}

impl NetworkBlocker {
    pub fn new() -> Self {
        Self {
            allowed_ips: HashSet::new(),
            rules_active: false,
        }
    }

    /// Resolve allowed domains to IP addresses
    pub fn resolve_allowed_domains(&mut self, allowed_domains: &[String]) -> Result<(), NetworkError> {
        self.allowed_ips.clear();

        for domain in allowed_domains {
            match (domain.as_str(), 443).to_socket_addrs() {
                Ok(addrs) => {
                    for addr in addrs {
                        let ip = addr.ip().to_string();
                        debug!("Resolved {} -> {}", domain, ip);
                        self.allowed_ips.insert(ip);
                    }
                }
                Err(e) => {
                    warn!("Failed to resolve {}: {}", domain, e);
                }
            }
        }

        info!("Resolved {} allowed IPs", self.allowed_ips.len());
        Ok(())
    }

    /// Generate pf rules - allowlist mode: block all except allowed IPs
    fn generate_rules(&self) -> String {
        let mut rules = String::from(
            "# Moonstone blocking rules - DO NOT EDIT\n\
             # Generated automatically\n\
             # Allow loopback\n\
             pass out quick on lo0 all\n\
             # Allow DNS for resolution\n\
             pass out quick proto { tcp, udp } to any port 53\n\
             # Allow DHCP\n\
             pass out quick proto udp to any port { 67, 68 }\n",
        );

        if !self.allowed_ips.is_empty() {
            let ips: Vec<&str> = self.allowed_ips.iter().map(|s| s.as_str()).collect();
            let ip_list = ips.join(", ");
            rules.push_str(&format!(
                "# Allow specific domains\n\
                 pass out quick proto {{ tcp, udp }} to {{ {} }}\n",
                ip_list
            ));
        }

        // Block everything else
        rules.push_str(
            "# Block all other outbound traffic\n\
             block drop out quick proto { tcp, udp } all\n",
        );

        rules
    }

    /// Write and load pf rules
    pub fn enable_blocking(&mut self) -> Result<(), NetworkError> {
        if self.allowed_ips.is_empty() {
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
        blocker.allowed_ips.insert("1.2.3.4".to_string());
        blocker.allowed_ips.insert("5.6.7.8".to_string());

        let rules = blocker.generate_rules();
        // Should pass allowed IPs
        assert!(rules.contains("1.2.3.4"));
        assert!(rules.contains("5.6.7.8"));
        assert!(rules.contains("pass out quick proto"));
        // Should block everything else
        assert!(rules.contains("block drop out quick"));
    }
}
