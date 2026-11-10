//! The app's own note at the corner of the screen (D56): a message about a
//! copy while the window is not in front, in the look of the window's
//! messages and with the app icon. It never takes the keyboard: you are
//! usually about to paste somewhere else.

use super::messages::{lay_out, paint_content, Asked};
use super::*;

/// The note's width, as the window's messages.
const WIDTH: f32 = 356.0;
/// From the screen's edges, the taskbar left out.
const EDGE: f32 = 16.0;

fn note_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("note")
}

/// Whether the note's window exists (it may be hidden).
fn note_made(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.raw.viewports.contains_key(&note_id()))
}

fn note_builder() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(t!("note-title"))
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
        // Shown without taking the keyboard from the program in front.
        .with_active(false)
        .with_window_level(egui::WindowLevel::AlwaysOnTop)
}

impl App {
    /// While a message waits for the note and its window isn't made yet,
    /// the hidden main window keeps drawing, so it can be made. Runs from
    /// `logic`, which runs in every frame.
    pub(super) fn note_logic(&mut self, ctx: &egui::Context) {
        if self.toasts.iter().any(|t| t.on_screen) && !note_made(ctx) {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }

    /// The message the note shows: the newest one meant for the screen.
    fn note_toast(&self) -> Option<&Toast> {
        self.toasts.iter().rev().find(|t| t.on_screen)
    }

    /// Whether a message about a copy goes on the screen now: notes are on,
    /// neither window is in front, and Windows doesn't ask for quiet (a
    /// full-screen program or a presentation).
    pub(crate) fn note_wanted(&self) -> bool {
        self.cfg.screen_notes
            && !self.focused.load(Ordering::SeqCst)
            && !self.win.popup_focused.load(Ordering::SeqCst)
            && !quiet_time()
    }

    /// Draws the note's window while a message is meant for the screen. As
    /// with the popup, its window is first made empty and hidden, and drawn
    /// from the next frame on (eframe can't make it during a frame while the
    /// main window is hidden). A frame without it closes it.
    pub(super) fn note_window(&mut self, ctx: &egui::Context) {
        if self.note_toast().is_none() {
            self.note_at = None;
            self.note_hovered = false;
            // A new window is made next time.
            self.note_window_handle = None;
            return;
        }
        if !note_made(ctx) {
            ctx.show_viewport_deferred(note_id(), note_builder().with_visible(false), |_, _| {});
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
            return;
        }
        let size = Vec2::new(WIDTH, self.note_height);
        // Placed once, where the pointer is, so it doesn't follow the mouse.
        if self.note_at.is_none() {
            self.note_at = note_corner(ctx, size);
        }
        let mut builder = note_builder().with_inner_size(size).with_visible(true);
        if let Some(at) = self.note_at {
            builder = builder.with_position(at);
        }
        ctx.show_viewport_immediate(note_id(), builder, |ui, _| self.note_ui(ui));
        // winit writes a window's whole extended style again each time one
        // of its own flags changes, which drops the note's: it is checked in
        // every frame.
        if self.note_window_handle.is_none() {
            self.note_window_handle = find_own_window(&t!("note-title"));
        }
        if let Some(handle) = self.note_window_handle {
            if !keep_quiet(handle) {
                self.note_window_handle = None;
            }
        }
    }

    fn note_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let Some(at) = self.toasts.iter().rposition(|t| t.on_screen) else {
            return;
        };
        let icon = self.app_icon(&ctx);
        let card = ctx.content_rect();
        let v = ui.visuals().clone();
        let fill = if v.dark_mode {
            v.window_fill
        } else {
            Color32::WHITE
        };
        ui.painter().rect(
            card,
            CornerRadius::ZERO,
            fill,
            Stroke::new(1.0, theme::line_color(v.dark_mode)),
            egui::StrokeKind::Inside,
        );
        let toast = &self.toasts[at];
        let laid = lay_out(ui, toast, card.width(), false, Some(&icon));
        // Its window takes the height the message needs, from the next
        // frame, placed again so its bottom stays above the taskbar.
        if (laid.height - self.note_height).abs() > 0.5 {
            self.note_height = laid.height;
            self.note_at = None;
            ctx.request_repaint();
        }
        let body = ui.interact(card, egui::Id::unique("note-body"), Sense::click());
        let mut asked = Asked::default();
        paint_content(ui, card, toast, &laid, &mut asked, Some(&icon));
        self.note_hovered = ui.rect_contains_pointer(card);
        let id = toast.id;
        if asked.action.is_some() {
            let toast = self.toasts.remove(at);
            if let Some((_, command)) = toast.action {
                self.run(command);
            }
        } else if body.clicked() {
            // The window opens with the same message, where Show works.
            if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                toast.on_screen = false;
                toast.shown_for = Duration::ZERO;
            }
            self.win.show_full();
        }
    }

    /// The app icon as a texture, made once.
    pub(super) fn app_icon(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        self.app_icon
            .get_or_insert_with(|| {
                let (rgba, size) = crate::icon_rgba();
                let size = size as usize;
                let image = egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba);
                ctx.load_texture("app-icon", image, egui::TextureOptions::LINEAR)
            })
            .clone()
    }
}

