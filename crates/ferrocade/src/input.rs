//! Keyboard input: the key table, held-key state, and event filtering.
//!
//! Keys are physical positions, named after the W3C UI Events `code` values (`"KeyW"`,
//! `"ArrowLeft"`). winit's `KeyCode` and the browser's `KeyboardEvent.code` use the same
//! names, so both platforms feed this module the same keys. The platform layer pushes raw
//! events; the game loop drains them once per frame, before the fixed-timestep updates.

macro_rules! keys {
    ($($variant:ident = $code:literal, $w3c:literal, $py:literal;)*) => {
        /// A physical key position. The discriminant is the stable code Python sees.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum Key {
            $($variant = $code,)*
        }

        /// Every key: (key, W3C `code` name, Python constant name in `ferrocade.key`).
        pub const KEYS: &[(Key, &str, &str)] = &[$((Key::$variant, $w3c, $py),)*];
    };
}

keys! {
    A = 1, "KeyA", "A"; B = 2, "KeyB", "B"; C = 3, "KeyC", "C"; D = 4, "KeyD", "D";
    E = 5, "KeyE", "E"; F = 6, "KeyF", "F"; G = 7, "KeyG", "G"; H = 8, "KeyH", "H";
    I = 9, "KeyI", "I"; J = 10, "KeyJ", "J"; K = 11, "KeyK", "K"; L = 12, "KeyL", "L";
    M = 13, "KeyM", "M"; N = 14, "KeyN", "N"; O = 15, "KeyO", "O"; P = 16, "KeyP", "P";
    Q = 17, "KeyQ", "Q"; R = 18, "KeyR", "R"; S = 19, "KeyS", "S"; T = 20, "KeyT", "T";
    U = 21, "KeyU", "U"; V = 22, "KeyV", "V"; W = 23, "KeyW", "W"; X = 24, "KeyX", "X";
    Y = 25, "KeyY", "Y"; Z = 26, "KeyZ", "Z";
    Digit0 = 30, "Digit0", "KEY_0"; Digit1 = 31, "Digit1", "KEY_1";
    Digit2 = 32, "Digit2", "KEY_2"; Digit3 = 33, "Digit3", "KEY_3";
    Digit4 = 34, "Digit4", "KEY_4"; Digit5 = 35, "Digit5", "KEY_5";
    Digit6 = 36, "Digit6", "KEY_6"; Digit7 = 37, "Digit7", "KEY_7";
    Digit8 = 38, "Digit8", "KEY_8"; Digit9 = 39, "Digit9", "KEY_9";
    F1 = 41, "F1", "F1"; F2 = 42, "F2", "F2"; F3 = 43, "F3", "F3"; F4 = 44, "F4", "F4";
    F5 = 45, "F5", "F5"; F6 = 46, "F6", "F6"; F7 = 47, "F7", "F7"; F8 = 48, "F8", "F8";
    F9 = 49, "F9", "F9"; F10 = 50, "F10", "F10"; F11 = 51, "F11", "F11"; F12 = 52, "F12", "F12";
    Left = 55, "ArrowLeft", "LEFT"; Right = 56, "ArrowRight", "RIGHT";
    Up = 57, "ArrowUp", "UP"; Down = 58, "ArrowDown", "DOWN";
    Space = 60, "Space", "SPACE"; Enter = 61, "Enter", "ENTER";
    Escape = 62, "Escape", "ESCAPE"; Tab = 63, "Tab", "TAB";
    Backspace = 64, "Backspace", "BACKSPACE"; Insert = 65, "Insert", "INSERT";
    Delete = 66, "Delete", "DELETE"; Home = 67, "Home", "HOME"; End = 68, "End", "END";
    PageUp = 69, "PageUp", "PAGE_UP"; PageDown = 70, "PageDown", "PAGE_DOWN";
    CapsLock = 71, "CapsLock", "CAPS_LOCK";
    LShift = 75, "ShiftLeft", "LSHIFT"; RShift = 76, "ShiftRight", "RSHIFT";
    LCtrl = 77, "ControlLeft", "LCTRL"; RCtrl = 78, "ControlRight", "RCTRL";
    LAlt = 79, "AltLeft", "LALT"; RAlt = 80, "AltRight", "RALT";
    LSystem = 81, "MetaLeft", "LSYSTEM"; RSystem = 82, "MetaRight", "RSYSTEM";
    Num0 = 85, "Numpad0", "NUM_0"; Num1 = 86, "Numpad1", "NUM_1";
    Num2 = 87, "Numpad2", "NUM_2"; Num3 = 88, "Numpad3", "NUM_3";
    Num4 = 89, "Numpad4", "NUM_4"; Num5 = 90, "Numpad5", "NUM_5";
    Num6 = 91, "Numpad6", "NUM_6"; Num7 = 92, "Numpad7", "NUM_7";
    Num8 = 93, "Numpad8", "NUM_8"; Num9 = 94, "Numpad9", "NUM_9";
    NumAdd = 95, "NumpadAdd", "NUM_ADD"; NumSubtract = 96, "NumpadSubtract", "NUM_SUBTRACT";
    NumMultiply = 97, "NumpadMultiply", "NUM_MULTIPLY";
    NumDivide = 98, "NumpadDivide", "NUM_DIVIDE";
    NumDecimal = 99, "NumpadDecimal", "NUM_DECIMAL"; NumEnter = 100, "NumpadEnter", "NUM_ENTER";
    NumLock = 101, "NumLock", "NUM_LOCK";
    Minus = 105, "Minus", "MINUS"; Equal = 106, "Equal", "EQUAL";
    BracketLeft = 107, "BracketLeft", "BRACKET_LEFT";
    BracketRight = 108, "BracketRight", "BRACKET_RIGHT";
    Backslash = 109, "Backslash", "BACKSLASH"; Semicolon = 110, "Semicolon", "SEMICOLON";
    Apostrophe = 111, "Quote", "APOSTROPHE"; Grave = 112, "Backquote", "GRAVE";
    Comma = 113, "Comma", "COMMA"; Period = 114, "Period", "PERIOD"; Slash = 115, "Slash", "SLASH";
}

