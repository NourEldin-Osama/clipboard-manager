//! Messages at the bottom of the window, in the style of shadcn's Sonner
//! (D55): a stack at the bottom right (bottom left in Arabic), above the
//! status bar. The newest is in front and up to two older ones peek out
//! above it; pointing at the stack opens it, and their time stops meanwhile.
//! The update's message (D65) stays in the stack until the update is done
//! or closed.

use std::sync::Arc;

use eframe::egui::Galley;

use super::*;

/// The messages' width, as Sonner's.
const WIDTH: f32 = 356.0;
/// From the window's side, and from its bottom: above the status bar.
const EDGE: f32 = 16.0;
const ABOVE: f32 = 44.0;
/// How far each older message peeks out above the one in front of it, and
/// the gap between them while the stack is open.
const PEEK: f32 = 12.0;
const GAP: f32 = 12.0;
const PAD: f32 = 14.0;
const ICON: f32 = 16.0;
const BUTTON_H: f32 = 24.0;
const BUTTON_PAD: f32 = 8.0;
/// How far a message slides as it comes, and how long that takes; it
/// fades out over the last part of its time.
const SLIDE: f32 = 16.0;
const SLIDE_TIME: f32 = 0.2;
const FADE_TIME: f32 = 0.15;
/// Between the icon, the text and the buttons.
const SPACE: f32 = 10.0;
/// The update message's close button: its size, how far it sits from the
/// card's top and end, and the room its text keeps free for it.
const CLOSE: f32 = 13.0;
const CLOSE_AT: f32 = 10.0;
const CLOSE_ROOM: f32 = 19.0;
/// Around the download's progress line, which sits under the title.
const PROGRESS_ABOVE: f32 = 7.0;
const PROGRESS_BELOW: f32 = 4.0;

/// How a button of the update's message looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Look {
    /// Filled, near black (near white in the dark theme), as the other
    /// messages' buttons: the next step.
    Main,
    /// Filled gray, as an ordinary button.
    Plain,
    Link,
}

/// What the update's message has that other messages don't (D65). It is
/// made again from `App::update` in every frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct UpdateParts {
    /// How much of the download is done, from 0 to 1: a thin line under
    /// the title.
    progress: Option<f32>,
    /// A spinner in place of the icon, while the download is checked.
    spinner: bool,
    /// The icon in red, after a failure.
    red: bool,
    /// From the far end: the next step first.
    buttons: Vec<(String, UpdateAction, Look)>,
}

/// A message laid out at its width.
pub(super) struct Laid {
    title: Vec<(Arc<Galley>, bool)>,
    detail: Vec<(Arc<Galley>, bool)>,
    /// The button's label, with its keys in a fainter color.
    action: Option<Arc<Galley>>,
    /// "Show", for the text of a copy that wasn't saved.
    show: Option<Arc<Galley>>,
    /// The update message's buttons, from the far end.
    update_buttons: Vec<(Arc<Galley>, UpdateAction, Look)>,
    /// The buttons sit in a row under the text: beside them, the text
    /// would be squeezed into short lines.
    below: bool,
    /// The width the text was laid out in, and the height of its lines
    /// with the progress line.
    text_w: f32,
    text_h: f32,
    pub(super) height: f32,
}

/// What a click or a press on the messages asks for.
#[derive(Default)]
pub(super) struct Asked {
    /// The id of the message whose button was clicked.
    pub(super) action: Option<u64>,
    /// The id of the message whose Show is held down.
    show: Option<u64>,
    /// A button of the update's message.
    update: Option<UpdateAction>,
}

