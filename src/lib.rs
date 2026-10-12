pub mod autostart;
pub mod combo;
#[cfg(target_os = "linux")]
pub mod desktop_entry;
pub mod gui;
pub mod hub;
pub mod icon;
pub mod input;
pub mod keymap;
pub mod log;
pub mod paths;
#[cfg(target_os = "linux")]
pub mod portal;
pub mod protocol;
pub mod server;
pub mod status;
pub mod token;
pub mod tray;
pub mod window;

// the id the desktop knows the bridge by: its window, tray item, desktop entry
// and portal registration all have to agree on it
pub const APP_ID: &str = "dev.sharkord.Bridge";
pub const PORT: u16 = 47821;
pub const PROTOCOL_VERSION: u32 = 1;
