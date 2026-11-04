//! Icons: SVGs from Fluent UI System Icons and Material Symbols (chosen per
//! icon, see assets/icons/LICENSES.md), rasterised at the exact pixel size they
//! are drawn at and tinted. They are drawn into a rect, so they sit exactly on
//! its centre in a row of buttons, unlike font glyphs. The logos of server
//! types (assets/logos) are drawn the same way; those in colour keep it.

use eframe::egui::load::{SizeHint, TexturePoll};
use eframe::egui::{Painter, Pos2, Rect, TextureOptions, Ui};

use super::view::{self, size};
use crate::config::ServerKind;
use crate::store::Kind;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Icon {
    Grid,
    List,
    Detail,
    Copy,
    Open,
    Delete,
    Pin,
    PinFilled,
    Pause,
    Play,
    Refresh,
    Eye,
    EyeOff,
    Add,
    Settings,
    Back,
    Check,
    Edit,
    General,
    Search,
    Clipboard,
    Keyboard,
    Palette,
    Lock,
    Close,
    Dropdown,
    Text,
    Link,
    Image,
    Folder,
    File,
    Save,
    Browser,
    Info,
    Tag,
    Reset,
    Warning,
    Time,
    Language,
    /// A plain filled circle (status light), not an icon.
    Dot,
}

/// The icon for an item kind (and its filter).
pub(super) fn kind(k: Kind) -> Icon {
    match k {
        Kind::Text => Icon::Text,
        Kind::Link => Icon::Link,
        Kind::Image => Icon::Image,
        Kind::Files => Icon::Folder,
    }
}

pub(super) fn view_icon(v: ViewMode) -> Icon {
    match v {
        ViewMode::Grid => Icon::Grid,
        ViewMode::List => Icon::List,
        ViewMode::Detail => Icon::Detail,
    }
}

macro_rules! svg {
    ($name:literal) => {
        (
            concat!("bytes://icons/", $name, ".svg"),
            include_bytes!(concat!("../../assets/icons/", $name, ".svg")) as &[u8],
        )
    };
}

macro_rules! logo {
    ($name:literal) => {
        (
            concat!("bytes://logos/", $name, ".svg"),
            include_bytes!(concat!("../../assets/logos/", $name, ".svg")) as &[u8],
        )
    };
}

/// The logo of a server type: its loader URI and bytes, and whether it is in
/// colour. The others are white, tinted like the icons; another server gets
/// Fluent's server icon.
fn logo_source(kind: ServerKind) -> ((&'static str, &'static [u8]), bool) {
    match kind {
        ServerKind::LlamaCpp => (logo!("llamacpp"), true),
        ServerKind::Ollama => (logo!("ollama"), false),
        ServerKind::LmStudio => (logo!("lmstudio"), false),
        ServerKind::Vllm => (logo!("vllm"), true),
        ServerKind::Unsloth => (logo!("unsloth"), true),
        ServerKind::OpenRouter => (logo!("openrouter"), false),
        ServerKind::Other => (svg!("server"), false),
    }
}

