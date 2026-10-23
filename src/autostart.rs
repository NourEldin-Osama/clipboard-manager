//! Start at sign-in: a per-user `Run` registry value on Windows, a launch agent
//! on macOS, an XDG autostart entry on Linux. The app is started with
//! `--minimized`, so it opens hidden (in the tray when that is on).

use anyhow::{Context, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};

pub const MINIMIZED_ARG: &str = "--minimized";
const APP_NAME: &str = "Clipboard Manager";

/// This exe as the entry should start it. On Windows the path is quoted:
/// without quotes, a path with spaces could start a different program, for
/// example `...\clipboard-manager-windows-x64.exe` instead of
/// `...\clipboard-manager-windows-x64 (3).exe`.
fn app_path() -> Result<String> {
    let exe = std::env::current_exe().context("finding the app's own path")?;
    let exe = exe.to_string_lossy();
    Ok(if cfg!(windows) {
        format!("\"{exe}\"")
    } else {
        exe.into_owned()
    })
}

fn launcher() -> Result<AutoLaunch> {
    AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(&app_path()?)
        .set_args(&[MINIMIZED_ARG])
        .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
        .build()
        .context("setting up start at sign-in")
}

/// Turns start at sign-in on or off (the switch in Settings).
pub fn set(enabled: bool) -> Result<()> {
    let l = launcher()?;
    if enabled {
        l.enable().context("turning on start at sign-in")?;
    } else if l.is_enabled().unwrap_or(true) {
        l.disable().context("turning off start at sign-in")?;
    }
    Ok(())
}

/// At start: if the entry points at another path (the app was moved), points
/// it at this exe. It never turns the entry back on, so "Disable" in Task
/// Manager's Startup apps, or a removed entry, stays as the user left it.
#[cfg(windows)]
pub fn refresh() -> Result<()> {
    const RUN: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    let expected = format!("{} {MINIMIZED_ARG}", app_path()?);
    let key = windows_registry::CURRENT_USER
        .create(RUN)
        .context("opening the sign-in list")?;
    match key.get_string(APP_NAME) {
        Ok(current) if current != expected => key
            .set_string(APP_NAME, &expected)
            .context("updating start at sign-in"),
        _ => Ok(()),
    }
}

/// Elsewhere there is no separate on/off switch to respect: write it again.
#[cfg(not(windows))]
pub fn refresh() -> Result<()> {
    set(true)
}

/// Launched by sign-in rather than by the user.
pub fn launched_minimized() -> bool {
    std::env::args().any(|a| a == MINIMIZED_ARG)
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn the_windows_path_is_quoted() {
        let path = super::app_path().unwrap();
        assert!(path.starts_with('"') && path.ends_with('"'), "{path}");
    }
}
