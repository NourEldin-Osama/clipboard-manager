//! Backups from the window. Settings, Privacy writes the history to a file
//! locked with a password and restores one; so does the message about a
//! history that couldn't be opened. A small window asks for the password,
//! the work runs on threads of its own (`Background`), and its progress
//! shows in Settings and the status bar. The file is `crate::backup`.

use eframe::egui::text_edit::TextEditState;
use eframe::egui::{Align, Frame, Margin, Rect, Ui};

use super::background::{BackupProgress, RestoreRequest};
use super::icons::{self, Icon};
use super::view::{
    far, lbl, primary_button, radius, row, row_layout, row_of, set_name, size, slow_spinner, space,
    text_shade, title_lbl, txt, warning_fill, AMBER, RED,
};
use super::*;
use crate::backup::{Problem, Restored};

/// The extension of a backup's file.
const EXTENSION: &str = "cmbackup";

/// The fewest characters a new backup's password has.
const SHORTEST_PASSWORD: usize = 8;

/// How wide the password window's content is at most, and the room it
/// keeps from the window's edges.
const DIALOG_WIDTH: f32 = 400.0;
const WINDOW_ROOM: f32 = 48.0;

/// How tall the title's row and the buttons' row are.
const BAR_HEIGHT: f32 = 32.0;

/// How big the buttons beside the title and the password are, and their
/// icons.
const ICON_BUTTON: f32 = 28.0;
const CLOSE_CROSS: f32 = 13.0;
const EYE: f32 = 16.0;

/// How big the warning's icon is.
const WARNING_ICON: f32 = 14.0;

/// How long the messages after a backup show.
const DONE_FOR: Duration = Duration::from_secs(8);
const PROBLEM_FOR: Duration = Duration::from_secs(12);

/// A backup being written or restored.
pub(super) struct BackupJob {
    /// Restoring a backup, not writing one.
    restoring: bool,
    progress: Arc<BackupProgress>,
}

/// What a running backup is doing, and how far it is.
#[derive(Clone, Copy)]
pub(super) enum BackupStage {
    Saving {
        done: usize,
        total: usize,
    },
    /// A restore reads the whole file before it adds anything.
    Checking,
    Restoring {
        done: usize,
        total: usize,
    },
}

/// The window that asks for a backup's password. It holds the password, so
/// it can't be printed.
pub(super) struct BackupDialog {
    /// The backup to restore; none for a new one.
    file: Option<PathBuf>,
    password: String,
    /// A new backup's password, typed again.
    again: String,
    /// The password shows as typed.
    shown: bool,
    /// Why the last password didn't open the backup.
    problem: Option<String>,
    /// The restore is trying the password: the window waits for it.
    checking: bool,
    /// The password field takes the keyboard in the next frame.
    focus: bool,
}

/// What a button of the password window asks for.
enum Answer {
    Cancel,
    /// Goes on with the password typed.
    Go,
}

impl App {
    /// What the running backup is doing.
    pub(super) fn backup_stage(&self) -> Option<BackupStage> {
        let job = self.backup_job.as_ref()?;
        let done = job.progress.done.load(Ordering::Relaxed);
        let total = job.progress.total.load(Ordering::Relaxed);
        Some(if !job.restoring {
            BackupStage::Saving { done, total }
        } else if total == 0 {
            BackupStage::Checking
        } else {
            BackupStage::Restoring { done, total }
        })
    }

    /// Opens the window that asks for a new backup's password.
    pub(super) fn ask_backup_password(&mut self) {
        if self.backup_job.is_none() && self.backup_dialog.is_none() {
            self.backup_dialog = Some(BackupDialog::new(None));
        }
    }