/// The SVG behind an icon: its loader URI and bytes.
fn source(icon: Icon) -> Option<(&'static str, &'static [u8])> {
    Some(match icon {
        Icon::Grid => svg!("grid"),
        Icon::List => svg!("list"),
        Icon::Detail => svg!("detail"),
        Icon::Copy => svg!("copy"),
        Icon::Open => svg!("open"),
        Icon::Delete => svg!("delete"),
        Icon::Pin => svg!("pin"),
        Icon::PinFilled => svg!("pin_filled"),
        Icon::Pause => svg!("pause"),
        Icon::Play => svg!("play"),
        Icon::Refresh => svg!("refresh"),
        Icon::Eye => svg!("eye"),
        Icon::EyeOff => svg!("eye_off"),
        Icon::Add => svg!("add"),
        Icon::Settings => svg!("settings"),
        Icon::Back => svg!("back"),
        Icon::Check => svg!("check"),
        Icon::Edit => svg!("edit"),
        Icon::General => svg!("general"),
        Icon::Search => svg!("search"),
        Icon::Clipboard => svg!("clipboard"),
        Icon::Keyboard => svg!("keyboard"),
        Icon::Palette => svg!("palette"),
        Icon::Lock => svg!("lock"),
        Icon::Close => svg!("close"),
        Icon::Dropdown => svg!("dropdown"),
        Icon::Text => svg!("text"),
        Icon::Link => svg!("link"),
        Icon::Image => svg!("image"),
        Icon::Folder => svg!("folder"),
        Icon::File => svg!("file"),
        Icon::Save => svg!("save"),
        Icon::Browser => svg!("browser"),
        Icon::Info => svg!("info"),
        Icon::Tag => svg!("tag"),
        Icon::Reset => svg!("reset"),
        Icon::Warning => svg!("warning"),
        Icon::Time => svg!("time"),
        Icon::Language => svg!("language"),
        Icon::Dot => return None,
    })
}

const ALL: [Icon; 40] = [
    Icon::Refresh,
    Icon::Eye,
    Icon::EyeOff,
    Icon::Grid,
    Icon::List,
    Icon::Detail,
    Icon::Copy,
    Icon::Open,
    Icon::Delete,
    Icon::Pin,
    Icon::PinFilled,
    Icon::Pause,
    Icon::Play,
    Icon::Add,
    Icon::Settings,
    Icon::Back,
    Icon::Check,
    Icon::Edit,
    Icon::General,
    Icon::Search,
    Icon::Clipboard,
    Icon::Keyboard,
    Icon::Palette,
    Icon::Lock,
    Icon::Close,
    Icon::Dropdown,
    Icon::Text,
    Icon::Link,
    Icon::Image,
    Icon::Folder,
    Icon::File,
    Icon::Save,
    Icon::Browser,
    Icon::Info,
    Icon::Tag,
    Icon::Reset,
    Icon::Warning,
    Icon::Time,
    Icon::Language,
    Icon::Dot,
];

/// Installs the SVG loader and hands it the bundled icons.
pub(super) fn register(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    for icon in ALL {
        if let Some((uri, bytes)) = source(icon) {
            ctx.include_bytes(uri, bytes);
        }
    }
    for kind in ServerKind::ALL {
        let ((uri, bytes), _) = logo_source(kind);
        ctx.include_bytes(uri, bytes);
    }
}

/// Icons that point somewhere, turned around for right-to-left.
fn mirrored(icon: Icon) -> bool {
    matches!(icon, Icon::Back)
}

/// Paints `icon` filling the square at the centre of `rect`, tinted `color`.
pub(super) fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), Vec2::splat(side));
    let Some((uri, _)) = source(icon) else {
        return paint_shape(p, square, icon, color);
    };
    paint_svg(p, square, uri, color, mirrored(icon) && i18n::rtl());
}

/// Paints the logo of a server type filling the square at the centre of
/// `rect`: tinted `color`, unless it is in colour.
pub(super) fn paint_logo(p: &Painter, rect: Rect, kind: ServerKind, color: Color32) {
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), Vec2::splat(side));
    let ((uri, _), coloured) = logo_source(kind);
    let tint = if coloured { Color32::WHITE } else { color };
    paint_svg(p, square, uri, tint, false);
}

/// The logo of a server type as an inline widget.
pub(super) fn logo(ui: &mut Ui, kind: ServerKind, size: f32, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_logo(ui.painter(), rect, kind, color);
    resp
}

