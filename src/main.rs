//! clipboard-manager: an offline clipboard history with on-device OCR and
//! semantic search over text and images.

// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod catalog;
mod clipboard;
mod config;
mod download;
mod embed;
mod i18n;
mod images;
mod import;
mod installed;
mod library;
mod llama;
mod net;
mod ocr;
mod paste;
mod privacy;
mod rules;
mod screen;
mod search;
mod secure;
mod single_instance;
mod store;
mod tray;
mod update;
mod vault;
mod worker;

use eframe::egui;

fn main() -> eframe::Result {
    let paths = match config::Paths::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cannot create data directory: {e:#}");
            std::process::exit(1);
        }
    };
    // One copy per data folder, checked before logging so a second start
    // doesn't touch the running copy's log. Right after an update the
    // previous version is still closing: wait for it to go, so its hotkeys
    // are free and its exe can be removed.
    let after_update = std::env::args().any(|a| a == update::AFTER_UPDATE_ARG);
    let instance = match single_instance::claim(&paths.root, after_update) {
        single_instance::Claim::First(guard) => guard,
        single_instance::Claim::AlreadyRunning => return Ok(()),
    };
    init_logging(&paths);
    // An update downloaded last time but never installed (a Windows shutdown
    // ends the app without the install on quit) goes in now.
    if !after_update {
        match update::install_staged() {
            Ok(Some(exe)) => match update::relaunch(&exe) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    log::warn!("{e:#}");
                    if let Err(e) = update::roll_back() {
                        log::error!("{e:#}");
                    }
                }
            },
            Ok(None) => {}
            Err(e) => log::warn!("installing the downloaded update: {e:#}"),
        }
    }
    update::clean_up();
    log::info!(
        "clipboard-manager {} starting, data in {}",
        env!("CARGO_PKG_VERSION"),
        paths.root.display()
    );
    // The window title is set before the app starts, so read the language now.
    let (cfg, _) = config::Config::load(&paths.config);
    i18n::set(cfg.language, cfg.time_format);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::t!("app-title"))
            .with_app_id("clipboard-manager")
            .with_inner_size(app::WINDOW_SIZE)
            .with_min_inner_size(app::SMALLEST_WINDOW)
            .with_drag_and_drop(true)
            .with_icon(icon()),
        persist_window: true,
        // eframe's own default is the installed app's folder, even for a
        // portable copy or a dev build with CLIPBOARD_MANAGER_HOME set.
        persistence_path: Some(paths.root.join("app.ron")),
        ..Default::default()
    };
    let result = eframe::run_native(
        "clipboard-manager",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, paths, instance)?))),
    );
    // A release build has no console, so a failed start needs a visible message.
    if let Err(e) = &result {
        log::error!("could not start: {e}");
        rfd::MessageDialog::new()
            .set_title(i18n::t!("app-title"))
            .set_description(format!("{}\n\n{e}", i18n::t!("error-start-failed")))
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
    result
}

/// Logs to `clipboard-manager.log` in the data folder (release builds on
/// Windows have no console). The previous run's log is kept as `.old.log`.
fn init_logging(paths: &config::Paths) {
    let log_path = paths.root.join("clipboard-manager.log");
    let old_path = paths.root.join("clipboard-manager.old.log");
    let _ = std::fs::rename(&log_path, &old_path);
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,winit=warn,wgpu=warn,eframe=warn"),
    );
    if let Ok(log) = LogFile::create(log_path, old_path, LOG_LIMIT) {
        builder.target(env_logger::Target::Pipe(Box::new(log)));
    }
    builder.init();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
}

/// How big the log grows before it starts again, so a copy that runs in the
/// tray for weeks keeps at most two files of this size.
const LOG_LIMIT: u64 = 10 * 1024 * 1024;

/// The log file. Past `limit` it becomes the `.old.log`, replacing the one
/// there, and a new log starts.
struct LogFile {
    path: std::path::PathBuf,
    old: std::path::PathBuf,
    file: std::fs::File,
    written: u64,
    limit: u64,
}

impl LogFile {
    fn create(
        path: std::path::PathBuf,
        old: std::path::PathBuf,
        limit: u64,
    ) -> std::io::Result<Self> {
        Ok(Self {
            file: std::fs::File::create(&path)?,
            path,
            old,
            written: 0,
            limit,
        })
    }

    fn start_again(&mut self) {
        use std::io::{Seek, Write};
        let _ = self.file.flush();
        // Windows renames a file this process has open: std opens files
        // letting others rename and delete them.
        if std::fs::rename(&self.path, &self.old).is_ok() {
            // If no new file can be made, the log goes on in the moved one.
            if let Ok(file) = std::fs::File::create(&self.path) {
                self.file = file;
            }
        } else {
            // The old log is held open elsewhere: this one starts over.
            let _ = self.file.set_len(0);
            let _ = self.file.rewind();
        }
        self.written = 0;
    }
}

impl std::io::Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.written > 0 && self.written + buf.len() as u64 > self.limit {
            self.start_again();
        }
        let n = self.file.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn icon() -> egui::IconData {
    let (rgba, n) = icon_rgba();
    egui::IconData {
        rgba,
        width: n,
        height: n,
    }
}

