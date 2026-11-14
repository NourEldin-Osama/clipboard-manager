//! The menu of an item: a right-click on a row or a card, or Shift+F10 for
//! the selected one. Every action with its key, what suits the item, and
//! Delete last, in red, apart from the rest.

use super::messages::{ask_to_ignore, close_question};
use super::*;

/// What an entry of an item's menu does.
#[derive(Clone)]
enum Action {
    Copy(Paste),
    Pin,
    Tag,
    Details,
    OpenLink,
    ShowInFolder(PathBuf),
    ViewFullScreen,
    SaveImage,
    CopyImageText,
    IgnoreApp(String),
    Delete,
    /// The same for every item selected together.
    CopyGroup(Paste),
    PinGroup,
    TagGroup,
    DeleteGroup,
}

/// One entry: its text, its keys and what it does.
struct Entry {
    text: String,
    keys: String,
    action: Action,
    /// Why it can't be used now. It is then grayed out, a click on it does
    /// nothing, and pointing at it says why.
    unavailable: Option<String>,
}

/// A Paste entry while there is no program to paste into: grayed out, and
/// pointing at it says how to get one.
fn unavailable_paste(text: String, keys: &str, action: Action) -> Entry {
    Entry {
        text,
        keys: keys.to_string(),
        action,
        unavailable: Some(t!("item-paste-unavailable")),
    }
}

/// The ways to put an item back, as the first entries of its menu: Paste
/// first, with its key. Without a program to paste into, the ways start
/// with Copy, and Paste comes before them grayed out.
fn copy_entries(ways: Vec<CopyWay>, paste_keys: &str) -> Vec<Entry> {
    let can_paste = ways.first().is_some_and(|way| way.paste == Paste::Yes);
    let paste = (!can_paste)
        .then(|| unavailable_paste(t!("item-paste"), paste_keys, Action::Copy(Paste::Yes)));
    let ways = ways.into_iter().map(|way| Entry {
        text: way.text,
        keys: way.keys,
        action: Action::Copy(way.paste),
        unavailable: None,
    });
    paste.into_iter().chain(ways).collect()
}

/// What a frame of a menu's rows did.
#[derive(Default)]
struct Picked {
    /// The entry chosen, and whether with Enter or Space, not the mouse.
    chosen: Option<(Action, bool)>,
    /// An entry took the keyboard, as `focus_first` asked.
    focused: bool,
}

/// The rows of an item's menu, as wide as the widest, in groups that lines
/// set apart. With `focus_first`, the first entry that can be used takes
/// the keyboard.
fn menu_rows(ui: &mut Ui, groups: &[Vec<Entry>], focus_first: bool) -> Picked {
    // Rows close together, so the menu covers less of the list.
    ui.spacing_mut().item_spacing.y = 2.0;
    let entries = groups.iter().flatten();
    ui.set_width(menu_width(
        ui,
        entries.map(|e| (e.text.as_str(), e.keys.as_str())),
    ));
    let mut picked = Picked::default();
    // The first frame only measures the menu.
    let mut focus = focus_first && !ui.is_sizing_pass();
    for (g, group) in groups.iter().enumerate() {
        if g > 0 {
            divider(ui);
        }
        for e in group {
            let r = entry_row(ui, e);
            if focus && r.enabled() {
                r.request_focus();
                picked.focused = true;
                focus = false;
            }
            if r.clicked() {
                let by_key = !r.clicked_by(egui::PointerButton::Primary);
                picked.chosen = Some((e.action.clone(), by_key));
            }
        }
    }
    picked
}

/// One row of an item's menu. One that can't be used is grayed out, and
/// pointing at it says why; screen readers hear that it can't be used, and
/// why.
fn entry_row(ui: &mut Ui, e: &Entry) -> egui::Response {
    let red = matches!(e.action, Action::Delete | Action::DeleteGroup).then_some(RED);
    let icon = Some(e.action.icon());
    let Some(why) = &e.unavailable else {
        return menu_entry(ui, &e.text, &e.keys, red, icon);
    };
    let r = ui
        .add_enabled_ui(false, |ui| menu_entry(ui, &e.text, &e.keys, red, icon))
        .inner;
    let description = why.clone();
    ui.ctx()
        .accesskit_node_builder(r.id, |node| node.set_description(description));
    tip_when_unavailable(r, why)
}

