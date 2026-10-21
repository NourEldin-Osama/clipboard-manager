//! Icons: SVGs from Fluent UI System Icons and Material Symbols (chosen per
//! icon, see assets/icons/LICENSES.md), rasterised at the exact pixel size they
//! are drawn at and tinted. They are drawn into a rect, so they sit exactly on
//! its centre in a row of buttons, unlike font glyphs.

use eframe::egui::load::{SizeHint, TexturePoll};
use eframe::egui::{Painter, Pos2, Rect, TextureOptions, Ui};

use super::view;
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
    /// Still the old painted triangle until its icon is chosen.
    Play,
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
        (concat!("bytes://icons/", $name, ".svg"), include_bytes!(concat!("../../assets/icons/", $name, ".svg")) as &[u8])
    };
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
        Icon::Play | Icon::Dot => return None,
    })
}

const ALL: [Icon; 37] = [
    Icon::Grid, Icon::List, Icon::Detail, Icon::Copy, Icon::Open, Icon::Delete, Icon::Pin, Icon::PinFilled, Icon::Pause, Icon::Play,
    Icon::Add, Icon::Settings, Icon::Back, Icon::Check, Icon::Edit, Icon::General, Icon::Search, Icon::Clipboard,
    Icon::Keyboard, Icon::Palette, Icon::Lock, Icon::Close, Icon::Dropdown, Icon::Text, Icon::Link, Icon::Image, Icon::Folder,
    Icon::File, Icon::Save, Icon::Browser, Icon::Info, Icon::Tag, Icon::Reset, Icon::Warning, Icon::Time, Icon::Language, Icon::Dot,
];

/// Installs the SVG loader and hands it the bundled icons.
pub(super) fn register(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    for icon in ALL {
        if let Some((uri, bytes)) = source(icon) {
            ctx.include_bytes(uri, bytes);
        }
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
    let ctx = p.ctx();
    // Rasterised at the pixel size it's shown at, so edges stay sharp.
    let px = (side * ctx.pixels_per_point()).round().max(1.0) as u32;
    let hint = SizeHint::Size { width: px, height: px, maintain_aspect_ratio: true };
    match ctx.try_load_texture(uri, TextureOptions::LINEAR, hint) {
        Ok(TexturePoll::Ready { texture }) => {
            let uv = if mirrored(icon) && i18n::rtl() {
                Rect::from_min_max(Pos2::new(1.0, 0.0), Pos2::new(0.0, 1.0))
            } else {
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
            };
            p.image(texture.id, square, uv, color);
        }
        Ok(TexturePoll::Pending { .. }) => ctx.request_repaint(),
        Err(e) => log::warn!("icon {uri}: {e}"),
    }
}

/// The two shapes that aren't SVG icons.
fn paint_shape(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let s = rect.width() / 16.0;
    match icon {
        Icon::Play => {
            let pts = vec![Pos2::new(c.x - 4.0 * s, c.y - 6.0 * s), Pos2::new(c.x + 6.0 * s, c.y), Pos2::new(c.x - 4.0 * s, c.y + 6.0 * s)];
            p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        _ => {
            p.circle_filled(c, 3.5 * s, color);
        }
    }
}

/// A flat icon button: background only on hover, or `fill` when given.
pub(super) fn icon_button_ex(ui: &mut Ui, icon: Icon, tip: &str, size: Vec2, glyph: f32, fill: Option<Color32>, fg: Option<Color32>) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let tip = tr(tip);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), tip));
    paint_button(ui, rect, &resp, icon, glyph, fill, fg);
    view::tip(resp, tip)
}

pub(super) fn icon_button(ui: &mut Ui, icon: Icon, tip: &str) -> egui::Response {
    icon_button_ex(ui, icon, tip, Vec2::splat(34.0), 16.0, None, None)
}

/// An icon button at a fixed spot, for actions drawn over a row or card.
pub(super) fn icon_button_at(ui: &Ui, rect: Rect, id: egui::Id, icon: Icon, tip: &str) -> egui::Response {
    let resp = ui.interact(rect, id, Sense::click());
    paint_button(ui, rect, &resp, icon, 15.0, None, None);
    view::tip(resp, tr(tip))
}