    /// Asks which backup to restore and checks that this version can read
    /// it, then asks for its password.
    pub(super) fn ask_backup_file(&mut self) {
        if self.backup_job.is_some() || self.backup_dialog.is_some() {
            return;
        }
        let picked = rfd::FileDialog::new()
            .add_filter(t!("backup-file-type"), &[EXTENSION])
            .add_filter(t!("backup-all-files"), &["*"])
            .pick_file();
        self.after_dialog();
        let Some(file) = picked else {
            return;
        };
        match crate::backup::check_file(&file) {
            Ok(()) => self.backup_dialog = Some(BackupDialog::new(Some(file))),
            Err(problem) => {
                log::warn!("the chosen file can't be restored: {problem}");
                self.say_backup_problem(&problem);
            }
        }
    }

    /// Draws the password window while it is open, and acts on its answer.
    pub(super) fn backup_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.backup_dialog.take() else {
            return;
        };
        let width = (ctx.content_rect().width() - 2.0 * WINDOW_ROOM).clamp(200.0, DIALOG_WIDTH);
        let frame = Frame::popup(&ctx.global_style()).inner_margin(Margin::same(space::HUGE as i8));
        let mut answer = None;
        let shown = egui::Modal::new(egui::Id::unique("backup-password"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(width);
                answer = dialog.show(ui);
            });
        let title = dialog.title();
        ctx.accesskit_node_builder(shown.response.id, |node| {
            node.set_role(egui::accesskit::Role::Dialog);
            node.set_label(title);
        });
        // Esc or a click beside it cancels, except while the password is tried.
        if shown.should_close() && !dialog.checking {
            answer = Some(Answer::Cancel);
        }
        match answer {
            None => self.backup_dialog = Some(dialog),
            Some(Answer::Cancel) => self.close_backup_dialog(),
            Some(Answer::Go) => match dialog.file.clone() {
                Some(file) => {
                    let password = dialog.password.clone();
                    dialog.checking = true;
                    dialog.problem = None;
                    self.backup_dialog = Some(dialog);
                    self.start_restore(file, password);
                }
                None => {
                    self.close_backup_dialog();
                    self.save_backup(dialog.password);
                }
            },
        }
    }

    /// Closes the password window. egui keeps the texts a field had, for
    /// its Undo; they go too, so the password doesn't stay in memory.
    fn close_backup_dialog(&mut self) {
        self.backup_dialog = None;
        self.win.ctx.data_mut(|data| {
            for again in [false, true] {
                data.remove::<TextEditState>(password_field(again));
            }
        });
        self.after_dialog();
    }

    /// Asks where to save a new backup locked with `password`, then writes
    /// it on a thread of its own.
    fn save_backup(&mut self, password: String) {
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let name = t!("backup-file-name", date = date);
        let picked = rfd::FileDialog::new()
            .set_file_name(format!("{name}.{EXTENSION}"))
            .add_filter(t!("backup-file-type"), &[EXTENSION])
            .save_file();
        self.after_dialog();
        let Some(file) = picked else {
            return;
        };
        let items = self.library.items().to_vec();
        let progress = Arc::new(BackupProgress::default());
        progress.total.store(items.len(), Ordering::Relaxed);
        log::info!("saving a backup of {} items", items.len());
        self.backup_job = Some(BackupJob {
            restoring: false,
            progress: progress.clone(),
        });
        let store = self.library.store().clone();
        self.background
            .export_backup(store, items, file, password, progress);
    }

    /// Restores the backup in `file` with `password`, on a thread of its own.
    fn start_restore(&mut self, file: PathBuf, password: String) {
        let progress = Arc::new(BackupProgress::default());
        log::info!("restoring a backup");
        self.backup_job = Some(BackupJob {
            restoring: true,
            progress: progress.clone(),
        });
        let request = RestoreRequest {
            file,
            password,
            store: self.library.store().clone(),
            known: self.library.contents(),
            ids: self.library.ids(),
        };
        self.background.restore_backup(request, progress);
    }

    /// A backup is written, or failed.
    pub(super) fn backup_saved(&mut self, saved: Result<(usize, PathBuf), String>) {
        self.backup_job = None;
        match saved {
            Ok((count, file)) => {
                log::info!("saved a backup of {count} items");
                let toast = self.toast_with(t!("toast-backup-saved", count = count), DONE_FOR);
                toast.icon = Some(Icon::Check);
                toast.detail = Some(bidi::left_to_right(&file.display().to_string()));
            }
            Err(error) => {
                log::error!("could not save the backup: {error}");
                self.backup_message(t!("toast-backup-not-saved"), Some(error));
            }
        }
    }

    /// The password opened the backup: its window closes while the restore
    /// goes on.
    pub(super) fn backup_opened(&mut self) {
        if self.backup_dialog.as_ref().is_some_and(|d| d.checking) {
            self.close_backup_dialog();
        }
    }

    /// Adds restored items to the history. They are indexed after new
    /// copies, as files added are.
    pub(super) fn add_restored(&mut self, items: Vec<Item>) {
        match self.library.add_restored(items) {
            Ok(added) => {
                for (id, kind) in added {
                    for job in self.indexing_jobs(id, kind) {
                        self.worker.send_backfill(job);
                    }
                }
                self.dirty = true;
            }
            Err(e) => self.set_error(log_error!("error-saving-item", error = e.to_string())),
        }
    }

    /// A restore is over: says what it added and skipped, or why it stopped.
    pub(super) fn restore_finished(&mut self, finished: Result<Restored, Problem>) {
        self.backup_job = None;
        let asking = self.backup_dialog.as_ref().is_some_and(|d| d.checking);
        match finished {
            Ok(restored) => {
                log::info!(
                    "restored a backup: added {} items, skipped {} already there",
                    restored.added,
                    restored.skipped
                );
                if asking {
                    self.close_backup_dialog();
                }
                let text = t!("toast-backup-restored", count = restored.added);
                let toast = self.toast_with(text, DONE_FOR);
                toast.icon = Some(Icon::Check);
                toast.detail = Some(t!("toast-backup-skipped", count = restored.skipped));
                // The history's limit and age apply to the items it brought.
                self.prune(true);
            }
            // The password didn't open it: the window asks again.
            Err(Problem::Refused) if asking => {
                log::warn!("restoring a backup: {}", Problem::Refused);
                if let Some(dialog) = self.backup_dialog.as_mut() {
                    dialog.checking = false;
                    dialog.password.clear();
                    dialog.problem = Some(t!("backup-refused"));
                    dialog.focus = true;
                }
            }
            Err(problem) => {
                log::error!("could not restore the backup: {problem}");
                if asking {
                    self.close_backup_dialog();
                }
                self.say_backup_problem(&problem);
            }
        }
    }

    /// Says why a backup can't be restored.
    fn say_backup_problem(&mut self, problem: &Problem) {
        let (text, detail) = match problem {
            Problem::NotBackup => (t!("toast-backup-not-a-backup"), None),
            Problem::Newer => (t!("toast-backup-newer"), None),
            Problem::Refused => (t!("toast-backup-refused"), None),
            Problem::Failed(e) => (t!("toast-backup-not-restored"), Some(format!("{e:#}"))),
        };
        self.backup_message(text, detail);
    }

    /// A warning about a backup, with an error's details under it.
    fn backup_message(&mut self, text: String, detail: Option<String>) {
        let toast = self.toast_with(text, PROBLEM_FOR);
        toast.icon = Some(Icon::Warning);
        toast.detail = detail;
    }
}