impl Action {
    /// The icon the app shows for the same action elsewhere.
    fn icon(&self) -> Icon {
        match self {
            Action::Copy(Paste::Yes) => Icon::Clipboard,
            Action::Copy(Paste::Plain) | Action::CopyImageText => Icon::Text,
            Action::Copy(Paste::No) | Action::CopyGroup(_) => Icon::Copy,
            Action::Pin | Action::PinGroup => Icon::Pin,
            Action::Tag | Action::TagGroup => Icon::Tag,
            Action::Details => Icon::Open,
            Action::OpenLink => Icon::Browser,
            Action::ShowInFolder(_) => Icon::Folder,
            Action::ViewFullScreen => Icon::FullScreen,
            Action::SaveImage => Icon::Save,
            Action::IgnoreApp(_) => Icon::EyeOff,
            Action::Delete | Action::DeleteGroup => Icon::Delete,
        }
    }
}

impl App {
    /// The entries of an item's menu, in groups that lines set apart.
    fn menu_groups(&mut self, it: &Item, in_popup: bool) -> Vec<Vec<Entry>> {
        // On one of several items selected together, the menu acts on them all.
        let group = if in_popup { Vec::new() } else { self.group() };
        if group.contains(&it.id) {
            return self.group_menu(&group);
        }
        let k = self.cfg.keys.clone();
        let entry = |text: String, keys: &str, action: Action| Entry {
            text,
            keys: keys.to_string(),
            action,
            unavailable: None,
        };
        let mut main = copy_entries(self.ways_to_copy(it), &k.paste);
        let pin = if it.pinned {
            t!("item-unpin")
        } else {
            t!("item-pin")
        };
        main.push(entry(pin, &k.pin, Action::Pin));
        // The popup has no details pane.
        if !in_popup {
            main.push(entry(t!("details-add-tag-tip"), &k.tag, Action::Tag));
            if self.cfg.view != ViewMode::Detail {
                main.push(entry(t!("item-open-details"), &k.details, Action::Details));
            }
        }
        let mut own = Vec::new();
        if it.kind == Kind::Link {
            own.push(entry(t!("details-open-in-browser"), "", Action::OpenLink));
        }
        if let Some(place) = place_of(it) {
            let action = Action::ShowInFolder(place.to_path_buf());
            own.push(entry(t!("details-show-in-folder"), "", action));
        }
        if it.kind == Kind::Image {
            // Full screen takes the full window, which the popup isn't.
            if !in_popup {
                let text = t!("item-view-full-screen");
                own.push(entry(text, "F11", Action::ViewFullScreen));
            }
            own.push(entry(t!("details-save-image"), "", Action::SaveImage));
            if it.ocr.as_deref().is_some_and(|t| !t.trim().is_empty()) {
                own.push(entry(t!("item-copy-image-text"), "", Action::CopyImageText));
            }
        }
        let mut rule = Vec::new();
        if let Some(app) = it.app.as_deref() {
            if !self.cfg.rules.iter().any(|r| r.skips_app(app)) {
                // Short, so the menu stays narrow; the question it asks
                // names the program.
                let text = t!("item-ignore-app");
                rule.push(entry(text, "", Action::IgnoreApp(app.to_string())));
            }
        }
        let delete = vec![entry(t!("item-delete"), &k.delete, Action::Delete)];
        [main, own, rule, delete]
            .into_iter()
            .filter(|group| !group.is_empty())
            .collect()
    }

    /// The menu of several items selected together.
    fn group_menu(&self, group: &[u64]) -> Vec<Vec<Entry>> {
        let k = &self.cfg.keys;
        let n = group.len();
        let entry = |text: String, keys: &str, action: Action| Entry {
            text,
            keys: keys.to_string(),
            action,
            unavailable: None,
        };
        // Paste first, grayed out without a program to paste into.
        let text = t!("item-paste-group", count = n);
        let action = Action::CopyGroup(Paste::Yes);
        let mut main = vec![if paste::exists(self.win.previous.load(Ordering::SeqCst)) {
            entry(text, &k.paste, action)
        } else {
            unavailable_paste(text, &k.paste, action)
        }];
        let text = t!("item-copy-group", count = n);
        main.push(entry(text, &k.copy_only, Action::CopyGroup(Paste::No)));
        let all_pinned = group
            .iter()
            .all(|&id| self.library.get(id).is_some_and(|it| it.pinned));
        let pin = if all_pinned {
            t!("item-unpin-group", count = n)
        } else {
            t!("item-pin-group", count = n)
        };
        main.push(entry(pin, &k.pin, Action::PinGroup));
        main.push(entry(
            t!("item-tag-group", count = n),
            &k.tag,
            Action::TagGroup,
        ));
        let delete = t!("item-delete-group", count = n);
        vec![main, vec![entry(delete, &k.delete, Action::DeleteGroup)]]
    }

