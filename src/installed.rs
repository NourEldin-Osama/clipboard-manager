//! The entry the installer adds to Installed apps (installer/clipboard-manager.iss).
//! The app updates itself, so at start the installed copy sets the version
//! shown there to its own.

/// The entry's key: the installer's `AppId`, plus the `_is1` Inno Setup adds.
#[cfg(windows)]
const ENTRY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{2869B14A-B2DA-4F94-87D5-ED74302ED192}_is1";

#[cfg(windows)]
pub fn refresh_version() -> anyhow::Result<()> {
    let Ok(entry) = windows_registry::CURRENT_USER.open(ENTRY) else {
        return Ok(()); // not installed
    };
    let exe = std::env::current_exe()?;
    let Some(folder) = exe.parent() else {
        return Ok(());
    };
    let installed = entry.get_string("InstallLocation")?;
    // A portable copy or a build elsewhere leaves the entry alone.
    if !same_folder(&installed, &folder.to_string_lossy()) {
        return Ok(());
    }
    let version = env!("CARGO_PKG_VERSION");
    if entry.get_string("DisplayVersion").ok().as_deref() != Some(version) {
        windows_registry::CURRENT_USER
            .create(ENTRY)?
            .set_string("DisplayVersion", version)?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn refresh_version() -> anyhow::Result<()> {
    Ok(())
}

/// Windows paths, ignoring case and a final backslash.
#[cfg(windows)]
fn same_folder(a: &str, b: &str) -> bool {
    let trim = |path: &str| path.trim_end_matches('\\').to_lowercase();
    trim(a) == trim(b)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn folders_match_without_case_or_a_final_backslash() {
        assert!(same_folder(
            r"C:\Users\Ann\AppData\Local\Programs\Clipboard Manager\",
            r"c:\users\ann\appdata\local\programs\clipboard manager"
        ));
        assert!(!same_folder(r"C:\Apps\Clipboard Manager", r"C:\Apps\Other"));
    }
}
