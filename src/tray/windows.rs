use super::{COPY_LABEL, OPEN_LABEL, QUIT_LABEL, TITLE};
use crate::icon;
use crate::window;
use crate::{log_info, log_warn};
use arboard::Clipboard;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

fn copy_token(token: &str) {
    match Clipboard::new() {
        Ok(mut clipboard) => super::copy_token(&mut clipboard, token),
        Err(error) => log_warn!("could not open the clipboard: {error}"),
    }
}

// a tray icon lives on the thread that pumps its window messages, so it gets
// its own instead of sharing the async runtime
pub fn spawn(token: String) {
    std::thread::spawn(move || {
        let open = MenuItem::new(OPEN_LABEL, true, None);
        let copy = MenuItem::new(COPY_LABEL, true, None);
        let quit = MenuItem::new(QUIT_LABEL, true, None);
        let menu = Menu::new();

        if menu.append_items(&[&open, &copy, &quit]).is_err() {
            return;
        }

        let loaded = icon::load();

        let Ok(icon) = Icon::from_rgba(loaded.rgba, loaded.width, loaded.height) else {
            return;
        };

        let open_id = open.id().clone();
        let copy_id = copy.id().clone();
        let quit_id = quit.id().clone();

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == open_id {
                window::open();
            } else if event.id == copy_id {
                copy_token(&token);
            } else if event.id == quit_id {
                window::quit();
            }
        }));

        // a left click opens the window, the menu is for the right button
        TrayIconEvent::set_event_handler(Some(|event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::open();
            }
        }));

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip(TITLE)
            .with_icon(icon)
            .build();

        if let Err(error) = tray {
            log_warn!("could not create the tray icon: {error}");

            return;
        }

        log_info!("system tray icon is up");

        // SAFETY: plain Win32 message loop on the thread that owns the tray
        unsafe {
            let mut message: MSG = std::mem::zeroed();

            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    });
}