    /// Opens the menu of item `id` at `at`, in the popup's list or the full
    /// window's. On an item outside the ones selected together, it selects
    /// that one alone.
    fn open_item_menu(&mut self, id: u64, at: Pos2, in_popup: bool, by_key: bool) {
        if !self.marked.contains(&id) {
            self.clear_marks();
        }
        self.selected = Some(id);
        self.item_menu = Some(ItemMenu {
            id,
            at,
            in_popup,
            focus_first: by_key,
        });
        if by_key {
            self.menu_by_key = false;
        }
    }

    /// After a list or grid was drawn: opens the menu one of its rows or
    /// cards asked for. A Shift+F10 waits for the selected item to be
    /// drawn, unless it is no longer shown.
    pub(super) fn after_rows(&mut self, open_menu: Option<(u64, Pos2, bool)>, in_popup: bool) {
        if let Some((id, at, by_key)) = open_menu {
            self.open_item_menu(id, at, in_popup, by_key);
        } else if self.menu_by_key && !self.results.iter().any(|h| Some(h.id) == self.selected) {
            self.menu_by_key = false;
        }
    }

    /// Draws the open item menu of this list, the popup's or the full
    /// window's, and carries out the entry picked.
    pub(super) fn item_menu(&mut self, ui: &Ui, in_popup: bool) {
        let Some(menu) = self.item_menu.as_ref().filter(|m| m.in_popup == in_popup) else {
            return;
        };
        let (id, at, focus_first) = (menu.id, menu.at, menu.focus_first);
        let Some(it) = self.library.get(id).cloned() else {
            self.item_menu = None;
            return;
        };
        let groups = self.menu_groups(&it, in_popup);
        let mut open = true;
        // It opens toward the reading end.
        let align = if i18n::rtl() {
            egui::RectAlign::BOTTOM_END
        } else {
            egui::RectAlign::BOTTOM_START
        };
        let picked = egui::Popup::new(
            egui::Id::unique(("item-menu", id)),
            ui.ctx().clone(),
            at,
            ui.layer_id(),
        )
        .kind(egui::PopupKind::Menu)
        .layout(Layout::top_down_justified(Align::Min))
        .style(egui::containers::menu::menu_style)
        .align(align)
        .accessible_name(t!("item-menu"))
        .open_bool(&mut open)
        .show(|ui| menu_rows(ui, &groups, focus_first))
        .map(|shown| shown.inner)
        .unwrap_or_default();
        if let Some(menu) = self.item_menu.as_mut().filter(|_| picked.focused) {
            menu.focus_first = false;
        }
        let chosen = picked.chosen;
        if !open || chosen.is_some() {
            self.item_menu = None;
            // Typing goes to the search field again; Esc took the keyboard
            // away from it. Tagging takes it for its own field, and a
            // question asked from the keyboard for its button.
            let keeps_keyboard = match &chosen {
                Some((Action::Tag, _)) => true,
                Some((Action::IgnoreApp(_), by_key)) => *by_key,
                _ => false,
            };
            if !keeps_keyboard {
                self.focus_search.store(true, Ordering::SeqCst);
            }
        }
        if let Some((action, by_key)) = chosen {
            self.menu_action(ui.ctx(), &it, action, in_popup, by_key);
        }
    }

    /// Asks before a rule that stops recording copies from the program
    /// `app`, shown as `name`, in the popup's window (`in_popup`) or the
    /// full window: the rule waits for Never record. It replaces a
    /// question asked before.
    fn ask_to_ignore_app(&mut self, app: &str, name: &str, in_popup: bool, by_key: bool) {
        close_question(&mut self.toasts, true);
        close_question(&mut self.toasts, false);
        let toast = self.toast_with(String::new(), Duration::MAX);
        ask_to_ignore(toast, app, name, in_popup, by_key);
    }

