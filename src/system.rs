//! The PC the app runs on, as a bug report needs it: Windows' version, and
//! paths without the user's name in them.

use std::path::Path;

/// "Windows 11 24H2 (26100)", from the registry; "Windows" when it can't be
/// read. Windows 11 still names itself Windows 10 there, so its build tells.
#[cfg(windows)]
pub fn windows_version() -> String {
    let read = || -> Option<String> {
        let key = windows_registry::LOCAL_MACHINE
            .open(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
            .ok()?;
        let build: u32 = key.get_string("CurrentBuild").ok()?.trim().parse().ok()?;
        let release = key.get_string("DisplayVersion").unwrap_or_default();
        let name = if build >= 22000 {
            "Windows 11"
        } else {
            "Windows 10"
        };
        Some(format!("{name} {release} ({build})").replace("  ", " "))
    };
    read().unwrap_or_else(|| "Windows".to_string())
}

#[cfg(not(windows))]
pub fn windows_version() -> String {
    std::env::consts::OS.to_string()
}

/// `path` with the user's own folders written as `%APPDATA%` or
/// `%USERPROFILE%`, so a log pasted into a bug report doesn't carry the
/// user's name.
pub fn without_user(path: &Path) -> String {
    let shown = path.display().to_string();
    for var in ["APPDATA", "LOCALAPPDATA", "USERPROFILE"] {
        if let Some(home) = std::env::var_os(var) {
            let home = home.to_string_lossy().into_owned();
            if !home.is_empty() && shown.to_lowercase().starts_with(&home.to_lowercase()) {
                return format!("%{var}%{}", &shown[home.len()..]);
            }
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_leave_the_user_name_out() {
        let Some(home) = std::env::var_os("USERPROFILE") else {
            return;
        };
        let inside = Path::new(&home).join("AppData").join("Roaming").join("x");
        let shown = without_user(&inside);
        assert!(shown.starts_with('%'), "{shown}");
        assert!(shown.ends_with('x'), "{shown}");
        assert_eq!(without_user(Path::new("D:/data")), "D:/data");
    }

    #[test]
    #[cfg(windows)]
    fn the_windows_version_reads() {
        assert!(
            windows_version().starts_with("Windows 1"),
            "{}",
            windows_version()
        );
    }
}
