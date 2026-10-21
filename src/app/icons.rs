//! Vector icons painted with strokes on a 16-unit grid. Font glyphs (⏸ + ⚙)
//! each carry their own ascent and padding, so they never line up in a row of
//! buttons; painted icons sit exactly on the centre of their rect.

use eframe::egui::{Painter, Pos2, Rect, Shape, Ui};

use super::view;
use crate::store::Kind;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Icon {
    Grid,
    List,
    Detail,
    Copy,
    Expand,
    Trash,
    Pause,
    Play,
    Plus,
    Gear,
    Back,
    Check,
    Star,
    StarOutline,
    Edit,
    Sliders,
    Search,
    Clipboard,
    Keyboard,
    Palette,
    Lock,
    Close,
    ChevronDown,
    Text,
    Link,
    Image,
    Folder,
    File,
    Save,
    Globe,
    Info,
    Tag,
    Reset,
    Warning,
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

/// Paints `icon` centred in `rect`, scaled to its shorter side.
pub(super) fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 16.0;
    let pt = |x: f32, y: f32| Pos2::new(c.x + x * s, c.y + y * s);
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(pt(x0, y0), pt(x1, y1));
    let stroke = Stroke::new(1.5 * s, color);
    let line = |pts: &[(f32, f32)]| Shape::line(pts.iter().map(|&(x, y)| pt(x, y)).collect(), stroke);
    let radius = CornerRadius::same((1.5 * s).round() as u8);
    let outline = |rect: Rect| p.rect_stroke(rect, radius, stroke, egui::StrokeKind::Middle);
    match icon {
        Icon::Grid => {
            for (x, y) in [(-6.5, -6.5), (1.0, -6.5), (-6.5, 1.0), (1.0, 1.0)] {
                outline(r(x, y, x + 5.5, y + 5.5));
            }
        }
        Icon::List => {
            for y in [-5.0, 0.0, 5.0] {
                p.circle_filled(pt(-6.0, y), 1.1 * s, color);
                p.add(line(&[(-3.0, y), (6.5, y)]));
            }
        }
        Icon::Detail => {
            outline(r(-7.0, -6.0, 7.0, 6.0));
            p.add(line(&[(-1.5, -6.0), (-1.5, 6.0)]));
            for y in [-3.0, 0.0, 3.0] {
                p.add(line(&[(-5.0, y), (-3.5, y)]));
            }
        }
        Icon::Copy => {
            outline(r(-2.0, -2.0, 6.5, 6.5));
            p.add(line(&[(-4.0, 2.5), (-6.5, 2.5), (-6.5, -6.5), (2.5, -6.5), (2.5, -4.0)]));
        }
        Icon::Expand => {
            p.add(line(&[(1.0, -1.0), (6.0, -6.0)]));
            p.add(line(&[(1.5, -6.0), (6.0, -6.0), (6.0, -1.5)]));
            p.add(line(&[(-1.0, 1.0), (-6.0, 6.0)]));
            p.add(line(&[(-6.0, 1.5), (-6.0, 6.0), (-1.5, 6.0)]));
        }
        Icon::Trash => {
            p.add(line(&[(-6.5, -4.0), (6.5, -4.0)]));
            p.add(line(&[(-2.5, -4.0), (-2.5, -6.5), (2.5, -6.5), (2.5, -4.0)]));
            p.add(line(&[(-5.0, -4.0), (-4.0, 6.5), (4.0, 6.5), (5.0, -4.0)]));
            p.add(line(&[(-1.5, -1.0), (-1.5, 3.5)]));
            p.add(line(&[(1.5, -1.0), (1.5, 3.5)]));
        }
        Icon::Pause => {
            p.rect_filled(r(-5.0, -6.0, -1.5, 6.0), radius, color);
            p.rect_filled(r(1.5, -6.0, 5.0, 6.0), radius, color);
        }
        Icon::Play => {
            p.add(Shape::convex_polygon(vec![pt(-4.0, -6.0), pt(6.0, 0.0), pt(-4.0, 6.0)], color, Stroke::NONE));
        }
        Icon::Plus => {
            p.add(line(&[(-6.0, 0.0), (6.0, 0.0)]));
            p.add(line(&[(0.0, -6.0), (0.0, 6.0)]));
        }
        Icon::Gear => {
            let teeth = Stroke::new(3.0 * s, color);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let (sin, cos) = a.sin_cos();
                p.line_segment([pt(cos * 4.0, sin * 4.0), pt(cos * 7.6, sin * 7.6)], teeth);
            }
            p.circle_stroke(c, 4.0 * s, Stroke::new(2.4 * s, color));
        }
        Icon::Back => {
            p.add(line(&[(3.0, -6.0), (-3.0, 0.0), (3.0, 6.0)]));
        }
        Icon::Check => {
            p.add(line(&[(-5.5, 0.5), (-1.5, 4.5), (5.5, -4.0)]));
        }
        Icon::Edit => {
            p.add(line(&[(-5.5, 5.5), (-5.0, 2.5), (3.0, -5.5), (5.5, -3.0), (-2.5, 5.0), (-5.5, 5.5)]));
            p.add(line(&[(1.0, -3.5), (3.5, -1.0)]));
        }
        Icon::Sliders => {
            for (y, x) in [(-4.5, 2.5), (0.0, -2.5), (4.5, 1.0)] {
                p.add(line(&[(-6.5, y), (6.5, y)]));
                p.circle_filled(pt(x, y), 2.1 * s, color);
            }
        }
        Icon::Search => {
            p.circle_stroke(pt(-1.2, -1.2), 4.6 * s, stroke);
            p.add(line(&[(2.3, 2.3), (6.3, 6.3)]));
        }
        Icon::Clipboard => {
            outline(r(-5.5, -5.0, 5.5, 7.0));
            p.rect_filled(r(-2.5, -7.0, 2.5, -3.5), radius, color);
            p.add(line(&[(-2.5, 0.0), (2.5, 0.0)]));
            p.add(line(&[(-2.5, 3.5), (1.0, 3.5)]));
        }
        Icon::Keyboard => {
            outline(r(-7.5, -4.5, 7.5, 4.5));
            for x in [-4.5, -1.5, 1.5, 4.5] {
                p.circle_filled(pt(x, -1.5), 0.9 * s, color);
            }
            p.add(line(&[(-3.5, 1.8), (3.5, 1.8)]));
        }
        Icon::Palette => {
            p.circle_stroke(c, 6.5 * s, stroke);
            for (x, y) in [(-3.0, -2.0), (0.5, -3.8), (3.5, -1.0)] {
                p.circle_filled(pt(x, y), 1.2 * s, color);
            }
            p.circle_filled(pt(-1.0, 3.2), 1.6 * s, color);
        }
        Icon::Lock => {
            p.rect_filled(r(-5.5, -1.0, 5.5, 7.0), radius, color);
            p.add(line(&[(-3.5, -1.0), (-3.5, -3.5), (-2.0, -5.8), (2.0, -5.8), (3.5, -3.5), (3.5, -1.0)]));
        }
        Icon::Close => {
            p.add(line(&[(-5.0, -5.0), (5.0, 5.0)]));
            p.add(line(&[(5.0, -5.0), (-5.0, 5.0)]));
        }
        Icon::ChevronDown => {
            p.add(line(&[(-5.0, -2.5), (0.0, 2.5), (5.0, -2.5)]));
        }
        Icon::Text => {
            outline(r(-5.5, -7.0, 5.5, 7.0));
            for (y, w) in [(-3.5, 3.0), (0.0, 3.0), (3.5, 1.0)] {
                p.add(line(&[(-3.0, y), (w, y)]));
            }
        }
        Icon::Link => {
            p.rect_stroke(r(-7.0, -3.0, 1.0, 3.0), CornerRadius::same((3.0 * s) as u8), stroke, egui::StrokeKind::Middle);
            p.rect_stroke(r(-1.0, -3.0, 7.0, 3.0), CornerRadius::same((3.0 * s) as u8), stroke, egui::StrokeKind::Middle);
        }
        Icon::Image => {
            outline(r(-7.0, -5.5, 7.0, 5.5));
            p.circle_filled(pt(3.0, -2.0), 1.4 * s, color);
            p.add(line(&[(-7.0, 4.0), (-2.5, -0.5), (1.5, 3.0), (3.5, 1.5), (7.0, 4.5)]));
        }
        Icon::Folder => {
            p.add(Shape::closed_line(vec![pt(-7.0, -5.0), pt(-2.5, -5.0), pt(-1.0, -3.0), pt(7.0, -3.0), pt(7.0, 5.5), pt(-7.0, 5.5)], stroke));
        }
        Icon::File => {
            p.add(Shape::closed_line(vec![pt(-5.0, -7.0), pt(1.5, -7.0), pt(5.0, -3.5), pt(5.0, 7.0), pt(-5.0, 7.0)], stroke));
            p.add(line(&[(1.5, -7.0), (1.5, -3.5), (5.0, -3.5)]));
        }
        Icon::Save => {
            p.add(Shape::closed_line(vec![pt(-6.5, -6.5), pt(4.0, -6.5), pt(6.5, -4.0), pt(6.5, 6.5), pt(-6.5, 6.5)], stroke));
            outline(r(-3.5, -6.5, 2.5, -2.5));
            outline(r(-3.5, 1.5, 3.5, 6.5));
        }
        Icon::Globe => {
            p.circle_stroke(c, 7.0 * s, stroke);
            p.add(line(&[(-7.0, 0.0), (7.0, 0.0)]));
            let arc = |x: f32| -> Vec<Pos2> { (0..=12).map(|i| { let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 12.0; pt(x * a.cos(), 7.0 * a.sin()) }).collect() };
            p.add(Shape::line(arc(3.2), stroke));
            p.add(Shape::line(arc(-3.2), stroke));
        }
        Icon::Info => {
            p.circle_stroke(c, 6.8 * s, Stroke::new(1.3 * s, color));
            p.circle_filled(pt(0.0, -3.2), 1.0 * s, color);
            p.add(line(&[(0.0, -0.8), (0.0, 4.0)]));
        }
        Icon::Tag => {
            p.add(Shape::closed_line(vec![pt(-6.5, -6.5), pt(0.0, -6.5), pt(6.5, 0.0), pt(0.0, 6.5), pt(-6.5, 0.0)], stroke));
            p.circle_filled(pt(-3.3, -3.3), 1.2 * s, color);
        }
        Icon::Reset => {
            let pts: Vec<Pos2> = (0..=18).map(|i| { let a = -0.6 + i as f32 * 4.9 / 18.0; pt(5.5 * a.cos(), 5.5 * a.sin()) }).collect();
            let end = pts[0];
            p.add(Shape::line(pts, stroke));
            p.add(Shape::line(vec![end + Vec2::new(-3.5, -0.5) * s, end, end + Vec2::new(0.5, -3.5) * s], stroke));
        }
        Icon::Warning => {
            p.add(Shape::closed_line(vec![pt(0.0, -7.0), pt(7.5, 6.0), pt(-7.5, 6.0)], stroke));
            p.add(line(&[(0.0, -2.5), (0.0, 1.8)]));
            p.circle_filled(pt(0.0, 4.0), 0.9 * s, color);
        }
        Icon::Dot => {
            p.circle_filled(c, 3.5 * s, color);
        }
        Icon::Star | Icon::StarOutline => {
            let pts: Vec<Pos2> = (0..10)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    let rad = if i % 2 == 0 { 7.0 } else { 2.9 };
                    pt(a.cos() * rad, a.sin() * rad + 0.5)
                })
                .collect();
            if icon == Icon::Star {
                // A star isn't convex: fill the inner pentagon and each point separately.
                let inner: Vec<Pos2> = pts.iter().skip(1).step_by(2).copied().collect();
                p.add(Shape::convex_polygon(inner.clone(), color, Stroke::NONE));
                for i in 0..5 {
                    p.add(Shape::convex_polygon(vec![inner[(i + 4) % 5], pts[i * 2], inner[i]], color, Stroke::NONE));
                }
            } else {
                p.add(Shape::closed_line(pts, stroke));
            }
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
    const MAP: [(&str, Icon); 8] = [("✔", Icon::Check), ("⚠", Icon::Warning), ("★", Icon::Star), ("📂", Icon::Folder), ("⏸", Icon::Pause), ("💡", Icon::Info), ("🔍", Icon::Search), ("●", Icon::Dot)];
    MAP.iter().find_map(|(g, i)| text.strip_prefix(g).map(|rest| (*i, rest.trim_start())))
}
