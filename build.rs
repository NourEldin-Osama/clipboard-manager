//! Windows exe resources: the app icon (Explorer, taskbar, Alt+Tab) and the
//! file details Windows shows in Properties. Nothing to do for other targets.

fn main() {
    const ICON: &str = "assets/app-icon/app.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set("ProductName", "Clipboard Manager").set("FileDescription", "Clipboard Manager").set("LegalCopyright", "MIT or Apache-2.0");
    if std::path::Path::new(ICON).exists() {
        res.set_icon(ICON);
    }
    // A missing resource compiler shouldn't stop a build; the exe just has no icon.
    if let Err(e) = res.compile() {
        println!("cargo:warning=Windows resources not embedded: {e}");
    }
}
