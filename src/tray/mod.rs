#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

pub const TITLE: &str = "Sharkord Bridge";
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