/// The update's message for `state`: its title, second line, icon and the
/// rest. `None` while there is nothing to show.
fn update_message(state: &UpdateState) -> Option<(String, Option<String>, Icon, UpdateParts)> {
    let whats_new = || (t!("update-whats-new"), UpdateAction::WhatsNew, Look::Link);
    let mut parts = UpdateParts::default();
    let (title, detail, icon) = match state {
        UpdateState::Available(r) => {
            parts.buttons = vec![
                (t!("update-download"), UpdateAction::Download, Look::Main),
                (t!("update-skip"), UpdateAction::Skip, Look::Plain),
                whats_new(),
            ];
            let title = t!("update-available", version = r.label());
            (title, None, Icon::Refresh)
        }
        UpdateState::Downloading(r, p) => {
            let (done, total) = download_amounts(r, p);
            if done >= total {
                parts.spinner = true;
                let title = t!("update-verifying", version = r.label());
                (title, None, Icon::Refresh)
            } else {
                parts.progress = Some(done as f32 / total as f32);
                parts.buttons = vec![(t!("common-cancel"), UpdateAction::Cancel, Look::Plain)];
                let detail = match p.retry_in() {
                    Some(seconds) => t!("download-retrying", seconds = seconds),
                    None => t!("update-progress", done = done >> 20, total = total >> 20),
                };
                let title = t!("update-downloading", version = r.label());
                (title, Some(detail), Icon::Refresh)
            }
        }
        UpdateState::Ready(r, _) => {
            parts.buttons = vec![
                (t!("update-restart"), UpdateAction::Restart, Look::Main),
                whats_new(),
            ];
            let title = t!("update-ready", version = r.label());
            (title, Some(t!("update-ready-detail")), Icon::Check)
        }
        UpdateState::Failed { error, release } => {
            parts.red = true;
            parts.buttons = vec![(t!("update-try-again"), UpdateAction::TryAgain, Look::Main)];
            if release.is_some() {
                parts.buttons.push(whats_new());
            }
            (
                t!("update-failed-title"),
                Some(sentence(error)),
                Icon::Warning,
            )
        }
        UpdateState::Idle | UpdateState::Checking | UpdateState::UpToDate => return None,
    };
    Some((title, detail, icon, parts))
}

/// `text` with a capital first letter, as a sentence on its own line.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

impl App {
    /// Puts the update's message in the stack, or takes it out, to match
    /// `update`. It comes in front when it appears; closing it hides it
    /// until the update's next step.
    pub(crate) fn sync_update_message(&mut self) {
        let message = if self.update_hidden {
            None
        } else {
            update_message(&self.update)
        };
        let at = self.toasts.iter().position(|t| t.update.is_some());
        let Some((title, detail, icon, parts)) = message else {
            if let Some(at) = at {
                self.toasts.remove(at);
            }
            return;
        };
        let toast = match at {
            Some(at) => &mut self.toasts[at],
            // Its time never runs out.
            None => self.toast_with(String::new(), Duration::MAX),
        };
        toast.text = title;
        toast.detail = detail;
        toast.icon = Some(icon);
        toast.update = Some(parts);
    }