/// The app icon as 64 by 64 RGBA pixels, for the window and the tray: the
/// drawing for small sizes (assets/app-icon/app-small.svg), since both show
/// it at 16 to 24 px. The exe has every size in app.ico.
pub fn icon_rgba() -> (Vec<u8>, u32) {
    const SIZE: u32 = 64;
    let size = egui::SizeHint::Size {
        width: SIZE,
        height: SIZE,
        maintain_aspect_ratio: true,
    };
    let image = egui_extras::image::load_svg_bytes_with_size(
        include_bytes!("../assets/app-icon/app-small.svg"),
        size,
        &Default::default(),
    )
    .expect("the app icon is a valid drawing");
    let rgba = image
        .pixels
        .iter()
        .flat_map(|c| c.to_srgba_unmultiplied())
        .collect();
    (rgba, SIZE)
}

/// The app icon in gray, a little faded: the tray while recording is
/// paused (D59).
pub fn gray_icon_rgba() -> (Vec<u8>, u32) {
    let (mut rgba, size) = icon_rgba();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        let [r, g, b, a] = pixel.map(f32::from);
        let gray = (0.2126 * r + 0.7152 * g + 0.0722 * b) as u8;
        *pixel = [gray, gray, gray, (a * 0.6) as u8];
    }
    (rgba, size)
}

/// The app icon with a badge at its corner: the "eye off" icon on the
/// accent color, for the tray after Alt+C (D59).
pub fn badged_icon_rgba() -> (Vec<u8>, u32) {
    let (mut rgba, size) = icon_rgba();
    const BADGE: f32 = 34.0;
    let n = size as usize;
    let center = size as f32 - BADGE / 2.0;
    let glyph = egui_extras::image::load_svg_bytes_with_size(
        include_bytes!("../assets/icons/eye_off.svg"),
        egui::SizeHint::Size {
            width: 22,
            height: 22,
            maintain_aspect_ratio: true,
        },
        &Default::default(),
    )
    .expect("the eye-off icon is a valid drawing");
    let glyph_at = (center - 11.0) as usize;
    let accent = [88.0, 101.0, 242.0];
    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            // Covered part of the pixel at the badge's edge, for a smooth rim.
            let cover = (BADGE / 2.0 - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if cover == 0.0 {
                continue;
            }
            let mut color = accent;
            let (gx, gy) = (x.wrapping_sub(glyph_at), y.wrapping_sub(glyph_at));
            if gx < 22 && gy < 22 {
                let g = glyph.pixels[gy * glyph.size[0] + gx].to_srgba_unmultiplied();
                let a = g[3] as f32 / 255.0;
                for (c, white) in color.iter_mut().zip([255.0; 3]) {
                    *c = *c * (1.0 - a) + white * a;
                }
            }
            let at = (y * n + x) * 4;
            let under = rgba[at + 3] as f32 / 255.0;
            let alpha = cover + under * (1.0 - cover);
            for (i, c) in color.iter().enumerate() {
                let below = rgba[at + i] as f32;
                rgba[at + i] =
                    ((c * cover + below * under * (1.0 - cover)) / alpha.max(1e-6)) as u8;
            }
            rgba[at + 3] = (alpha * 255.0) as u8;
        }
    }
    (rgba, size)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_badge_sits_in_the_corner_of_the_app_icon() {
        let (plain, size) = super::icon_rgba();
        let (badged, _) = super::badged_icon_rgba();
        let pixel = |rgba: &[u8], x: u32, y: u32| {
            let at = ((y * size + x) * 4) as usize;
            [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
        };
        assert_eq!(pixel(&plain, 2, 2), pixel(&badged, 2, 2), "the rest stays");
        assert_ne!(
            pixel(&plain, size - 4, size - 17),
            pixel(&badged, size - 4, size - 17),
            "the corner has the badge"
        );
        assert_eq!(pixel(&badged, size - 4, size - 17)[3], 255);
    }

    #[test]
    fn the_log_starts_again_past_its_limit() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("cbm-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (path, old) = (dir.join("a.log"), dir.join("a.old.log"));
        let mut log = super::LogFile::create(path.clone(), old.clone(), 100).unwrap();
        // One write per record, as env_logger does, 20 bytes each.
        for line in 0..12 {
            log.write_all(format!("line {line:02} of the log!\n").as_bytes())
                .unwrap();
        }
        log.flush().unwrap();
        let size = |p: &std::path::Path| std::fs::metadata(p).unwrap().len();
        assert!(size(&path) <= 100, "the log stays within its limit");
        assert_eq!(size(&old), 100, "the full part moved to the old log");
        let now = std::fs::read_to_string(&path).unwrap();
        assert!(now.ends_with("line 11 of the log!\n"), "{now}");
        drop(log);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_app_icon_draws_on_a_transparent_background() {
        let (rgba, size) = super::icon_rgba();
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        let alpha = |x: u32, y: u32| rgba[((y * size + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "a corner shows what is behind the icon");
        assert_eq!(alpha(size / 2, size / 2), 255, "the middle is drawn");
    }
}