fn paint_button(ui: &Ui, rect: Rect, resp: &egui::Response, icon: Icon, glyph: f32, fill: Option<Color32>, fg: Option<Color32>) {
    let v = ui.visuals();
    let p = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    match fill {
        Some(f) => {
            p.rect_filled(rect, CornerRadius::same(8), f);
            if resp.hovered() {
                p.rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(if pressed { 10 } else { 28 }));
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
    let color = fg.unwrap_or(if resp.hovered() { v.strong_text_color() } else { v.text_color() });
    paint(p, Rect::from_center_size(rect.center(), Vec2::splat(glyph)), icon, color);
}

/// An icon and a label side by side, both centred on the row's midline.
/// Font glyphs can't do this: each font puts its symbols at its own height.
pub(super) fn icon_text_size(ui: &Ui, text: &egui::WidgetText, size: f32, icon: f32) -> (std::sync::Arc<egui::Galley>, Vec2) {
    let g = text.clone().into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, FontId::proportional(size));
    let gap = if g.size().x > 0.0 { 6.0 } else { 0.0 };
    let w = icon + gap + g.size().x;
    let size = Vec2::new(w, g.size().y.max(icon));
    (g, size)
}

/// The icon goes on the reading-start side: left, or right in Arabic.
pub(super) fn paint_icon_text(p: &Painter, rect: Rect, icon: Icon, icon_size: f32, g: std::sync::Arc<egui::Galley>, color: Color32) {
    let ic = view::flip(rect, Rect::from_center_size(Pos2::new(rect.left() + icon_size / 2.0, rect.center().y), Vec2::splat(icon_size)));
    paint(p, ic, icon, color);
    let x = if i18n::rtl() { ic.left() - 6.0 - g.size().x } else { ic.right() + 6.0 };
    p.galley(Pos2::new(x, rect.center().y - g.size().y / 2.0), g, color);
}

/// A rounded tinted label with an icon: kind, state or status.
pub(super) fn chip(ui: &mut Ui, icon: Icon, text: &str, color: Color32) -> egui::Response {
    let (g, size) = icon_text_size(ui, &view::txt(tr(text), 12.0, Some(color)), 12.0, 12.0);
    let pad = Vec2::new(8.0, 3.0);
    let (rect, resp) = ui.allocate_exact_size(size + pad * 2.0 + Vec2::new(0.0, 2.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(10), color.gamma_multiply(0.14));
    paint_icon_text(ui.painter(), rect.shrink2(pad), icon, 12.0, g, color);
    resp
}

/// A pill button with an icon and a label; tinted while `selected`.
pub(super) fn pill(ui: &mut Ui, icon: Icon, job: egui::text::LayoutJob, color: Color32, selected: bool) -> egui::Response {
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let pad = Vec2::new(10.0, 5.0);
    let size = Vec2::new(14.0 + 6.0 + g.size().x, g.size().y.max(14.0)) + pad * 2.0;
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let v = ui.visuals();
    if selected {
        ui.painter().rect_filled(rect, CornerRadius::same(14), color.gamma_multiply(0.13));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(14), v.widgets.hovered.weak_bg_fill);
    }
    let icon_color = if selected { color } else { v.text_color() };
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
    let w = ui.available_width().max(200.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
    let v = ui.visuals();
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), v.widgets.hovered.weak_bg_fill);
    }
    let color = if resp.hovered() { v.strong_text_color() } else { v.text_color() };
    paint(ui.painter(), view::flip(rect, Rect::from_center_size(Pos2::new(rect.left() + 16.0, rect.center().y), Vec2::splat(15.0))), icon, color);
    let at = view::flip(rect, Rect::from_min_size(Pos2::new(rect.left() + 34.0, rect.center().y), Vec2::ZERO)).min;
    view::paint_text(ui, ui.painter(), at, view::start_anchor(egui::Align::Center), tr(text), FontId::proportional(14.0), color);
    resp
}

/// The painted icon for a leading glyph in older label text ("✔ Done").
pub(super) fn from_prefix(text: &str) -> Option<(Icon, &str)> {
    const MAP: [(&str, Icon); 8] = [("✔", Icon::Check), ("⚠", Icon::Warning), ("★", Icon::PinFilled), ("📂", Icon::Folder), ("⏸", Icon::Pause), ("💡", Icon::Info), ("🔍", Icon::Search), ("●", Icon::Dot)];
    MAP.iter().find_map(|(g, i)| text.strip_prefix(g).map(|rest| (*i, rest.trim_start())))
}
