use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerStatus {
    pub name: String,
    pub id: String,
    pub hotkeys: usize,
}

pub fn status_path() -> Option<PathBuf> {
    paths::app_config_dir()
        .ok()
        .map(|dir| dir.join("status.json"))
}

// the window is a separate short lived process, so what it shows travels
// through this file instead of a connection to the running bridge. written to a
// temporary file first so the window never reads half of it
pub fn write_to(path: &Path, servers: &[ServerStatus]) -> io::Result<()> {
    let temporary = path.with_extension("json.tmp");

    fs::write(&temporary, serde_json::to_vec(servers)?)?;
    fs::rename(temporary, path)
}

pub fn read_from(path: &Path) -> Vec<ServerStatus> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn write(servers: &[ServerStatus]) {
    if let Some(path) = status_path() {
        let _ = write_to(&path, servers);
    }
}

pub fn read() -> Vec<ServerStatus> {
    status_path()
        .map(|path| read_from(&path))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sharkord-bridge-test-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn round_trips_through_the_file() {
        let path = temp_file("status.json");
        let servers = vec![ServerStatus {
            name: "My Guild".to_string(),
            id: "abc".to_string(),
            hotkeys: 3,
        }];

        write_to(&path, &servers).unwrap();

        assert_eq!(read_from(&path), servers);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn a_missing_or_broken_file_reads_as_empty() {
        assert!(read_from(&temp_file("missing.json")).is_empty());

        let broken = temp_file("broken.json");

        fs::write(&broken, "not json").unwrap();

        assert!(read_from(&broken).is_empty());

        let _ = fs::remove_file(broken);
    }
}
