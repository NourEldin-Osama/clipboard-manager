//! The mouse pointer and the screen it is on, for placing the quick popup.

use eframe::egui::{Pos2, Rect, Vec2};

/// The mouse pointer and its screen, in physical pixels.
pub struct Pointer {
    pub at: Pos2,
    /// The screen without the taskbar.
    pub work_area: Rect,
    /// The screen's scale: 1 at 96 DPI (100 %), 1.5 at 144 DPI (150 %).
    pub scale: f32,
}

/// Where the mouse pointer is, and its screen.
#[cfg(windows)]
pub fn pointer() -> Option<Pointer> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    // SAFETY: plain Win32 calls with local out values; `cbSize` gives the
    // size of `info`, as GetMonitorInfoW requires.
    unsafe {
        let mut at = POINT::default();
        if GetCursorPos(&mut at) == 0 {
            return None;
        }
        let monitor = MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        let (mut dpi, mut dpi_y) = (96, 96);
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) != 0 {
            dpi = 96;
        }
        let work = info.rcWork;
        Some(Pointer {
            at: Pos2::new(at.x as f32, at.y as f32),
            work_area: Rect::from_min_max(
                Pos2::new(work.left as f32, work.top as f32),
                Pos2::new(work.right as f32, work.bottom as f32),
            ),
            scale: dpi as f32 / 96.0,
        })
    }
}

#[cfg(not(windows))]
pub fn pointer() -> Option<Pointer> {
    None
}

/// The top-left corner of a popup of `size` at `pointer`: right of and below
/// the pointer, or left of it in right-to-left languages, moved to fit inside
/// `area`.
pub fn popup_corner(pointer: Pos2, area: Rect, size: Vec2, rtl: bool) -> Pos2 {
    let x = if rtl { pointer.x - size.x } else { pointer.x };
    // Pushed in from the far edge, then from the near one, which wins when
    // the popup is bigger than the area.
    let fit = |start: f32, low: f32, high: f32, length: f32| start.min(high - length).max(low);
    Pos2::new(
        fit(x, area.left(), area.right(), size.x),
        fit(pointer.y, area.top(), area.bottom(), size.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vec2 = Vec2::new(440.0, 520.0);

    fn area(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom))
    }

    #[test]
    fn the_popup_opens_at_the_pointer_when_it_fits() {
        let screen = area(0.0, 0.0, 1920.0, 1040.0);
        let at = Pos2::new(100.0, 100.0);
        assert_eq!(popup_corner(at, screen, SIZE, false), at);
        let at = Pos2::new(1000.0, 100.0);
        assert_eq!(
            popup_corner(at, screen, SIZE, true),
            Pos2::new(560.0, 100.0),
            "right to left: left of the pointer"
        );
    }

    #[test]
    fn the_popup_stays_inside_the_work_area() {
        let screen = area(0.0, 0.0, 1920.0, 1040.0);
        let near_corner = Pos2::new(1900.0, 1000.0);
        assert_eq!(
            popup_corner(near_corner, screen, SIZE, false),
            Pos2::new(1480.0, 520.0)
        );
        let near_left = Pos2::new(100.0, 100.0);
        assert_eq!(
            popup_corner(near_left, screen, SIZE, true),
            Pos2::new(0.0, 100.0)
        );
        let left_screen = area(-1920.0, 0.0, 0.0, 1040.0);
        assert_eq!(
            popup_corner(Pos2::new(-100.0, 50.0), left_screen, SIZE, false),
            Pos2::new(-440.0, 50.0),
            "a screen left of the main one"
        );
        let tiny = area(0.0, 0.0, 400.0, 300.0);
        assert_eq!(
            popup_corner(Pos2::new(200.0, 200.0), tiny, SIZE, false),
            Pos2::new(0.0, 0.0),
            "bigger than the screen: from its top-left corner"
        );
    }
}
