#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Mods {
    // extra modifiers held on top of the required ones still count, so a
    // push to talk key keeps working while the player holds shift to sprint
    pub fn contains(&self, required: &Mods) -> bool {
        (!required.ctrl || self.ctrl)
            && (!required.alt || self.alt)
            && (!required.shift || self.shift)
            && (!required.meta || self.meta)
    }

    // the set modifiers, in ctrl, alt, shift, meta order. everything that writes
    // a combo needs these four with its own spelling
    pub fn names(&self, labels: [&'static str; 4]) -> Vec<&'static str> {
        let flags = [self.ctrl, self.alt, self.shift, self.meta];

        labels
            .into_iter()
            .zip(flags)
            .filter(|(_, set)| *set)
            .map(|(label, _)| label)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combo {
    pub mods: Mods,
    pub key: String,
}

impl Combo {
    // the client sends `ctrl+alt+shift+meta+<KeyboardEvent.code>`
    pub fn parse(raw: &str) -> Option<Combo> {
        let mut mods = Mods::default();
        let mut key: Option<String> = None;

        for part in raw.split('+') {
            match part {
                "ctrl" => mods.ctrl = true,
                "alt" => mods.alt = true,
                "shift" => mods.shift = true,
                "meta" => mods.meta = true,
                "" => return None,
                other => {
                    if key.is_some() {
                        return None;
                    }

                    key = Some(other.to_string());
                }
            }
        }

        key.map(|key| Combo { mods, key })
    }

    // for people to read, such as in the desktop's shortcut list
    pub fn display(&self) -> String {
        let mut parts: Vec<String> = self
            .mods
            .names(["Ctrl", "Alt", "Shift", "Meta"])
            .into_iter()
            .map(String::from)
            .collect();

        let key = if let Some(letter) = self.key.strip_prefix("Key") {
            letter.to_string()
        } else if let Some(digit) = self.key.strip_prefix("Digit") {
            digit.to_string()
        } else {
            self.key.clone()
        };

        parts.push(key);

        parts.join(" + ")
    }

    // the exact form the client sends, so it can stand in as an id
    pub fn canonical(&self) -> String {
        let mut parts = self.mods.names(["ctrl", "alt", "shift", "meta"]);

        parts.push(&self.key);

        parts.join("+")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifiers_and_key() {
        let combo = Combo::parse("ctrl+shift+KeyM").unwrap();

        assert_eq!(combo.key, "KeyM");
        assert!(combo.mods.ctrl && combo.mods.shift);
        assert!(!combo.mods.alt && !combo.mods.meta);
    }

    #[test]
    fn parses_a_lone_key() {
        let combo = Combo::parse("F9").unwrap();

        assert_eq!(combo.key, "F9");
        assert_eq!(combo.mods, Mods::default());
    }

    #[test]
    fn rejects_malformed_combos() {
        assert!(Combo::parse("").is_none());
        assert!(Combo::parse("ctrl+").is_none());
        assert!(Combo::parse("ctrl+shift").is_none());
        assert!(Combo::parse("KeyA+KeyB").is_none());
    }

    #[test]
    fn displays_for_people() {
        assert_eq!(
            Combo::parse("ctrl+alt+KeyL").unwrap().display(),
            "Ctrl + Alt + L"
        );
        assert_eq!(Combo::parse("shift+Digit5").unwrap().display(), "Shift + 5");
        assert_eq!(Combo::parse("F9").unwrap().display(), "F9");
    }

    #[test]
    fn canonical_form_round_trips() {
        let combo = Combo::parse("ctrl+alt+shift+meta+F5").unwrap();

        assert_eq!(combo.canonical(), "ctrl+alt+shift+meta+F5");
        assert_eq!(Combo::parse(&combo.canonical()), Some(combo));
    }

    #[test]
    fn extra_held_modifiers_still_match() {
        let held = Mods {
            ctrl: true,
            shift: true,
            ..Mods::default()
        };
        let required = Mods {
            ctrl: true,
            ..Mods::default()
        };

        assert!(held.contains(&required));
        assert!(!required.contains(&held));
    }
}
