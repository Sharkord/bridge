use super::TITLE;
use crate::icon;
use crate::window;
use crate::{log_info, log_warn};
use arboard::Clipboard;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Tray, TrayMethods};

struct BridgeTray {
    token: String,
    // the clipboard owner has to stay alive for the text to stay pasteable
    clipboard: Option<Clipboard>,
}

impl BridgeTray {
    fn copy_token(&mut self) {
        if self.clipboard.is_none() {
            self.clipboard = Clipboard::new().ok();
        }

        let Some(clipboard) = self.clipboard.as_mut() else {
            log_warn!("could not open the clipboard, the token is in the token file");

            return;
        };

        match clipboard.set_text(self.token.clone()) {
            Ok(()) => log_info!("pairing token copied to the clipboard"),
            Err(error) => log_warn!("could not copy the token: {error}"),
        }
    }
}

impl Tray for BridgeTray {
    fn id(&self) -> String {
        "dev.sharkord.Bridge".into()
    }

    fn title(&self) -> String {
        TITLE.into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let loaded = icon::load();
        let mut data = loaded.rgba;

        // ksni wants ARGB, the png decodes as RGBA
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }

        vec![Icon {
            width: loaded.width as i32,
            height: loaded.height as i32,
            data,
        }]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        window::open();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open".into(),
                activate: Box::new(|_| window::open()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Copy pairing token".into(),
                activate: Box::new(|tray: &mut Self| tray.copy_token()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| window::quit()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn(token: String) {
    tokio::spawn(async move {
        let tray = BridgeTray {
            token,
            clipboard: None,
        };

        // desktops without a tray (some GNOME setups) just do not show it, the
        // bridge works the same
        match tray.spawn().await {
            // the handle keeps the tray alive, so it has to outlive this task
            Ok(handle) => {
                log_info!("system tray icon is up");
                std::mem::forget(handle);
            }
            Err(error) => log_warn!("no system tray available: {error}"),
        }
    });
}
