//! Network blocking via macOS pf firewall.

use color_eyre::eyre::WrapErr as _;
use std::io::Write as _;
use std::process::Command;

use crate::policy::WebsitePolicy;

/// Path to the moonstone anchor file.
const ANCHOR_PATH: &str = "/etc/pf.anchors/com.moonstone";

/// Network enforcer using pf firewall.
pub struct NetworkEnforcer {
    enabled: bool,
}

impl NetworkEnforcer {
    /// Create a new network enforcer.
    pub fn new() -> Self {
        Self { enabled: false }
    }

    /// Apply a website policy.
    pub fn apply(&mut self, policy: &WebsitePolicy) -> color_eyre::eyre::Result<()> {
        let rules = generate_rules(policy)?;
        write_anchor(&rules)?;
        reload_pf()?;
        self.enabled = true;

        tracing::info!("network blocking enabled");
        Ok(())
    }

    /// Disable network blocking.
    pub fn disable(&mut self) -> color_eyre::eyre::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Write empty rules
        write_anchor("")?;
        reload_pf()?;
        self.enabled = false;

        tracing::info!("network blocking disabled");
        Ok(())
    }
}

impl Default for NetworkEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for NetworkEnforcer {
    fn drop(&mut self) {
        if self.enabled {
            let _ = self.disable();
        }
    }
}

/// Generate pf rules for a website policy.
fn generate_rules(policy: &WebsitePolicy) -> color_eyre::eyre::Result<String> {
    let mut rules = String::new();

    match policy {
        WebsitePolicy::Allowlist { domains } => {
            // Resolve domains to IPs
            let allowed_ips = resolve_domains(domains)?;

            // Pass allowed, block rest
            for ip in &allowed_ips {
                rules.push_str(&format!("pass out quick to {}\n", ip));
            }
            rules.push_str("block out quick proto tcp\n");
            rules.push_str("block out quick proto udp\n");
        }
        WebsitePolicy::Blocklist { domains } => {
            // Resolve domains to IPs
            let blocked_ips = resolve_domains(domains)?;

            // Block specific IPs
            for ip in &blocked_ips {
                rules.push_str(&format!("block out quick to {}\n", ip));
            }
        }
    }

    Ok(rules)
}

/// Resolve domains to IP addresses.
fn resolve_domains(domains: &[String]) -> color_eyre::eyre::Result<Vec<String>> {
    let mut ips = Vec::new();

    for domain in domains {
        // Use dig to resolve
        let output = Command::new("dig")
            .args(["+short", domain])
            .output()
            .wrap_err_with(|| format!("failed to resolve {}", domain))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                // Check if it looks like an IP
                if line.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    ips.push(line.to_string());
                }
            }
        }
    }

    // Add known CIDR ranges for common distractions
    // (In production, maintain a database of these)

    Ok(ips)
}

/// Write rules to the anchor file.
fn write_anchor(rules: &str) -> color_eyre::eyre::Result<()> {
    let mut file = std::fs::File::create(ANCHOR_PATH)
        .wrap_err_with(|| format!("failed to create {}", ANCHOR_PATH))?;

    file.write_all(rules.as_bytes())
        .wrap_err("failed to write anchor")?;

    Ok(())
}

/// Reload pf configuration.
fn reload_pf() -> color_eyre::eyre::Result<()> {
    // Enable pf if not already
    let _ = Command::new("pfctl").args(["-e"]).output();

    // Reload anchor
    let output = Command::new("pfctl")
        .args(["-a", "com.moonstone", "-f", ANCHOR_PATH])
        .output()
        .wrap_err("failed to run pfctl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(stderr = %stderr, "pfctl reload warning");
    }

    Ok(())
}
