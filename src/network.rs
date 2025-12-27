use crate::config::BlockMode;
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
    blocked_ips: HashSet<String>,
    mode: BlockMode,
    rules_active: bool,
}

impl NetworkBlocker {
    pub fn new() -> Self {
        Self {
            allowed_ips: HashSet::new(),
            blocked_ips: HashSet::new(),
            mode: BlockMode::Allowlist,
            rules_active: false,
        }
    }

    /// Set the blocking mode
    pub fn set_mode(&mut self, mode: BlockMode) {
        self.mode = mode;
    }

    /// Resolve allowed domains to IP addresses (for allowlist mode)
    pub fn resolve_allowed_domains(&mut self, allowed_domains: &[String]) -> Result<(), NetworkError> {
        self.allowed_ips.clear();

        for domain in allowed_domains {
            self.resolve_domain_to_set(domain, &mut self.allowed_ips.clone())
                .map(|ips| self.allowed_ips.extend(ips))
                .ok();
        }

        info!("Resolved {} allowed IPs", self.allowed_ips.len());
        Ok(())
    }

    /// Resolve blocked domains to IP addresses (for blocklist mode)
    pub fn resolve_blocked_domains(&mut self, blocked_domains: &[String]) -> Result<(), NetworkError> {
        self.blocked_ips.clear();

        for domain in blocked_domains {
            self.resolve_domain_to_set(domain, &mut self.blocked_ips.clone())
                .map(|ips| self.blocked_ips.extend(ips))
                .ok();
        }

        info!("Resolved {} blocked IPs", self.blocked_ips.len());
        Ok(())
    }

    fn resolve_domain_to_set(&self, domain: &str, _existing: &mut HashSet<String>) -> Result<HashSet<String>, NetworkError> {
        let mut ips = HashSet::new();
        match (domain, 443).to_socket_addrs() {
            Ok(addrs) => {
                for addr in addrs {
                    let ip = addr.ip().to_string();
                    debug!("Resolved {} -> {}", domain, ip);
                    ips.insert(ip);
                }
            }
            Err(e) => {
                warn!("Failed to resolve {}: {}", domain, e);
            }
        }
        Ok(ips)
    }

    /// Generate pf rules based on mode
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

        match self.mode {
            BlockMode::Allowlist => {
                // Allowlist mode: allow specific IPs, block everything else
                rules.push_str(
                    "# Allow GitHub CIDR ranges (they use many IPs)\n\
                     pass out quick proto { tcp, udp } to 140.82.112.0/20\n\
                     pass out quick proto { tcp, udp } to 185.199.108.0/22\n\
                     pass out quick proto { tcp, udp } to 192.30.252.0/22\n\
                     # Allow Apple services (Music, iCloud, etc)\n\
                     pass out quick proto { tcp, udp } to 17.0.0.0/8\n\
                     # Allow Akamai CDN (used by Apple Music, etc)\n\
                     pass out quick proto { tcp, udp } to 23.0.0.0/8\n\
                     pass out quick proto { tcp, udp } to 104.64.0.0/10\n",
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
            }
            BlockMode::Blocklist => {
                // Blocklist mode: block specific IPs, allow everything else
                // Common social media/distraction CIDR ranges
                rules.push_str(
                    "# Block common distraction site ranges\n\
                     # Twitter/X\n\
                     block drop out quick proto { tcp, udp } to 104.244.42.0/24\n\
                     block drop out quick proto { tcp, udp } to 104.244.43.0/24\n\
                     # TikTok\n\
                     block drop out quick proto { tcp, udp } to 142.250.0.0/16\n\
                     # Meta (Facebook, Instagram)\n\
                     block drop out quick proto { tcp, udp } to 157.240.0.0/16\n\
                     block drop out quick proto { tcp, udp } to 31.13.24.0/21\n\
                     block drop out quick proto { tcp, udp } to 31.13.64.0/18\n\
                     # Reddit\n\
                     block drop out quick proto { tcp, udp } to 151.101.0.0/16\n",
                );

                if !self.blocked_ips.is_empty() {
                    // Block specific resolved IPs
                    for ip in &self.blocked_ips {
                        rules.push_str(&format!(
                            "block drop out quick proto {{ tcp, udp }} to {}\n",
                            ip
                        ));
                    }
                }

                // Allow everything else
                rules.push_str(
                    "# Allow all other outbound traffic\n\
                     pass out quick all\n",
                );
            }
        }

        rules
    }

    /// Write and load pf rules
    pub fn enable_blocking(&mut self) -> Result<(), NetworkError> {
        // In allowlist mode, we need allowed IPs to know what to pass
        // In blocklist mode, we can block even without specific IPs (using hardcoded ranges)
        if self.mode == BlockMode::Allowlist && self.allowed_ips.is_empty() {
            info!("No allowed IPs configured, skipping pf setup");
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

/// Known CIDR ranges for common distraction sites
/// These are used in blocklist mode as a fallback
pub const DISTRACTION_CIDRS: &[(&str, &str)] = &[
    // Twitter/X
    ("twitter.com", "104.244.42.0/24"),
    ("twitter.com", "104.244.43.0/24"),
    // Meta (Facebook, Instagram, WhatsApp)
    ("facebook.com", "157.240.0.0/16"),
    ("facebook.com", "31.13.24.0/21"),
    ("facebook.com", "31.13.64.0/18"),
    // Reddit (uses Fastly)
    ("reddit.com", "151.101.0.0/16"),
    // YouTube (uses Google)
    ("youtube.com", "142.250.0.0/16"),
    ("youtube.com", "172.217.0.0/16"),
    // TikTok
    ("tiktok.com", "142.250.0.0/16"),
    // Snapchat
    ("snapchat.com", "34.120.0.0/14"),
];

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
