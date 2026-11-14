//! A picture full screen: the main window covers its monitor, black, with the
//! picture fitted to it. The mouse wheel zooms around the pointer, dragging
//! moves a zoomed picture, and the arrow keys and buttons go to the picture
//! before or after it in the list or grid. Esc, F11 or Close puts the window
//! back as it was.

use super::*;

/// One notch of the mouse wheel zooms by this much.
const ZOOM_PER_NOTCH: f32 = 1.25;

/// Zoomed in, one pixel of a picture covers at most this many of the
/// screen's.
const MOST_PIXEL_SCALE: f32 = 8.0;

/// How big the arrow buttons and Close are.
const ARROW_BUTTON: f32 = 48.0;
const CLOSE_BUTTON: f32 = 40.0;

/// Between those buttons and the screen's edges.
const EDGE_ROOM: f32 = space::LARGE;

/// A picture shown full screen.
pub(crate) struct Viewer {
    id: u64,
    /// The picture at its own size once decoded: `None` inside when it
    /// can't be read, or is too big for the graphics card.
    full: Option<Option<TextureHandle>>,
    view: View,
}

impl Viewer {
    fn new(id: u64) -> Self {
        Self {
            id,
            full: None,
            view: View::default(),
        }
    }

    /// The picture of item `id` at its own size came, or `None` when it
    /// can't be shown. It is kept while that item shows.
    pub(crate) fn picture_came(&mut self, id: u64, texture: Option<TextureHandle>) {
        if id == self.id {
            self.full = Some(texture);
        }
    }
}

/// How the picture is zoomed and moved: `zoom` times its fitted size, its
/// middle `offset` points from the screen's middle.
#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    zoom: f32,
    offset: Vec2,
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
}

impl View {
    /// Where the picture shows on `screen`, `fitted` big before zooming.
    fn place(self, screen: Rect, fitted: Vec2) -> Rect {
        Rect::from_center_size(screen.center() + self.offset, fitted * self.zoom)
    }

    /// Zoomed by `factor` around `pointer`, so the part of the picture under
    /// it stays there; never smaller than fitted, nor more than `most`.
    fn zoomed(self, factor: f32, pointer: Pos2, screen: Rect, fitted: Vec2, most: f32) -> Self {
        let zoom = (self.zoom * factor).clamp(1.0, most.max(1.0));
        let middle = screen.center() + self.offset;
        let middle = pointer + (middle - pointer) * (zoom / self.zoom);
        let offset = middle - screen.center();
        Self { zoom, offset }.kept_on(screen.size(), fitted)
    }

    /// Moved by `delta`, as far as the picture's edges allow.
    fn moved(self, delta: Vec2, screen: Vec2, fitted: Vec2) -> Self {
        let offset = self.offset + delta;
        Self { offset, ..self }.kept_on(screen, fitted)
    }

    /// Kept on a screen `screen` big: along a side where the picture is
    /// bigger than the screen, its edges stay beyond the screen's, so no
    /// black shows there; along a side where it fits, it stays in the
    /// middle.
    fn kept_on(self, screen: Vec2, fitted: Vec2) -> Self {
        let slack = ((fitted * self.zoom - screen) / 2.0).max(Vec2::ZERO);
        let offset = self.offset.clamp(-slack, slack);
        Self { offset, ..self }
    }
}

/// A picture's own size in pixels: as wide as its item says, in the shape
/// of `texture`, which can be a smaller copy standing in for it.
fn own_pixels(recorded_width: u32, texture: Vec2) -> Vec2 {
    if texture.x <= 0.0 {
        return Vec2::ZERO;
    }
    let width = if recorded_width > 0 {
        recorded_width as f32
    } else {
        texture.x
    };
    Vec2::new(width, width * texture.y / texture.x)
}

