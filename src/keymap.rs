use rdev::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub token: &'static str,
    pub phase: Phase,
}

pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

pub fn modifier_of(token: &str) -> Option<Modifier> {
    match token {
        "ControlLeft" | "ControlRight" => Some(Modifier::Ctrl),
        "AltLeft" | "AltRight" => Some(Modifier::Alt),
        "ShiftLeft" | "ShiftRight" => Some(Modifier::Shift),
        "MetaLeft" | "MetaRight" => Some(Modifier::Meta),
        _ => None,
    }
}

// the names the browser calls KeyboardEvent.code, which is what combos hold
pub fn key_token(key: Key) -> Option<&'static str> {
    let token = match key {
        Key::KeyA => "KeyA",
        Key::KeyB => "KeyB",
        Key::KeyC => "KeyC",
        Key::KeyD => "KeyD",
        Key::KeyE => "KeyE",
        Key::KeyF => "KeyF",
        Key::KeyG => "KeyG",
        Key::KeyH => "KeyH",
        Key::KeyI => "KeyI",
        Key::KeyJ => "KeyJ",
        Key::KeyK => "KeyK",
        Key::KeyL => "KeyL",
        Key::KeyM => "KeyM",
        Key::KeyN => "KeyN",
        Key::KeyO => "KeyO",
        Key::KeyP => "KeyP",
        Key::KeyQ => "KeyQ",
        Key::KeyR => "KeyR",
        Key::KeyS => "KeyS",
        Key::KeyT => "KeyT",
        Key::KeyU => "KeyU",
        Key::KeyV => "KeyV",
        Key::KeyW => "KeyW",
        Key::KeyX => "KeyX",
        Key::KeyY => "KeyY",
        Key::KeyZ => "KeyZ",
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Kp0 => "Numpad0",
        Key::Kp1 => "Numpad1",
        Key::Kp2 => "Numpad2",
        Key::Kp3 => "Numpad3",
        Key::Kp4 => "Numpad4",
        Key::Kp5 => "Numpad5",
        Key::Kp6 => "Numpad6",
        Key::Kp7 => "Numpad7",
        Key::Kp8 => "Numpad8",
        Key::Kp9 => "Numpad9",
        Key::KpReturn => "NumpadEnter",
        Key::KpMinus => "NumpadSubtract",
        Key::KpPlus => "NumpadAdd",
        Key::KpMultiply => "NumpadMultiply",
        Key::KpDivide => "NumpadDivide",
        Key::KpDelete => "NumpadDecimal",
        Key::ControlLeft => "ControlLeft",
        Key::ControlRight => "ControlRight",
        Key::ShiftLeft => "ShiftLeft",
        Key::ShiftRight => "ShiftRight",
        Key::Alt => "AltLeft",
        Key::AltGr => "AltRight",
        Key::MetaLeft => "MetaLeft",
        Key::MetaRight => "MetaRight",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::Delete => "Delete",
        Key::DownArrow => "ArrowDown",
        Key::UpArrow => "ArrowUp",
        Key::LeftArrow => "ArrowLeft",
        Key::RightArrow => "ArrowRight",
        Key::End => "End",
        Key::Home => "Home",
        Key::PageDown => "PageDown",
        Key::PageUp => "PageUp",
        Key::Escape => "Escape",
        Key::Return => "Enter",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::Insert => "Insert",
        Key::PrintScreen => "PrintScreen",
        Key::ScrollLock => "ScrollLock",
        Key::Pause => "Pause",
        Key::NumLock => "NumLock",
        Key::BackQuote => "Backquote",
        Key::Minus => "Minus",
        Key::Equal => "Equal",
        Key::LeftBracket => "BracketLeft",
        Key::RightBracket => "BracketRight",
        Key::SemiColon => "Semicolon",
        Key::Quote => "Quote",
        Key::BackSlash => "Backslash",
        Key::IntlBackslash => "IntlBackslash",
        Key::Comma => "Comma",
        Key::Dot => "Period",
        Key::Slash => "Slash",
        _ => return None,
    };

    Some(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys_to_browser_codes() {
        assert_eq!(key_token(Key::KeyM), Some("KeyM"));
        assert_eq!(key_token(Key::Num5), Some("Digit5"));
        assert_eq!(key_token(Key::Return), Some("Enter"));
        assert_eq!(key_token(Key::Unknown(999)), None);
    }

    #[test]
    fn classifies_modifiers() {
        assert!(matches!(modifier_of("ControlRight"), Some(Modifier::Ctrl)));
        assert!(matches!(modifier_of("AltRight"), Some(Modifier::Alt)));
        assert!(modifier_of("KeyM").is_none());
    }
}
