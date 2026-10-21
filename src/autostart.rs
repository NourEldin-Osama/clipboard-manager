//! Start at sign-in: a per-user `Run` registry value on Windows, a launch agent
//! on macOS, an XDG autostart entry on Linux. The app is started with
//! `--minimized`, so it opens hidden (in the tray when that is on).

use anyhow::{Context, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};

pub const MINIMIZED_ARG: &str = "--minimized";

fn launcher() -> Result<AutoLaunch> {
    let exe = std::env::current_exe().context("finding the app's own path")?;
    AutoLaunchBuilder::new()
        .set_app_name("Clipboard Manager")
        .set_app_path(&exe.to_string_lossy())
        .set_args(&[MINIMIZED_ARG])
        .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
        .build()
        .context("setting up start at sign-in")
}

/// Turns start at sign-in on or off. Turning it on again also refreshes the
/// stored path, in case the app was moved.
pub fn set(enabled: bool) -> Result<()> {
    let l = launcher()?;
    if enabled {
        l.enable().context("turning on start at sign-in")?;
    } else if l.is_enabled().unwrap_or(true) {
        l.disable().context("turning off start at sign-in")?;
    }
    Ok(())
}

/// Launched by sign-in rather than by the user.
pub fn launched_minimized() -> bool {
    std::env::args().any(|a| a == MINIMIZED_ARG)
}