/// How big a picture `pixels` big shows on a screen `screen` big, in
/// points at `pixels_per_point`: whole and as big as fits, but no bigger
/// than its own pixels, so a small picture stays sharp.
fn fitted_size(pixels: Vec2, screen: Vec2, pixels_per_point: f32) -> Vec2 {
    let own = pixels / pixels_per_point;
    if own.x <= 0.0 || own.y <= 0.0 {
        return Vec2::ZERO;
    }
    own * (screen.x / own.x).min(screen.y / own.y).clamp(0.0, 1.0)
}

/// How far a picture `pixels` big, `fitted` big on the screen, zooms in:
/// until one of its pixels covers `MOST_PIXEL_SCALE` of the screen's.
fn most_zoom(pixels: Vec2, fitted: Vec2, pixels_per_point: f32) -> f32 {
    if fitted.x <= 0.0 {
        return 1.0;
    }
    (MOST_PIXEL_SCALE * pixels.x / pixels_per_point / fitted.x).max(1.0)
}

/// The picture next to `current` among the items `shown`, in their order,
/// passing over those that aren't pictures: after it when `forward`, else
/// before it. None at either end, or when `current` isn't shown.
fn neighbour_picture(
    shown: &[Hit],
    current: u64,
    forward: bool,
    is_picture: impl Fn(u64) -> bool,
) -> Option<u64> {
    let at = shown.iter().position(|hit| hit.id == current)?;
    let (before, after) = (&shown[..at], &shown[at + 1..]);
    if forward {
        after.iter().map(|hit| hit.id).find(|&id| is_picture(id))
    } else {
        before
            .iter()
            .rev()
            .map(|hit| hit.id)
            .find(|&id| is_picture(id))
    }
}

/// Whether an arrow key, the right one or else the left, goes to the next
/// picture: the right one, and in Arabic, read from the right, the left one.
fn goes_forward(right: bool, rtl: bool) -> bool {
    right != rtl
}

/// Where the viewer's buttons go on `screen`: the arrow to the previous
/// picture at the side reading starts from, the arrow to the next one at
/// the other, both halfway down, and Close in the top corner at the
/// reading end.
fn button_places(screen: Rect, rtl: bool) -> [Rect; 3] {
    let arrow = Vec2::splat(ARROW_BUTTON);
    let y = screen.center().y;
    let start = screen.left() + EDGE_ROOM + ARROW_BUTTON / 2.0;
    let end = screen.right() - EDGE_ROOM - ARROW_BUTTON / 2.0;
    let close_corner = Pos2::new(
        screen.right() - EDGE_ROOM - CLOSE_BUTTON,
        screen.top() + EDGE_ROOM,
    );
    // Laid out left to right, then mirrored for Arabic.
    [
        Rect::from_center_size(Pos2::new(start, y), arrow),
        Rect::from_center_size(Pos2::new(end, y), arrow),
        Rect::from_min_size(close_corner, Vec2::splat(CLOSE_BUTTON)),
    ]
    .map(|place| flip_if(rtl, screen, place))
}

/// A round button over a picture: a white icon on a dark disc, so it shows
/// on any picture and on the black around it. `mirrored` turns the icon the
/// other way. It isn't in the Tab order: keys do the same.
pub(super) fn button_on_picture(
    ui: &Ui,
    place: Rect,
    id: egui::Id,
    icon: Icon,
    mirrored: bool,
    name: &str,
) -> egui::Response {
    let response = ui.interact(place, id, Sense::CLICK);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    let p = ui.painter();
    let radius = place.width().min(place.height()) / 2.0;
    p.circle_filled(place.center(), radius, PICTURE_BUTTON);
    if response.hovered() {
        let lit = picture_button_lit(response.is_pointer_button_down_on());
        p.circle_filled(place.center(), radius, lit);
    }
    let glyph = Rect::from_center_size(place.center(), Vec2::splat(radius));
    if mirrored {
        icons::paint_mirrored(p, glyph, icon, Color32::WHITE);
    } else {
        icons::paint(p, glyph, icon, Color32::WHITE);
    }
    tip(response, name)
}