/// The note's top-left corner in points: at the bottom right of the screen
/// with the mouse pointer, or bottom left in Arabic, above the taskbar.
fn note_corner(ctx: &egui::Context, size: Vec2) -> Option<Pos2> {
    let pointer = crate::screen::pointer()?;
    let zoom = ctx.zoom_factor();
    let pixels = size * pointer.scale * zoom;
    let edge = EDGE * pointer.scale * zoom;
    let area = pointer.work_area;
    let x = if i18n::rtl() {
        area.left() + edge
    } else {
        area.right() - edge - pixels.x
    };
    let corner = Pos2::new(x, area.bottom() - edge - pixels.y);
    Some((corner.to_vec2() / (pointer.scale * zoom)).to_pos2())
}

/// Whether Windows asks for quiet: a full-screen program, a game or a
/// presentation is in front.
#[cfg(windows)]
fn quiet_time() -> bool {
    use windows_sys::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    let mut state = 0;
    // SAFETY: a plain call with a local out value.
    let asked = unsafe { SHQueryUserNotificationState(&mut state) };
    asked == 0
        && [
            QUNS_BUSY,
            QUNS_RUNNING_D3D_FULL_SCREEN,
            QUNS_PRESENTATION_MODE,
        ]
        .contains(&state)
}

#[cfg(not(windows))]
fn quiet_time() -> bool {
    false
}

/// This app's own window with this title, as Windows' handle for it, given
/// Windows 11's round corners. The style changes in `keep_quiet` leave
/// those as they are.
#[cfg(windows)]
fn find_own_window(title: &str) -> Option<isize> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId,
    };

    struct Search {
        title: Vec<u16>,
        found: HWND,
    }
    unsafe extern "system" fn each(hwnd: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` is the `Search` below, alive for the whole call to
        // EnumWindows; the buffers are as long as the lengths passed.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut process = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut process) };
        if process != unsafe { GetCurrentProcessId() } {
            return 1;
        }
        let mut text = [0u16; 256];
        let n = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
        if n > 0 && text[..n as usize] == search.title[..] {
            search.found = hwnd;
            return 0;
        }
        1
    }
    let mut search = Search {
        title: title.encode_utf16().collect(),
        found: std::ptr::null_mut(),
    };
    // SAFETY: the callback only reads and writes `search` through the
    // pointer passed here, and only during this call.
    unsafe { EnumWindows(Some(each), &mut search as *mut Search as LPARAM) };
    if search.found.is_null() {
        return None;
    }
    let round = DWMWCP_ROUND;
    // SAFETY: `search.found` is this app's own window, found just now.
    unsafe {
        DwmSetWindowAttribute(
            search.found,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&round as *const i32).cast(),
            size_of::<i32>() as u32,
        );
    }
    Some(search.found as isize)
}

/// Makes the note's window one that a click doesn't bring to the front, so
/// the keyboard stays with the program you were in, and keeps it off the
/// taskbar and Alt+Tab. False when the window is gone.
#[cfg(windows)]
fn keep_quiet(handle: isize) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, IsWindow, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_APPWINDOW,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    let hwnd = handle as windows_sys::Win32::Foundation::HWND;
    // SAFETY: plain Win32 calls on a window handle; IsWindow checks first
    // that it still names a window.
    unsafe {
        if IsWindow(hwnd) == 0 {
            return false;
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let quiet =
            (style | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize) & !(WS_EX_APPWINDOW as isize);
        if quiet != style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, quiet);
        }
    }
    true
}

#[cfg(not(windows))]
fn find_own_window(_title: &str) -> Option<isize> {
    None
}

#[cfg(not(windows))]
fn keep_quiet(_handle: isize) -> bool {
    true
}