pub const MOD_SHIFT: u8 = 1;
pub const MOD_CTRL: u8 = 2;
pub const MOD_ALT: u8 = 4;
pub const MOD_SYSTEM: u8 = 8;

/// Modifier flags with the keys that set them (either side counts).
const MODIFIER_KEYS: [(u8, Key, Key); 4] = [
    (MOD_SHIFT, Key::LShift, Key::RShift),
    (MOD_CTRL, Key::LCtrl, Key::RCtrl),
    (MOD_ALT, Key::LAlt, Key::RAlt),
    (MOD_SYSTEM, Key::LSystem, Key::RSystem),
];

impl Key {
    /// The stable code Python sees (`ferrocade.key.LEFT`, ...).
    pub fn code(self) -> u16 {
        self as u16
    }

    pub fn from_code(code: u16) -> Option<Key> {
        KEYS.iter().find(|(k, _, _)| k.code() == code).map(|e| e.0)
    }

    /// Look up a W3C UI Events `code` name (the browser's `KeyboardEvent.code`).
    pub fn from_w3c(name: &str) -> Option<Key> {
        KEYS.iter().find(|(_, w, _)| *w == name).map(|e| e.0)
    }

    pub fn w3c_name(self) -> &'static str {
        KEYS.iter().find(|(k, _, _)| *k == self).map_or("", |e| e.1)
    }

    pub fn py_name(self) -> &'static str {
        KEYS.iter().find(|(k, _, _)| *k == self).map_or("", |e| e.2)
    }

    fn bit(self) -> u128 {
        1u128 << self.code()
    }
}

/// A key event as reported by the platform, before filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawKeyEvent {
    pub key: Key,
    pub pressed: bool,
    /// The operating system's auto-repeat for a held key.
    pub repeat: bool,
}

/// A key event delivered to the game: one per press and one per release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub pressed: bool,
    /// Modifier flags held after this event (`MOD_SHIFT | ...`).
    pub modifiers: u8,
}

/// Queued raw key events plus the set of held keys.
#[derive(Debug, Default)]
pub struct Keyboard {
    queue: Vec<RawKeyEvent>,
    held: u128,
}

impl Keyboard {
    pub fn push(&mut self, event: RawKeyEvent) {
        self.queue.push(event);
    }

    /// Queue a release for every held key (the window lost focus).
    pub fn release_all(&mut self) {
        let mut pending = self.held;
        for event in &self.queue {
            // Keys pressed in the queue but not yet applied are released too.
            if event.pressed && !event.repeat {
                pending |= event.key.bit();
            }
        }
        for (key, _, _) in KEYS {
            if pending & key.bit() != 0 {
                self.queue.push(RawKeyEvent {
                    key: *key,
                    pressed: false,
                    repeat: false,
                });
            }
        }
    }

    /// Apply the queued events in order and return the ones the game should see.
    pub fn drain(&mut self) -> Vec<KeyEvent> {
        let mut out = Vec::new();
        for raw in std::mem::take(&mut self.queue) {
            let held = self.held & raw.key.bit() != 0;
            match (raw.pressed, held) {
                (true, false) if !raw.repeat => self.held |= raw.key.bit(),
                (false, true) => self.held &= !raw.key.bit(),
                // Repeats, duplicate presses, and releases of keys that aren't held.
                _ => continue,
            }
            out.push(KeyEvent {
                key: raw.key,
                pressed: raw.pressed,
                modifiers: self.modifiers(),
            });
        }
        out
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.held & key.bit() != 0
    }

    pub fn modifiers(&self) -> u8 {
        MODIFIER_KEYS
            .iter()
            .filter(|(_, l, r)| self.is_held(*l) || self.is_held(*r))
            .fold(0, |acc, (flag, _, _)| acc | flag)
    }