    /// Draws the messages; `popup` leaves out the update's, which belongs
    /// to the full window.
    pub(super) fn toasts(&mut self, ctx: &egui::Context, popup: bool) {
        // Said when a window first draws, so a start hidden at sign-in
        // doesn't use the message up unseen.
        if std::mem::take(&mut self.whats_new_pending) {
            let version = crate::update::VERSION;
            let has_notes = !crate::update::notes(version).is_empty();
            let toast = self.toast_with(
                t!("toast-updated", version = version),
                Duration::from_secs(15),
            );
            if has_notes {
                toast.action = Some((t!("update-whats-new"), Command::ShowWhatsNew));
            }
        }
        if let Some((_, at)) = &self.error {
            if at.elapsed() > Duration::from_secs(20) {
                self.error = None;
            }
        }
        // Messages on the screen's corner show in the note instead.
        let in_window = |t: &Toast| !t.on_screen && !(popup && t.update.is_some());
        let shown: Vec<&Toast> = self.toasts.iter().filter(|t| in_window(t)).collect();
        if shown.is_empty() {
            self.toasts_hovered = false;
            return;
        }
        let content = ctx.content_rect();
        let width = WIDTH.min(content.width() - 2.0 * EDGE).max(120.0);
        let (anchor, x) = if i18n::rtl() {
            (Align2::LEFT_BOTTOM, EDGE)
        } else {
            (Align2::RIGHT_BOTTOM, -EDGE)
        };
        let open = self.toasts_hovered;
        let showing = self.toast_showing;
        let mut asked = Asked::default();
        let area = egui::Area::new(egui::Id::unique("toasts"))
            .order(egui::Order::Foreground)
            .anchor(anchor, Vec2::new(x, -ABOVE))
            .show(ctx, |ui| {
                let laid: Vec<Laid> = shown
                    .iter()
                    .map(|t| lay_out(ui, t, width, showing == Some(t.id), None))
                    .collect();
                if open {
                    // All of them, the newest at the bottom.
                    let total =
                        laid.iter().map(|l| l.height).sum::<f32>() + GAP * (laid.len() - 1) as f32;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, total), Sense::hover());
                    let mut top = rect.top();
                    for (toast, l) in shown.iter().zip(&laid) {
                        let card = Rect::from_min_size(
                            Pos2::new(rect.left(), top),
                            Vec2::new(width, l.height),
                        );
                        let (shift, opacity) = motion(ui.ctx(), toast);
                        let card = card.translate(Vec2::new(0.0, shift));
                        ui.scope(|ui| {
                            ui.multiply_opacity(opacity);
                            paint_card(ui, card);
                            paint_content(ui, card, toast, l, &mut asked, None);
                        });
                        top += l.height + GAP;
                    }
                } else {
                    // The newest in front; up to two older ones peek out.
                    let behind = (laid.len() - 1).min(2);
                    let front = laid.last().expect("there is a message");
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(width, front.height + PEEK * behind as f32),
                        Sense::hover(),
                    );
                    let card = Rect::from_min_size(
                        Pos2::new(rect.left(), rect.bottom() - front.height),
                        Vec2::new(width, front.height),
                    );
                    // The farthest first, so the nearer ones cover it.
                    for step in (1..=behind).rev() {
                        let narrower = width * (1.0 - 0.05 * step as f32);
                        paint_card(
                            ui,
                            Rect::from_center_size(
                                card.center() - Vec2::new(0.0, PEEK * step as f32),
                                Vec2::new(narrower, front.height),
                            ),
                        );
                    }
                    let toast = shown.last().expect("there is a message");
                    let (shift, opacity) = motion(ui.ctx(), toast);
                    let card = card.translate(Vec2::new(0.0, shift));
                    ui.scope(|ui| {
                        ui.multiply_opacity(opacity);
                        paint_card(ui, card);
                        paint_content(ui, card, toast, front, &mut asked, None);
                    });
                }
            });
        self.toasts_hovered = ctx
            .pointer_hover_pos()
            .is_some_and(|p| area.response.rect.contains(p));
        let mut moving = false;
        for toast in self.toasts.iter_mut().filter(|t| in_window(t)) {
            let (shift, opacity) = motion(ctx, toast);
            moving |= shift > 0.0 || opacity < 1.0;
            toast.drawn = true;
        }
        if moving {
            ctx.request_repaint();
        }
        self.toast_showing = asked.show;
        if let Some(id) = asked.action {
            if let Some(at) = self.toasts.iter().position(|t| t.id == id) {
                let toast = self.toasts.remove(at);
                if let Some((_, command)) = toast.action {
                    self.run(command);
                }
            }
        }
        if let Some(action) = asked.update {
            self.update_action(action, ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

/// How far a message is still below its place, and how opaque it is: it
/// slides up as it comes and fades as its time runs out (19.1). Nothing
/// moves when animations are off.
fn motion(ctx: &egui::Context, toast: &Toast) -> (f32, f32) {
    if ctx.style_of(ctx.theme()).animation_time == 0.0 {
        return (0.0, 1.0);
    }
    let id = egui::Id::unique(("toast-slide", toast.id));
    let shift = if toast.drawn {
        ctx.animate_value_with_time(id, 0.0, SLIDE_TIME)
    } else {
        ctx.animate_value_with_time(id, SLIDE, 0.0)
    };
    let left = toast.how_long.saturating_sub(toast.shown_for).as_secs_f32();
    let fading = (left / FADE_TIME).clamp(0.0, 1.0);
    let coming = 1.0 - shift / SLIDE;
    (shift + (1.0 - fading) * SLIDE / 2.0, coming.min(fading))
}

/// The card behind a message: its shadow, fill and edge.
fn paint_card(ui: &Ui, rect: Rect) {
    let v = ui.visuals();
    let round = CornerRadius::same(radius::MEDIUM);
    let p = ui.painter();
    p.add(v.popup_shadow.as_shape(rect, round));
    let fill = if v.dark_mode {
        v.window_fill
    } else {
        Color32::WHITE
    };
    p.rect(
        rect,
        round,
        fill,
        Stroke::new(1.0, theme::line_color(v.dark_mode)),
        egui::StrokeKind::Inside,
    );
}

/// Lays out a message at `width`: its text beside its icon and buttons, or,
/// when there it would take more than two lines and fewer across the whole
/// width, above a row with the buttons. The update's message always has its
/// buttons under its text, and room for its close button. While `showing`,
/// the second line is the copy's whole text. With `app_icon` it is the note
/// on the screen: the app icon, and never Show.
pub(super) fn lay_out(
    ui: &Ui,
    toast: &Toast,
    width: f32,
    showing: bool,
    app_icon: Option<&egui::TextureHandle>,
) -> Laid {
    let v = ui.visuals();
    let (_, label_color) = theme::message_button(v.dark_mode);
    let action = toast.action.as_ref().map(|(label, _)| {
        // The keys go after the label, or before it in Arabic.
        let keys = toast
            .keys
            .clone()
            .map(|keys| (keys, label_color.gamma_multiply(0.65)));
        let job = pill_job(label, size::SMALL, label_color, keys);
        ui.fonts_mut(|f| f.layout_job(job))
    });
    let update_buttons: Vec<(Arc<Galley>, UpdateAction, Look)> = toast
        .update
        .iter()
        .flat_map(|parts| &parts.buttons)
        .map(|(label, action, look)| {
            let color = match look {
                Look::Main => label_color,
                Look::Plain => v.widgets.inactive.fg_stroke.color,
                Look::Link => v.hyperlink_color,
            };
            let job = pill_job(label, size::SMALL, color, None);
            (ui.fonts_mut(|f| f.layout_job(job)), *action, *look)
        })
        .collect();
    let show = toast
        .secret
        .as_ref()
        .filter(|_| app_icon.is_none())
        .map(|_| {
            let job = pill_job(&t!("toast-show"), size::SMALL, v.hyperlink_color, None);
            ui.fonts_mut(|f| f.layout_job(job))
        });
    let buttons = action
        .as_ref()
        .map_or(0.0, |g| g.size().x + 2.0 * BUTTON_PAD + SPACE)
        + show
            .as_ref()
            .map_or(0.0, |g| ICON + 4.0 + g.size().x + SPACE);
    let icon = if toast.icon.is_some() || app_icon.is_some() {
        ICON + SPACE
    } else {
        0.0
    };
    let close = if toast.update.is_some() {
        CLOSE_ROOM
    } else {
        0.0
    };
    let room = width - 2.0 * PAD - icon - close;
    let second = match (&toast.secret, showing) {
        (Some(secret), true) => Some(secret.as_str()),
        _ => toast.detail.as_deref(),
    };
    // The title and the second line in `text_w`, up to three lines each.
    let text_in = |text_w: f32| {
        let lines = |text: &str, size: f32, color: Color32| {
            bidi::lines(
                ui,
                text,
                &[],
                FontId::proportional(size),
                color,
                Color32::TRANSPARENT,
                text_w,
                3,
                None,
            )
        };
        let title = lines(&toast.text, size::BODY, v.strong_text_color());
        let detail = second.map_or_else(Vec::new, |d| lines(d, size::SMALL, v.weak_text_color()));
        (title, detail)
    };
    let mut text_w = (room - buttons).max(40.0);
    let (mut title, mut detail) = text_in(text_w);
    let mut below = !update_buttons.is_empty();
    if buttons > 0.0 && title.len() + detail.len() > 2 {
        let (wide_title, wide_detail) = text_in(room);
        if wide_title.len() + wide_detail.len() < title.len() + detail.len() {
            (title, detail, below, text_w) = (wide_title, wide_detail, true, room);
        }
    }
    let progress = toast.update.as_ref().is_some_and(|p| p.progress.is_some());
    let text_h = text_height(&title, &detail, progress);
    let height = if below {
        2.0 * PAD + text_h + SPACE + BUTTON_H
    } else {
        2.0 * PAD + text_h.max(BUTTON_H)
    };
    Laid {
        title,
        detail,
        action,
        show,
        update_buttons,
        below,
        text_w,
        text_h,
        height,
    }
}

/// The height of a message's lines, with a small gap before the second
/// line, or the progress line between them.
fn text_height(
    title: &[(Arc<Galley>, bool)],
    detail: &[(Arc<Galley>, bool)],
    progress: bool,
) -> f32 {
    let gap = if progress {
        PROGRESS_ABOVE + PROGRESS_LINE + PROGRESS_BELOW
    } else if detail.is_empty() {
        0.0
    } else {
        2.0
    };
    title
        .iter()
        .chain(detail)
        .map(|(g, _)| g.size().y)
        .sum::<f32>()
        + gap
}

/// Paints a message's icon (or `app_icon`), text and buttons in its card,
/// mirrored in Arabic, and notes a click on its button or a press on Show.
pub(super) fn paint_content(
    ui: &mut Ui,
    card: Rect,
    toast: &Toast,
    laid: &Laid,
    asked: &mut Asked,
    app_icon: Option<&egui::TextureHandle>,
) {
    let v = ui.visuals().clone();
    let inner = card.shrink(PAD);
    let text_h = laid.text_h;
    // The icon and the text sit in the middle beside the buttons, or at the
    // top, above the row of buttons.
    let (mid, buttons_mid) = if laid.below {
        (inner.top() + text_h / 2.0, inner.bottom() - BUTTON_H / 2.0)
    } else {
        (inner.center().y, inner.center().y)
    };
    // Laid out left to right, then mirrored for Arabic.
    let m = |r: Rect| flip(card, r);
    let parts = toast.update.as_ref();
    let mut left = inner.left();
    let icon_at = m(Rect::from_min_size(
        Pos2::new(left, mid - ICON / 2.0),
        Vec2::splat(ICON),
    ));
    if let Some(texture) = app_icon {
        let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        ui.painter()
            .image(texture.id(), icon_at, whole, Color32::WHITE);
        left += ICON + SPACE;
    } else if parts.is_some_and(|p| p.spinner) {
        paint_spinner(ui, icon_at, v.text_color());
        left += ICON + SPACE;
    } else if let Some(icon) = toast.icon {
        let color = if parts.is_some_and(|p| p.red) {
            text_shade(RED, v.dark_mode)
        } else {
            v.text_color()
        };
        icons::paint(ui.painter(), icon_at, icon, color);
        left += ICON + SPACE;
    }
    if parts.is_some() {
        let at = m(Rect::from_min_size(
            Pos2::new(card.right() - CLOSE_AT - CLOSE, card.top() + CLOSE_AT),
            Vec2::splat(CLOSE),
        ));
        // Easier to hit than it looks.
        let r = ui.interact(
            at.expand(4.0),
            egui::Id::unique(("toast-close", toast.id)),
            Sense::click(),
        );
        let label = t!("update-close");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let color = if r.hovered() {
            v.strong_text_color()
        } else {
            v.weak_text_color()
        };
        icons::paint(ui.painter(), at, Icon::Close, color);
        if tip(r, &label).clicked() {
            asked.update = Some(UpdateAction::Close);
        }
    }
    let mut right = inner.right();
    let labels = parts.iter().flat_map(|p| &p.buttons);
    for ((g, action, look), (label, ..)) in laid.update_buttons.iter().zip(labels) {
        let w = match look {
            Look::Link => g.size().x,
            Look::Main | Look::Plain => g.size().x + 2.0 * BUTTON_PAD,
        };
        let at = m(Rect::from_min_size(
            Pos2::new(right - w, buttons_mid - BUTTON_H / 2.0),
            Vec2::new(w, BUTTON_H),
        ));
        let id = egui::Id::unique(("toast-update", toast.id, *action));
        let r = ui.interact(at, id, Sense::click());
        let role = if *look == Look::Link {
            egui::Role::Link
        } else {
            egui::Role::Button
        };
        r.widget_info(|| egui::WidgetInfo::labeled(role, true, label));
        let fill = match look {
            Look::Main => {
                let (fill, _) = theme::message_button(v.dark_mode);
                Some(if r.hovered() {
                    fill.gamma_multiply(0.85)
                } else {
                    fill
                })
            }
            Look::Plain if r.hovered() => Some(v.widgets.hovered.weak_bg_fill),
            Look::Plain => Some(v.widgets.inactive.weak_bg_fill),
            Look::Link => None,
        };
        if let Some(fill) = fill {
            ui.painter()
                .rect_filled(at, CornerRadius::same(radius::SMALL), fill);
        }
        let text = Rect::from_center_size(at.center(), g.size());
        ui.painter()
            .galley(text.min, g.clone(), Color32::PLACEHOLDER);
        if *look == Look::Link && r.hovered() {
            ui.painter().hline(
                text.x_range(),
                text.bottom() - 1.0,
                Stroke::new(1.0, v.hyperlink_color),
            );
        }
        let r = if *look == Look::Link {
            r.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            r
        };
        if r.clicked() {
            asked.update = Some(*action);
        }
        right -= w + SPACE;
    }
    if let Some(g) = &laid.action {
        let w = g.size().x + 2.0 * BUTTON_PAD;
        let at = m(Rect::from_min_size(
            Pos2::new(right - w, buttons_mid - BUTTON_H / 2.0),
            Vec2::new(w, BUTTON_H),
        ));
        let label = toast.action.as_ref().map_or("", |(l, _)| l.as_str());
        let r = ui.interact(
            at,
            egui::Id::unique(("toast-action", toast.id)),
            Sense::click(),
        );
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
        let (fill, _) = theme::message_button(v.dark_mode);
        let fill = if r.hovered() {
            fill.gamma_multiply(0.85)
        } else {
            fill
        };
        ui.painter()
            .rect_filled(at, CornerRadius::same(radius::SMALL), fill);
        let text_at = at.center() - g.size() / 2.0;
        ui.painter()
            .galley(text_at, g.clone(), Color32::PLACEHOLDER);
        if r.clicked() {
            asked.action = Some(toast.id);
        }
        right -= w + SPACE;
    }
    if let Some(g) = &laid.show {
        let w = ICON + 4.0 + g.size().x;
        let at = Rect::from_min_size(
            Pos2::new(right - w, buttons_mid - BUTTON_H / 2.0),
            Vec2::new(w, BUTTON_H),
        );
        let r = ui.interact(
            m(at),
            egui::Id::unique(("toast-show", toast.id)),
            Sense::click_and_drag(),
        );
        let label = t!("toast-show");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let eye = Rect::from_min_size(
            Pos2::new(at.left(), buttons_mid - ICON / 2.0),
            Vec2::splat(ICON),
        );
        icons::paint(ui.painter(), m(eye), icons::Icon::Eye, v.hyperlink_color);
        let text = m(Rect::from_min_size(
            Pos2::new(at.left() + ICON + 4.0, buttons_mid - g.size().y / 2.0),
            g.size(),
        ));
        ui.painter()
            .galley(text.min, g.clone(), Color32::PLACEHOLDER);
        if r.is_pointer_button_down_on() {
            asked.show = Some(toast.id);
        }
    }
    let mut y = mid - text_h / 2.0;
    let colors = [v.strong_text_color(), v.weak_text_color()];
    let progress = parts.and_then(|p| p.progress);
    for (part, lines) in [&laid.title, &laid.detail].into_iter().enumerate() {
        if part == 1 {
            if let Some(done) = progress {
                y += PROGRESS_ABOVE;
                let track = m(Rect::from_min_size(
                    Pos2::new(left, y),
                    Vec2::new(laid.text_w, PROGRESS_LINE),
                ));
                paint_progress(ui.painter(), track, done, v.dark_mode);
                y += PROGRESS_LINE + PROGRESS_BELOW;
            } else if !lines.is_empty() {
                y += 2.0;
            }
        }
        for (g, _) in lines {
            let at = m(Rect::from_min_size(Pos2::new(left, y), g.size()));
            ui.painter().galley(at.min, g.clone(), colors[part]);
            y += g.size().y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(text: &str, detail: Option<&str>, button: Option<&str>) -> Toast {
        Toast {
            text: text.into(),
            detail: detail.map(Into::into),
            icon: Some(icons::Icon::Lock),
            how_long: Duration::from_secs(8),
            shown_for: Duration::ZERO,
            action: button.map(|b| (b.to_string(), Command::ShowWeaker)),
            keys: None,
            secret: Some("sk-proj-abcdefghijklmnopqrstuvwxyz0123456789".into()),
            on_screen: false,
            drawn: false,
            id: 1,
            update: None,
        }
    }

    /// The update's message at `state`, as `sync_update_message` makes it.
    fn update_toast(state: &UpdateState) -> Toast {
        let (title, detail, icon, parts) = update_message(state).expect("a message to show");
        let mut toast = message(&title, detail.as_deref(), None);
        toast.icon = Some(icon);
        toast.secret = None;
        toast.update = Some(parts);
        toast
    }

    fn buttons(state: &UpdateState) -> Vec<UpdateAction> {
        update_message(state).map_or_else(Vec::new, |(.., parts)| {
            parts.buttons.iter().map(|(_, action, _)| *action).collect()
        })
    }

    #[test]
    fn the_update_message_follows_each_step() {
        use UpdateAction::*;
        let release = Release::made_up("0.7.2", 29, 30 << 20);
        let found = UpdateState::Available(release.clone());
        assert_eq!(buttons(&found), [Download, Skip, WhatsNew]);
        // Before the server names the size, the release's own counts.
        let progress = Arc::new(Progress::default());
        progress.done.store(6 << 20, Ordering::Relaxed);
        let downloading = UpdateState::Downloading(release.clone(), progress.clone());
        let (_, detail, _, parts) = update_message(&downloading).unwrap();
        assert_eq!(parts.progress, Some(0.2));
        assert!(detail.is_some_and(|d| d.contains("6 / 30")));
        assert_eq!(buttons(&downloading), [Cancel]);
        // All of it is there: the download is being checked.
        progress.done.store(30 << 20, Ordering::Relaxed);
        let (_, _, _, parts) = update_message(&downloading).unwrap();
        assert!(parts.spinner && parts.progress.is_none() && parts.buttons.is_empty());
        let ready = UpdateState::Ready(release.clone(), PathBuf::new());
        assert_eq!(buttons(&ready), [Restart, WhatsNew]);
        let failed = UpdateState::Failed {
            error: "the download is damaged".into(),
            release: Some(release),
        };
        let (_, detail, _, parts) = update_message(&failed).unwrap();
        assert!(parts.red);
        assert_eq!(detail.as_deref(), Some("The download is damaged"));
        assert_eq!(buttons(&failed), [TryAgain, WhatsNew]);
        // A check that failed has no release to tell about.
        let check_failed = UpdateState::Failed {
            error: "offline".into(),
            release: None,
        };
        assert_eq!(buttons(&check_failed), [TryAgain]);
        for quiet in [
            UpdateState::Idle,
            UpdateState::Checking,
            UpdateState::UpToDate,
        ] {
            assert!(update_message(&quiet).is_none());
        }
    }

    #[test]
    fn the_update_message_has_its_buttons_under_its_text() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let release = Release::made_up("0.7.2", 29, 30 << 20);
            let found = lay_out(
                ui,
                &update_toast(&UpdateState::Available(release.clone())),
                WIDTH,
                false,
                None,
            );
            assert!(found.below);
            assert_eq!(found.update_buttons.len(), 3);
            let row: f32 = found
                .update_buttons
                .iter()
                .map(|(g, _, look)| match look {
                    Look::Link => g.size().x + SPACE,
                    _ => g.size().x + 2.0 * BUTTON_PAD + SPACE,
                })
                .sum();
            assert!(
                row - SPACE <= WIDTH - 2.0 * PAD,
                "the buttons fit in one row"
            );
            assert!(
                found.text_w <= WIDTH - 2.0 * PAD - ICON - SPACE - CLOSE_ROOM,
                "the text keeps clear of the close button"
            );
            // The progress line sits between the title and the amount.
            let progress = Arc::new(Progress::default());
            progress.done.store(6 << 20, Ordering::Relaxed);
            let downloading = UpdateState::Downloading(release, progress);
            let laid = lay_out(ui, &update_toast(&downloading), WIDTH, false, None);
            let lines: f32 = laid
                .title
                .iter()
                .chain(&laid.detail)
                .map(|(g, _)| g.size().y)
                .sum();
            let line = PROGRESS_ABOVE + PROGRESS_LINE + PROGRESS_BELOW;
            assert_eq!(laid.text_h, lines + line);
            assert_eq!(laid.height, 2.0 * PAD + laid.text_h + SPACE + BUTTON_H);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
    }

    #[test]
    fn a_message_grows_with_its_text_and_shows_the_copy_only_while_held() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let short = lay_out(ui, &message("Copied", None, None), WIDTH, false, None);
            let long = message(
                "Not saved: looks like a password or API key, from a program with a long name",
                Some("From Visual Studio Code · sk-…6789"),
                Some("Save anyway"),
            );
            let laid = lay_out(ui, &long, WIDTH, false, None);
            assert!(
                laid.height > short.height,
                "a longer text wraps onto more lines"
            );
            assert!(laid.title.len() > 1);
            assert!(laid.action.is_some() && laid.show.is_some());
            let hint: String = laid
                .detail
                .iter()
                .map(|(g, _)| g.text().to_string())
                .collect();
            assert!(
                !hint.contains("abcdefgh"),
                "the copy's text stays hidden: {hint}"
            );
            let held = lay_out(ui, &long, WIDTH, true, None);
            let shown: String = held
                .detail
                .iter()
                .map(|(g, _)| g.text().to_string())
                .collect();
            assert!(shown.contains("abcdefgh"), "Show reveals it: {shown}");
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
    }

    #[test]
    fn buttons_go_under_text_they_would_squeeze() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            // Beside Show and Save anyway, this text would take short lines.
            let squeezed = message(
                "Not saved: a password",
                Some("From Clipboard Manager · 28 characters"),
                Some("Save anyway"),
            );
            let laid = lay_out(ui, &squeezed, WIDTH, false, None);
            assert!(laid.below, "the buttons go under the text");
            assert_eq!(laid.title.len(), 1, "the title then takes one line");
            // A short text keeps its button beside it.
            let mut undo = message("Deleted", None, Some("Undo"));
            undo.secret = None;
            undo.keys = Some("Ctrl+Z".into());
            assert!(!lay_out(ui, &undo, WIDTH, false, None).below);
        });
        // The font atlas upload no renderer takes here.
        output.textures_delta.clear();
    }
}