/// Makes `toast` the message about a history this account couldn't open,
/// kept in the data folder as `aside`: it stays until it is closed, and its
/// button restores a backup.
pub(super) fn history_moved_aside(toast: &mut Toast, aside: &Path) {
    // The file's name only: the whole path would be cut short. It reads
    // left to right inside an Arabic sentence too.
    let name = aside.file_name().map_or_else(
        || aside.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    toast.text = t!("toast-history-moved-aside");
    toast.detail = Some(t!(
        "toast-history-kept-as",
        file = bidi::left_to_right(&name)
    ));
    toast.icon = Some(Icon::Warning);
    toast.how_long = Duration::MAX;
    toast.action = Some((t!("toast-restore-backup"), Command::ImportBackup));
}

impl BackupDialog {
    fn new(file: Option<PathBuf>) -> Self {
        Self {
            file,
            password: String::new(),
            again: String::new(),
            shown: false,
            problem: None,
            checking: false,
            focus: true,
        }
    }

    fn title(&self) -> String {
        if self.file.is_some() {
            t!("backup-restore-title")
        } else {
            t!("backup-export-title")
        }
    }

    /// Whether the password can be used: a new backup's is long enough and
    /// typed the same twice.
    fn ready(&self) -> bool {
        if self.checking {
            return false;
        }
        match self.file {
            Some(_) => !self.password.is_empty(),
            None => {
                self.password.chars().count() >= SHORTEST_PASSWORD && self.again == self.password
            }
        }
    }

    /// Draws the window's content: the title with its close button, the
    /// password with the eye that shows it, and for a new backup the
    /// password again and the warning; then the buttons. Returns what was
    /// asked for.
    fn show(&mut self, ui: &mut Ui) -> Option<Answer> {
        let mut answer = None;
        let (strong, weak) = (
            ui.visuals().strong_text_color(),
            ui.visuals().weak_text_color(),
        );
        let red = text_shade(RED, ui.visuals().dark_mode);
        let title = self.title();
        row_of(ui, BAR_HEIGHT, |ui| {
            title_lbl(ui, &title, size::LARGE, strong);
            far(ui, |ui| {
                let close = ui.add_enabled_ui(!self.checking, |ui| {
                    let size = Vec2::splat(ICON_BUTTON);
                    let tip = t!("common-cancel");
                    icons::icon_button_ex(ui, Icon::Close, &tip, size, CLOSE_CROSS, None, None)
                });
                if close.inner.clicked() {
                    answer = Some(Answer::Cancel);
                }
            });
        });
        if let Some(file) = &self.file {
            let name = file.file_name().map_or_else(
                || file.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            row(ui, |ui| lbl(ui, &name, size::SMALL, weak));
        }
        ui.add_space(space::MEDIUM);
        let mut entered = false;
        row(ui, |ui| {
            lbl(ui, &t!("backup-password"), size::CONTROL, strong)
        });
        let field = row(ui, |ui| {
            let width = ui.available_width() - ICON_BUTTON - ui.spacing().item_spacing.x;
            let field = ui.add_enabled(
                !self.checking,
                egui::TextEdit::singleline(&mut self.password)
                    .id(password_field(false))
                    .password(!self.shown)
                    .desired_width(width),
            );
            set_name(&field, &t!("backup-password"));
            let (eye, tip) = if self.shown {
                (Icon::EyeOff, t!("backup-hide-password"))
            } else {
                (Icon::Eye, t!("backup-show-password"))
            };
            let size = Vec2::splat(ICON_BUTTON);
            if icons::icon_button_ex(ui, eye, &tip, size, EYE, None, None).clicked() {
                self.shown = !self.shown;
            }
            field
        });
        if std::mem::take(&mut self.focus) {
            field.request_focus();
        }
        entered |= enter_pressed(ui, &field);
        if self.file.is_none() {
            ui.add_space(space::SMALL);
            row(ui, |ui| {
                lbl(ui, &t!("backup-password-again"), size::CONTROL, strong)
            });
            let again = row(ui, |ui| {
                let width = ui.available_width() - ICON_BUTTON - ui.spacing().item_spacing.x;
                let again = ui.add(
                    egui::TextEdit::singleline(&mut self.again)
                        .id(password_field(true))
                        .password(!self.shown)
                        .desired_width(width),
                );
                set_name(&again, &t!("backup-password-again"));
                again
            });
            entered |= enter_pressed(ui, &again);
            // Why the button waits.
            if self.password.chars().count() < SHORTEST_PASSWORD {
                let short = t!("backup-password-short", count = SHORTEST_PASSWORD);
                note_line(ui, &short, weak);
            } else if !self.again.is_empty() && self.again != self.password {
                note_line(ui, &t!("backup-passwords-differ"), red);
            }
            ui.add_space(space::MEDIUM);
            warning(ui, &t!("backup-forget-warning"));
        }
        if let Some(problem) = &self.problem {
            note_line(ui, problem, red);
        }
        if self.checking {
            ui.add_space(space::SMALL);
            row(ui, |ui| {
                slow_spinner(ui, 14.0, Some(weak));
                lbl(ui, &t!("backup-checking"), size::SMALL, weak);
            });
        }
        ui.add_space(space::LARGE);
        let ready = self.ready();
        row_of(ui, BAR_HEIGHT, |ui| {
            far(ui, |ui| {
                let label = if self.file.is_some() {
                    t!("backup-restore-start")
                } else {
                    t!("backup-export-next")
                };
                if ui
                    .add_enabled_ui(ready, |ui| primary_button(ui, &label))
                    .inner
                    .clicked()
                {
                    answer = Some(Answer::Go);
                }
                let cancel = egui::Button::new(txt(&t!("common-cancel"), size::BODY, None))
                    .min_size(Vec2::new(0.0, BAR_HEIGHT));
                if ui.add_enabled(!self.checking, cancel).clicked() {
                    answer = Some(Answer::Cancel);
                }
            });
        });
        if entered && ready {
            answer = Some(Answer::Go);
        }
        answer
    }
}

/// The id of a password field: the first, or the one it is typed `again` in.
fn password_field(again: bool) -> egui::Id {
    egui::Id::unique(("backup-password", again))
}

/// Whether Enter was pressed in `field`.
fn enter_pressed(ui: &Ui, field: &egui::Response) -> bool {
    field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
}

/// A short line under the fields, in `color`.
fn note_line(ui: &mut Ui, text: &str, color: Color32) {
    ui.add_space(space::TINY);
    bidi::paragraph(ui, text, FontId::proportional(size::SMALL), color);
}

/// A warning on a faint amber tint, with its icon at the reading start.
fn warning(ui: &mut Ui, text: &str) {
    let dark = ui.visuals().dark_mode;
    let font = FontId::proportional(size::SMALL);
    Frame::new()
        .fill(warning_fill(dark))
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(Margin::symmetric(space::MEDIUM as i8, space::SMALL as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(row_layout().with_cross_align(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.x = space::SMALL;
                // On the middle of the text's first line.
                let line = ui.fonts_mut(|f| f.row_height(&font));
                let (place, _) =
                    ui.allocate_exact_size(Vec2::new(WARNING_ICON, line), Sense::hover());
                let at = Rect::from_center_size(place.center(), Vec2::splat(WARNING_ICON));
                icons::paint(ui.painter(), at, Icon::Warning, text_shade(AMBER, dark));
                let color = ui.visuals().text_color();
                bidi::paragraph(ui, text, font, color);
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_backup_waits_for_a_long_enough_password_typed_twice() {
        let mut dialog = BackupDialog::new(None);
        dialog.password = "short".into();
        dialog.again = "short".into();
        assert!(!dialog.ready(), "too short");
        dialog.password = "long enough".into();
        assert!(!dialog.ready(), "typed once");
        dialog.again = "long enough".into();
        assert!(dialog.ready());
        // Letters, not bytes: eight Arabic letters are enough.
        dialog.password = "كلمةسرية".into();
        dialog.again = dialog.password.clone();
        assert!(dialog.ready());
        let mut restore = BackupDialog::new(Some(PathBuf::from("history.cmbackup")));
        assert!(!restore.ready(), "no password yet");
        restore.password = "x".into();
        assert!(restore.ready(), "any password is tried");
        restore.checking = true;
        assert!(!restore.ready(), "one at a time");
    }
}