/// What the viewer shows for its picture.
enum Shown {
    Picture(TextureHandle),
    Loading,
    Missing,
}

/// Draws `texture` on `screen`, `recorded_width` pixels wide as its item
/// says, as `view` places it once the mouse wheel, a pinch or a drag moved
/// it; the view after them.
fn picture(
    ui: &mut Ui,
    screen: Rect,
    texture: &TextureHandle,
    recorded_width: u32,
    view: View,
) -> View {
    let pixels = own_pixels(recorded_width, texture.size_vec2());
    let pixels_per_point = ui.ctx().pixels_per_point();
    let fitted = fitted_size(pixels, screen.size(), pixels_per_point);
    let most = most_zoom(pixels, fitted, pixels_per_point);
    // Under the buttons, which come after it.
    let response = ui.interact(screen, ui.scope_id().with("viewer-picture"), Sense::DRAG);
    let line_points = ui.ctx().options(|o| o.input_options.line_scroll_speed);
    // The wheel zooms here; nothing else scrolls.
    let (scrolled, pinched, pointer) = ui.input_mut(|i| {
        let scrolled = std::mem::take(&mut i.smooth_scroll_delta).y;
        (scrolled, i.zoom_delta(), i.pointer.hover_pos())
    });
    // The screen grows as the window goes full screen.
    let mut view = view.kept_on(screen.size(), fitted);
    let factor = ZOOM_PER_NOTCH.powf(scrolled / line_points) * pinched;
    if let Some(at) = pointer.filter(|_| factor != 1.0) {
        view = view.zoomed(factor, at, screen, fitted, most);
    }
    if response.dragged() {
        view = view.moved(response.drag_delta(), screen.size(), fitted);
    }
    let place = view.place(screen, fitted);
    let movable = place.width() > screen.width() + 0.5 || place.height() > screen.height() + 0.5;
    if movable {
        let cursor = if response.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        };
        response.on_hover_cursor(cursor);
    }
    let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    ui.painter()
        .image(texture.id(), place, whole, Color32::WHITE);
    view
}

impl App {
    /// Shows picture `id` full screen, over its whole monitor.
    pub(super) fn view_full_screen(&mut self, id: u64) {
        if self.viewer.is_some() || !self.is_picture(id) {
            return;
        }
        self.viewer = Some(Viewer::new(id));
        self.select_viewed(id);
        self.win
            .main_window(egui::ViewportCommand::Fullscreen(true));
    }

    /// Ends full screen. Windows puts the window back as it was: maximized
    /// or not, in its place and size.
    pub(crate) fn close_viewer(&mut self) {
        if self.viewer.take().is_some() {
            self.win
                .main_window(egui::ViewportCommand::Fullscreen(false));
            // Typing searches again.
            self.focus_search.store(true, Ordering::SeqCst);
        }
    }

    fn is_picture(&self, id: u64) -> bool {
        self.library
            .get(id)
            .is_some_and(|it| it.kind == Kind::Image)
    }

    /// Selects picture `id` in the list or grid, so it is selected when the
    /// window comes back. The user chose it: a better match found later
    /// doesn't take its place.
    fn select_viewed(&mut self, id: u64) {
        self.run(Command::Select(id));
        self.auto_selected = None;
        self.scroll_to_selected = true;
    }

    /// Goes to the picture after the one shown in the list or grid, or
    /// before it.
    fn step_picture(&mut self, forward: bool) {
        let Some(current) = self.viewer.as_ref().map(|viewer| viewer.id) else {
            return;
        };
        let next = neighbour_picture(&self.results, current, forward, |id| self.is_picture(id));
        if let Some(id) = next {
            self.viewer = Some(Viewer::new(id));
            self.select_viewed(id);
        }
    }

