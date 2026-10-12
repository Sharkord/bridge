#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

const TITLE: &str = "Sharkord Bridge";
const OPEN_LABEL: &str = "Open";
const COPY_LABEL: &str = "Copy pairing token";
const QUIT_LABEL: &str = "Quit";

// both platforms copy the same way and say the same thing about it
#[cfg(any(target_os = "linux", windows))]
fn copy_token(clipboard: &mut arboard::Clipboard, token: &str) {
    match clipboard.set_text(token.to_string()) {
        Ok(()) => crate::log_info!("pairing token copied to the clipboard"),
        Err(error) => crate::log_warn!("could not copy the token: {error}"),
    }
}

// sits in the system tray so the bridge is visible and can be quit, and so the
// pairing token can be copied without a console
pub fn spawn(token: String) {
    #[cfg(target_os = "linux")]
    linux::spawn(token);

    #[cfg(windows)]
    windows::spawn(token);

    #[cfg(not(any(target_os = "linux", windows)))]
    let _ = token;
}
