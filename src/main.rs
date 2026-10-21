//! clipboard-manager: an offline clipboard history with on-device OCR and
//! semantic search over text and images.

// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod clipboard;
mod config;
mod embed;
mod images;
mod import;
mod models;
mod ocr;
mod search;
mod store;
mod tray;
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
    init_logging(&paths);
    log::info!("clipboard-manager {} starting, data in {}", env!("CARGO_PKG_VERSION"), paths.root.display());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Clipboard Manager")
            .with_app_id("clipboard-manager")
            .with_inner_size([1040.0, 680.0])
            .with_min_inner_size([560.0, 380.0])
            .with_drag_and_drop(true)
            .with_icon(icon()),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "clipboard-manager",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, paths)?))),
    )
}

/// Logs to `clipboard-manager.log` in the data folder (release builds on
/// Windows have no console). The previous run's log is kept as `.old.log`.
fn init_logging(paths: &config::Paths) {
    let log_path = paths.root.join("clipboard-manager.log");
    let _ = std::fs::rename(&log_path, paths.root.join("clipboard-manager.old.log"));
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,winit=warn,wgpu=warn,eframe=warn"));
    if let Ok(file) = std::fs::File::create(&log_path) {
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    builder.init();
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
}

fn icon() -> egui::IconData {
    let (rgba, n) = icon_rgba();
    egui::IconData { rgba, width: n, height: n }
}

/// A simple generated icon: a rounded clipboard in the accent colour. Also
/// used for the tray.
pub fn icon_rgba() -> (Vec<u8>, u32) {
    let n = 64u32;
    let mut rgba = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32, y as f32);
            let board = rounded(fx, fy, 10.0, 8.0, 54.0, 60.0, 8.0);
            let clip = rounded(fx, fy, 22.0, 3.0, 42.0, 14.0, 4.0);
            let line = (18.0..46.0).contains(&fx) && [26.0, 36.0, 46.0].iter().any(|l| (fy - l).abs() < 2.5);
            let px = &mut rgba[((y * n + x) * 4) as usize..][..4];
            if clip {
                px.copy_from_slice(&[230, 232, 255, 255]);
            } else if board {
                if line {
                    px.copy_from_slice(&[255, 255, 255, 255]);
                } else {
                    px.copy_from_slice(&[88, 101, 242, 255]);
                }
            }
        }
    }
    (rgba, n)
}

fn rounded(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}
