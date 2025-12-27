use std::process::Command;

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub pid: i32,
    pub bundle_id: String,
    pub name: String,
}

/// Get the currently focused (frontmost) application using lsappinfo
/// This works without Accessibility permissions
pub fn get_frontmost_app() -> Option<AppInfo> {
    // Get the ASN (Application Serial Number) of frontmost app
    let front_output = Command::new("lsappinfo")
        .arg("front")
        .output()
        .ok()?;

    if !front_output.status.success() {
        return None;
    }

    let asn = String::from_utf8_lossy(&front_output.stdout).trim().to_string();
    if asn.is_empty() {
        return None;
    }

    // Get info about that app
    let info_output = Command::new("lsappinfo")
        .args(["info", "-only", "bundleid,pid,name", &asn])
        .output()
        .ok()?;

    if !info_output.status.success() {
        return None;
    }

    let info = String::from_utf8_lossy(&info_output.stdout);

    let mut pid = None;
    let mut bundle_id = None;
    let mut name = None;

    for line in info.lines() {
        let line = line.trim();
        if line.starts_with("\"pid\"=") {
            pid = line.split('=').nth(1).and_then(|s| s.parse().ok());
        } else if line.starts_with("\"CFBundleIdentifier\"=") {
            bundle_id = line.split('=').nth(1).map(|s| s.trim_matches('"').to_string());
        } else if line.starts_with("\"LSDisplayName\"=") || line.starts_with("\"name\"=") {
            name = line.split('=').nth(1).map(|s| s.trim_matches('"').to_string());
        }
    }

    Some(AppInfo {
        pid: pid?,
        bundle_id: bundle_id.unwrap_or_default(),
        name: name.unwrap_or_else(|| "Unknown".to_string()),
    })
}

/// Get all PIDs for a given bundle ID
pub fn get_pids_for_bundle(bundle_id: &str) -> Vec<i32> {
    // Use pgrep to find all processes, then filter by bundle
    let output = Command::new("lsappinfo")
        .args(["list"])
        .output();

    let Ok(output) = output else {
        return vec![];
    };

    if !output.status.success() {
        return vec![];
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pids = vec![];
    let mut current_asn = None;

    for line in stdout.lines() {
        let line = line.trim();

        // New app entry
        if line.starts_with("ASN:") || line.contains(") \"") {
            current_asn = line.split_whitespace().next().map(|s| s.to_string());
        }

        // Check if this app matches our bundle ID
        if line.contains("\"CFBundleIdentifier\"=") {
            let app_bundle = line.split('=')
                .nth(1)
                .map(|s| s.trim_matches('"'))
                .unwrap_or("");

            if app_bundle == bundle_id {
                if let Some(ref asn) = current_asn {
                    // Get PID for this ASN
                    if let Some(pid) = get_pid_for_asn(asn) {
                        pids.push(pid);
                    }
                }
            }
        }
    }

    pids
}

fn get_pid_for_asn(asn: &str) -> Option<i32> {
    let output = Command::new("lsappinfo")
        .args(["info", "-only", "pid", asn])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("\"pid\"=") {
            return line.split('=').nth(1).and_then(|s| s.parse().ok());
        }
    }

    None
}

/// Get all running apps
pub fn get_all_running_apps() -> Vec<AppInfo> {
    let output = Command::new("lsappinfo")
        .args(["list"])
        .output();

    let Ok(output) = output else {
        return vec![];
    };

    if !output.status.success() {
        return vec![];
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = vec![];
    let mut current_app: Option<(String, Option<i32>, Option<String>, Option<String>)> = None;

    for line in stdout.lines() {
        let line = line.trim();

        // New app entry - line contains ASN
        if line.contains(") \"") && line.contains("ASN:") {
            // Save previous app if complete
            if let Some((_, Some(pid), Some(bundle_id), name)) = current_app.take() {
                if !bundle_id.is_empty() {
                    apps.push(AppInfo {
                        pid,
                        bundle_id,
                        name: name.unwrap_or_else(|| "Unknown".to_string()),
                    });
                }
            }
            // Extract name from line like: 0) "Finder" ASN:0x0-0x1001:
            let name = line.split('"').nth(1).map(|s| s.to_string());
            current_app = Some((String::new(), None, None, name));
        }

        if let Some(ref mut app) = current_app {
            if line.starts_with("\"pid\"=") {
                app.1 = line.split('=').nth(1).and_then(|s| s.parse().ok());
            } else if line.starts_with("\"CFBundleIdentifier\"=") {
                app.2 = line.split('=').nth(1).map(|s| s.trim_matches('"').to_string());
            }
        }
    }

    // Don't forget the last app
    if let Some((_, Some(pid), Some(bundle_id), name)) = current_app {
        if !bundle_id.is_empty() {
            apps.push(AppInfo {
                pid,
                bundle_id,
                name: name.unwrap_or_else(|| "Unknown".to_string()),
            });
        }
    }

    apps
}

/// Check if we have accessibility permissions (not needed with lsappinfo)
pub fn check_accessibility_permissions() -> bool {
    // lsappinfo doesn't require accessibility permissions
    get_frontmost_app().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_frontmost_app() {
        let app = get_frontmost_app();
        println!("Frontmost app: {:?}", app);
        assert!(app.is_some());
    }

    #[test]
    fn test_get_all_running_apps() {
        let apps = get_all_running_apps();
        println!("Running apps: {}", apps.len());
        for app in &apps {
            println!("  {} ({})", app.name, app.bundle_id);
        }
        assert!(!apps.is_empty());
    }
}