fn paint_svg(p: &Painter, square: Rect, uri: &str, tint: Color32, mirror: bool) {
    let ctx = p.ctx();
    // Rasterised at the pixel size it's shown at, so edges stay sharp.
    let px = (square.width() * ctx.pixels_per_point()).round().max(1.0) as u32;
    let hint = SizeHint::Size {
        width: px,
        height: px,
        maintain_aspect_ratio: true,
    };
    match ctx.try_load_texture(uri, TextureOptions::LINEAR, hint) {
        Ok(TexturePoll::Ready { texture }) => {
            let uv = if mirror {
                Rect::from_min_max(Pos2::new(1.0, 0.0), Pos2::new(0.0, 1.0))
            } else {
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
            };
            p.image(texture.id, square, uv, tint);
        }
        Ok(TexturePoll::Pending { .. }) => ctx.request_repaint(),
        Err(e) => log::warn!("icon {uri}: {e}"),
    }
}

/// The status light: a plain circle, not an icon.
fn paint_shape(p: &Painter, rect: Rect, _icon: Icon, color: Color32) {
    p.circle_filled(rect.center(), 3.5 * rect.width() / 16.0, color);
}

/// A flat icon button: background only on hover, or `fill` when given.
pub(super) fn icon_button_ex(
    ui: &mut Ui,
    icon: Icon,
    tip: &str,
    size: Vec2,
    glyph: f32,
    fill: Option<Color32>,
    fg: Option<Color32>,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), tip));
    paint_button(ui, rect, &resp, icon, glyph, fill, fg);
    view::tip(resp, tip)
}

pub(super) fn icon_button(ui: &mut Ui, icon: Icon, tip: &str) -> egui::Response {
    icon_button_ex(ui, icon, tip, Vec2::splat(34.0), 16.0, None, None)
}

/// An icon button at a fixed spot, for actions drawn over a row or card.
pub(super) fn icon_button_at(
    ui: &Ui,
    rect: Rect,
    id: egui::Id,
    icon: Icon,
    tip: &str,
) -> egui::Response {
    let resp = ui.interact(rect, id, Sense::click());
    paint_button(ui, rect, &resp, icon, 15.0, None, None);
    view::tip(resp, tip)
}

fn paint_button(
    ui: &Ui,
    rect: Rect,
    resp: &egui::Response,
    icon: Icon,
    glyph: f32,
    fill: Option<Color32>,
    fg: Option<Color32>,
) {
    let v = ui.visuals();
    let p = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    match fill {
        Some(f) => {
            p.rect_filled(rect, CornerRadius::same(8), f);
            if resp.hovered() {
                p.rect_filled(
                    rect,
                    CornerRadius::same(8),
                    Color32::from_white_alpha(if pressed { 10 } else { 28 }),
                );
            }
        }
        None if pressed => {
            p.rect_filled(rect, CornerRadius::same(8), v.widgets.active.weak_bg_fill);
        }
        None if resp.hovered() => {
            p.rect_filled(rect, CornerRadius::same(8), v.widgets.hovered.weak_bg_fill);
        }
        None => {}
    }
    let color = fg.unwrap_or(if resp.hovered() {
        v.strong_text_color()
    } else {
        v.text_color()
    });
    paint(
        p,
        Rect::from_center_size(rect.center(), Vec2::splat(glyph)),
        icon,
        color,
    );
}

/// An icon and a label side by side, both centred on the row's midline.
/// Font glyphs can't do this: each font puts its symbols at its own height.
pub(super) fn icon_text_size(
    ui: &Ui,
    text: &egui::WidgetText,
    size: f32,
    icon: f32,
) -> (std::sync::Arc<egui::Galley>, Vec2) {
    let g = text.clone().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        FontId::proportional(size),
    );
    let gap = if g.size().x > 0.0 { 6.0 } else { 0.0 };
    let w = icon + gap + g.size().x;
    let size = Vec2::new(w, g.size().y.max(icon));
    (g, size)
}

