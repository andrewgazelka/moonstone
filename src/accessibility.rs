use std::process::Command;

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub pid: i32,
    pub bundle_id: String,
    pub name: String,
}

/// Get the currently focused (frontmost) application
pub fn get_frontmost_app() -> Option<AppInfo> {
    // Use AppleScript to get frontmost app info - most reliable method
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                set appPID to unix id of frontApp
                set bundleID to bundle identifier of frontApp
                return appPID & "||" & bundleID & "||" & appName
            end tell
            "#,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split("||").collect();

    if parts.len() >= 3 {
        let pid = parts[0].parse().ok()?;
        let bundle_id = parts[1].to_string();
        let name = parts[2].to_string();
        Some(AppInfo { pid, bundle_id, name })
    } else {
        None
    }
}

/// Get all PIDs for a given bundle ID
pub fn get_pids_for_bundle(bundle_id: &str) -> Vec<i32> {
    let output = Command::new("osascript")
        .args([
            "-e",
            &format!(
                r#"
                tell application "System Events"
                    set pidList to {{}}
                    repeat with proc in (every application process whose bundle identifier is "{}")
                        set end of pidList to unix id of proc
                    end repeat
                    return pidList
                end tell
                "#,
                bundle_id
            ),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .trim()
                .split(", ")
                .filter_map(|s| s.parse().ok())
                .collect()
        }
        _ => vec![],
    }
}

/// Check if we have accessibility permissions
pub fn check_accessibility_permissions() -> bool {
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"
            tell application "System Events"
                return name of first application process whose frontmost is true
            end tell
            "#,
        ])
        .output();

    matches!(output, Ok(out) if out.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_frontmost_app() {
        // This test requires running on macOS with accessibility permissions
        if cfg!(target_os = "macos") {
            let app = get_frontmost_app();
            // Should at least return something (even if just the test runner)
            println!("Frontmost app: {:?}", app);
        }
    }
}
