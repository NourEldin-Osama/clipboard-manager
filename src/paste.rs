//! Pasting into the program the user was in. The item is already on the
//! clipboard: that program's window comes back to the front and gets Ctrl+V,
//! as if the user pressed it.

/// A window of another program, as its Windows handle.
pub type Window = isize;

/// No window to paste into.
pub const NO_WINDOW: Window = 0;

/// The window in front, or `NO_WINDOW` when it is one of this app's own.
#[cfg(windows)]
pub fn window_in_front() -> Window {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    // SAFETY: plain Win32 calls; `process_id` is a local out value.
    unsafe {
        let window = GetForegroundWindow();
        let mut process_id = 0;
        GetWindowThreadProcessId(window, &mut process_id);
        if process_id == 0 || process_id == std::process::id() {
            return NO_WINDOW;
        }
        window as Window
    }
}

#[cfg(not(windows))]
pub fn window_in_front() -> Window {
    NO_WINDOW
}

/// Whether the window is still open: its program may have closed since.
#[cfg(windows)]
pub fn exists(window: Window) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;
    // SAFETY: IsWindow accepts any value and only checks it.
    window != NO_WINDOW && unsafe { IsWindow(window as _) } != 0
}

#[cfg(not(windows))]
pub fn exists(_window: Window) -> bool {
    false
}

/// Whether Windows blocks this app from pressing keys in the window: its
/// program runs as administrator and this app doesn't. A program that
/// Windows won't even describe to this app counts as blocked too.
#[cfg(windows)]
pub fn is_out_of_reach(window: Window) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    // SAFETY: plain Win32 calls; the process handle is closed before returning.
    unsafe {
        if is_elevated(GetCurrentProcess()) == Some(true) {
            return false;
        }
        let mut process_id = 0;
        GetWindowThreadProcessId(window as _, &mut process_id);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return true;
        }
        let elevated = is_elevated(process);
        CloseHandle(process);
        elevated != Some(false)
    }
}

#[cfg(not(windows))]
pub fn is_out_of_reach(_window: Window) -> bool {
    false
}

/// Whether a process runs as administrator; `None` when Windows won't say.
///
/// # Safety
/// `process` must be an open process handle with query rights.
#[cfg(windows)]
unsafe fn is_elevated(process: windows_sys::Win32::Foundation::HANDLE) -> Option<bool> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::OpenProcessToken;
    let mut token = std::ptr::null_mut();
    if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
        return None;
    }
    let mut elevation = TOKEN_ELEVATION::default();
    let mut size = 0;
    let found = GetTokenInformation(
        token,
        TokenElevation,
        (&raw mut elevation).cast(),
        size_of::<TOKEN_ELEVATION>() as u32,
        &mut size,
    );
    CloseHandle(token);
    (found != 0).then_some(elevation.TokenIsElevated != 0)
}

/// How long to wait for the user to let go of Shift, Ctrl, Alt and the
/// Windows key: held with ours, they would make Ctrl+V another shortcut.
#[cfg(windows)]
const KEYS_UP_WITHIN: std::time::Duration = std::time::Duration::from_millis(1500);
/// How long the window may take to come to the front.
#[cfg(windows)]
const FRONT_WITHIN: std::time::Duration = std::time::Duration::from_millis(1000);

/// Brings the window to the front and presses Ctrl+V in it.
#[cfg(windows)]
pub fn paste_into(window: Window) -> anyhow::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    // SAFETY: plain Win32 calls on a window handle; Windows checks it.
    unsafe {
        if IsIconic(window as _) != 0 {
            ShowWindow(window as _, SW_RESTORE);
        }
        SetForegroundWindow(window as _);
    }
    anyhow::ensure!(
        wait_until(KEYS_UP_WITHIN, || !modifier_held()),
        "a key such as Shift was still held"
    );
    anyhow::ensure!(
        // SAFETY: GetForegroundWindow takes nothing.
        wait_until(FRONT_WITHIN, || unsafe { GetForegroundWindow() } as Window
            == window),
        "the program didn't come to the front"
    );
    // A moment for the program to give its text field the focus back.
    std::thread::sleep(std::time::Duration::from_millis(50));
    press_ctrl_v()
}

#[cfg(not(windows))]
pub fn paste_into(_window: Window) -> anyhow::Result<()> {
    anyhow::bail!("pasting into another program works only on Windows")
}

/// Checks `done` every few milliseconds until it is true or `within` ends.
#[cfg(windows)]
fn wait_until(within: std::time::Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while !done() {
        if start.elapsed() > within {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    true
}

#[cfg(windows)]
fn modifier_held() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
        .into_iter()
        // SAFETY: GetAsyncKeyState only reads the key's state. The top bit
        // (a negative value) means the key is down.
        .any(|key| unsafe { GetAsyncKeyState(key.into()) } < 0)
}

#[cfg(windows)]
fn press_ctrl_v() -> anyhow::Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_CONTROL, VK_V,
    };
    let key = |key: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                // SAFETY: MapVirtualKeyW only translates a key code.
                wScan: unsafe { MapVirtualKeyW(key.into(), MAPVK_VK_TO_VSC) } as u16,
                dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                ..Default::default()
            },
        },
    };
    let presses = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    // SAFETY: `presses` holds `presses.len()` INPUT values of the size given.
    let sent = unsafe {
        SendInput(
            presses.len() as u32,
            presses.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    anyhow::ensure!(
        sent == presses.len() as u32,
        "Windows took {sent} of the {} key presses",
        presses.len()
    );
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn this_app_can_tell_whether_it_runs_as_administrator() {
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        // SAFETY: the pseudo handle of this process needs no closing.
        assert!(unsafe { is_elevated(GetCurrentProcess()) }.is_some());
    }

    #[test]
    fn a_closed_or_missing_window_does_not_exist() {
        assert!(!exists(NO_WINDOW));
        assert!(!exists(0x7fff_fff0), "a handle no window has");
    }
}