/// The icon goes on the reading-start side: left, or right in Arabic.
pub(super) fn paint_icon_text(
    p: &Painter,
    rect: Rect,
    icon: Icon,
    icon_size: f32,
    g: std::sync::Arc<egui::Galley>,
    color: Color32,
) {
    let ic = view::flip(
        rect,
        Rect::from_center_size(
            Pos2::new(rect.left() + icon_size / 2.0, rect.center().y),
            Vec2::splat(icon_size),
        ),
    );
    paint(p, ic, icon, color);
    let x = if i18n::rtl() {
        ic.left() - 6.0 - g.size().x
    } else {
        ic.right() + 6.0
    };
    p.galley(Pos2::new(x, rect.center().y - g.size().y / 2.0), g, color);
}

/// A rounded tinted label with an icon: kind, state or status. The tint is
/// `color`; the icon and text take its text shade.
pub(super) fn chip(ui: &mut Ui, icon: Icon, text: &str, color: Color32) -> egui::Response {
    let shade = view::text_shade(color, ui.visuals().dark_mode);
    let (g, size) = icon_text_size(ui, &view::txt(text, 12.0, Some(shade)), 12.0, 12.0);
    let pad = Vec2::new(8.0, 3.0);
    let (rect, resp) =
        ui.allocate_exact_size(size + pad * 2.0 + Vec2::new(0.0, 2.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), color.gamma_multiply(0.14));
    paint_icon_text(ui.painter(), rect.shrink2(pad), icon, 12.0, g, shade);
    resp
}

/// A pill button with an icon and a label; tinted while `selected`.
pub(super) fn pill(
    ui: &mut Ui,
    icon: Icon,
    job: egui::text::LayoutJob,
    color: Color32,
    selected: bool,
) -> egui::Response {
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let pad = Vec2::new(10.0, 5.0);
    let size = Vec2::new(14.0 + 6.0 + g.size().x, g.size().y.max(14.0)) + pad * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let v = ui.visuals();
    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(14), color.gamma_multiply(0.13));
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(14), v.widgets.hovered.weak_bg_fill);
    }
    let icon_color = if selected {
        view::text_shade(color, v.dark_mode)
    } else {
        v.text_color()
    };
    paint_icon_text(ui.painter(), rect.shrink2(pad), icon, 14.0, g, icon_color);
    resp
}

/// An icon as an inline widget; in a centred row it sits on the midline.
pub(super) fn glyph(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint(ui.painter(), rect, icon, color);
    resp
}

/// A full-width menu entry with an icon.
pub(super) fn menu_item(ui: &mut Ui, icon: Icon, text: &str) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
    let v = ui.visuals();
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), v.widgets.hovered.weak_bg_fill);
    }
    let color = if resp.hovered() {
        v.strong_text_color()
    } else {
        v.text_color()
    };
    paint(
        ui.painter(),
        view::flip(
            rect,
            Rect::from_center_size(
                Pos2::new(rect.left() + 16.0, rect.center().y),
                Vec2::splat(15.0),
            ),
        ),
        icon,
        color,
    );
    let at = view::flip(
        rect,
        Rect::from_min_size(Pos2::new(rect.left() + 34.0, rect.center().y), Vec2::ZERO),
    )
    .min;
    view::paint_text(
        ui,
        ui.painter(),
        at,
        view::start_anchor(egui::Align::Center),
        text,
        FontId::proportional(size::BODY),
        color,
    );
    resp
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_logo_draws_and_only_those_in_colour_keep_colours() {
        for kind in ServerKind::ALL {
            let ((uri, bytes), coloured) = logo_source(kind);
            let image = egui_extras::image::load_svg_bytes(bytes, &Default::default())
                .unwrap_or_else(|e| panic!("{uri}: {e}"));
            let drawn: Vec<&Color32> = image.pixels.iter().filter(|p| p.a() > 0).collect();
            assert!(!drawn.is_empty(), "{uri} draws nothing");
            // White shapes keep red, green and blue equal at every edge.
            let white = drawn.iter().all(|p| p.r() == p.g() && p.g() == p.b());
            assert_eq!(!white, coloured, "{uri}");
        }
    }
}
