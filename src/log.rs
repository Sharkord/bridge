use std::fmt::Arguments;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    // every key the listener sees, so only on request
    Trace,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN ",
            Level::Info => "INFO ",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    fn from_u8(value: u8) -> Level {
        match value {
            0 => Level::Error,
            1 => Level::Warn,
            2 => Level::Info,
            3 => Level::Debug,
            _ => Level::Trace,
        }
    }

    pub fn parse(raw: &str) -> Option<Level> {
        match raw.trim().to_lowercase().as_str() {
            "error" => Some(Level::Error),
            "warn" => Some(Level::Warn),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            "trace" => Some(Level::Trace),
            _ => None,
        }
    }
}

static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);
static FILE: OnceLock<Mutex<File>> = OnceLock::new();

pub fn level() -> Level {
    Level::from_u8(LEVEL.load(Ordering::Relaxed))
}

pub fn log_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("sharkord-bridge").join("bridge.log"))
}

// UTC, because local time would need a timezone database for one log line
fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() % 86_400;

    format!(
        "{:02}:{:02}:{:02}.{:03}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60,
        now.subsec_millis()
    )
}

// SHARKORD_BRIDGE_LOG picks the level. the file matters on Windows, where a
// release build has no console to print to. it is replaced on every start
pub fn init() -> Option<PathBuf> {
    if let Some(level) = std::env::var("SHARKORD_BRIDGE_LOG")
        .ok()
        .and_then(|raw| Level::parse(&raw))
    {
        LEVEL.store(level as u8, Ordering::Relaxed);
    }

    let path = log_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    let file = File::create(&path).ok()?;

    let _ = FILE.set(Mutex::new(file));

    Some(path)
}

pub fn write(level: Level, args: Arguments) {
    if level > self::level() {
        return;
    }

    let line = format!("{} {} {}\n", timestamp(), level.label(), args);

    let _ = std::io::stderr().write_all(line.as_bytes());

    if let Some(file) = FILE.get() {
        if let Ok(mut file) = file.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::log::write($crate::log::Level::Error, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::log::write($crate::log::Level::Warn, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::log::write($crate::log::Level::Info, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => { $crate::log::write($crate::log::Level::Debug, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => { $crate::log::write($crate::log::Level::Trace, format_args!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels_case_insensitively() {
        assert_eq!(Level::parse("DEBUG"), Some(Level::Debug));
        assert_eq!(Level::parse(" trace "), Some(Level::Trace));
        assert_eq!(Level::parse("loud"), None);
    }

    #[test]
    fn levels_order_from_quiet_to_noisy() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Debug < Level::Trace);
    }

    #[test]
    fn timestamp_has_the_expected_shape() {
        let stamp = timestamp();

        assert_eq!(stamp.len(), 13);
        assert!(stamp.ends_with('Z'));
    }
}
