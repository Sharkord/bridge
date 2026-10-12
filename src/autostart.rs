use std::io;

// starting with the user's session, per user and without admin rights: an XDG
// autostart entry on Linux and a Run registry value on Windows

#[cfg(target_os = "linux")]
mod platform {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    const FILE_NAME: &str = "dev.sharkord.Bridge.desktop";

    // the desktop entry spec wants a path with special characters in quotes
    fn quote_exec(path: &Path) -> String {
        let mut quoted = String::from("\"");

        for character in path.display().to_string().chars() {
            match character {
                '"' | '`' | '$' | '\\' => {
                    quoted.push('\\');
                    quoted.push(character);
                }
                '%' => quoted.push_str("%%"),
                other => quoted.push(other),
            }
        }

        quoted.push('"');

        quoted
    }

    fn entry(exe: &Path) -> String {
        format!(
            "[Desktop Entry]\nType=Application\nName=Bridge\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            quote_exec(exe)
        )
    }

    fn entry_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("autostart").join(FILE_NAME))
    }

    // only an entry that starts this exact executable counts as enabled, so one
    // left behind by a moved or older copy shows as off and turning it on fixes it
    pub(super) fn is_enabled_at(path: &Path, exe: &Path) -> bool {
        fs::read_to_string(path).is_ok_and(|content| content == entry(exe))
    }

    pub(super) fn set_enabled_at(path: &Path, exe: &Path, enabled: bool) -> io::Result<()> {
        if !enabled {
            return match fs::remove_file(path) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            };
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, entry(exe))
    }

    pub fn is_enabled() -> bool {
        let (Ok(exe), Some(path)) = (std::env::current_exe(), entry_path()) else {
            return false;
        };

        is_enabled_at(&path, &exe)
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let exe = std::env::current_exe()?;
        let path = entry_path()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory"))?;

        set_enabled_at(&path, &exe, enabled)
    }

    #[cfg(test)]
    pub(super) fn quote(path: &Path) -> String {
        quote_exec(path)
    }
}

#[cfg(windows)]
mod platform {
    use std::io;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "SharkordBridge";

    fn command() -> io::Result<String> {
        let exe = std::env::current_exe()?;

        Ok(format!("\"{}\"", exe.display()))
    }

    pub fn is_enabled() -> bool {
        let Ok(expected) = command() else {
            return false;
        };

        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_READ)
            .and_then(|key| key.get_value::<String, _>(VALUE_NAME))
            .is_ok_and(|value| value == expected)
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_WRITE)?;

        if enabled {
            return key.set_value(VALUE_NAME, &command()?);
        }

        match key.delete_value(VALUE_NAME) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use std::io;

    pub fn is_enabled() -> bool {
        false
    }

    pub fn set_enabled(_enabled: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "starting with the session is not supported here",
        ))
    }
}

pub fn is_enabled() -> bool {
    platform::is_enabled()
}

pub fn set_enabled(enabled: bool) -> io::Result<()> {
    platform::set_enabled(enabled)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::platform::{is_enabled_at, quote, set_enabled_at};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("sharkord-bridge-autostart-{}", std::process::id()))
            .join(name)
    }

    #[test]
    fn enabling_writes_an_entry_that_starts_this_executable() {
        let path = temp_file("a/bridge.desktop");
        let exe = Path::new("/opt/bridge/sharkord-bridge");

        assert!(!is_enabled_at(&path, exe));

        set_enabled_at(&path, exe, true).unwrap();

        assert!(is_enabled_at(&path, exe));

        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("Exec=\"/opt/bridge/sharkord-bridge\"\n"));
        assert!(content.contains("Terminal=false\n"));

        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn disabling_removes_it_and_is_fine_when_there_is_nothing() {
        let path = temp_file("b/bridge.desktop");
        let exe = Path::new("/opt/bridge/sharkord-bridge");

        set_enabled_at(&path, exe, true).unwrap();
        set_enabled_at(&path, exe, false).unwrap();

        assert!(!path.exists());
        assert!(set_enabled_at(&path, exe, false).is_ok());

        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn an_entry_for_another_executable_does_not_count() {
        let path = temp_file("c/bridge.desktop");

        set_enabled_at(&path, Path::new("/old/sharkord-bridge"), true).unwrap();

        assert!(!is_enabled_at(&path, Path::new("/new/sharkord-bridge")));

        set_enabled_at(&path, Path::new("/new/sharkord-bridge"), true).unwrap();

        assert!(is_enabled_at(&path, Path::new("/new/sharkord-bridge")));

        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn quotes_paths_with_spaces_and_special_characters() {
        assert_eq!(quote(Path::new("/my apps/bridge")), "\"/my apps/bridge\"");
        assert_eq!(quote(Path::new("/a$b\"c")), "\"/a\\$b\\\"c\"");
        assert_eq!(quote(Path::new("/100%")), "\"/100%%\"");
    }
}