    fn menu_action(
        &mut self,
        ctx: &egui::Context,
        it: &Item,
        action: Action,
        in_popup: bool,
        by_key: bool,
    ) {
        match action {
            Action::Copy(paste) => self.run(Command::Copy(it.id, paste)),
            Action::Pin => self.run(Command::Pin(it.id)),
            Action::Tag => self.start_tagging(it.id),
            Action::Details => self.run(Command::Open(it.id)),
            Action::OpenLink => ctx.open_url(egui::OpenUrl::new_tab(it.text.trim())),
            Action::ShowInFolder(place) => self.show_in_folder(&place),
            Action::ViewFullScreen => self.view_full_screen(it.id),
            Action::SaveImage => self.save_image_as(it),
            Action::CopyImageText => {
                if let Some(text) = it.ocr.as_deref() {
                    self.run(Command::CopyText(text.to_string()));
                }
            }
            Action::IgnoreApp(app) => {
                let program = it.program.as_deref().and_then(|p| self.programs.get(p));
                let name = crate::app::programs::shown_name(&app, program);
                self.ask_to_ignore_app(&app, &name, in_popup, by_key);
            }
            Action::Delete => self.run(Command::Delete(it.id)),
            Action::CopyGroup(paste) => self.run(Command::CopyTogether(self.group(), paste)),
            Action::PinGroup => self.run(Command::PinGroup(self.group())),
            Action::TagGroup => self.group_tag = Some(String::new()),
            Action::DeleteGroup => self.run(Command::DeleteGroup(self.group())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A formatted text's menu with the usual keys, in the window's
    /// language: the ways to put it back as `App::ways_to_copy` gives
    /// them, with a program to paste into or without, then Delete.
    fn menu(can_paste: bool) -> Vec<Vec<Entry>> {
        let keys = crate::config::Keys::default();
        let way = |text: String, keys: &str, paste: Paste| CopyWay {
            text,
            keys: keys.to_string(),
            paste,
        };
        let ways = if can_paste {
            vec![
                way(
                    t!("item-paste-into", app = "Microsoft Word"),
                    &keys.paste,
                    Paste::Yes,
                ),
                way(t!("item-paste-plain"), &keys.paste_plain, Paste::Plain),
                way(t!("item-copy-only"), &keys.copy_only, Paste::No),
            ]
        } else {
            vec![
                way(t!("item-copy"), &keys.copy_only, Paste::No),
                way(t!("item-copy-plain"), &keys.paste_plain, Paste::Plain),
            ]
        };
        let delete = Entry {
            text: t!("item-delete"),
            keys: keys.delete.clone(),
            action: Action::Delete,
            unavailable: None,
        };
        vec![copy_entries(ways, &keys.paste), vec![delete]]
    }

    /// Without a program to paste into, the menu starts with Paste and its
    /// key, grayed out with why, then Copy with its key, as before. With
    /// one, it is as before: Paste into it, and nothing grayed out.
    #[test]
    fn the_menu_starts_with_paste() {
        let keys = crate::config::Keys::default();
        let shown = |groups: &[Vec<Entry>]| -> Vec<(String, String, bool)> {
            let entries = groups.iter().flatten();
            entries
                .map(|e| (e.text.clone(), e.keys.clone(), e.unavailable.is_some()))
                .collect()
        };
        let without = menu(false);
        assert_eq!(
            shown(&without),
            [
                (t!("item-paste"), keys.paste.clone(), true),
                (t!("item-copy"), keys.copy_only.clone(), false),
                (t!("item-copy-plain"), keys.paste_plain.clone(), false),
                (t!("item-delete"), keys.delete.clone(), false),
            ]
        );
        assert_eq!(
            without[0][0].unavailable,
            Some(t!("item-paste-unavailable"))
        );
        let paste_into = t!("item-paste-into", app = "Microsoft Word");
        assert_eq!(
            shown(&menu(true)),
            [
                (paste_into, keys.paste.clone(), false),
                (t!("item-paste-plain"), keys.paste_plain.clone(), false),
                (t!("item-copy-only"), keys.copy_only.clone(), false),
                (t!("item-delete"), keys.delete.clone(), false),
            ]
        );
    }

    /// An entry as screen readers get it.
    #[derive(Debug)]
    struct Heard {
        name: String,
        place: Rect,
        usable: bool,
        /// Why it can't be used.
        why: String,
    }

    /// A menu's rows in one frame of `ctx` with `events`: what they did,
    /// the texts drawn, and the entries screen readers get, from the top.
    fn rows_frame(
        ctx: &egui::Context,
        groups: &[Vec<Entry>],
        focus_first: bool,
        events: Vec<egui::Event>,
    ) -> (Picked, Vec<String>, Vec<Heard>) {
        let input = egui::RawInput {
            events,
            focused: true,
            ..Default::default()
        };
        let mut picked = Picked::default();
        let mut output = ctx.run_ui(input, |ui| picked = menu_rows(ui, groups, focus_first));
        output.textures_delta.clear();
        let texts = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_string()),
                _ => None,
            })
            .collect();
        let mut entries: Vec<Heard> = output
            .platform_output
            .accesskit_update
            .iter()
            .flat_map(|update| &update.nodes)
            .filter(|(_, node)| node.role() == egui::accesskit::Role::MenuItem)
            .map(|(_, node)| {
                let b = node.bounds().expect("a place");
                Heard {
                    name: node.label().unwrap_or("").to_string(),
                    place: Rect::from_min_max(
                        Pos2::new(b.x0 as f32, b.y0 as f32),
                        Pos2::new(b.x1 as f32, b.y1 as f32),
                    ),
                    usable: !node.is_disabled(),
                    why: node.description().unwrap_or("").to_string(),
                }
            })
            .collect();
        entries.sort_by(|a, b| a.place.top().total_cmp(&b.place.top()));
        (picked, texts, entries)
    }

