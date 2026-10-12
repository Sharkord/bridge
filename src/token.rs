use crate::paths;
use std::fs;
use std::io;
use std::path::PathBuf;

fn token_path() -> io::Result<PathBuf> {
    Ok(paths::app_config_dir()?.join("token"))
}

fn generate() -> io::Result<String> {
    let mut bytes = [0u8; 16];

    getrandom::fill(&mut bytes).map_err(|err| io::Error::other(err.to_string()))?;

    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

// one token per machine, created on first run and shown to the user to paste
// into Sharkord once
pub fn load_or_create() -> io::Result<(String, PathBuf)> {
    let path = token_path()?;

    if let Ok(existing) = fs::read_to_string(&path) {
        let existing = existing.trim().to_string();

        if !existing.is_empty() {
            return Ok((existing, path));
        }
    }

    let token = generate()?;

    paths::write_creating_dirs(&path, &token)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok((token, path))
}
