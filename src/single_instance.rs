//! One running copy per data folder. Starting the app again asks the copy
//! that is already running to show its window and then quits, instead of
//! failing on the locked database without a word. A dev build with its own
//! `CLIPBOARD_MANAGER_HOME` still runs next to the installed app.

use std::path::Path;

pub enum Claim {
    /// This is the only copy; pass the guard to the app.
    First(Guard),
    /// Another copy runs, and was asked to show its window.
    AlreadyRunning,
}

/// The names for one data folder: the same folder always gives the same names.
fn names(root: &Path) -> (String, String) {
    let id = crate::store::fnv1a(root.to_string_lossy().to_lowercase().as_bytes());
    (
        format!("Local\\clipboard-manager-{id:016x}"),
        format!("Local\\clipboard-manager-show-{id:016x}"),
    )
}

#[cfg(windows)]
pub use windows::{claim, Guard};

#[cfg(windows)]
mod windows {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
        INFINITE,
    };

    use super::Claim;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The named mutex that marks the running copy, and the event it waits
    /// on. Never closed by the app: Windows closes both when the process
    /// exits, which is exactly when the next copy may start, and no thread is
    /// ever left waiting on a closed handle.
    pub struct Guard {
        event: HANDLE,
    }

    /// Claims the data folder at `root`. With `wait_for_previous` (right after
    /// an update, while the old version is closing) it waits up to 15 seconds
    /// for the previous copy to exit.
    pub fn claim(root: &Path, wait_for_previous: bool) -> Claim {
        let (mutex_name, event_name) = super::names(root);
        let (mutex_name, event_name) = (wide(&mutex_name), wide(&event_name));
        let wait = Duration::from_secs(if wait_for_previous { 15 } else { 0 });
        let deadline = Instant::now() + wait;
        loop {
            // SAFETY: the name is NUL-terminated UTF-16; no security attributes.
            let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
            let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            if mutex.is_null() {
                // Without the mutex nothing is known; don't block the start.
                log::warn!(
                    "single-instance check failed: {}",
                    std::io::Error::last_os_error()
                );
                return Claim::First(Guard {
                    event: std::ptr::null_mut(),
                });
            }
            if !existed {
                // SAFETY: as above; an auto-reset event, not signalled yet.
                let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr()) };
                return Claim::First(Guard { event });
            }
            // SAFETY: our own handle to the existing mutex.
            unsafe { CloseHandle(mutex) };
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        // SAFETY: as above.
        let event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr()) };
        if !event.is_null() {
            // SAFETY: a valid handle we just opened.
            unsafe {
                SetEvent(event);
                CloseHandle(event);
            }
        }
        Claim::AlreadyRunning
    }

    impl Guard {
        /// Calls `show` each time another start asks for the window.
        pub fn on_show_request(&self, show: impl Fn() + Send + 'static) {
            if self.event.is_null() {
                return;
            }
            // A raw handle isn't Send; the event lives until the process exits.
            let event = self.event as usize;
            let spawned = std::thread::Builder::new()
                .name("single-instance".into())
                .spawn(move || {
                    // SAFETY: the event is never closed while the process runs.
                    while unsafe { WaitForSingleObject(event as HANDLE, INFINITE) } == WAIT_OBJECT_0
                    {
                        show();
                    }
                });
            if let Err(e) = spawned {
                log::warn!("can't listen for a second start: {e}");
            }
        }
    }
}

/// Other platforms have no check: the app starts as before.
#[cfg(not(windows))]
pub struct Guard;

#[cfg(not(windows))]
impl Guard {
    pub fn on_show_request(&self, _show: impl Fn() + Send + 'static) {}
}

#[cfg(not(windows))]
pub fn claim(_root: &Path, _wait_for_previous: bool) -> Claim {
    let _ = names;
    Claim::First(Guard)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_second_claim_on_the_same_folder_is_refused() {
        let root = std::env::temp_dir().join(format!("cbm-instance-{}", std::process::id()));
        let first = claim(&root, false);
        assert!(matches!(first, Claim::First(_)));
        assert!(matches!(claim(&root, false), Claim::AlreadyRunning));
        let other = root.join("other");
        assert!(matches!(claim(&other, false), Claim::First(_)));
    }
}
