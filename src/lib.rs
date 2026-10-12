pub mod autostart;
pub mod combo;
pub mod gui;
pub mod hub;
pub mod icon;
pub mod input;
pub mod keymap;
pub mod log;
#[cfg(target_os = "linux")]
pub mod portal;
pub mod server;
pub mod status;
pub mod token;
pub mod tray;
pub mod window;

pub const PORT: u16 = 47821;
pub const PROTOCOL_VERSION: u32 = 1;
