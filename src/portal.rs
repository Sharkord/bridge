use crate::combo::Combo;
use crate::hub::HubMsg;
use crate::icon;
use crate::keymap::Phase;
use crate::{log_debug, log_info, log_warn};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::Session;
use ashpd::AppID;
use futures_util::StreamExt;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const APP_ID: &str = "dev.sharkord.Bridge";

pub fn is_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

// the shortcuts spec names keys the way XKB does, so only keys with a
// well known name get a suggested trigger. the user can always assign any
// other one in the desktop's own settings
fn xkb_key(code: &str) -> Option<String> {
    if let Some(letter) = code.strip_prefix("Key") {
        return Some(letter.to_lowercase());
    }

    if let Some(digit) = code.strip_prefix("Digit") {
        return Some(digit.to_string());
    }

    if let Some(digit) = code.strip_prefix("Numpad") {
        if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() {
            return Some(format!("KP_{digit}"));
        }

        return None;
    }

    let known = match code {
        "Space" => "space",
        "Enter" => "Return",
        "Escape" => "Escape",
        "Tab" => "Tab",
        "Backspace" => "BackSpace",
        "Delete" => "Delete",
        "Insert" => "Insert",
        "Home" => "Home",
        "End" => "End",
        "PageUp" => "Prior",
        "PageDown" => "Next",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        other if other.starts_with('F') && other[1..].parse::<u8>().is_ok() => other,
        _ => return None,
    };

    Some(known.to_string())
}

fn preferred_trigger(combo: &Combo) -> Option<String> {
    let key = xkb_key(&combo.key)?;
    let mut parts: Vec<&str> = Vec::new();

    if combo.mods.ctrl {
        parts.push("CTRL");
    }

    if combo.mods.alt {
        parts.push("ALT");
    }

    if combo.mods.shift {
        parts.push("SHIFT");
    }

    if combo.mods.meta {
        parts.push("LOGO");
    }

    parts.push(&key);

    Some(parts.join("+"))
}

fn desktop_file_path() -> io::Result<PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data directory"))?;

    Ok(dir.join("applications").join(format!("{APP_ID}.desktop")))
}

// the desktop entry points at the icon by absolute path. a bare icon name has
// to be found through the desktop's icon theme, whose cache does not notice a
// file that appeared after it was built, which leaves the taskbar with a
// generic icon
fn icon_path() -> io::Result<PathBuf> {
    let dir = dirs::data_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data directory"))?;

    Ok(dir.join("sharkord-bridge").join("icon.png"))
}

fn ensure_icon_file(path: &Path) -> io::Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == icon::png_bytes()) {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, icon::png_bytes())
}

fn desktop_entry(exe: &Path, icon: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Bridge\nExec={}\nIcon={}\nNoDisplay=true\nTerminal=false\n",
        exe.display(),
        icon.display()
    )
}

// the portal knows an unsandboxed app only through a desktop file named after
// its id, so one is kept pointing at this binary
fn ensure_desktop_file() -> io::Result<()> {
    let path = desktop_file_path()?;
    let exe = std::env::current_exe()?;
    let icon = icon_path()?;

    ensure_icon_file(&icon)?;

    let content = desktop_entry(&exe, &icon);

    if fs::read_to_string(&path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }

    log_info!(
        "writing the desktop file the portal needs: {}",
        path.display()
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)
}

async fn bind<'a>(
    proxy: &GlobalShortcuts<'a>,
    combos: &[Combo],
) -> Result<Session<'a, GlobalShortcuts<'a>>, Box<dyn Error>> {
    let session = proxy.create_session().await?;
    let triggers: Vec<Option<String>> = combos.iter().map(preferred_trigger).collect();
    let shortcuts: Vec<NewShortcut> = combos
        .iter()
        .zip(&triggers)
        .map(|(combo, trigger)| {
            NewShortcut::new(
                combo.canonical(),
                format!("Sharkord hotkey: {}", combo.display()),
            )
            .preferred_trigger(trigger.as_deref())
        })
        .collect();

    for (combo, trigger) in combos.iter().zip(&triggers) {
        log_info!(
            "asking the portal to bind {} (suggested trigger: {})",
            combo.canonical(),
            trigger.as_deref().unwrap_or("none")
        );
    }

    let bound = proxy
        .bind_shortcuts(&session, &shortcuts, None)
        .await?
        .response()?;

    // an empty trigger means the desktop accepted the shortcut but nothing is
    // assigned to it, so it will never fire until the user sets one
    for shortcut in bound.shortcuts() {
        if shortcut.trigger_description().is_empty() {
            log_warn!(
                "the desktop has no key assigned to {}, set one in its shortcut settings",
                shortcut.id()
            );
        } else {
            log_info!(
                "the desktop bound {} to {}",
                shortcut.id(),
                shortcut.trigger_description()
            );
        }
    }

    Ok(session)
}

