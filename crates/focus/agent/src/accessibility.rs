//! macOS accessibility via lsappinfo.

use color_eyre::eyre::WrapErr as _;
use std::process::Command;

/// Get the frontmost application.
///
/// Returns (bundle_id, name) if available.
pub fn get_frontmost_app() -> color_eyre::eyre::Result<Option<(String, String)>> {
    let output = Command::new("lsappinfo")
        .args([
            "info", "-only", "bundleid", "-only", "name", "-app", "front",
        ])
        .output()
        .wrap_err("failed to run lsappinfo")?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_lsappinfo_output(&stdout)
}

/// Get all PIDs for a bundle ID.
pub fn get_pids_for_bundle(bundle_id: &str) -> color_eyre::eyre::Result<Vec<i32>> {
    let output = Command::new("lsappinfo")
        .args(["info", "-only", "pid", "-app", bundle_id])
        .output()
        .wrap_err("failed to run lsappinfo")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pids = Vec::new();

    for line in stdout.lines() {
        if let Some(pid_str) = line.strip_prefix("\"pid\"=") {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                pids.push(pid);
            }
        }
    }

    Ok(pids)
}

/// Get all running applications.
pub fn get_all_running_apps() -> color_eyre::eyre::Result<Vec<(String, String)>> {
    let output = Command::new("lsappinfo")
        .args(["list", "-only", "bundleid", "-only", "name"])
        .output()
        .wrap_err("failed to run lsappinfo")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();

    // Parse each app entry
    for block in stdout.split("\n\n") {
        if let Some(app) = parse_lsappinfo_output(block)? {
            apps.push(app);
        }
    }

    Ok(apps)
}

fn parse_lsappinfo_output(output: &str) -> color_eyre::eyre::Result<Option<(String, String)>> {
    let mut bundle_id = None;
    let mut name = None;

    for line in output.lines() {
        let line = line.trim();

        if let Some(value) = line.strip_prefix("\"bundleid\"=") {
            bundle_id = Some(value.trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("\"name\"=") {
            name = Some(value.trim_matches('"').to_string());
        }
    }

    match (bundle_id, name) {
        (Some(b), Some(n)) => Ok(Some((b, n))),
        (Some(b), None) => Ok(Some((b.clone(), b))),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lsappinfo() {
        let output = r#"
"bundleid"="com.apple.finder"
"name"="Finder"
"#;
        let result = parse_lsappinfo_output(output).unwrap();
        assert_eq!(result, Some(("com.apple.finder".into(), "Finder".into())));
    }
}
