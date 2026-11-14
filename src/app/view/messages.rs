//! Messages at the bottom of the window, in the style of shadcn's Sonner
//! (D55): a stack at the bottom right (bottom left in Arabic), above the
//! status bar. The newest is in front and up to two older ones peek out
//! above it, each a little narrower and fainter; pointing at the stack opens
//! it, and their time stops meanwhile. The cards glide to their new places.
//! Every message has a close button. The update's message (D65) stays in
//! the stack until the update is done or closed.

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
/// How many older messages peek out behind the newest, and how much
/// narrower and fainter each is than the one in front of it.
const BEHIND: usize = MAX_TOASTS - 1;
const NARROWER: f32 = 0.05;
const FAINTER: f32 = 0.2;
/// How long the cards take to glide to their places: as the stack opens or
/// closes, and as a message comes or goes.
const STACK_TIME: f32 = 0.2;
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
/// A message's close button: its size, how far it sits from the card's top
/// and end, and the room its text and the buttons beside it keep free for
/// it.
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

/// What a question has that other messages don't: it waits for its
/// answer, and shows only in the window that asked it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Question {
    /// Asked in the popup: it shows there, and goes when the popup closes.
    pub(crate) in_popup: bool,
    /// Its button takes the keyboard when it first shows: the question was
    /// asked from the keyboard.
    pub(crate) focus: bool,
}

/// Makes `toast` the question before a rule that stops recording copies
/// from the program `app`, shown as `name`: Never record makes the rule;
/// it waits for its answer in the window that asked it (`in_popup` or
/// not), and takes the keyboard when it was asked from the keyboard.
pub(super) fn ask_to_ignore(
    toast: &mut Toast,
    app: &str,
    name: &str,
    in_popup: bool,
    by_key: bool,
) {
    toast.text = t!("toast-never-record-question", app = bidi::isolate(name));
    toast.how_long = Duration::MAX;
    let rule = Command::IgnoreApp(app.to_string(), name.to_string());
    toast.action = Some((t!("toast-never-record"), rule));
    toast.question = Some(Question {
        in_popup,
        focus: by_key,
    });
}

/// Takes the question of the popup's window (`in_popup`) or the full
/// window out of `toasts`, with no answer. Whether there was one.
pub(super) fn close_question(toasts: &mut Vec<Toast>, in_popup: bool) -> bool {
    let before = toasts.len();
    toasts.retain(|t| t.question.as_ref().is_none_or(|q| q.in_popup != in_popup));
    toasts.len() < before
}