fn forward(hub: &UnboundedSender<HubMsg>, shortcut_id: &str, phase: Phase) {
    log_debug!("portal reported {shortcut_id} {phase:?}");

    if let Some(combo) = Combo::parse(shortcut_id) {
        let _ = hub.send(HubMsg::Shortcut { combo, phase });
    }
}

async fn run(
    hub: UnboundedSender<HubMsg>,
    mut wanted: UnboundedReceiver<Vec<Combo>>,
) -> Result<(), Box<dyn Error>> {
    ensure_desktop_file()?;
    ashpd::register_host_app(AppID::try_from(APP_ID)?).await?;

    log_info!("registered with the desktop portal as {APP_ID}");

    let proxy = GlobalShortcuts::new().await?;
    let mut activated = Box::pin(proxy.receive_activated().await?);
    let mut deactivated = Box::pin(proxy.receive_deactivated().await?);
    let mut session: Option<Session<'_, GlobalShortcuts<'_>>> = None;
    let mut current: Vec<Combo> = Vec::new();

    loop {
        tokio::select! {
            next = wanted.recv() => {
                let Some(next) = next else {
                    return Ok(());
                };

                if next == current {
                    continue;
                }

                // a fresh session per change: the desktop remembers the keys
                // the user assigned by shortcut id, so nothing is asked twice
                if let Some(old) = session.take() {
                    let _ = old.close().await;
                }

                current = next;

                if current.is_empty() {
                    continue;
                }

                match bind(&proxy, &current).await {
                    Ok(bound) => session = Some(bound),
                    Err(error) => log_warn!("could not bind global shortcuts: {error}"),
                }
            }
            Some(event) = activated.next() => forward(&hub, event.shortcut_id(), Phase::Down),
            Some(event) = deactivated.next() => forward(&hub, event.shortcut_id(), Phase::Up),
        }
    }
}

// returns the channel the hub uses to say which combos should be bound
pub fn spawn(hub: UnboundedSender<HubMsg>) -> UnboundedSender<Vec<Combo>> {
    let (tx, rx) = unbounded_channel();

    tokio::spawn(async move {
        if let Err(error) = run(hub, rx).await {
            log_warn!("global shortcuts portal unavailable: {error}");
        }
    });

    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(raw: &str) -> Option<String> {
        preferred_trigger(&Combo::parse(raw).unwrap())
    }

    #[test]
    fn desktop_entry_refers_to_the_icon_by_absolute_path() {
        let entry = desktop_entry(Path::new("/opt/bridge"), Path::new("/data/icon.png"));

        assert!(entry.contains("Exec=/opt/bridge\n"));
        assert!(entry.contains("Icon=/data/icon.png\n"));
        assert!(entry.contains("Name=Bridge\n"));
    }

    #[test]
    fn suggests_xkb_triggers() {
        assert_eq!(trigger("ctrl+shift+KeyM").as_deref(), Some("CTRL+SHIFT+m"));
        assert_eq!(trigger("meta+Digit5").as_deref(), Some("LOGO+5"));
        assert_eq!(trigger("alt+F9").as_deref(), Some("ALT+F9"));
        assert_eq!(trigger("PageUp").as_deref(), Some("Prior"));
        assert_eq!(trigger("Numpad3").as_deref(), Some("KP_3"));
    }

    #[test]
    fn leaves_unknown_keys_to_the_user() {
        assert_eq!(trigger("NumpadAdd"), None);
        assert_eq!(trigger("IntlBackslash"), None);
    }
}
