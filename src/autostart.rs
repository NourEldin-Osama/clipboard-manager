//! Start at sign-in: a per-user `Run` registry value on Windows, a launch agent
//! on macOS, an XDG autostart entry on Linux. The app is started with
//! `--minimized`, so it opens hidden (in the tray when that is on). On Windows
//! the app writes the entry itself, as auto-launch did; elsewhere auto-launch
//! does it.

use anyhow::{Context, Result};

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

/// Windows' list of the programs it starts at sign-in, for this user.
#[cfg(windows)]
const RUN: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";

/// Where Task Manager's Startup apps keeps its own on/off switch for each
/// program in the list.
#[cfg(windows)]
const STARTUP_APPROVED: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// Task Manager's "Enabled": a 2, then zeros.
#[cfg(windows)]
const APPROVED: [u8; 12] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// What the entry runs: this exe, opening hidden.
#[cfg(windows)]
fn command() -> Result<String> {
    Ok(format!("{} {MINIMIZED_ARG}", app_path()?))
}

/// Turns start at sign-in on or off (the switch in Settings).
#[cfg(windows)]
pub fn set(enabled: bool) -> Result<()> {
    set_in(
        windows_registry::CURRENT_USER,
        RUN,
        STARTUP_APPROVED,
        enabled,
    )
}

/// `set` in the lists at `run` and `approved` under `root`. Turned on, the
/// entry is also on in Task Manager, as Settings says. Turned off, the
/// entry goes, unless Task Manager has it off already: then it stays as the
/// user left it there.
#[cfg(windows)]
fn set_in(root: &windows_registry::Key, run: &str, approved: &str, enabled: bool) -> Result<()> {
    if enabled {
        let command = command()?;
        root.create(run)
            .and_then(|list| list.set_string(APP_NAME, &command))
            .context("turning on start at sign-in")?;
        match root.options().write().open(approved) {
            Ok(switches) => switches
                .set_bytes(APP_NAME, windows_registry::Type::Bytes, &APPROVED)
                .context("turning on start at sign-in")?,
            Err(e) if e.code().0 as u32 == NOT_FOUND => {}
            Err(e) => return Err(e).context("turning on start at sign-in"),
        }
    } else if is_on(root, run, approved).unwrap_or(true) {
        let removed = root
            .options()
            .write()
            .open(run)
            .and_then(|list| list.remove_value(APP_NAME));
        match removed {
            Ok(()) => {}
            Err(e) if e.code().0 as u32 == NOT_FOUND => {}
            Err(e) => return Err(e).context("turning off start at sign-in"),
        }
    }
    Ok(())
}

/// Whether the entry is in the list at `run` and Task Manager hasn't turned
/// it off.
#[cfg(windows)]
fn is_on(
    root: &windows_registry::Key,
    run: &str,
    approved: &str,
) -> windows_registry::Result<bool> {
    match root.open(run).and_then(|list| list.get_string(APP_NAME)) {
        Ok(_) => {}
        Err(e) if e.code().0 as u32 == NOT_FOUND => return Ok(false),
        Err(e) => return Err(e),
    }
    match root
        .open(approved)
        .and_then(|switches| switches.get_value(APP_NAME))
    {
        Ok(switch) => Ok(approved_says_on(&switch)),
        Err(e) if e.code().0 as u32 == NOT_FOUND => Ok(true),
        Err(e) => Err(e),
    }
}

/// Task Manager writes when it turned a program off into the last eight
/// bytes of its switch; zeros there mean on, as does a switch too short to
/// say.
#[cfg(windows)]
fn approved_says_on(switch: &[u8]) -> bool {
    switch.len() < 8 || switch[switch.len() - 8..].iter().all(|&byte| byte == 0)
}

/// The registry's answer when the key or the value isn't there:
/// ERROR_FILE_NOT_FOUND as an HRESULT.
#[cfg(windows)]
const NOT_FOUND: u32 = 0x8007_0002;

/// At start: if the entry points at another path (the app was moved), points
/// it at this exe. It never turns the entry back on, so "Disable" in Task
/// Manager's Startup apps, or a removed entry, stays as the user left it.
#[cfg(windows)]
pub fn refresh() -> Result<()> {
    let expected = command()?;
    let list = windows_registry::CURRENT_USER
        .create(RUN)
        .context("opening the sign-in list")?;
    match list.get_string(APP_NAME) {
        Ok(current) if current != expected => list
            .set_string(APP_NAME, &expected)
            .context("updating start at sign-in"),
        _ => Ok(()),
    }
}

#[cfg(not(windows))]
fn launcher() -> Result<auto_launch::AutoLaunch> {
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(&app_path()?)
        .set_args(&[MINIMIZED_ARG])
        .build()
        .context("setting up start at sign-in")
}

/// Turns start at sign-in on or off (the switch in Settings).
#[cfg(not(windows))]
pub fn set(enabled: bool) -> Result<()> {
    let l = launcher()?;
    if enabled {
        l.enable().context("turning on start at sign-in")?;
    } else if l.is_enabled().unwrap_or(true) {
        l.disable().context("turning off start at sign-in")?;
    }
    Ok(())
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

    /// On, off and Task Manager's switch, in lists of the test's own under
    /// this user's registry, never Windows' real ones.
    #[cfg(windows)]
    #[test]
    fn the_sign_in_entry_goes_on_and_off_as_auto_launch_did() {
        use super::*;
        /// The test's own key, removed with all in it however the test ends.
        struct Scratch(String);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = windows_registry::CURRENT_USER.remove_tree(&self.0);
            }
        }
        let root = windows_registry::CURRENT_USER;
        let place = Scratch(format!(
            r"SOFTWARE\clipboard-manager-test-sign-in-{}",
            std::process::id()
        ));
        let (run, approved) = (
            format!(r"{}\Run", place.0),
            format!(r"{}\Approved", place.0),
        );
        let entry = |root: &windows_registry::Key| {
            root.open(&run)
                .and_then(|list| list.get_string(APP_NAME))
                .ok()
        };

        // Without Task Manager's list: on writes the entry, off removes it,
        // and off again is no error.
        set_in(root, &run, &approved, true).unwrap();
        assert_eq!(entry(root), Some(command().unwrap()));
        assert!(is_on(root, &run, &approved).unwrap());
        set_in(root, &run, &approved, false).unwrap();
        assert_eq!(entry(root), None);
        set_in(root, &run, &approved, false).unwrap();

        // Turned off in Task Manager: off leaves the entry as it is, and on
        // turns Task Manager's switch on again.
        set_in(root, &run, &approved, true).unwrap();
        let switches = root.create(&approved).unwrap();
        let disabled = [3, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];
        switches
            .set_bytes(APP_NAME, windows_registry::Type::Bytes, &disabled)
            .unwrap();
        assert!(!is_on(root, &run, &approved).unwrap());
        set_in(root, &run, &approved, false).unwrap();
        assert_eq!(entry(root), Some(command().unwrap()));
        set_in(root, &run, &approved, true).unwrap();
        let switch = switches.get_value(APP_NAME).unwrap();
        assert_eq!(&switch[..], &APPROVED[..]);
        assert!(is_on(root, &run, &approved).unwrap());
    }

    #[cfg(windows)]
    #[test]
    fn task_managers_switch_is_read_from_its_last_eight_bytes() {
        use super::approved_says_on;
        assert!(approved_says_on(&super::APPROVED));
        assert!(approved_says_on(&[3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        assert!(!approved_says_on(&[3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]));
        assert!(approved_says_on(&[3, 1, 2]));
    }
}
