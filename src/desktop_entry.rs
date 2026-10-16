use crate::{icon, log_info, paths, APP_ID};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

// the entries the bridge writes (autostart, and the one the desktop portal
// needs) are the same program and differ only in their extra keys
pub fn render(exe: &Path, extra: &[&str]) -> String {
    let mut lines = vec![
        "[Desktop Entry]".to_string(),
        "Type=Application".to_string(),
        "Name=Bridge".to_string(),
        format!("Exec={}", quote_exec(exe)),
    ];

    lines.extend(extra.iter().map(|line| line.to_string()));

    format!("{}\n", lines.join("\n"))
}

fn desktop_file_path() -> io::Result<PathBuf> {
    Ok(paths::data_dir()?
        .join("applications")
        .join(format!("{APP_ID}.desktop")))
}

// the entry points at the icon by absolute path. a bare icon name has to be
// found through the desktop's icon theme, whose cache does not notice a file
// that appeared after it was built, which leaves the taskbar with a generic icon
fn icon_path() -> io::Result<PathBuf> {
    Ok(paths::app_data_dir()?.join("icon.png"))
}

fn ensure_icon_file(path: &Path) -> io::Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == icon::png_bytes()) {
        return Ok(());
    }

    paths::write_creating_dirs(path, icon::png_bytes())
}

fn window_entry(exe: &Path, icon: &Path) -> String {
    let icon_line = format!("Icon={}", icon.display());

    render(exe, &[&icon_line, "NoDisplay=true", "Terminal=false"])
}

// Wayland has no window icon of its own: the taskbar finds the app through a
// desktop entry named after the window's app id, so one is kept pointing at
// this binary and its icon
pub fn install() -> io::Result<()> {
    let path = desktop_file_path()?;
    let exe = std::env::current_exe()?;
    let icon = icon_path()?;

    ensure_icon_file(&icon)?;

    let content = window_entry(&exe, &icon);

    if fs::read_to_string(&path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }

    log_info!("writing the desktop entry: {}", path.display());

    paths::write_creating_dirs(&path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_entry_refers_to_the_icon_by_absolute_path() {
        let entry = window_entry(Path::new("/opt/bridge"), Path::new("/data/icon.png"));

        assert!(entry.contains("Exec=\"/opt/bridge\"\n"));
        assert!(entry.contains("Icon=/data/icon.png\n"));
        assert!(entry.contains("Name=Bridge\n"));
    }

    #[test]
    fn renders_the_common_keys_then_the_extra_ones() {
        let entry = render(Path::new("/opt/bridge"), &["Terminal=false"]);

        assert_eq!(
            entry,
            "[Desktop Entry]\nType=Application\nName=Bridge\nExec=\"/opt/bridge\"\nTerminal=false\n"
        );
    }

    #[test]
    fn quotes_paths_with_spaces_and_special_characters() {
        assert_eq!(
            quote_exec(Path::new("/my apps/bridge")),
            "\"/my apps/bridge\""
        );
        assert_eq!(quote_exec(Path::new("/a$b\"c")), "\"/a\\$b\\\"c\"");
        assert_eq!(quote_exec(Path::new("/100%")), "\"/100%%\"");
    }
}