    /// A click on the entry named `name`, over three frames: what the
    /// rows did.
    fn click(ctx: &egui::Context, groups: &[Vec<Entry>], name: &str) -> Picked {
        let (_, _, entries) = rows_frame(ctx, groups, false, Vec::new());
        let at = entries
            .iter()
            .find(|heard| heard.name == name)
            .unwrap_or_else(|| panic!("{name} in {entries:?}"))
            .place
            .center();
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        rows_frame(ctx, groups, false, vec![egui::Event::PointerMoved(at)]);
        rows_frame(ctx, groups, false, vec![button(true)]);
        rows_frame(ctx, groups, false, vec![button(false)]).0
    }

    /// Without a program to paste into, the grayed Paste comes first:
    /// screen readers hear it can't be used and why, pointing at it says
    /// why, a click on it does nothing, and the keyboard starts on Copy.
    /// With one, the first entry pastes.
    fn check_the_grayed_paste() {
        let ctx = egui::Context::default();
        add_fonts(&ctx);
        ctx.enable_accesskit();
        ctx.all_styles_mut(|s| s.interaction.tooltip_delay = 0.0);
        // The fonts apply from the next frame.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let why = t!("item-paste-unavailable");
        let groups = menu(false);
        let (_, texts, entries) = rows_frame(&ctx, &groups, false, Vec::new());
        let (paste, copy) = (&entries[0], &entries[1]);
        assert_eq!((&paste.name, paste.usable), (&t!("item-paste"), false));
        assert_eq!(paste.why, why, "screen readers hear why");
        assert_eq!((&copy.name, copy.usable), (&t!("item-copy"), true));
        assert!(copy.why.is_empty());
        // Pointing at it shows why, word by word as drawn.
        let at = paste.place.center();
        rows_frame(&ctx, &groups, false, vec![egui::Event::PointerMoved(at)]);
        let (_, pointed, _) = rows_frame(&ctx, &groups, false, Vec::new());
        assert!(pointed.len() > texts.len(), "a tip: {pointed:?}");
        let drawn = pointed.join(" ");
        for word in why.split_whitespace() {
            assert!(drawn.contains(word), "{word} in {drawn}");
        }
        rows_frame(&ctx, &groups, false, vec![egui::Event::PointerGone]);
        let picked = click(&ctx, &groups, &t!("item-paste"));
        assert!(picked.chosen.is_none(), "a click on Paste does nothing");
        let picked = click(&ctx, &groups, &t!("item-copy"));
        assert!(matches!(
            picked.chosen,
            Some((Action::Copy(Paste::No), false))
        ));
        // Shift+F10: the keyboard starts on Copy, the first that works.
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (picked, _, entries) = rows_frame(&ctx, &groups, true, Vec::new());
        assert!(picked.focused);
        let focused = ctx
            .memory(|m| m.focused())
            .and_then(|id| ctx.read_response(id));
        let focused = focused.expect("an entry has the keyboard").rect;
        assert_eq!(focused, entries[1].place, "{entries:?}");
        // With a program to paste into, the first entry pastes.
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let groups = menu(true);
        let paste_into = t!("item-paste-into", app = "Microsoft Word");
        let picked = click(&ctx, &groups, &paste_into);
        assert!(matches!(
            picked.chosen,
            Some((Action::Copy(Paste::Yes), false))
        ));
    }

    #[test]
    fn the_grayed_paste_says_why_and_does_nothing() {
        check_the_grayed_paste();
    }

    #[test]
    fn the_grayed_paste_says_why_and_does_nothing_right_to_left() {
        crate::i18n::passes_in_arabic(
            module_path!(),
            "the_grayed_paste_says_why_and_does_nothing_in_arabic",
        );
    }

    #[test]
    #[ignore = "run in a process of its own by the_grayed_paste_says_why_and_does_nothing_right_to_left"]
    fn the_grayed_paste_says_why_and_does_nothing_in_arabic() {
        if crate::i18n::arabic_run() {
            check_the_grayed_paste();
        }
    }
}
