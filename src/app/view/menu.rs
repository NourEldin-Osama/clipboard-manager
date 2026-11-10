//! The menu of an item: a right-click on a row or a card, or Shift+F10 for
//! the selected one. Every action with its key, what suits the item, and
//! Delete last, in red, apart from the rest.

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
        };
        let mut main: Vec<Entry> = self
            .ways_to_copy(it)
            .into_iter()
            .map(|way| entry(way.text, &way.keys, Action::Copy(way.paste)))
            .collect();
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
                main.push(entry(t!("item-open-details"), "", Action::Details));
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
            own.push(entry(t!("details-save-image"), "", Action::SaveImage));
            if it.ocr.as_deref().is_some_and(|t| !t.trim().is_empty()) {
                own.push(entry(t!("item-copy-image-text"), "", Action::CopyImageText));
            }
        }
        let mut rule = Vec::new();
        if let Some(app) = it.app.as_deref() {
            if !self.cfg.rules.iter().any(|r| r.skips_app(app)) {
                // The program's own name once it is known, else its file name.
                let program = it.program.as_deref().and_then(|p| self.programs.get(p));
                let name = crate::app::programs::shown_name(app, program);
                let text = t!("item-ignore-app", app = name.as_str());
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
        };
        let mut main = Vec::new();
        if paste::exists(self.win.previous.load(Ordering::SeqCst)) {
            let text = t!("item-paste-group", count = n);
            main.push(entry(text, &k.paste, Action::CopyGroup(Paste::Yes)));
        }
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
        let mut chosen = None;
        let mut focused = false;
        // It opens toward the reading end.
        let align = if i18n::rtl() {
            egui::RectAlign::BOTTOM_END
        } else {
            egui::RectAlign::BOTTOM_START
        };
        egui::Popup::new(
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
        .show(|ui| {
            let entries = groups.iter().flatten();
            ui.set_width(menu_width(
                ui,
                entries.map(|e| (e.text.as_str(), e.keys.as_str())),
            ));
            for (g, group) in groups.iter().enumerate() {
                if g > 0 {
                    divider(ui);
                }
                for (i, e) in group.iter().enumerate() {
                    let red =
                        matches!(e.action, Action::Delete | Action::DeleteGroup).then_some(RED);
                    let r = menu_entry(ui, &e.text, &e.keys, red);
                    // The first frame only measures the menu.
                    if focus_first && g == 0 && i == 0 && !ui.is_sizing_pass() {
                        r.request_focus();
                        focused = true;
                    }
                    if r.clicked() {
                        chosen = Some(e.action.clone());
                    }
                }
            }
        });
        if let Some(menu) = self.item_menu.as_mut().filter(|_| focused) {
            menu.focus_first = false;
        }
        if !open || chosen.is_some() {
            self.item_menu = None;
            // Typing goes to the search field again; Esc took the keyboard
            // away from it. Tagging takes it for its own field.
            if !matches!(chosen, Some(Action::Tag)) {
                self.focus_search.store(true, Ordering::SeqCst);
            }
        }
        if let Some(action) = chosen {
            self.menu_action(ui.ctx(), &it, action);
        }
    }

    fn menu_action(&mut self, ctx: &egui::Context, it: &Item, action: Action) {
        match action {
            Action::Copy(paste) => self.run(Command::Copy(it.id, paste)),
            Action::Pin => self.run(Command::Pin(it.id)),
            Action::Tag => self.start_tagging(it.id),
            Action::Details => self.run(Command::Open(it.id)),
            Action::OpenLink => ctx.open_url(egui::OpenUrl::new_tab(it.text.trim())),
            Action::ShowInFolder(place) => self.show_in_folder(&place),
            Action::SaveImage => self.save_image_as(it),
            Action::CopyImageText => {
                if let Some(text) = it.ocr.as_deref() {
                    self.run(Command::CopyText(text.to_string()));
                }
            }
            Action::IgnoreApp(app) => self.ignore_app(&app),
            Action::Delete => self.run(Command::Delete(it.id)),
            Action::CopyGroup(paste) => self.run(Command::CopyTogether(self.group(), paste)),
            Action::PinGroup => self.run(Command::PinGroup(self.group())),
            Action::TagGroup => self.group_tag = Some(String::new()),
            Action::DeleteGroup => self.run(Command::DeleteGroup(self.group())),
        }
    }
}
