use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;

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
