use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const APP_DIR: &str = "sharkord-bridge";

fn missing(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("no {what} directory"))
}

// the user's own directories, where other programs look for what we write
pub fn config_dir() -> io::Result<PathBuf> {
    dirs::config_dir().ok_or_else(|| missing("config"))
}

pub fn data_dir() -> io::Result<PathBuf> {
    dirs::data_dir().ok_or_else(|| missing("data"))
}

// where the bridge keeps its own files
pub fn app_config_dir() -> io::Result<PathBuf> {
    Ok(config_dir()?.join(APP_DIR))
}

pub fn app_data_dir() -> io::Result<PathBuf> {
    Ok(data_dir()?.join(APP_DIR))
}

pub fn write_creating_dirs(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, contents)
}