    /// The viewer's keys: the arrows go to another picture, Esc and F11
    /// close it. No key reaches the list behind it.
    pub(super) fn viewer_keys(&mut self, ctx: &egui::Context) {
        let (close, left, right) = ctx.input_mut(|i| {
            let close = i.consume_key(egui::Modifiers::NONE, Key::Escape)
                || i.consume_key(egui::Modifiers::NONE, Key::F11);
            let (left, right) = (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
            );
            i.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { .. }
                        | egui::Event::Text(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                )
            });
            (close, left, right)
        });
        if close {
            self.close_viewer();
        } else if left != right {
            self.step_picture(goes_forward(right, i18n::rtl()));
        }
    }

    /// What the viewer shows of item `it`: its picture at its own size once
    /// decoded, and meanwhile a smaller copy already loaded. A picture that
    /// can't be shown at its own size shows its preview.
    fn viewer_picture(&mut self, it: &Item) -> Shown {
        let full = self.viewer.as_ref().and_then(|viewer| viewer.full.clone());
        let stand_in = self.stand_in(it.id);
        match full {
            Some(Some(texture)) => Shown::Picture(texture),
            Some(None) => match self.preview(it) {
                Preview::Ready(texture) => Shown::Picture(texture),
                Preview::Loading => stand_in.map_or(Shown::Loading, Shown::Picture),
                Preview::Missing => Shown::Missing,
            },
            None => {
                let Some(path) = &it.image else {
                    return Shown::Missing;
                };
                self.loader.request(it.id, path, Size::Full);
                stand_in.map_or(Shown::Loading, Shown::Picture)
            }
        }
    }

    /// A smaller copy of picture `id` already loaded: the details' preview,
    /// or else the thumbnail.
    fn stand_in(&self, id: u64) -> Option<TextureHandle> {
        let preview = match &self.preview {
            Some((shown, Some(texture))) if *shown == id => Some(texture.clone()),
            _ => None,
        };
        preview.or_else(|| self.thumbs.get(&id).cloned().flatten())
    }

    /// The picture full screen on black, with the arrows and Close over it.
    pub(super) fn viewer_ui(&mut self, ui: &mut Ui) {
        let Some(id) = self.viewer.as_ref().map(|viewer| viewer.id) else {
            return;
        };
        // Gone meanwhile: deleted by age or by the history's limit.
        let Some(it) = self.library.get(id).cloned() else {
            self.close_viewer();
            return;
        };
        let shown = self.viewer_picture(&it);
        let is_picture = |other| self.is_picture(other);
        let has_previous = neighbour_picture(&self.results, id, false, is_picture).is_some();
        let has_next = neighbour_picture(&self.results, id, true, is_picture).is_some();
        let mut view = self
            .viewer
            .as_ref()
            .map_or_else(View::default, |viewer| viewer.view);
        let (mut close, mut step) = (false, None);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Color32::BLACK))
            .show(ui, |ui| {
                let screen = ui.max_rect();
                match &shown {
                    Shown::Picture(texture) => {
                        view = picture(ui, screen, texture, it.width, view);
                    }
                    Shown::Loading => {
                        let middle = Rect::from_center_size(screen.center(), Vec2::splat(32.0));
                        paint_spinner(ui, middle, Color32::WHITE);
                    }
                    Shown::Missing => {
                        paint_text(
                            ui,
                            ui.painter(),
                            screen.center(),
                            Align2::CENTER_CENTER,
                            &t!("viewer-image-missing"),
                            FontId::proportional(size::LARGE),
                            ON_BLACK_TEXT,
                        );
                    }
                }
                let [previous_place, next_place, close_place] = button_places(screen, i18n::rtl());
                // The keys follow the arrows' sides: in Arabic the previous
                // picture is to the right.
                let (previous_key, next_key) = if i18n::rtl() {
                    ("Right", "Left")
                } else {
                    ("Left", "Right")
                };
                let arrows = [
                    (
                        has_previous,
                        previous_place,
                        false,
                        hint(&t!("viewer-previous"), previous_key),
                    ),
                    (
                        has_next,
                        next_place,
                        true,
                        hint(&t!("viewer-next"), next_key),
                    ),
                ];
                for (shows, place, forward, name) in arrows {
                    if !shows {
                        continue;
                    }
                    let id = egui::Id::unique(("viewer-arrow", forward));
                    // The arrow to the next picture is the Back arrow turned
                    // around.
                    if button_on_picture(ui, place, id, Icon::Back, forward, &name).clicked() {
                        step = Some(forward);
                    }
                }
                let name = hint(&t!("viewer-close"), "Escape");
                let id = egui::Id::unique("viewer-close");
                close = button_on_picture(ui, close_place, id, Icon::Close, false, &name).clicked();
            });
        if let Some(viewer) = &mut self.viewer {
            viewer.view = view;
        }
        if close {
            self.close_viewer();
        } else if let Some(forward) = step {
            self.step_picture(forward);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1920.0, 1080.0));

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 0.01
    }

    /// A big picture shrinks to fit the screen whole; a small one keeps
    /// its own pixels, also on a screen at 150%.
    #[test]
    fn a_picture_fits_the_screen_and_never_grows_past_its_pixels() {
        let screen = SCREEN.size();
        let wide = fitted_size(Vec2::new(3840.0, 1080.0), screen, 1.0);
        assert!(near(wide, Vec2::new(1920.0, 540.0)), "{wide:?}");
        let tall = fitted_size(Vec2::new(1000.0, 4000.0), screen, 1.0);
        assert!(near(tall, Vec2::new(270.0, 1080.0)), "{tall:?}");
        let small = fitted_size(Vec2::new(300.0, 200.0), screen, 1.0);
        assert!(near(small, Vec2::new(300.0, 200.0)), "{small:?}");
        // 300 pixels are 200 points at 150%.
        let scaled = fitted_size(Vec2::new(300.0, 150.0), screen, 1.5);
        assert!(near(scaled, Vec2::new(200.0, 100.0)), "{scaled:?}");
        assert_eq!(fitted_size(Vec2::ZERO, screen, 1.0), Vec2::ZERO);
    }

    /// Zooming in goes on until a picture's pixel covers 8 of the
    /// screen's; a small picture still zooms in that far.
    #[test]
    fn zooming_in_stops_at_eight_screen_pixels_for_one() {
        let pixels = Vec2::new(3840.0, 2160.0);
        let fitted = fitted_size(pixels, SCREEN.size(), 1.0);
        assert!((most_zoom(pixels, fitted, 1.0) - 16.0).abs() < 0.01);
        let small = Vec2::new(100.0, 100.0);
        assert!((most_zoom(small, small, 1.0) - 8.0).abs() < 0.01);
        assert_eq!(most_zoom(Vec2::ZERO, Vec2::ZERO, 1.0), 1.0);
    }

    /// A smaller copy standing in keeps the picture's own size and shape.
    #[test]
    fn a_smaller_copy_shows_at_the_size_of_the_picture() {
        let own = own_pixels(3840, Vec2::new(280.0, 157.5));
        assert!(near(own, Vec2::new(3840.0, 2160.0)), "{own:?}");
        // Without a recorded width, the texture's own.
        assert_eq!(
            own_pixels(0, Vec2::new(640.0, 480.0)),
            Vec2::new(640.0, 480.0)
        );
    }

    /// Zooming keeps the spot under the pointer in place, never goes below
    /// the fitted size nor past the most, and zooming out again brings the
    /// picture back to the middle.
    #[test]
    fn zooming_keeps_the_spot_under_the_pointer() {
        let fitted = Vec2::new(1920.0, 1080.0);
        let pointer = Pos2::new(1500.0, 300.0);
        let before = View::default();
        // Where the pointer is on the picture, from 0 to 1 across it.
        let spot = |view: View| {
            let place = view.place(SCREEN, fitted);
            (pointer - place.min) / place.size()
        };
        let zoomed = before.zoomed(2.0, pointer, SCREEN, fitted, 8.0);
        assert_eq!(zoomed.zoom, 2.0);
        assert!(near(spot(zoomed), spot(before)), "{zoomed:?}");
        let most = zoomed.zoomed(100.0, pointer, SCREEN, fitted, 8.0);
        assert_eq!(most.zoom, 8.0);
        assert!(near(spot(most), spot(before)), "{most:?}");
        let back = most.zoomed(0.01, pointer, SCREEN, fitted, 8.0);
        assert_eq!(back, View::default());
    }

    /// A zoomed picture moves only as far as its edges reach the screen's;
    /// one that fits stays in the middle.
    #[test]
    fn dragging_stops_at_the_pictures_edges() {
        let fitted = Vec2::new(1000.0, 1080.0);
        let zoomed = View {
            zoom: 2.0,
            offset: Vec2::ZERO,
        };
        // 2000 by 2160 on 1920 by 1080: 40 to each side, 540 up and down.
        let moved = zoomed.moved(Vec2::new(500.0, -900.0), SCREEN.size(), fitted);
        assert!(near(moved.offset, Vec2::new(40.0, -540.0)), "{moved:?}");
        let place = moved.place(SCREEN, fitted);
        assert!((place.left() - SCREEN.left()).abs() < 0.01, "{place:?}");
        assert!((place.bottom() - SCREEN.bottom()).abs() < 0.01, "{place:?}");
        let fits = View::default().moved(Vec2::new(300.0, 300.0), SCREEN.size(), fitted);
        assert_eq!(fits, View::default());
    }

    fn hits(ids: &[u64]) -> Vec<Hit> {
        ids.iter()
            .map(|&id| Hit {
                id,
                closeness: None,
                weak: false,
            })
            .collect()
    }

    /// The arrows go through the pictures in the order shown, passing over
    /// texts, links and files, and stop at the ends.
    #[test]
    fn the_arrows_pass_over_items_that_are_not_pictures() {
        // Pictures are the odd numbers; 4 and 6 are texts, 8 a link.
        let shown = hits(&[1, 4, 6, 3, 8, 5, 9]);
        let is_picture = |id: u64| id % 2 == 1;
        let step = |from, forward| neighbour_picture(&shown, from, forward, is_picture);
        assert_eq!(step(1, true), Some(3));
        assert_eq!(step(3, true), Some(5));
        assert_eq!(step(5, false), Some(3));
        assert_eq!(step(3, false), Some(1));
        assert_eq!(step(1, false), None, "the first picture");
        assert_eq!(step(9, true), None, "the last picture");
        assert_eq!(step(7, true), None, "not shown");
        let no_more = hits(&[2, 1, 4, 6]);
        assert_eq!(
            neighbour_picture(&no_more, 1, true, is_picture),
            None,
            "only texts after it"
        );
    }

    /// In English the right arrow key goes to the next picture, whose
    /// button is on the right. In Arabic the pictures go from the right:
    /// the next one's button is on the left, and so is its key.
    #[test]
    fn the_arrow_keys_and_buttons_follow_the_reading_direction() {
        assert!(goes_forward(true, false));
        assert!(!goes_forward(false, false));
        assert!(goes_forward(false, true));
        assert!(!goes_forward(true, true));
        let [previous, next, close] = button_places(SCREEN, false);
        assert!(previous.center().x < SCREEN.center().x);
        assert!(next.center().x > SCREEN.center().x);
        assert!(close.center().x > SCREEN.center().x && close.top() < previous.top());
        let [previous, next, close] = button_places(SCREEN, true);
        assert!(previous.center().x > SCREEN.center().x);
        assert!(next.center().x < SCREEN.center().x);
        assert!(close.center().x < SCREEN.center().x && close.top() < previous.top());
        for place in [previous, next, close] {
            assert!(SCREEN.contains_rect(place), "{place:?}");
        }
    }
}