    /// Forget all state (a new run starts).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn press(key: Key) -> RawKeyEvent {
        RawKeyEvent {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: Key) -> RawKeyEvent {
        RawKeyEvent {
            key,
            pressed: false,
            repeat: false,
        }
    }

    fn summary(events: &[KeyEvent]) -> Vec<(Key, bool, u8)> {
        events
            .iter()
            .map(|e| (e.key, e.pressed, e.modifiers))
            .collect()
    }

    #[test]
    fn table_is_unique_and_round_trips() {
        let codes: HashSet<_> = KEYS.iter().map(|e| e.0.code()).collect();
        let w3c: HashSet<_> = KEYS.iter().map(|e| e.1).collect();
        let py: HashSet<_> = KEYS.iter().map(|e| e.2).collect();
        assert_eq!(codes.len(), KEYS.len());
        assert_eq!(w3c.len(), KEYS.len());
        assert_eq!(py.len(), KEYS.len());
        for (key, name, py_name) in KEYS {
            assert!(key.code() > 0 && key.code() < 128, "{key:?}");
            assert_eq!(Key::from_code(key.code()), Some(*key));
            assert_eq!(Key::from_w3c(name), Some(*key));
            assert_eq!(key.w3c_name(), *name);
            assert_eq!(key.py_name(), *py_name);
        }
        assert_eq!(Key::from_code(0), None);
        assert_eq!(Key::from_w3c("MediaPlayPause"), None);
    }

    #[test]
    fn press_and_release_once_each() {
        let mut kb = Keyboard::default();
        kb.push(press(Key::Space));
        assert!(!kb.is_held(Key::Space), "not applied until drained");
        let events = kb.drain();
        assert_eq!(summary(&events), [(Key::Space, true, 0)]);
        assert!(kb.is_held(Key::Space));
        kb.push(release(Key::Space));
        assert_eq!(summary(&kb.drain()), [(Key::Space, false, 0)]);
        assert!(!kb.is_held(Key::Space));
    }

    #[test]
    fn repeats_and_duplicates_are_ignored() {
        let mut kb = Keyboard::default();
        kb.push(press(Key::A));
        for _ in 0..5 {
            kb.push(RawKeyEvent {
                repeat: true,
                ..press(Key::A)
            });
        }
        kb.push(press(Key::A)); // duplicate without the repeat flag
        kb.push(release(Key::A));
        kb.push(release(Key::A)); // release of a key that isn't held
        kb.push(RawKeyEvent {
            repeat: true,
            ..press(Key::B)
        }); // a repeat for a key never pressed
        assert_eq!(
            summary(&kb.drain()),
            [(Key::A, true, 0), (Key::A, false, 0)]
        );
    }

    #[test]
    fn order_is_preserved() {
        let mut kb = Keyboard::default();
        kb.push(press(Key::A));
        kb.push(press(Key::B));
        kb.push(release(Key::A));
        assert_eq!(
            summary(&kb.drain()),
            [(Key::A, true, 0), (Key::B, true, 0), (Key::A, false, 0)]
        );
    }

    #[test]
    fn modifiers_reflect_held_keys_after_the_event() {
        let mut kb = Keyboard::default();
        kb.push(press(Key::LShift));
        kb.push(press(Key::A));
        kb.push(release(Key::LShift));
        kb.push(press(Key::RCtrl));
        kb.push(press(Key::B));
        assert_eq!(
            summary(&kb.drain()),
            [
                (Key::LShift, true, MOD_SHIFT),
                (Key::A, true, MOD_SHIFT),
                (Key::LShift, false, 0),
                (Key::RCtrl, true, MOD_CTRL),
                (Key::B, true, MOD_CTRL),
            ]
        );
    }

    #[test]
    fn release_all_releases_exactly_the_held_keys() {
        let mut kb = Keyboard::default();
        kb.push(press(Key::Left));
        kb.push(press(Key::W));
        kb.drain();
        kb.push(press(Key::Space)); // queued, not yet applied
        kb.release_all();
        let released: HashSet<_> = kb
            .drain()
            .into_iter()
            .filter(|e| !e.pressed)
            .map(|e| e.key)
            .collect();
        assert_eq!(released, HashSet::from([Key::Left, Key::W, Key::Space]));
        assert!(KEYS.iter().all(|e| !kb.is_held(e.0)));
    }

    #[test]
    fn random_taps_leave_nothing_held() {
        let mut kb = Keyboard::default();
        let mut seed: u32 = 12345;
        let mut next = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            seed >> 16
        };
        for _ in 0..100 {
            let key = KEYS[next() as usize % KEYS.len()].0;
            kb.push(RawKeyEvent {
                key,
                pressed: next() % 2 == 0,
                repeat: next() % 4 == 0,
            });
            if next() % 3 == 0 {
                kb.drain();
            }
        }
        kb.release_all();
        kb.drain();
        assert!(KEYS.iter().all(|e| !kb.is_held(e.0)));
        assert_eq!(kb.modifiers(), 0);
    }
}
