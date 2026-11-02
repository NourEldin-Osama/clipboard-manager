//! clipboard-manager: an offline clipboard history with on-device OCR and
//! semantic search over text and images.

// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod clipboard;
mod config;
mod download;
mod embed;
mod i18n;
mod images;
mod import;
mod installed;
mod library;
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
    let (cfg, _, _) = config::Config::load(&paths.config);
    i18n::set(cfg.language, cfg.time_format);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::t!("app-title"))
            .with_app_id("clipboard-manager")
            .with_inner_size([1040.0, 680.0])
            .with_min_inner_size([560.0, 380.0])
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
    let _ = std::fs::rename(&log_path, paths.root.join("clipboard-manager.old.log"));
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,winit=warn,wgpu=warn,eframe=warn"),
    );
    if let Ok(file) = std::fs::File::create(&log_path) {
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    builder.init();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
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

#[cfg(test)]
mod tests {
    #[test]
    fn the_app_icon_draws_on_a_transparent_background() {
        let (rgba, size) = super::icon_rgba();
        assert_eq!(rgba.len(), (size * size * 4) as usize);
        let alpha = |x: u32, y: u32| rgba[((y * size + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "a corner shows what is behind the icon");
        assert_eq!(alpha(size / 2, size / 2), 255, "the middle is drawn");
    }
}
