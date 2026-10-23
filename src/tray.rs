//! Notification-area (system tray) icon. Clicking it shows the window; its
//! menu shows, pauses recording or quits. Events go to the app over a channel
//! and are handled in `App::logic`, which keeps running while the window is hidden.

use std::sync::mpsc::Sender;

use anyhow::Result;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::i18n::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCmd {
    Show,
    TogglePause,
    Quit,
}

pub struct Tray {
    // Dropping the icon removes it from the tray.
    _icon: TrayIcon,
    pause: MenuItem,
    paused: bool,
}

impl Tray {
    /// `wake` is called after each command is sent, so a hidden window still handles it.
    pub fn new(
        rgba: Vec<u8>,
        size: u32,
        paused: bool,
        tx: Sender<TrayCmd>,
        wake: impl Fn() + Send + Sync + Clone + 'static,
    ) -> Result<Self> {
        // Fixed ids: muda and tray-icon keep only the first event handler they
        // are given, so a menu built again (for a new language, or after the
        // tray was turned off and on) must use the ids that handler knows.
        let show = MenuItem::with_id("show", t!("tray-show"), true, None);
        let pause = MenuItem::with_id("pause", pause_label(paused), true, None);
        let quit = MenuItem::with_id("quit", t!("tray-quit"), true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &pause,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        let ids = [
            (show.id().clone(), TrayCmd::Show),
            (pause.id().clone(), TrayCmd::TogglePause),
            (quit.id().clone(), TrayCmd::Quit),
        ];
        let (menu_tx, menu_wake) = (tx.clone(), wake.clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Some((_, cmd)) = ids.iter().find(|(id, _)| *id == e.id) {
                let _ = menu_tx.send(*cmd);
                menu_wake();
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            let show = matches!(
                e,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            );
            if show {
                let _ = tx.send(TrayCmd::Show);
                wake();
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_icon(tray_icon::Icon::from_rgba(rgba, size, size)?)
            .with_tooltip(t!("app-title"))
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()?;
        Ok(Self {
            _icon: icon,
            pause,
            paused,
        })
    }

    /// Keeps the menu's pause item in step with the recording state.
    pub fn set_paused(&mut self, paused: bool) {
        if paused != self.paused {
            self.paused = paused;
            self.pause.set_text(pause_label(paused));
        }
    }
}

fn pause_label(paused: bool) -> String {
    if paused {
        t!("tray-resume-recording")
    } else {
        t!("tray-pause-recording")
    }
}