/// A message laid out at its width.
pub(super) struct Laid {
    title: Vec<(Arc<Galley>, bool)>,
    detail: Vec<(Arc<Galley>, bool)>,
    /// The button's label, with its keys in a fainter color.
    action: Option<Arc<Galley>>,
    /// A question's Cancel, a plain text button after its main one.
    cancel: Option<Arc<Galley>>,
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
    /// The id of the question whose Cancel was clicked.
    cancel: Option<u64>,
    /// The id of the message whose close button was clicked.
    close: Option<u64>,
    /// The id of the question whose button took the keyboard.
    focused: Option<u64>,
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
        // Messages on the screen's corner show in the note instead, and a
        // question only in the window that asked it.
        let in_window = |t: &Toast| {
            !t.on_screen
                && !(popup && t.update.is_some())
                && t.question.as_ref().is_none_or(|q| q.in_popup == popup)
        };
        let shown: Vec<&Toast> = self.toasts.iter().filter(|t| in_window(t)).collect();
        if shown.is_empty() {
            self.toasts_hovered = false;
            return;
        }
        let content = ctx.content_rect();
        let width = WIDTH.min(content.width() - 2.0 * EDGE).max(120.0);
        let rtl = i18n::rtl();
        let (anchor, x) = if rtl {
            (Align2::LEFT_BOTTOM, EDGE)
        } else {
            (Align2::RIGHT_BOTTOM, -EDGE)
        };
        let open = self.toasts_hovered;
        let showing = self.toast_showing;
        let mut asked = Asked::default();
        let mut stack = Rect::NOTHING;
        let mut gliding = false;
        egui::Area::new(egui::Id::unique("toasts"))
            .order(egui::Order::Foreground)
            .anchor(anchor, Vec2::new(x, -ABOVE))
            .show(ctx, |ui| {
                let laid: Vec<Laid> = shown
                    .iter()
                    .map(|t| lay_out(ui, t, width, showing == Some(t.id), None))
                    .collect();
                let heights: Vec<f32> = laid.iter().map(|l| l.height).collect();
                let places = stack_places(&heights, open);
                let height = places.iter().map(|p| p.lift + p.height).fold(0.0, f32::max);
                ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
                // Placed from the stack's bottom edge, which stays put while
                // the area takes a frame to follow a new size.
                stack = stack_rect(content, width, height, rtl);
                let ids: Vec<u64> = shown.iter().map(|t| t.id).collect();
                let on_the_way = glide(ui.ctx(), popup, &ids, &places);
                gliding = on_the_way != places;
                // The oldest first, so the newer ones cover it.
                for ((toast, l), place) in shown.iter().zip(&laid).zip(&on_the_way) {
                    let (shift, opacity) = motion(ui.ctx(), toast);
                    let card = place.card(stack).translate(Vec2::new(0.0, shift));
                    ui.scope(|ui| {
                        ui.multiply_opacity(opacity * place.opacity);
                        paint_card(ui, card);
                        if place.content > 0.0 {
                            ui.multiply_opacity(place.content);
                            paint_content(ui, card, toast, l, &mut asked, None);
                        }
                    });
                }
            });
        self.toasts_hovered = ctx.pointer_hover_pos().is_some_and(|p| stack.contains(p));
        let mut moving = gliding;
        for toast in self.toasts.iter_mut().filter(|t| in_window(t)) {
            let (shift, opacity) = motion(ctx, toast);
            moving |= shift > 0.0 || opacity < 1.0;
            toast.drawn = true;
        }
        if moving {
            ctx.request_repaint();
        }
        self.toast_showing = asked.show;
        if let Some(id) = asked.focused {
            let question = self.toasts.iter_mut().find(|t| t.id == id);
            if let Some(question) = question.and_then(|t| t.question.as_mut()) {
                question.focus = false;
            }
            // The keyboard is on the question's button: show where.
            self.keyboard_focus = true;
        }
        if let Some(id) = asked.action {
            if let Some(at) = self.toasts.iter().position(|t| t.id == id) {
                let toast = self.toasts.remove(at);
                // Typing goes to the search field again, as after a menu.
                if toast.question.is_some() {
                    self.focus_search.store(true, Ordering::SeqCst);
                }
                if let Some((_, command)) = toast.action {
                    self.run(command);
                }
            }
        }
        if let Some(id) = asked.cancel {
            self.toasts.retain(|t| t.id != id);
            self.focus_search.store(true, Ordering::SeqCst);
        }
        // Only the message goes: a delete it could undo stays in the undo
        // history, for Ctrl+Z.
        if let Some(at) = asked
            .close
            .and_then(|id| self.toasts.iter().position(|t| t.id == id))
        {
            let toast = self.toasts.remove(at);
            // A question closed so answers no, as Cancel does.
            if toast.question.is_some() {
                self.focus_search.store(true, Ordering::SeqCst);
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

/// Where a message's card sits in the stack, and how it shows there.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    /// From the stack's bottom edge to the card's.
    lift: f32,
    height: f32,
    /// Its width, as a part of the stack's.
    scale: f32,
    opacity: f32,
    /// How much its text and buttons show: not at all on an older card
    /// behind the newest while the stack is closed.
    content: f32,
}

impl Place {
    /// Its card in `stack`, in the middle of its width.
    fn card(&self, stack: Rect) -> Rect {
        let half = stack.width() * self.scale / 2.0;
        let bottom = stack.bottom() - self.lift;
        Rect::from_min_max(
            Pos2::new(stack.center().x - half, bottom - self.height),
            Pos2::new(stack.center().x + half, bottom),
        )
    }

    /// The way from here to `to`, `done` of it gone.
    fn toward(self, to: Place, done: f32) -> Place {
        if done >= 1.0 {
            return to;
        }
        let mix = |from: f32, to: f32| from + (to - from) * done;
        Place {
            lift: mix(self.lift, to.lift),
            height: mix(self.height, to.height),
            scale: mix(self.scale, to.scale),
            opacity: mix(self.opacity, to.opacity),
            content: mix(self.content, to.content),
        }
    }
}

/// The places of the cards of messages `heights` tall, the oldest first.
/// Closed, the newest is in front and up to two older ones peek out above
/// it, each a little narrower and fainter, as tall as the newest and
/// blank. Open, all of them show whole, the newest at the bottom.
fn stack_places(heights: &[f32], open: bool) -> Vec<Place> {
    let Some(&front) = heights.last() else {
        return Vec::new();
    };
    let mut lift = 0.0;
    let mut places: Vec<Place> = heights
        .iter()
        .rev()
        .enumerate()
        .map(|(behind, &height)| {
            if open {
                let place = Place {
                    lift,
                    height,
                    scale: 1.0,
                    opacity: 1.0,
                    content: 1.0,
                };
                lift += height + GAP;
                return place;
            }
            let step = behind.min(BEHIND) as f32;
            Place {
                lift: PEEK * step,
                height: front,
                scale: 1.0 - NARROWER * step,
                opacity: if behind > BEHIND {
                    0.0
                } else {
                    1.0 - FAINTER * step
                },
                content: if behind == 0 { 1.0 } else { 0.0 },
            }
        })
        .collect();
    places.reverse();
    places
}

/// Where a stack `width` wide and `height` tall goes in a window whose
/// content is `content`: at the bottom right above the status bar, or the
/// bottom left in Arabic, as the area that holds it is anchored.
fn stack_rect(content: Rect, width: f32, height: f32, rtl: bool) -> Rect {
    let left = if rtl {
        content.left() + EDGE
    } else {
        content.right() - EDGE - width
    };
    let bottom = content.bottom() - ABOVE;
    Rect::from_min_max(
        Pos2::new(left, bottom - height),
        Pos2::new(left + width, bottom),
    )
}

/// A card on its way to its place, from where it was, since when.
#[derive(Clone, Copy)]
struct Glide {
    from: Place,
    to: Place,
    since: f64,
}

/// Where the cards of the messages `ids` are in this frame: each glides to
/// its place in `places` over `STACK_TIME`, or goes at once when animations
/// are off. Each window keeps its own.
fn glide(ctx: &egui::Context, popup: bool, ids: &[u64], places: &[Place]) -> Vec<Place> {
    let time = if ctx.style_of(ctx.theme()).animation_time == 0.0 {
        0.0
    } else {
        STACK_TIME
    };
    let now = ctx.input(|i| i.time);
    ctx.data_mut(|data| {
        let key = egui::Id::unique(("toast-glides", popup));
        let glides = data.get_temp_mut_or_default::<HashMap<u64, Glide>>(key);
        glide_at(glides, ids, places, now, time)
    })
}

/// `glide` at the time `now`, each card taking `time` seconds, with the
/// glides in `glides`, which forgets the cards no longer shown. A new card
/// starts at its place.
fn glide_at(
    glides: &mut HashMap<u64, Glide>,
    ids: &[u64],
    places: &[Place],
    now: f64,
    time: f32,
) -> Vec<Place> {
    let done = |since: f64| {
        if time > 0.0 {
            ((now - since) as f32 / time).clamp(0.0, 1.0)
        } else {
            1.0
        }
    };
    glides.retain(|id, _| ids.contains(id));
    ids.iter()
        .zip(places)
        .map(|(&id, &to)| {
            let glide = glides.entry(id).or_insert(Glide {
                from: to,
                to,
                since: now,
            });
            if glide.to != to {
                // From wherever it is on its way, to its new place.
                let from = glide.from.toward(glide.to, done(glide.since));
                *glide = Glide {
                    from,
                    to,
                    since: now,
                };
            }
            glide.from.toward(glide.to, done(glide.since))
        })
        .collect()
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
/// buttons under its text. Every message keeps room for its close button.
/// While `showing`, the second line is the copy's whole text. With
/// `app_icon` it is the note on the screen: the app icon, and never Show or
/// a close button.
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
    let cancel = toast.question.as_ref().map(|_| {
        let job = pill_job(&t!("common-cancel"), size::SMALL, v.text_color(), None);
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
    let buttons = [&action, &cancel]
        .into_iter()
        .flatten()
        .map(|g| g.size().x + 2.0 * BUTTON_PAD + SPACE)
        .sum::<f32>()
        + show
            .as_ref()
            .map_or(0.0, |g| ICON + 4.0 + g.size().x + SPACE);
    let icon = if toast.icon.is_some() || app_icon.is_some() {
        ICON + SPACE
    } else {
        0.0
    };
    let close = if app_icon.is_none() { CLOSE_ROOM } else { 0.0 };
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
    // The update's message and a question have their buttons under their
    // text, which takes the whole width.
    let mut below = !update_buttons.is_empty() || cancel.is_some();
    let mut text_w = if below {
        room
    } else {
        (room - buttons).max(40.0)
    };
    let (mut title, mut detail) = text_in(text_w);
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
        cancel,
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

/// Paints a message's icon (or `app_icon`), text, buttons and close button
/// in its card, mirrored in Arabic, and notes a click on a button or a press
/// on Show. The note on the screen (`app_icon`) has no close button.
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
    let closes = app_icon.is_none();
    if closes {
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
        let label = t!("toast-close");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let color = if r.hovered() {
            v.strong_text_color()
        } else {
            v.weak_text_color()
        };
        icons::paint(ui.painter(), at, Icon::Close, color);
        if tip(r, &label).clicked() {
            // The update's message hides until the update's next step.
            if parts.is_some() {
                asked.update = Some(UpdateAction::Close);
            } else {
                asked.close = Some(toast.id);
            }
        }
    }
    // Buttons beside the text keep clear of the close button.
    let mut right = if closes && !laid.below {
        inner.right() - CLOSE_ROOM
    } else {
        inner.right()
    };
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
    // A question's Cancel comes after its main button, at the far end.
    let cancel_at = laid.cancel.as_ref().map(|g| {
        let w = g.size().x + 2.0 * BUTTON_PAD;
        let at = m(Rect::from_min_size(
            Pos2::new(right - w, buttons_mid - BUTTON_H / 2.0),
            Vec2::new(w, BUTTON_H),
        ));
        right -= w + SPACE;
        at
    });
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
        // A question asked from the keyboard hands it to its button.
        if toast.question.as_ref().is_some_and(|q| q.focus) && !ui.is_sizing_pass() {
            r.request_focus();
            asked.focused = Some(toast.id);
        }
        let (fill, _) = theme::message_button(v.dark_mode);
        let fill = if r.hovered() || r.has_focus() {
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
    // Made after the main button, so Tab goes from it to Cancel.
    if let (Some(g), Some(at)) = (&laid.cancel, cancel_at) {
        let r = ui.interact(
            at,
            egui::Id::unique(("toast-cancel", toast.id)),
            Sense::click(),
        );
        let label = t!("common-cancel");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        if r.hovered() || r.has_focus() {
            let lit = v.widgets.hovered.weak_bg_fill;
            ui.painter()
                .rect_filled(at, CornerRadius::same(radius::SMALL), lit);
        }
        let text_at = at.center() - g.size() / 2.0;
        ui.painter()
            .galley(text_at, g.clone(), Color32::PLACEHOLDER);
        if r.clicked() {
            asked.cancel = Some(toast.id);
        }
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
            question: None,
        }
    }

    /// The question before a rule for Notepad++, asked in the full window,
    /// from the keyboard when `by_key`.
    fn question(by_key: bool) -> Toast {
        let mut toast = message("", None, None);
        (toast.icon, toast.secret, toast.id) = (None, None, 7);
        ask_to_ignore(&mut toast, "notepad++.exe", "Notepad++", false, by_key);
        toast
    }

    /// Asking makes no rule: the question waits, with no time limit, and
    /// only its Never record carries the command that makes the rule,
    /// the same rule as before there was a question.
    #[test]
    fn asking_makes_no_rule_until_never_record() {
        let toast = question(false);
        assert!(toast.text.contains("Notepad++"), "{}", toast.text);
        assert_eq!(toast.how_long, Duration::MAX);
        let Some((label, Command::IgnoreApp(app, name))) = &toast.action else {
            panic!("Never record makes the rule");
        };
        assert_eq!(label, &t!("toast-never-record"));
        assert_eq!(
            (app.as_str(), name.as_str()),
            ("notepad++.exe", "Notepad++")
        );
        let cfg = crate::config::Config::default();
        assert!(!cfg.rules.iter().any(|r| r.skips_app("notepad++.exe")));
        let ruled = crate::app::ignoring_app(&cfg, "notepad++.exe");
        assert!(ruled.rules.iter().any(|r| r.skips_app("notepad++.exe")));
        assert_eq!(ruled.rules.len(), cfg.rules.len() + 1);
    }

    /// The texts `toast`'s card draws, each from the left, without the
    /// invisible marks that set a part apart.
    fn drawn_texts(toast: &Toast) -> String {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let laid = lay_out(ui, toast, WIDTH, false, None);
            let card = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(WIDTH, laid.height));
            paint_content(ui, card, toast, &laid, &mut Asked::default(), None);
        });
        output.textures_delta.clear();
        let marks = ['\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}'];
        let texts: Vec<String> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) => Some(shape.galley.text().to_string()),
                _ => None,
            })
            .collect();
        texts
            .join(" ")
            .chars()
            .filter(|c| !marks.contains(c))
            .collect()
    }

    /// The item menu's "Never record from this program…" names no program;
    /// the question after it does, and "Notepad++" keeps "++" right after
    /// "Notepad", also in an Arabic sentence, where without the marks it
    /// went to the name's other end.
    fn check_the_question_keeps_the_name_in_order() {
        let shown = drawn_texts(&question(false));
        assert!(shown.contains("Notepad++"), "{shown}");
        if i18n::rtl() {
            let mut bare = question(false);
            bare.text = t!("toast-never-record-question", app = "Notepad++");
            let bare = drawn_texts(&bare);
            assert!(
                bare.contains("++Notepad"),
                "the test sees the fault: {bare}"
            );
        }
    }

    #[test]
    fn the_question_keeps_the_program_s_name_in_order() {
        check_the_question_keeps_the_name_in_order();
    }

    #[test]
    fn the_question_keeps_the_program_s_name_in_order_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_question_keeps_the_program_s_name_in_order_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_question_keeps_the_program_s_name_in_order_right_to_left"]
    fn the_question_keeps_the_program_s_name_in_order_in_arabic() {
        if crate::i18n::arabic_run() {
            check_the_question_keeps_the_name_in_order();
        }
    }

    /// Cancel and Esc close only the question, in its own window, and make
    /// no rule: nothing runs.
    #[test]
    fn cancel_or_esc_closes_only_the_question() {
        let mut older = message("Copied", None, None);
        older.id = 3;
        let mut toasts = vec![older, question(false)];
        // The popup's Esc leaves the full window's question alone.
        assert!(!close_question(&mut toasts, true));
        assert_eq!(toasts.len(), 2);
        assert!(close_question(&mut toasts, false));
        let left: Vec<u64> = toasts.iter().map(|t| t.id).collect();
        assert_eq!(left, [3], "the other message stays");
        assert!(!close_question(&mut toasts, false), "closed once");
    }

    /// One frame of the question's card, with `events`: what its buttons
    /// asked for, and the screen reader's buttons with where they are.
    fn question_frame(
        ctx: &egui::Context,
        toast: &Toast,
        events: Vec<egui::Event>,
    ) -> (Asked, Vec<(String, Rect)>) {
        let frame = card_frame(ctx, toast, events);
        assert!(frame.below, "the buttons sit under the question");
        (frame.asked, frame.buttons)
    }

    /// What one frame of a message's card showed.
    struct Frame {
        card: Rect,
        /// What its buttons asked for.
        asked: Asked,
        /// The screen reader's buttons, with where they are.
        buttons: Vec<(String, Rect)>,
        /// Where its texts were drawn, also those in its buttons.
        texts: Vec<Rect>,
        /// Its buttons sit under its text.
        below: bool,
    }

    /// One frame of `toast`'s card, with `events`.
    fn card_frame(ctx: &egui::Context, toast: &Toast, events: Vec<egui::Event>) -> Frame {
        let input = egui::RawInput {
            events,
            focused: true,
            ..Default::default()
        };
        let mut asked = Asked::default();
        let (mut card, mut below) = (Rect::NOTHING, false);
        let mut output = ctx.run_ui(input, |ui| {
            let laid = lay_out(ui, toast, WIDTH, false, None);
            below = laid.below;
            card = Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(WIDTH, laid.height));
            paint_content(ui, card, toast, &laid, &mut asked, None);
        });
        output.textures_delta.clear();
        let texts = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) => Some(shape.galley.rect.translate(shape.pos.to_vec2())),
                _ => None,
            })
            .collect();
        // Only while a screen reader listens, or a test asks for it.
        let tree = output.platform_output.accesskit_update;
        let buttons = tree
            .iter()
            .flat_map(|tree| &tree.nodes)
            .filter(|(_, node)| node.role() == egui::accesskit::Role::Button)
            .map(|(_, node)| {
                let b = node.bounds().expect("a place");
                let rect = Rect::from_min_max(
                    Pos2::new(b.x0 as f32, b.y0 as f32),
                    Pos2::new(b.x1 as f32, b.y1 as f32),
                );
                (node.label().unwrap_or("").to_string(), rect)
            })
            .collect();
        Frame {
            card,
            asked,
            buttons,
            texts,
            below,
        }
    }

    /// A click on the button named `name`, over three frames.
    fn click(ctx: &egui::Context, toast: &Toast, name: &str) -> Asked {
        let buttons = card_frame(ctx, toast, Vec::new()).buttons;
        let at = buttons
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {buttons:?}"))
            .1
            .center();
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        card_frame(ctx, toast, vec![egui::Event::PointerMoved(at)]);
        card_frame(ctx, toast, vec![button(true)]);
        card_frame(ctx, toast, vec![button(false)]).asked
    }

    /// Every message in the window has a close button at its top far
    /// corner, the top left in Arabic, named for screen readers. Its texts
    /// and other buttons keep clear of it, and a click on it closes only
    /// that message, or hides the update's.
    fn check_every_message_has_a_close_button() {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        ctx.enable_accesskit();
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let mut deleted = message(&t!("toast-deleted"), None, Some(&t!("toast-undo")));
        (deleted.secret, deleted.keys) = (None, Some(UNDO_KEYS.to_string()));
        let app = "Visual Studio Code";
        let mut long = message(&t!("toast-app-ignored", app = app), None, None);
        long.secret = None;
        let skipped = message(
            &t!("toast-skipped-password"),
            Some("From Clipboard Manager · 28 characters"),
            Some(&t!("toast-save-anyway")),
        );
        let release = Release::made_up("0.7.2", 29, 30 << 20);
        let update = update_toast(&UpdateState::Available(release));
        let close = t!("toast-close");
        for toast in [deleted, long, skipped, question(false), update] {
            let frame = card_frame(&ctx, &toast, Vec::new());
            let (card, buttons) = (frame.card, &frame.buttons);
            let at = buttons
                .iter()
                .find(|(name, _)| *name == close)
                .unwrap_or_else(|| panic!("{close} in {buttons:?}"))
                .1;
            let from_end = if i18n::rtl() {
                at.left() - card.left()
            } else {
                card.right() - at.right()
            };
            assert!(
                (from_end - (CLOSE_AT - 4.0)).abs() < 0.5,
                "{at:?} in {card:?}"
            );
            assert!((at.top() - card.top() - (CLOSE_AT - 4.0)).abs() < 0.5);
            for (name, other) in buttons.iter().filter(|(name, _)| *name != close) {
                assert!(
                    !other.intersects(at),
                    "{name} at {other:?}, {close} at {at:?}"
                );
            }
            for text in &frame.texts {
                assert!(
                    !text.intersects(at),
                    "a text at {text:?}, {close} at {at:?}"
                );
            }
            let asked = click(&ctx, &toast, &close);
            if toast.update.is_some() {
                assert_eq!(asked.update, Some(UpdateAction::Close));
            } else {
                assert_eq!((asked.close, asked.action), (Some(toast.id), None));
            }
        }
    }

    #[test]
    fn every_message_has_a_close_button() {
        check_every_message_has_a_close_button();
    }

    #[test]
    fn every_message_has_a_close_button_right_to_left() {
        crate::i18n::passes_in_arabic(module_path!(), "every_message_has_a_close_button_in_arabic");
    }

    #[test]
    #[ignore = "run in a process of its own by every_message_has_a_close_button_right_to_left"]
    fn every_message_has_a_close_button_in_arabic() {
        if crate::i18n::arabic_run() {
            check_every_message_has_a_close_button();
        }
    }

    /// Never record, filled, then Cancel at the far end; each answers.
    #[test]
    fn the_question_s_buttons_answer_it() {
        let toast = question(false);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (_, buttons) = question_frame(&ctx, &toast, Vec::new());
        let never = t!("toast-never-record");
        let cancel = t!("common-cancel");
        let place = |name: &str| buttons.iter().find(|(n, _)| n == name).map(|(_, r)| *r);
        let (never_at, cancel_at) = (
            place(&never).expect("Never record"),
            place(&cancel).expect("Cancel"),
        );
        assert!(never_at.right() < cancel_at.left(), "{buttons:?}");
        assert!((cancel_at.right() - (20.0 + WIDTH - PAD)).abs() <= 0.5);
        let asked = click(&ctx, &toast, &never);
        assert_eq!((asked.action, asked.cancel), (Some(7), None));
        let asked = click(&ctx, &toast, &cancel);
        assert_eq!((asked.action, asked.cancel), (None, Some(7)));
    }

    /// Asked from the keyboard, Never record takes the keyboard: Enter
    /// answers with it, and Tab goes on to Cancel. Asked with the mouse,
    /// the keyboard stays where it was.
    #[test]
    fn a_question_asked_from_the_keyboard_takes_it() {
        let key = |key: egui::Key| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let never = egui::Id::unique(("toast-action", 7_u64));
        let cancel = egui::Id::unique(("toast-cancel", 7_u64));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (asked, _) = question_frame(&ctx, &question(true), Vec::new());
        assert_eq!(asked.focused, Some(7));
        // The app marks the question's keyboard as handed over.
        let toast = question(false);
        question_frame(&ctx, &toast, Vec::new());
        assert!(ctx.memory(|m| m.has_focus(never)));
        let (asked, _) = question_frame(&ctx, &toast, vec![key(egui::Key::Enter)]);
        assert_eq!(asked.action, Some(7), "Enter answers");
        question_frame(&ctx, &toast, vec![key(egui::Key::Tab)]);
        question_frame(&ctx, &toast, Vec::new());
        assert!(ctx.memory(|m| m.has_focus(cancel)), "Tab reaches Cancel");
        // With the mouse, nothing takes the keyboard.
        let ctx = egui::Context::default();
        let (asked, _) = question_frame(&ctx, &question(false), Vec::new());
        assert_eq!(asked.focused, None);
        assert_eq!(ctx.memory(|m| m.focused()), None);
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

    /// A place's lift, height, width part, opacity and content, rounded to
    /// hundredths.
    fn parts(place: &Place) -> [f32; 5] {
        [
            place.lift,
            place.height,
            place.scale,
            place.opacity,
            place.content,
        ]
        .map(|value| (value * 100.0).round() / 100.0)
    }

    /// Closed, the newest is in front and the two before it peek out above
    /// it, each narrower, fainter and blank, as tall as the newest; an
    /// older fourth would hide behind them.
    #[test]
    fn the_closed_stack_shows_the_newest_with_two_older_ones_peeking_out() {
        let places = stack_places(&[40.0, 70.0, 52.0, 60.0], false);
        let parts: Vec<[f32; 5]> = places.iter().map(parts).collect();
        assert_eq!(
            parts,
            [
                [24.0, 60.0, 0.9, 0.0, 0.0],
                [24.0, 60.0, 0.9, 0.6, 0.0],
                [12.0, 60.0, 0.95, 0.8, 0.0],
                [0.0, 60.0, 1.0, 1.0, 1.0],
            ]
        );
        assert!(stack_places(&[], false).is_empty());
    }

    /// Open, every message shows whole, the newest at the bottom and each
    /// older one above it.
    #[test]
    fn the_open_stack_shows_every_message_whole() {
        let places = stack_places(&[70.0, 52.0, 60.0], true);
        let parts: Vec<[f32; 5]> = places.iter().map(parts).collect();
        let above_newest = 60.0 + GAP;
        assert_eq!(
            parts,
            [
                [above_newest + 52.0 + GAP, 70.0, 1.0, 1.0, 1.0],
                [above_newest, 52.0, 1.0, 1.0, 1.0],
                [0.0, 60.0, 1.0, 1.0, 1.0],
            ]
        );
    }

    /// The stack sits at the window's bottom right above the status bar, or
    /// its bottom left in Arabic; the older cards are centered over the
    /// newest, their tops apart by the part that peeks out.
    #[test]
    fn the_cards_sit_in_the_window_s_corner_in_both_directions() {
        let content = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let ltr = stack_rect(content, WIDTH, 76.0, false);
        let rtl = stack_rect(content, WIDTH, 76.0, true);
        assert_eq!((ltr.right(), ltr.bottom()), (1000.0 - EDGE, 700.0 - ABOVE));
        assert_eq!((rtl.left(), rtl.bottom()), (EDGE, 700.0 - ABOVE));
        assert_eq!(
            (ltr.size(), rtl.size()),
            (Vec2::new(WIDTH, 76.0), ltr.size())
        );
        let cards: Vec<Rect> = stack_places(&[52.0, 52.0, 52.0], false)
            .iter()
            .map(|place| place.card(ltr))
            .collect();
        assert_eq!(cards[2].width(), WIDTH);
        assert_eq!(cards[2].bottom(), ltr.bottom());
        for (older, newer) in cards.iter().zip(&cards[1..]) {
            assert!((older.center().x - newer.center().x).abs() < 0.01);
            assert!(older.width() < newer.width());
            assert!((newer.top() - older.top() - PEEK).abs() < 0.01);
        }
        assert!(
            (cards[0].top() - ltr.top()).abs() < 0.01,
            "the stack holds them"
        );
    }

    /// As a message comes, the one in front glides back over `STACK_TIME`
    /// while the new one takes its place; a card that is gone is
    /// forgotten; with animations off, the cards go to their places at once.
    #[test]
    fn the_cards_glide_to_their_new_places() {
        let mut glides = HashMap::new();
        let one = stack_places(&[52.0], false);
        assert_eq!(glide_at(&mut glides, &[1], &one, 10.0, STACK_TIME), one);
        let two = stack_places(&[52.0, 52.0], false);
        let start = glide_at(&mut glides, &[1, 2], &two, 11.0, STACK_TIME);
        assert_eq!(start, [one[0], two[1]], "each starts where it was");
        let quarter = 11.0 + f64::from(STACK_TIME) / 4.0;
        let on_the_way = glide_at(&mut glides, &[1, 2], &two, quarter, STACK_TIME);
        assert_eq!(parts(&on_the_way[0]), [3.0, 52.0, 0.99, 0.95, 0.75]);
        assert_eq!(glide_at(&mut glides, &[1, 2], &two, 12.0, STACK_TIME), two);
        glide_at(&mut glides, &[2], &one, 12.5, STACK_TIME);
        assert_eq!(glides.len(), 1, "the first is forgotten");
        // The second message grew taller as it opened.
        let open = stack_places(&[52.0, 70.0], true);
        assert_eq!(glide_at(&mut glides, &[1, 2], &open, 12.5, 0.0), open);
    }
}
